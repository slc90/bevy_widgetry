use crate::model::contract_error;
use crate::*;
use bevy::prelude::BevyError;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogSelectionJob {
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) mode: WidgetryFileDialogMode,
    pub(crate) base: Arc<BTreeSet<WidgetryFileDialogEntryId>>,
    pub(crate) active: Option<WidgetryFileDialogEntryId>,
    pub(crate) anchor: Option<WidgetryFileDialogEntryId>,
    pub(crate) intents: Vec<(Arc<WidgetryFileDialogSnapshot>, WidgetryFileDialogAction)>,
    pub(crate) latest: Arc<WidgetryFileDialogSnapshot>,
    pub(crate) repair: bool,
}

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogPreparedSelection {
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) selected: Arc<BTreeSet<WidgetryFileDialogEntryId>>,
    pub(crate) active: Option<WidgetryFileDialogEntryId>,
    pub(crate) anchor: Option<WidgetryFileDialogEntryId>,
    pub(crate) snapshot: Arc<WidgetryFileDialogSnapshot>,
    pub(crate) repair_changed_selection: bool,
}

impl WidgetryFileDialogSelectionJob {
    pub fn token(&self) -> WidgetryFileDialogToken {
        self.token
    }

    pub fn prepare(self) -> Result<WidgetryFileDialogPreparedSelection, BevyError> {
        let mut selected = self.base.clone();
        let mut active = self.active;
        let mut anchor = self.anchor;
        for (snapshot, action) in &self.intents {
            match action {
                WidgetryFileDialogAction::ClearSelection => {
                    selected = Arc::new(BTreeSet::new());
                }
                WidgetryFileDialogAction::SelectAll => {
                    selected = Arc::new(
                        snapshot
                            .visible
                            .iter()
                            .filter(|id| {
                                snapshot
                                    .entry(**id)
                                    .is_some_and(|entry| self.mode.accepts(entry.kind))
                            })
                            .copied()
                            .collect(),
                    );
                }
                WidgetryFileDialogAction::Active(id) => {
                    active = *id;
                }
                WidgetryFileDialogAction::MoveActive(movement) => {
                    let len = snapshot.visible.len();
                    if len == 0 {
                        continue;
                    }
                    let current = active.and_then(|id| snapshot.positions.get(&id).copied());
                    let index = match movement {
                        WidgetryFileDialogMove::First => 0,
                        WidgetryFileDialogMove::Last => len - 1,
                        WidgetryFileDialogMove::Next => {
                            current.map_or(0, |index| index.saturating_add(1).min(len - 1))
                        }
                        WidgetryFileDialogMove::Previous => {
                            current.map_or(len - 1, |index| index.saturating_sub(1))
                        }
                        WidgetryFileDialogMove::PageDown(size) => {
                            current.unwrap_or(0).saturating_add(*size).min(len - 1)
                        }
                        WidgetryFileDialogMove::PageUp(size) => {
                            current.unwrap_or(len - 1).saturating_sub(*size)
                        }
                    };
                    active = Some(snapshot.visible[index]);
                }
                WidgetryFileDialogAction::Select { id, operation, .. } => {
                    let index = snapshot
                        .positions
                        .get(id)
                        .copied()
                        .ok_or_else(|| contract_error("selection entry disappeared"))?;
                    active = Some(*id);
                    match operation {
                        WidgetryFileDialogSelection::Replace => {
                            selected = Arc::new(BTreeSet::from([*id]));
                            anchor = Some(*id);
                        }
                        WidgetryFileDialogSelection::Toggle => {
                            if !Arc::make_mut(&mut selected).remove(id) {
                                Arc::make_mut(&mut selected).insert(*id);
                            }
                            anchor = Some(*id);
                        }
                        WidgetryFileDialogSelection::Range
                        | WidgetryFileDialogSelection::ExtendRange => {
                            if *operation == WidgetryFileDialogSelection::Range {
                                selected = Arc::new(BTreeSet::new());
                            }
                            let start = anchor
                                .and_then(|id| snapshot.positions.get(&id).copied())
                                .unwrap_or(index);
                            for id in snapshot
                                .visible
                                .iter()
                                .skip(start.min(index))
                                .take(start.abs_diff(index) + 1)
                            {
                                if snapshot
                                    .entry(*id)
                                    .is_some_and(|entry| self.mode.accepts(entry.kind))
                                {
                                    Arc::make_mut(&mut selected).insert(*id);
                                }
                            }
                            if anchor.is_none() {
                                anchor = Some(*id);
                            }
                        }
                    }
                }
                _ => return Err(contract_error("invalid selection intent")),
            }
        }
        let selectable = |id: &WidgetryFileDialogEntryId| {
            self.latest.positions.contains_key(id)
                && self
                    .latest
                    .entry(*id)
                    .is_some_and(|entry| self.mode.accepts(entry.kind))
        };
        // pending intents 可能已通过 Selection reply 提交，再由较晚的 snapshot 重放。
        // 只记录 repair 对重放后 selection 的改变，避免将相同 selection 再次视为变化。
        let repair_changed_selection = self.repair && selected.iter().any(|id| !selectable(id));
        if repair_changed_selection {
            Arc::make_mut(&mut selected).retain(selectable);
        }
        active = active.filter(|id| self.latest.positions.contains_key(id));
        anchor = anchor.filter(|id| selectable(id));
        Ok(WidgetryFileDialogPreparedSelection {
            token: self.token,
            selected,
            active,
            anchor,
            snapshot: self.latest,
            repair_changed_selection,
        })
    }
}

