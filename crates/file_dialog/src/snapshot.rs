const MAX_SNAPSHOT_BYTES: usize = 128 * 1024 * 1024;
const ESTIMATED_PATH_BYTES_PER_UNIT: usize = 6;
const ESTIMATED_NAME_BYTES_PER_UNIT: usize = 2;
const ESTIMATED_ENTRY_OVERHEAD: usize = 512;

use crate::error::directory_failure;
use crate::{FileDialogDirectoryContext, FileDialogDirectoryFault, FileDialogFailure};
use crate::{
    WidgetryFileDialogEntry, WidgetryFileDialogEntryId, WidgetryFileDialogEntryKind,
    WidgetryFileDialogFilter, WidgetryFileDialogSort, WidgetryFileDialogToken,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

static NEXT_ENTRY: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidgetryFileDialogEntryData {
    pub path: PathBuf,
    pub name: OsString,
    pub kind: WidgetryFileDialogEntryKind,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub hidden: Option<bool>,
    pub system: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogQuery {
    pub filter: WidgetryFileDialogFilter,
    pub search: String,
    pub sort: WidgetryFileDialogSort,
    pub show_hidden: bool,
    pub show_system: bool,
}

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogSnapshot {
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) path: PathBuf,
    pub(crate) entries: Arc<im::Vector<WidgetryFileDialogEntry>>,
    pub(crate) indices: Arc<im::OrdMap<WidgetryFileDialogEntryId, usize>>,
    pub(crate) visible: Arc<im::Vector<WidgetryFileDialogEntryId>>,
    pub(crate) positions: im::OrdMap<WidgetryFileDialogEntryId, usize>,
    pub(crate) paths: Arc<im::OrdMap<PathBuf, WidgetryFileDialogEntryId>>,
    file_count: usize,
    directory_count: usize,
    estimated_bytes: usize,
}

fn project_entries(
    entries: &im::Vector<WidgetryFileDialogEntry>,
    query: &WidgetryFileDialogQuery,
) -> (
    Arc<im::Vector<WidgetryFileDialogEntryId>>,
    im::OrdMap<WidgetryFileDialogEntryId, usize>,
    usize,
    usize,
) {
    let search = query.search.to_lowercase();
    let mut order: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            (query.show_hidden || entry.hidden != Some(true))
                && (query.show_system || entry.system != Some(true))
                && (entry.kind.is_directory() || query.filter.matches(&entry.name))
                && (search.is_empty()
                    || entry
                        .name
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&search))
        })
        .map(|(index, _)| index)
        .collect();
    order.sort_by(|left, right| {
        let left = &entries[*left];
        let right = &entries[*right];
        right
            .kind
            .is_directory()
            .cmp(&left.kind.is_directory())
            .then_with(|| {
                match query.sort {
                    WidgetryFileDialogSort::NameAscending => left.name.cmp(&right.name),
                    WidgetryFileDialogSort::NameDescending => right.name.cmp(&left.name),
                    WidgetryFileDialogSort::SizeAscending => left.size.cmp(&right.size),
                    WidgetryFileDialogSort::ModifiedDescending => {
                        right.modified.cmp(&left.modified)
                    }
                }
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.path.cmp(&right.path))
            })
    });
    let file_count = order
        .iter()
        .filter(|index| entries[**index].kind.is_file())
        .count();
    let directory_count = order
        .iter()
        .filter(|index| entries[**index].kind.is_directory())
        .count();
    let visible: Arc<im::Vector<_>> =
        Arc::new(order.into_iter().map(|index| entries[index].id).collect());
    let positions = visible
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    (visible, positions, file_count, directory_count)
}

