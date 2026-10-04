use crate::model::contract_error;
use crate::{WidgetryFileDialogFilterId, WidgetryFileDialogSort};
use bevy::prelude::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Resource, Default)]
pub struct WidgetryFileDialogStorage {
    pub(crate) scopes: BTreeMap<String, StorageRecord>,
}

#[derive(Clone, Default)]
pub(crate) struct StorageRecord {
    pub(crate) snapshot: WidgetryFileDialogStorageSnapshot,
    pub(crate) revision: u64,
    pub(crate) saved_revision: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetryFileDialogStorageSnapshot {
    pub last_visited_dir: Option<PathBuf>,
    pub last_picked_dir: Option<PathBuf>,
    pub show_hidden: bool,
    pub show_system: bool,
    pub filter: WidgetryFileDialogFilterId,
    pub sort: WidgetryFileDialogSort,
    pub pinned: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum WidgetryFileDialogStorageState {
    #[default]
    Memory,
    Loading,
    Dirty(u64),
    Saving(u64),
    Saved(u64),
    Failed(String),
}

impl WidgetryFileDialogStorage {
    pub fn snapshot(&self, scope: &str) -> Option<&WidgetryFileDialogStorageSnapshot> {
        self.scopes.get(scope).map(|record| &record.snapshot)
    }

    pub fn revision(&self, scope: &str) -> Option<u64> {
        self.scopes.get(scope).map(|record| record.revision)
    }

    pub fn saved_revision(&self, scope: &str) -> Option<u64> {
        self.scopes.get(scope).map(|record| record.saved_revision)
    }

    pub fn import(
        &mut self,
        scope: String,
        snapshot: WidgetryFileDialogStorageSnapshot,
    ) -> Result<bool, BevyError> {
        self.validate_scope(&scope)?;
        validate_snapshot(&snapshot)?;
        let previous = self.scopes.get(&scope).cloned().unwrap_or_default();
        if previous.snapshot == snapshot && self.scopes.contains_key(&scope) {
            return Ok(false);
        }
        let revision = previous
            .revision
            .checked_add(1)
            .ok_or_else(|| contract_error("storage revision exhausted"))?;
        self.scopes.insert(
            scope,
            StorageRecord {
                snapshot,
                revision,
                saved_revision: previous.saved_revision,
            },
        );
        Ok(true)
    }

    pub fn export(&self) -> BTreeMap<String, WidgetryFileDialogStorageSnapshot> {
        self.scopes
            .iter()
            .map(|(scope, record)| (scope.clone(), record.snapshot.clone()))
            .collect()
    }

    pub fn acknowledge_saved(&mut self, scope: &str, revision: u64) -> Result<bool, BevyError> {
        let record = self
            .scopes
            .get_mut(scope)
            .ok_or_else(|| contract_error("unknown storage scope"))?;
        if revision > record.revision {
            return Err(contract_error(
                "storage acknowledgement from future revision",
            ));
        }
        if revision <= record.saved_revision {
            return Ok(false);
        }
        record.saved_revision = revision;
        Ok(true)
    }

    pub(crate) fn prepare_merge(
        &self,
        scope: &str,
        before: &WidgetryFileDialogStorageSnapshot,
        after: &WidgetryFileDialogStorageSnapshot,
        visited: bool,
        picked: bool,
        pin: Option<&(PathBuf, bool)>,
    ) -> Result<StorageRecord, BevyError> {
        self.validate_scope(scope)?;
        let mut record = self
            .scopes
            .get(scope)
            .cloned()
            .unwrap_or_else(|| StorageRecord {
                snapshot: before.clone(),
                ..Default::default()
            });
        let original = record.snapshot.clone();
        let current = &mut record.snapshot;
        if visited || before.last_visited_dir != after.last_visited_dir {
            current.last_visited_dir = after.last_visited_dir.clone();
        }
        if picked || before.last_picked_dir != after.last_picked_dir {
            current.last_picked_dir = after.last_picked_dir.clone();
        }
        if before.filter != after.filter {
            current.filter = after.filter.clone();
        }
        if before.sort != after.sort {
            current.sort = after.sort;
        }
        if before.show_hidden != after.show_hidden {
            current.show_hidden = after.show_hidden;
        }
        if before.show_system != after.show_system {
            current.show_system = after.show_system;
        }
        for path in &before.pinned {
            if !after.pinned.contains(path) {
                current.pinned.retain(|existing| existing != path);
            }
        }
        for path in &after.pinned {
            if !before.pinned.contains(path) && !current.pinned.contains(path) {
                current.pinned.push(path.clone());
            }
        }
        if let Some((path, add)) = pin {
            if *add && !current.pinned.contains(path) {
                current.pinned.push(path.clone());
            }
            if !*add {
                current.pinned.retain(|existing| existing != path);
            }
        }
        validate_snapshot(current)?;
        if original != *current || !self.scopes.contains_key(scope) {
            record.revision = record
                .revision
                .checked_add(1)
                .ok_or_else(|| contract_error("storage revision exhausted"))?;
        }
        Ok(record)
    }

    fn validate_scope(&self, scope: &str) -> Result<(), BevyError> {
        if scope.is_empty() || scope.len() > 128 {
            return Err(contract_error("storage scope must contain 1..128 bytes"));
        }
        if self.scopes.len() >= 256 && !self.scopes.contains_key(scope) {
            return Err(contract_error("storage scope capacity exceeded"));
        }
        Ok(())
    }
}

fn validate_snapshot(snapshot: &WidgetryFileDialogStorageSnapshot) -> Result<(), BevyError> {
    if snapshot.pinned.len() > 1024 {
        return Err(contract_error("pinned folder capacity exceeded"));
    }
    let mut paths = std::collections::BTreeSet::new();
    for path in &snapshot.pinned {
        if !path.is_absolute() || !paths.insert(path) {
            return Err(contract_error("pinned paths must be absolute and unique"));
        }
    }
    Ok(())
}

#[cfg(test)]
// 测试允许断言，workspace 的 macro 禁令只约束生产代码。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! storage 的 memory revision 与 saved revision 独立。
    //! stimuli 为 import/export、field delta merge 与乱序 acknowledgement。
    //! guards 为 scope/path/capacity，失败不能改变既有 snapshot 或 revision。
    use super::*;

    #[test]
    fn old_save_acknowledgement_does_not_clean_new_revision() {
        let mut store = WidgetryFileDialogStorage::default();
        let mut snapshot = WidgetryFileDialogStorageSnapshot::default();
        store.import("open".into(), snapshot.clone()).unwrap();
        let old = store.revision("open").unwrap();
        snapshot.show_hidden = true;
        store.import("open".into(), snapshot.clone()).unwrap();
        let current = store.revision("open").unwrap();
        store.acknowledge_saved("open", old).unwrap();
        assert!(store.saved_revision("open").unwrap() < current);
        store.acknowledge_saved("open", current).unwrap();
        assert!(!store.acknowledge_saved("open", old).unwrap());
        assert!(store.acknowledge_saved("open", current + 1).is_err());
        assert_eq!(store.export()["open"], snapshot);
    }

    #[test]
    fn invalid_import_keeps_previous_scope_and_revision() {
        let mut store = WidgetryFileDialogStorage::default();
        store
            .import("open".into(), WidgetryFileDialogStorageSnapshot::default())
            .unwrap();
        let previous = store.export();
        let revision = store.revision("open");
        let invalid = WidgetryFileDialogStorageSnapshot {
            pinned: vec!["relative".into()],
            ..Default::default()
        };
        assert!(store.import("open".into(), invalid).is_err());
        assert_eq!(store.export(), previous);
        assert_eq!(store.revision("open"), revision);
    }
}