impl WidgetryFileDialogState {
    pub(crate) fn select(&mut self, action: WidgetryFileDialogAction) -> Result<bool, BevyError> {
        if self.selection_job.is_none()
            && (matches!(action, WidgetryFileDialogAction::ClearSelection)
                && self.selected.is_empty()
                || matches!(action, WidgetryFileDialogAction::Active(None))
                    && self.active.is_none())
        {
            return Ok(false);
        }
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| contract_error("selection has no snapshot"))?;
        if self.projection_pending {
            return Err(contract_error("projection is pending"));
        }
        match &action {
            WidgetryFileDialogAction::Select {
                id,
                token,
                operation,
            } => {
                if !self.matches_query(*token) {
                    return Err(contract_error("stale selection view"));
                }
                let entry = snapshot
                    .entry(*id)
                    .filter(|_| snapshot.positions.contains_key(id))
                    .ok_or_else(|| contract_error("stale EntryId"))?;
                if !self.mode.accepts(entry.kind) {
                    return Err(contract_error("entry kind incompatible with mode"));
                }
                if !self.mode.is_multiple() && *operation != WidgetryFileDialogSelection::Replace {
                    return Err(contract_error("multiple selection requires multiple mode"));
                }
                if self.selection_job.is_none()
                    && *operation == WidgetryFileDialogSelection::Replace
                    && self.selected.len() == 1
                    && self.selected.contains(id)
                    && self.active == Some(*id)
                    && self.anchor == Some(*id)
                {
                    return Ok(false);
                }
            }
            WidgetryFileDialogAction::SelectAll if !self.mode.is_multiple() => {
                return Err(contract_error("SelectAll requires multiple mode"));
            }
            WidgetryFileDialogAction::SelectAll
                if self.selection_job.is_none()
                    && self.selected.len() == snapshot.selectable_count(self.mode) =>
            {
                return Ok(false);
            }
            WidgetryFileDialogAction::MoveActive(_) if snapshot.visible.is_empty() => {
                return Ok(false);
            }
            WidgetryFileDialogAction::Active(Some(id)) if !snapshot.positions.contains_key(id) => {
                return Err(contract_error("stale active EntryId"));
            }
            WidgetryFileDialogAction::Active(id)
                if self.selection_job.is_none() && self.active == *id =>
            {
                return Ok(false);
            }
            WidgetryFileDialogAction::ClearSelection
                if self.selection_job.is_none() && self.selected.is_empty() =>
            {
                return Ok(false);
            }
            _ => {}
        }
        if self
            .selection_job
            .as_ref()
            .is_some_and(|job| job.intents.len() >= 128)
        {
            return Err(contract_error("selection intent capacity exceeded"));
        }
        let revision = self
            .token
            .selection_revision
            .checked_add(1)
            .ok_or_else(|| contract_error("selection revision exhausted"))?;
        let mut token = self.token;
        token.selection_revision = revision;
        let job = self
            .selection_job
            .get_or_insert_with(|| WidgetryFileDialogSelectionJob {
                token,
                mode: self.mode,
                base: self.selected.clone(),
                active: self.active,
                anchor: self.anchor,
                intents: Vec::new(),
                latest: snapshot.clone(),
                repair: false,
            });
        job.token = token;
        job.intents.push((snapshot.clone(), action));
        self.token = token;
        self.confirmation = WidgetryFileDialogConfirmation::Idle;
        self.candidate = None;
        self.validation_job = None;
        self.activate_after_selection = false;
        Ok(true)
    }
}