impl WidgetryFileDialogSnapshot {
    pub(crate) fn append_batch(
        &self,
        data: Vec<WidgetryFileDialogEntryData>,
        query: &WidgetryFileDialogQuery,
    ) -> Result<Self, FileDialogFailure> {
        let mut next = self.clone();
        for item in data {
            if next.paths.contains_key(&item.path) {
                continue;
            }
            if next.entries.len() >= crate::limits::MAX_DIRECTORY_ENTRIES {
                return Err(directory_failure(
                    FileDialogDirectoryContext::snapshot(self.token, self.path.clone()),
                    FileDialogDirectoryFault::Capacity {
                        actual: next.entries.len() + 1,
                        limit: crate::limits::MAX_DIRECTORY_ENTRIES,
                    },
                ));
            }
            next.estimated_bytes = next
                .estimated_bytes
                .saturating_add(
                    item.path
                        .as_os_str()
                        .len()
                        .saturating_mul(ESTIMATED_PATH_BYTES_PER_UNIT),
                )
                .saturating_add(
                    item.name
                        .len()
                        .saturating_mul(ESTIMATED_NAME_BYTES_PER_UNIT),
                )
                .saturating_add(ESTIMATED_ENTRY_OVERHEAD);
            if next.estimated_bytes > MAX_SNAPSHOT_BYTES {
                return Err(directory_failure(
                    FileDialogDirectoryContext::snapshot(self.token, self.path.clone()),
                    FileDialogDirectoryFault::ByteCapacity {
                        actual: next.estimated_bytes,
                        limit: MAX_SNAPSHOT_BYTES,
                    },
                ));
            }
            let prepared = Self::prepare(self.token, self.path.clone(), vec![item], query, None)?;
            let entry = prepared.entries[0].clone();
            Arc::make_mut(&mut next.indices).insert(entry.id, next.entries.len());
            Arc::make_mut(&mut next.paths).insert(entry.path.clone(), entry.id);
            if !prepared.visible.is_empty() {
                next.positions.insert(entry.id, next.visible.len());
                Arc::make_mut(&mut next.visible).push_back(entry.id);
                next.file_count += usize::from(entry.kind.is_file());
                next.directory_count += usize::from(entry.kind.is_directory());
            }
            Arc::make_mut(&mut next.entries).push_back(entry);
        }
        Ok(next)
    }

    pub fn reproject(
        &self,
        token: WidgetryFileDialogToken,
        query: &WidgetryFileDialogQuery,
    ) -> Result<Self, FileDialogFailure> {
        if token.session != self.token.session || token.generation != self.token.generation {
            return Err(directory_failure(
                FileDialogDirectoryContext::snapshot(token, self.path.clone()),
                FileDialogDirectoryFault::Generation {
                    requested: token,
                    current: self.token,
                },
            ));
        }
        let (visible, positions, file_count, directory_count) =
            project_entries(&self.entries, query);
        Ok(Self {
            token,
            path: self.path.clone(),
            entries: self.entries.clone(),
            indices: self.indices.clone(),
            paths: self.paths.clone(),
            visible,
            positions,
            file_count,
            directory_count,
            estimated_bytes: self.estimated_bytes,
        })
    }

