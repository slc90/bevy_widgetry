use crate::model::contract_error;
use crate::*;
use bevy::prelude::BevyError;
use std::sync::Arc;

fn increment(value: u64) -> Result<u64, BevyError> {
    value
        .checked_add(1)
        .ok_or_else(|| contract_error("revision exhausted"))
}

impl WidgetryFileDialogState {
    pub(crate) fn act(&mut self, action: WidgetryFileDialogAction) -> Result<bool, BevyError> {
        if self.session_state != WidgetryFileDialogSessionState::Open {
            if action == WidgetryFileDialogAction::Reopen {
                let props = WidgetryFileDialogProps {
                    mode: self.mode,
                    initial_directory: self.preferences.last_visited_dir.clone(),
                    fallback_directory: self.fallback.clone(),
                    storage_scope: self.storage_scope.clone(),
                    filters: self.filters.clone(),
                    allow_different_extension: self.allow_different_extension,
                };
                let mut next = Self::new(props)?;
                next.preferences = self.preferences.clone();
                *self = next;
                return Ok(true);
            }
            return if matches!(
                action,
                WidgetryFileDialogAction::Cancel | WidgetryFileDialogAction::Confirm
            ) {
                Ok(false)
            } else {
                Err(contract_error("session is resolved"))
            };
        }
        match action {
            WidgetryFileDialogAction::Reopen => Err(contract_error("session is already open")),
            WidgetryFileDialogAction::Cancel => {
                Ok(self.resolve(WidgetryFileDialogResult::Cancelled))
            }
            WidgetryFileDialogAction::Navigate(navigation) => self.navigate(navigation),
            action @ (WidgetryFileDialogAction::Select { .. }
            | WidgetryFileDialogAction::SelectAll
            | WidgetryFileDialogAction::ClearSelection
            | WidgetryFileDialogAction::Active(_)) => self.select(action),
            action @ (WidgetryFileDialogAction::Filter(_)
            | WidgetryFileDialogAction::Search(_)
            | WidgetryFileDialogAction::Sort(_)
            | WidgetryFileDialogAction::ShowHidden(_)
            | WidgetryFileDialogAction::ShowSystem(_)) => self.change_query(action),
            WidgetryFileDialogAction::Filename(filename) => self.set_filename(filename),
            WidgetryFileDialogAction::Confirm => self.confirm(),
            WidgetryFileDialogAction::Overwrite { token, accept } => self.overwrite(token, accept),
            WidgetryFileDialogAction::Activate { id, token } => {
                if !self.matches_query(token) || self.projection_pending {
                    return Err(contract_error("stale activation view"));
                }
                let entry = self
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| {
                        snapshot
                            .entry(id)
                            .filter(|_| snapshot.positions.contains_key(&id))
                    })
                    .ok_or_else(|| contract_error("stale activation EntryId"))?;
                if entry.kind.is_directory() {
                    return self.navigate(WidgetryFileDialogNavigation::Path(entry.path.clone()));
                }
                if self.mode == WidgetryFileDialogMode::SaveFile && entry.kind.is_file() {
                    let name = entry.name.clone();
                    self.set_filename(name)?;
                    return self.confirm();
                }
                if !self.mode.accepts(entry.kind) {
                    return Err(contract_error("activation kind incompatible with mode"));
                }
                if self.selected.contains(&id) && self.selection_job.is_none() {
                    return self.confirm();
                }
                self.select(WidgetryFileDialogAction::Select {
                    id,
                    token,
                    operation: WidgetryFileDialogSelection::Replace,
                })?;
                self.activate_after_selection = true;
                Ok(true)
            }
            WidgetryFileDialogAction::Pin(path) => {
                if !path.is_absolute() {
                    return Err(contract_error("pinned path must be absolute"));
                }
                if self.preferences.pinned.contains(&path) {
                    return Ok(false);
                }
                if self.preferences.pinned.len() >= 1024 {
                    return Err(contract_error("pinned folder capacity exceeded"));
                }
                self.preferences.pinned.push(path);
                Ok(true)
            }
            WidgetryFileDialogAction::Unpin(path) => {
                if !path.is_absolute() {
                    return Err(contract_error("pinned path must be absolute"));
                }
                let before = self.preferences.pinned.len();
                self.preferences.pinned.retain(|existing| existing != &path);
                Ok(before != self.preferences.pinned.len())
            }
            WidgetryFileDialogAction::NewFolder(name) => {
                if self.folder_request.is_some() {
                    return Err(contract_error("folder creation already pending"));
                }
                if self.started_generation != Some(self.token.generation) {
                    return Err(contract_error("folder creation has no validated directory"));
                }
                crate::confirmation::valid_name(&name).map_err(|error| contract_error(&error))?;
                let path = self
                    .current_path
                    .as_ref()
                    .ok_or_else(|| contract_error("folder creation has no current directory"))?
                    .join(name);
                self.token.edit_revision = increment(self.token.edit_revision)?;
                self.folder_request = Some(WidgetryFileDialogFolderRequest {
                    token: self.token,
                    path,
                });
                self.confirmation = WidgetryFileDialogConfirmation::Idle;
                self.candidate = None;
                self.validation_job = None;
                Ok(true)
            }
            action @ WidgetryFileDialogAction::MoveActive(_) => self.select(action),
        }
    }

    pub(crate) fn receive(&mut self, reply: WidgetryFileDialogReply) -> Result<bool, BevyError> {
        if self.session_state != WidgetryFileDialogSessionState::Open {
            return Ok(false);
        }
        match reply {
            WidgetryFileDialogReply::Locations { token, outcome } => {
                if token.session != self.token.session {
                    return Ok(false);
                }
                match outcome {
                    Ok(locations) => {
                        self.locations = locations.into();
                        self.location_error = None;
                    }
                    Err(error) => self.location_error = Some(error),
                }
                Ok(true)
            }
            WidgetryFileDialogReply::Started { token, path } => {
                if !self.matches_directory(token) {
                    return Ok(false);
                }
                if self.started_generation == Some(token.generation) {
                    return Ok(false);
                }
                match self.navigation {
                    WidgetryFileDialogNavigation::Back => {
                        self.history_cursor =
                            self.history_cursor.and_then(|cursor| cursor.checked_sub(1));
                    }
                    WidgetryFileDialogNavigation::Forward => {
                        self.history_cursor = self.history_cursor.map(|cursor| cursor + 1);
                    }
                    WidgetryFileDialogNavigation::Refresh if self.history_cursor.is_some() => {}
                    _ => {
                        if let Some(cursor) = self.history_cursor {
                            Arc::make_mut(&mut self.history).truncate(cursor + 1);
                        }
                        if self.history.last() != Some(&path) {
                            Arc::make_mut(&mut self.history).push(path.clone());
                        }
                        self.history_cursor = self.history.len().checked_sub(1);
                    }
                }
                self.current_path = Some(path.clone());
                self.requested_path = Some(path.clone());
                self.preferences.last_visited_dir = Some(path);
                self.started_generation = Some(token.generation);
                Ok(true)
            }
            WidgetryFileDialogReply::Snapshot {
                snapshot,
                state,
                selection,
            } => {
                if !self.matches_query(snapshot.token) {
                    return Ok(false);
                }
                if selection.token.selection_revision != self.token.selection_revision
                    || !self.matches_query(selection.token)
                {
                    return Ok(false);
                }
                if !Arc::ptr_eq(&snapshot, &selection.snapshot) {
                    return Err(contract_error("projection and selection snapshots differ"));
                }
                if self.current_path.as_ref() != Some(&snapshot.path) {
                    return Err(contract_error("snapshot arrived before directory Started"));
                }
                // repair 改变 pending intents 的结果时也推进 revision，使其它已准备的 reply 失效。
                // 否则后续 snapshot 重放同一 repair 会再次取消已经基于新 selection 发起的确认。
                if selection.repair_changed_selection {
                    self.token.selection_revision = increment(self.token.selection_revision)?;
                    self.confirmation = WidgetryFileDialogConfirmation::Idle;
                    self.validation_job = None;
                    self.candidate = None;
                }
                if self
                    .snapshot
                    .as_ref()
                    .is_some_and(|old| Arc::ptr_eq(old, &snapshot))
                    && self.directory_state == state
                    && !self.selection_pending()
                {
                    return Ok(false);
                }
                self.selected = selection.selected;
                self.active = selection.active;
                self.anchor = selection.anchor;
                self.selection_job = None;
                if let Some(path) = &self.reveal_path
                    && let Some(id) = snapshot.paths.get(path)
                    && snapshot.positions.contains_key(id)
                {
                    self.active = Some(*id);
                    self.reveal_path = None;
                }
                self.entries = snapshot.entries.clone();
                self.visible = snapshot.visible.clone();
                self.snapshot = Some(snapshot);
                self.directory_state = state;
                self.projection_pending = false;
                if self.activate_after_selection {
                    self.activate_after_selection = false;
                    if !self.selected.is_empty()
                        && !matches!(
                            self.directory_state,
                            WidgetryFileDialogDirectoryState::Failed(_)
                        )
                    {
                        self.confirm()?;
                    }
                }
                Ok(true)
            }
            WidgetryFileDialogReply::FolderCreated { request, outcome } => {
                if !self.matches_directory(request.token)
                    || !self.folder_request.as_ref().is_some_and(|pending| {
                        pending.token == request.token && pending.path == request.path
                    })
                {
                    return Ok(false);
                }
                self.folder_request = None;
                match outcome {
                    Ok(()) => {
                        self.navigate(WidgetryFileDialogNavigation::Refresh)?;
                        self.reveal_path = Some(request.path);
                    }
                    Err(error) => self.error = Some(error),
                }
                Ok(true)
            }
            WidgetryFileDialogReply::Failed { token, error } => {
                if !self.matches_directory(token) {
                    return Ok(false);
                }
                self.directory_state = WidgetryFileDialogDirectoryState::Failed(error);
                Ok(true)
            }
            WidgetryFileDialogReply::Validated { candidate, exists } => {
                self.validated(candidate, exists)
            }
            WidgetryFileDialogReply::ValidationFailed { token, error } => {
                if token != self.token
                    || self.confirmation != WidgetryFileDialogConfirmation::Validating
                {
                    return Ok(false);
                }
                self.confirmation = WidgetryFileDialogConfirmation::Idle;
                self.validation_job = None;
                self.error = Some(error);
                Ok(true)
            }
            WidgetryFileDialogReply::Selection(prepared) => {
                if !self.matches_query(prepared.token)
                    || self.token.selection_revision != prepared.token.selection_revision
                    || self.selection_job.is_none()
                {
                    return Ok(false);
                }
                if !self
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| Arc::ptr_eq(snapshot, &prepared.snapshot))
                {
                    return Ok(false);
                }
                self.selected = prepared.selected;
                self.active = prepared.active;
                self.anchor = prepared.anchor;
                self.selection_job = None;
                if self.activate_after_selection {
                    self.activate_after_selection = false;
                    if !self.selected.is_empty()
                        && !matches!(
                            self.directory_state,
                            WidgetryFileDialogDirectoryState::Failed(_)
                        )
                    {
                        self.confirm()?;
                    }
                }
                Ok(true)
            }
        }
    }

    pub(crate) fn matches_directory(&self, token: WidgetryFileDialogToken) -> bool {
        self.token.session == token.session && self.token.generation == token.generation
    }

    pub(crate) fn matches_query(&self, token: WidgetryFileDialogToken) -> bool {
        self.matches_directory(token) && self.token.query_revision == token.query_revision
    }

    fn navigate(&mut self, navigation: WidgetryFileDialogNavigation) -> Result<bool, BevyError> {
        let path = match &navigation {
            WidgetryFileDialogNavigation::Path(path) => {
                if path.as_os_str().is_empty() {
                    return Err(contract_error("empty navigation path"));
                }
                if path.is_absolute() {
                    path.clone()
                } else {
                    self.current_path
                        .as_ref()
                        .map_or_else(|| path.clone(), |base| base.join(path))
                }
            }
            WidgetryFileDialogNavigation::Refresh => self
                .current_path
                .as_ref()
                .or(self.requested_path.as_ref())
                .cloned()
                .ok_or_else(|| contract_error("refresh has no directory"))?,
            WidgetryFileDialogNavigation::Back => {
                let Some(cursor) = self.history_cursor.and_then(|cursor| cursor.checked_sub(1))
                else {
                    return Ok(false);
                };
                self.history
                    .get(cursor)
                    .cloned()
                    .ok_or_else(|| contract_error("history cursor invalid"))?
            }
            WidgetryFileDialogNavigation::Forward => {
                let Some(cursor) = self
                    .history_cursor
                    .map(|cursor| cursor + 1)
                    .filter(|cursor| *cursor < self.history.len())
                else {
                    return Ok(false);
                };
                self.history[cursor].clone()
            }
            WidgetryFileDialogNavigation::Up => {
                let path = self
                    .current_path
                    .as_ref()
                    .ok_or_else(|| contract_error("up has no current directory"))?;
                let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) else {
                    return Ok(false);
                };
                parent.to_owned()
            }
        };
        let generation = increment(self.token.generation)?;
        let edit_revision = increment(self.token.edit_revision)?;
        self.token.generation = generation;
        self.token.edit_revision = edit_revision;
        self.navigation = navigation;
        self.requested_path = Some(path);
        self.directory_state = WidgetryFileDialogDirectoryState::Loading;
        self.selected = Arc::new(Default::default());
        self.selection_job = None;
        self.activate_after_selection = false;
        self.validation_job = None;
        self.folder_request = None;
        self.reveal_path = None;
        self.active = None;
        self.anchor = None;
        self.entries = Arc::new(im::Vector::new());
        self.visible = Arc::new(im::Vector::new());
        self.snapshot = None;
        self.projection_pending = false;
        self.confirmation = WidgetryFileDialogConfirmation::Idle;
        self.candidate = None;
        self.error = None;
        Ok(true)
    }

    fn change_query(&mut self, action: WidgetryFileDialogAction) -> Result<bool, BevyError> {
        let same = match &action {
            WidgetryFileDialogAction::Filter(id) => {
                if !self.filters.iter().any(|filter| &filter.id == id) {
                    return Err(contract_error("unknown FilterId"));
                }
                self.preferences.filter == *id
            }
            WidgetryFileDialogAction::Search(search) => self.search == *search,
            WidgetryFileDialogAction::Sort(sort) => self.preferences.sort == *sort,
            WidgetryFileDialogAction::ShowHidden(show) => self.preferences.show_hidden == *show,
            WidgetryFileDialogAction::ShowSystem(show) => self.preferences.show_system == *show,
            _ => return Err(contract_error("invalid query operation")),
        };
        if same {
            return Ok(false);
        }
        let query_revision = increment(self.token.query_revision)?;
        let edit_revision = increment(self.token.edit_revision)?;
        match action {
            WidgetryFileDialogAction::Filter(id) => self.preferences.filter = id,
            WidgetryFileDialogAction::Search(search) => self.search = search,
            WidgetryFileDialogAction::Sort(sort) => self.preferences.sort = sort,
            WidgetryFileDialogAction::ShowHidden(show) => self.preferences.show_hidden = show,
            WidgetryFileDialogAction::ShowSystem(show) => self.preferences.show_system = show,
            _ => return Err(contract_error("invalid query operation")),
        }
        self.token.query_revision = query_revision;
        self.token.edit_revision = edit_revision;
        self.confirmation = WidgetryFileDialogConfirmation::Idle;
        self.candidate = None;
        self.projection_pending = self.snapshot.is_some();
        self.selection_job = None;
        self.activate_after_selection = false;
        self.validation_job = None;
        Ok(true)
    }
}