impl WidgetryFileDialogState {
    pub fn projection_selection_job(
        &self,
        snapshot: Arc<WidgetryFileDialogSnapshot>,
    ) -> Result<WidgetryFileDialogSelectionJob, BevyError> {
        if !self.matches_query(snapshot.token()) {
            return Err(contract_error("stale projection snapshot"));
        }
        let mut job =
            self.selection_job
                .clone()
                .unwrap_or_else(|| WidgetryFileDialogSelectionJob {
                    token: self.token,
                    mode: self.mode,
                    base: self.selected.clone(),
                    active: self.active,
                    anchor: self.anchor,
                    intents: Vec::new(),
                    latest: snapshot.clone(),
                    repair: true,
                });
        job.latest = snapshot;
        job.repair = true;
        job.token = self.token;
        Ok(job)
    }
}

#[cfg(test)]
// 测试允许断言，workspace 的 macro 禁令只约束生产代码。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! active navigation 与 selection 是独立 state 维度。
    //! 已存在 bulk selection 时，active 更新不能复制完整 selection，保护工作量 invariant。
    use super::*;

    #[test]
    fn active_navigation_shares_unchanged_bulk_selection() {
        let mut state = WidgetryFileDialogState::new(WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickFiles,
            ..Default::default()
        })
        .unwrap();
        let path = std::path::PathBuf::from("C:/fixture");
        let snapshot = Arc::new(
            WidgetryFileDialogSnapshot::prepare(
                state.token(),
                path.clone(),
                (0..100)
                    .map(|index| {
                        let name = format!("{index}.txt");
                        WidgetryFileDialogEntryData {
                            path: path.join(&name),
                            name: name.into(),
                            kind: WidgetryFileDialogEntryKind::File,
                            size: None,
                            modified: None,
                            hidden: None,
                            system: None,
                        }
                    })
                    .collect(),
                &state.query(),
                None,
            )
            .unwrap(),
        );
        state
            .receive(WidgetryFileDialogReply::Started {
                token: state.token(),
                path,
            })
            .unwrap();
        let selection = state
            .projection_selection_job(snapshot.clone())
            .unwrap()
            .prepare()
            .unwrap();
        state
            .receive(WidgetryFileDialogReply::Snapshot {
                snapshot,
                state: WidgetryFileDialogDirectoryState::Ready,
                selection,
            })
            .unwrap();
        state.act(WidgetryFileDialogAction::SelectAll).unwrap();
        let selection = state.selection_job().unwrap().prepare().unwrap();
        state
            .receive(WidgetryFileDialogReply::Selection(selection))
            .unwrap();
        let base = state.selected.clone();
        state
            .act(WidgetryFileDialogAction::MoveActive(
                WidgetryFileDialogMove::Next,
            ))
            .unwrap();
        let prepared = state.selection_job().unwrap().prepare().unwrap();
        assert!(Arc::ptr_eq(&base, &prepared.selected));
        assert!(!prepared.repair_changed_selection);
    }
}