    pub fn prepare(
        token: WidgetryFileDialogToken,
        path: PathBuf,
        data: Vec<WidgetryFileDialogEntryData>,
        query: &WidgetryFileDialogQuery,
        previous: Option<&Self>,
    ) -> Result<Self, FileDialogFailure> {
        if data.len() > crate::limits::MAX_DIRECTORY_ENTRIES {
            return Err(directory_failure(
                FileDialogDirectoryContext::snapshot(token, path.clone()),
                FileDialogDirectoryFault::Capacity {
                    actual: data.len(),
                    limit: crate::limits::MAX_DIRECTORY_ENTRIES,
                },
            ));
        }
        let previous_ids: BTreeMap<_, _> = previous
            .into_iter()
            .filter(|snapshot| snapshot.token.session == token.session && snapshot.path == path)
            .flat_map(|snapshot| snapshot.entries.iter().map(|entry| (&entry.path, entry.id)))
            .collect();
        let mut seen = BTreeSet::new();
        let mut estimated_bytes = 0usize;
        let mut entries = im::Vector::new();
        for data in data {
            estimated_bytes = estimated_bytes
                .saturating_add(
                    data.path
                        .as_os_str()
                        .len()
                        .saturating_mul(ESTIMATED_PATH_BYTES_PER_UNIT),
                )
                .saturating_add(
                    data.name
                        .len()
                        .saturating_mul(ESTIMATED_NAME_BYTES_PER_UNIT),
                )
                .saturating_add(ESTIMATED_ENTRY_OVERHEAD);
            if estimated_bytes > MAX_SNAPSHOT_BYTES {
                return Err(directory_failure(
                    FileDialogDirectoryContext::snapshot(token, path.clone()),
                    FileDialogDirectoryFault::ByteCapacity {
                        actual: estimated_bytes,
                        limit: MAX_SNAPSHOT_BYTES,
                    },
                ));
            }
            if data.path.parent() != Some(path.as_path())
                || data.path.file_name() != Some(data.name.as_os_str())
            {
                return Err(directory_failure(
                    FileDialogDirectoryContext::snapshot(token, path.clone()),
                    FileDialogDirectoryFault::EntryPath {
                        path: data.path,
                        name: data.name,
                    },
                ));
            }
            if !seen.insert(data.path.clone()) {
                return Err(directory_failure(
                    FileDialogDirectoryContext::snapshot(token, path.clone()),
                    FileDialogDirectoryFault::DuplicatePath { path: data.path },
                ));
            }
            let id = if let Some(id) = previous_ids.get(&data.path) {
                *id
            } else {
                let serial = NEXT_ENTRY
                    .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                        next.checked_add(1)
                    })
                    .map_err(|next| {
                        directory_failure(
                            FileDialogDirectoryContext::snapshot(token, path.clone()),
                            FileDialogDirectoryFault::EntryIdExhausted { next },
                        )
                    })?;
                WidgetryFileDialogEntryId {
                    session: token.session,
                    serial,
                }
            };
            entries.push_back(WidgetryFileDialogEntry {
                id,
                path: data.path,
                name: data.name,
                kind: data.kind,
                size: data.size,
                modified: data.modified,
                hidden: data.hidden,
                system: data.system,
            });
        }
        let (visible, positions, file_count, directory_count) = project_entries(&entries, query);
        let indices = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.id, index))
            .collect::<im::OrdMap<_, _>>();
        let paths = entries
            .iter()
            .map(|entry| (entry.path.clone(), entry.id))
            .collect::<im::OrdMap<_, _>>();
        Ok(Self {
            token,
            path,
            entries: Arc::new(entries),
            indices: Arc::new(indices),
            visible,
            positions,
            paths: Arc::new(paths),
            file_count,
            directory_count,
            estimated_bytes,
        })
    }

    pub fn token(&self) -> WidgetryFileDialogToken {
        self.token
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn entries(&self) -> &im::Vector<WidgetryFileDialogEntry> {
        &self.entries
    }

    pub fn visible(&self) -> &im::Vector<WidgetryFileDialogEntryId> {
        &self.visible
    }

    pub fn entry(&self, id: WidgetryFileDialogEntryId) -> Option<&WidgetryFileDialogEntry> {
        self.indices
            .get(&id)
            .and_then(|index| self.entries.get(*index))
    }

    pub(crate) fn selectable_count(&self, mode: crate::WidgetryFileDialogMode) -> usize {
        match mode {
            crate::WidgetryFileDialogMode::PickFile | crate::WidgetryFileDialogMode::PickFiles => {
                self.file_count
            }
            crate::WidgetryFileDialogMode::PickDirectory
            | crate::WidgetryFileDialogMode::PickDirectories => self.directory_count,
            crate::WidgetryFileDialogMode::SaveFile => 0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {

    use super::*;
    use crate::{WidgetryFileDialogProps, WidgetryFileDialogState};
    use std::os::windows::ffi::OsStringExt;

    fn data(directory: &str, name: &str) -> WidgetryFileDialogEntryData {
        WidgetryFileDialogEntryData {
            path: PathBuf::from(directory).join(name),
            name: name.into(),
            kind: WidgetryFileDialogEntryKind::File,
            size: None,
            modified: None,
            hidden: None,
            system: None,
        }
    }

    #[test]
    fn streaming_keeps_arrival_order_identity_and_shared_previous_snapshot() {
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let empty = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/one".into(),
            vec![],
            &state.query(),
            None,
        )
        .unwrap();
        let first = empty
            .append_batch(vec![data("C:/one", "z.txt")], &state.query())
            .unwrap();
        let second = first
            .append_batch(
                vec![data("C:/one", "a.txt"), data("C:/one", "z.txt")],
                &state.query(),
            )
            .unwrap();
        assert_eq!(first.entries().len(), 1);
        assert_eq!(second.entries().len(), 2);
        assert_eq!(second.visible()[0], first.visible()[0]);
        assert_eq!(second.entry(second.visible()[0]).unwrap().name(), "z.txt");
        let sorted = second.reproject(state.token(), &state.query()).unwrap();
        assert_eq!(sorted.entry(sorted.visible()[0]).unwrap().name(), "a.txt");
        assert_eq!(sorted.entries()[0].id(), first.entries()[0].id());
    }

    #[test]
    fn entry_ids_do_not_reuse_across_directories_in_one_session() {
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let first = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/one".into(),
            vec![data("C:/one", "a")],
            &state.query(),
            None,
        )
        .unwrap();
        let second = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/two".into(),
            vec![data("C:/two", "b")],
            &state.query(),
            None,
        )
        .unwrap();
        assert_ne!(first.entries()[0].id(), second.entries()[0].id());
    }

    #[test]
    fn reproject_preserves_ids_and_filter_keeps_directories() {
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let mut folder = data("C:/one", "folder");
        folder.kind = WidgetryFileDialogEntryKind::SymlinkDirectory;
        let input = vec![data("C:/one", "a.TXT"), data("C:/one", "b.png"), folder];
        let first = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/one".into(),
            input.clone(),
            &state.query(),
            None,
        )
        .unwrap();
        let mut query = state.query();
        query.sort = WidgetryFileDialogSort::NameDescending;
        query.filter.suffixes = vec!["txt".into()];
        let second = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/one".into(),
            input,
            &query,
            Some(&first),
        )
        .unwrap();
        assert_eq!(first.entries()[0].id(), second.entries()[0].id());
        assert_eq!(second.entry(second.visible()[0]).unwrap().name(), "folder");
        assert_eq!(second.visible().len(), 2);
    }

    #[test]
    fn cached_reprojection_shares_entries_and_preserves_metadata_and_identity() {
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let first = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/one".into(),
            vec![data("C:/one", "a.txt"), data("C:/one", "b.png")],
            &state.query(),
            None,
        )
        .unwrap();
        let mut token = state.token();
        token.query_revision += 1;
        let mut query = state.query();
        query.filter.suffixes = vec!["txt".into()];
        let second = first.reproject(token, &query).unwrap();
        assert!(Arc::ptr_eq(&first.entries, &second.entries));
        assert_eq!(second.visible().len(), 1);
        assert_eq!(second.visible()[0], first.visible()[0]);
    }

    #[test]
    fn attributes_search_invalid_data_and_unknown_kind_are_distinct() {
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let mut hidden = data("C:/one", "hidden");
        hidden.hidden = Some(true);
        let mut system = data("C:/one", "system");
        system.system = Some(true);
        let mut unknown = data("C:/one", "unknown");
        unknown.kind = WidgetryFileDialogEntryKind::Unknown;
        let inputs = vec![hidden, system, unknown];
        let first = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/one".into(),
            inputs.clone(),
            &state.query(),
            None,
        )
        .unwrap();
        assert_eq!(first.visible().len(), 1);
        assert!(!first.entry(first.visible()[0]).unwrap().kind().is_file());
        let mut query = state.query();
        query.show_hidden = true;
        query.show_system = true;
        query.search = "HID".into();
        let second = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/one".into(),
            inputs,
            &query,
            Some(&first),
        )
        .unwrap();
        assert_eq!(second.visible().len(), 1);
        assert_eq!(second.entry(second.visible()[0]).unwrap().name(), "hidden");
        let duplicate = vec![data("C:/one", "a"), data("C:/one", "a")];
        assert!(
            WidgetryFileDialogSnapshot::prepare(
                state.token(),
                "C:/one".into(),
                duplicate,
                &query,
                None
            )
            .is_err()
        );
        assert!(
            WidgetryFileDialogSnapshot::prepare(
                state.token(),
                "C:/one".into(),
                vec![data("C:/other", "a")],
                &query,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn non_unicode_names_with_identical_display_keep_distinct_paths() {
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default()).unwrap();
        let left = OsString::from_wide(&[0xd800, 46, 116, 120, 116]);
        let right = OsString::from_wide(&[0xdc00, 46, 116, 120, 116]);
        assert_eq!(left.to_string_lossy(), right.to_string_lossy());
        let mut a = data("C:/one", "a");
        a.name = left.clone();
        a.path = PathBuf::from("C:/one").join(&left);
        let mut b = data("C:/one", "b");
        b.name = right.clone();
        b.path = PathBuf::from("C:/one").join(&right);
        let mut query = state.query();
        query.filter.suffixes = vec!["txt".into()];
        let snapshot = WidgetryFileDialogSnapshot::prepare(
            state.token(),
            "C:/one".into(),
            vec![a, b],
            &query,
            None,
        )
        .unwrap();
        assert_eq!(snapshot.visible().len(), 2);
        assert_ne!(snapshot.entries()[0].id(), snapshot.entries()[1].id());
        assert_ne!(snapshot.entries()[0].path(), snapshot.entries()[1].path());
        assert_eq!(snapshot.entries()[0].name(), left);
    }
}
