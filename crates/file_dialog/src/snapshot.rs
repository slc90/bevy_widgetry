use crate::model::contract_error;
use crate::{
    WidgetryFileDialogEntry, WidgetryFileDialogEntryId, WidgetryFileDialogEntryKind,
    WidgetryFileDialogFilter, WidgetryFileDialogSort, WidgetryFileDialogToken,
};
use bevy::prelude::BevyError;
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

#[derive(Debug)]
pub struct WidgetryFileDialogSnapshot {
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) path: PathBuf,
    pub(crate) entries: Arc<[WidgetryFileDialogEntry]>,
    pub(crate) indices: Arc<BTreeMap<WidgetryFileDialogEntryId, usize>>,
    pub(crate) visible: Arc<[WidgetryFileDialogEntryId]>,
    pub(crate) positions: BTreeMap<WidgetryFileDialogEntryId, usize>,
    pub(crate) paths: Arc<BTreeMap<PathBuf, WidgetryFileDialogEntryId>>,
    file_count: usize,
    directory_count: usize,
}

fn project_entries(
    entries: &[WidgetryFileDialogEntry],
    query: &WidgetryFileDialogQuery,
) -> (
    Arc<[WidgetryFileDialogEntryId]>,
    BTreeMap<WidgetryFileDialogEntryId, usize>,
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
    let visible: Arc<[_]> = order.into_iter().map(|index| entries[index].id).collect();
    let positions = visible
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    (visible, positions, file_count, directory_count)
}

impl WidgetryFileDialogSnapshot {
    pub fn reproject(
        &self,
        token: WidgetryFileDialogToken,
        query: &WidgetryFileDialogQuery,
    ) -> Result<Self, BevyError> {
        if token.session != self.token.session || token.generation != self.token.generation {
            return Err(contract_error(
                "cached snapshot belongs to another directory generation",
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
        })
    }

    pub fn prepare(
        token: WidgetryFileDialogToken,
        path: PathBuf,
        data: Vec<WidgetryFileDialogEntryData>,
        query: &WidgetryFileDialogQuery,
        previous: Option<&Self>,
    ) -> Result<Self, BevyError> {
        if data.len() > 100_000 {
            return Err(contract_error("directory snapshot capacity exceeded"));
        }
        let previous_ids: BTreeMap<_, _> = previous
            .into_iter()
            .filter(|snapshot| snapshot.token.session == token.session && snapshot.path == path)
            .flat_map(|snapshot| snapshot.entries.iter().map(|entry| (&entry.path, entry.id)))
            .collect();
        let mut seen = BTreeSet::new();
        let mut entries = Vec::with_capacity(data.len());
        for data in data {
            if data.path.parent() != Some(path.as_path())
                || data.path.file_name() != Some(data.name.as_os_str())
            {
                return Err(contract_error(
                    "entry path/name does not belong to snapshot directory",
                ));
            }
            if !seen.insert(data.path.clone()) {
                return Err(contract_error("duplicate entry path"));
            }
            let id = if let Some(id) = previous_ids.get(&data.path) {
                *id
            } else {
                let serial = NEXT_ENTRY
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                        next.checked_add(1)
                    })
                    .map_err(|_| contract_error("entry ID exhausted"))?;
                WidgetryFileDialogEntryId {
                    session: token.session,
                    serial,
                }
            };
            entries.push(WidgetryFileDialogEntry {
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
            .collect::<BTreeMap<_, _>>();
        let paths = entries
            .iter()
            .map(|entry| (entry.path.clone(), entry.id))
            .collect::<BTreeMap<_, _>>();
        Ok(Self {
            token,
            path,
            entries: entries.into(),
            indices: Arc::new(indices),
            visible,
            positions,
            paths: Arc::new(paths),
            file_count,
            directory_count,
        })
    }

    pub fn token(&self) -> WidgetryFileDialogToken {
        self.token
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn entries(&self) -> &[WidgetryFileDialogEntry] {
        &self.entries
    }

    pub fn visible(&self) -> &[WidgetryFileDialogEntryId] {
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
// 测试允许断言，workspace 的 macro 禁令只约束生产代码。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! snapshot 的原始 path/name、kind、属性和 query projection 是独立 state 维度。
    //! stimuli 为 prepare/reprepare，guard 为同目录、唯一 path 与 session identity。
    //! invariant 为 ID 不复用、sorting 不改变 identity、lossy display 不反向生成路径。

    use super::*;
    use crate::{WidgetryFileDialogProps, WidgetryFileDialogState};
    #[cfg(windows)]
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

    #[cfg(windows)]
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
