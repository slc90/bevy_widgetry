use crate::directory::DirectoryEvent;
use crate::model::contract_error;
use crate::preferences_job::Reply as PreferencesReply;
use crate::projection::ProjectionResult;
use crate::retention::{Control, Retention};
use crate::*;
use bevy::prelude::*;
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use bevy_widgetry_internal_log::widgetry_error;
use bevy_widgetry_job::{JobId, JobManager, JobMessage, JobOutcome, JobReceiver, TryRecvError};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[derive(Resource, Clone)]
pub struct WidgetryFileDialogRuntimeOptions {
    pub automatic: bool,
    pub max_sessions: usize,
    pub max_messages_per_update: usize,
    pub update_budget: std::time::Duration,
}

#[derive(Resource, Clone)]
pub struct WidgetryFileDialogPersistence {
    pub path: PathBuf,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct WidgetryFileDialogRuntimeStatus {
    pub retention_jobs: usize,
    pub active_sessions: usize,
    pub queued_retention: usize,
    pub retained_snapshots: usize,
    pub applied_last_update: usize,
    pub persistence: WidgetryFileDialogStorageState,
    pub error: Option<String>,
}

#[derive(Resource, Default)]
pub(crate) struct FlushRequest(pub(crate) bool);

#[derive(Resource, Default)]
pub(crate) struct Runtime {
    retention: Option<Arc<Retention>>,
    sessions: BTreeMap<Entity, Session>,
    start_failed: bool,
    persistence_path: Option<PathBuf>,
    load_admitted: bool,
    loaded: bool,
    saving: bool,
    preference_load: Option<PreparedJob<PreferencesReply>>,
    preference_save: Option<PreparedJob<PreferencesReply>>,
    save_revisions: Option<BTreeMap<String, u64>>,
    failed_revisions: Option<BTreeMap<String, u64>>,
    disk_revision: u64,
    retired: Vec<(JobManager, JobId)>,
    directory_cursor: usize,
    projection_cursor: usize,
    selection_cursor: usize,
    validation_cursor: usize,
    folder_cursor: usize,
    locations_cursor: usize,
    prefer_operations: bool,
    prefer_projection: bool,
    prefer_selection: bool,
    prefer_validation: bool,
    prefer_folder: bool,
    prefer_locations: bool,
}

enum Output {
    Reply(Entity, WidgetryFileDialogReply),
    Snapshot(
        Entity,
        Arc<WidgetryFileDialogSnapshot>,
        WidgetryFileDialogDirectoryState,
    ),
    Loaded(Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>, String>),
    Saved(BTreeMap<String, u64>, Result<(), String>),
}

struct DirectoryJob {
    manager: JobManager,
    id: JobId,
    receiver: JobReceiver<DirectoryEvent>,
    final_snapshot: Option<(
        Arc<WidgetryFileDialogSnapshot>,
        WidgetryFileDialogDirectoryState,
    )>,
}

struct Session {
    control: Control,
    admitted: bool,
    directory: Option<DirectoryJob>,
    locations_admitted: bool,
    locations: Option<PreparedJob<WidgetryFileDialogReply>>,
    cached: Option<(
        Arc<WidgetryFileDialogSnapshot>,
        WidgetryFileDialogDirectoryState,
    )>,
    version: u64,
    project_inflight: Option<(WidgetryFileDialogToken, u64)>,
    projection: Option<ProjectionJob>,
    project_applied: Option<(WidgetryFileDialogToken, u64)>,
    selection_submitted: Option<WidgetryFileDialogToken>,
    selection: Option<PreparedJob<WidgetryFileDialogPreparedSelection>>,
    validation_submitted: Option<WidgetryFileDialogToken>,
    validation: Option<PreparedJob<WidgetryFileDialogReply>>,
    folder_submitted: Option<WidgetryFileDialogToken>,
    folder: Option<PreparedJob<WidgetryFileDialogReply>>,
    failed: bool,
}

type ProjectionJob = PreparedJob<ProjectionResult>;

struct PreparedJob<T> {
    manager: JobManager,
    id: JobId,
    receiver: JobReceiver<T>,
    pending: Option<T>,
}

enum PreparedPoll<T> {
    Empty,
    Item,
    Finished(JobOutcome, Option<T>),
}

impl<T: Clone> PreparedJob<T> {
    fn poll(&mut self) -> Result<PreparedPoll<T>> {
        match self.receiver.try_recv() {
            Ok(JobMessage::Item(result)) => {
                self.pending = Some((*result).clone());
                Ok(PreparedPoll::Item)
            }
            Ok(JobMessage::Finished(outcome)) => {
                self.manager
                    .remove(self.id)
                    .map_err(|error| contract_error(&error.to_string()))?;
                Ok(PreparedPoll::Finished(outcome, self.pending.take()))
            }
            Err(TryRecvError::Empty) => Ok(PreparedPoll::Empty),
            Err(error) => Err(contract_error(&error.to_string())),
        }
    }
}

impl Default for WidgetryFileDialogRuntimeOptions {
    fn default() -> Self {
        Self {
            automatic: true,
            max_sessions: 8,
            max_messages_per_update: 8,
            update_budget: std::time::Duration::from_millis(2),
        }
    }
}

pub(crate) fn wake(world: &World) -> Result {
    if world
        .get_resource::<WidgetryFileDialogRuntimeOptions>()
        .is_some_and(|options| !options.automatic)
    {
        return Ok(());
    }
    if let Some(proxy) = world.get_resource::<EventLoopProxyWrapper>() {
        proxy
            .send_event(WinitUserEvent::WakeUp)
            .map_err(|error| contract_error(&error.to_string()))?;
    }
    Ok(())
}

pub(crate) fn update(world: &mut World) -> Result {
    let options = world.resource::<WidgetryFileDialogRuntimeOptions>().clone();
    if !options.automatic {
        return Ok(());
    }
    if options.update_budget.is_zero()
        || options.max_sessions == 0
        || options.max_sessions > 8
        || options.max_messages_per_update == 0
        || options.max_messages_per_update > 64
    {
        return Err(contract_error("invalid FileDialog runtime capacity"));
    }
    let mut runtime = world
        .remove_resource::<Runtime>()
        .ok_or_else(|| contract_error("FileDialog runtime missing"))?;
    let outcome = runtime.update(world, &options);
    world.insert_resource(runtime);
    outcome
}

impl Runtime {
    fn update(&mut self, world: &mut World, options: &WidgetryFileDialogRuntimeOptions) -> Result {
        let states: Vec<_> = world
            .query_filtered::<(Entity, &WidgetryFileDialogState), With<WidgetryFileDialog>>()
            .iter(world)
            .filter(|(_, state)| state.session_state() == WidgetryFileDialogSessionState::Open)
            .map(|(root, state)| {
                (
                    root,
                    state.token(),
                    matches!(
                        state.directory_state(),
                        WidgetryFileDialogDirectoryState::Failed(_)
                    ),
                )
            })
            .collect();
        self.reap_retired()?;
        self.persistence(world)?;
        if self.retention.is_none() && !self.start_failed && !states.is_empty() {
            match Retention::start(world.resource::<JobManager>()) {
                Ok(service) => self.retention = Some(service),
                Err(error) => {
                    self.start_failed = true;
                    widgetry_error!(error = %error, "FileDialog 无法启动后台服务");
                    world
                        .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                        .error = Some(error.to_string());
                }
            }
        }
        let Some(service) = self.retention.clone() else {
            if self.start_failed {
                for (root, token, failed) in states {
                    if failed {
                        continue;
                    }
                    WidgetryFileDialog::deliver(
                        world,
                        root,
                        WidgetryFileDialogReply::Failed {
                            token,
                            error: "FileDialog retention Job startup failed".into(),
                        },
                    )?;
                }
            }
            let start = Instant::now();
            let mut applied = 0;
            while applied < options.max_messages_per_update
                && start.elapsed() < options.update_budget
                && self.poll_preferences(world)?
            {
                applied += 1;
            }
            world
                .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                .applied_last_update = applied;
            self.persistence(world)?;
            if (applied == options.max_messages_per_update
                || start.elapsed() >= options.update_budget)
                && (self.preference_load.is_some() || self.preference_save.is_some())
            {
                wake(world)?;
            }
            return Ok(());
        };
        self.reap_retired()?;
        self.retain_sessions(|root, session| {
            states.iter().any(|(entity, token, _)| {
                entity == root
                    && token.session == session.control.token.session
                    && token.generation == session.control.token.generation
            })
        })?;
        for (root, token, failed) in &states {
            if !self.sessions.contains_key(root) {
                if self.sessions.len() >= options.max_sessions {
                    if !failed {
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Failed {
                                token: *token,
                                error: "FileDialog active session capacity exceeded".into(),
                            },
                        )?;
                    }
                    continue;
                }
                self.sessions.insert(*root, Session::new(*root, *token));
            }
        }
        if let Some(error) = service.error().map_err(|error| contract_error(&error))? {
            for session in self.sessions.values_mut() {
                session.stop(&mut self.retired)?;
            }
            let reported = world
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .error
                .as_ref()
                == Some(&error);
            world
                .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                .error = Some(error.clone());
            for (root, token, failed) in &states {
                if !failed {
                    WidgetryFileDialog::deliver(
                        world,
                        *root,
                        WidgetryFileDialogReply::Failed {
                            token: *token,
                            error: error.clone(),
                        },
                    )?;
                }
            }
            return if reported {
                Ok(())
            } else {
                Err(BevyError::error(error.clone()))
            };
        }
        if service.shutdown.load(Ordering::Acquire) || !service.active() {
            for (root, token, _) in &states {
                if let Some(session) = self.sessions.get_mut(root) {
                    session.stop(&mut self.retired)?;
                    session.failed = true;
                }
                WidgetryFileDialog::deliver(
                    world,
                    *root,
                    WidgetryFileDialogReply::OperationsStopped {
                        token: *token,
                        error: "FileDialog background operations stopped during shutdown".into(),
                    },
                )?;
            }
        }
        let start = Instant::now();
        let mut applied = 0;
        while applied < options.max_messages_per_update && start.elapsed() < options.update_budget {
            self.prefer_operations = !self.prefer_operations;
            let first = if self.prefer_operations {
                self.poll_jobs(world)?
            } else {
                self.poll_preferences(world)?
            };
            if !first {
                let second = if self.prefer_operations {
                    self.poll_preferences(world)?
                } else {
                    self.poll_jobs(world)?
                };
                if !second {
                    break;
                }
            }
            applied += 1;
        }
        // Result observers may end or despawn the root, so this reply must release its session and cache in the same frame.
        self.retain_sessions(|root, _| {
            world
                .get::<WidgetryFileDialogState>(*root)
                .is_some_and(|state| state.session_state() == WidgetryFileDialogSessionState::Open)
        })?;
        for (root, session) in &mut self.sessions {
            let Some(state) = world.get::<WidgetryFileDialogState>(*root) else {
                continue;
            };
            if state.session_state() != WidgetryFileDialogSessionState::Open {
                session.stop(&mut self.retired)?;
                continue;
            }
            // Replies such as FolderCreated to Refresh may advance the generation during this batch.
            // New requests must start with fresh cache and admission state rather than carrying the old directory projection.
            if state.token().session != session.control.token.session
                || state.token().generation != session.control.token.generation
            {
                session.stop(&mut self.retired)?;
                *session = Session::new(*root, state.token());
            }
            if session.failed {
                continue;
            }
            let mut control = session.control.clone();
            control.token = state.token();
            if !session.admitted {
                let path = state
                    .requested_path()
                    .or(state.fallback_directory())
                    .map(ToOwned::to_owned);
                let manager = world.resource::<JobManager>().clone();
                match crate::directory::spawn(
                    &manager,
                    world.resource::<WidgetryFileDialogBackend>().clone(),
                    service.clone(),
                    control.clone(),
                    path,
                    state.query(),
                ) {
                    Ok((id, receiver)) => {
                        session.directory = Some(DirectoryJob {
                            manager,
                            id,
                            receiver,
                            final_snapshot: None,
                        });
                        session.admitted = true;
                    }
                    Err(error) => {
                        session.failed = true;
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Failed {
                                token: control.token,
                                error: error.to_string(),
                            },
                        )?;
                        continue;
                    }
                }
            }
            if !session.locations_admitted {
                let manager = world.resource::<JobManager>().clone();
                session.locations_admitted = true;
                match crate::locations::spawn(
                    &manager,
                    world.resource::<WidgetryFileDialogBackend>().clone(),
                    control.token,
                ) {
                    Ok((id, receiver)) => {
                        session.locations = Some(PreparedJob {
                            manager,
                            id,
                            receiver,
                            pending: None,
                        });
                    }
                    Err(error) => {
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Locations {
                                token: control.token,
                                outcome: Err(error.to_string()),
                            },
                        )?;
                        continue;
                    }
                }
            }
            let signature = (state.token(), session.version);
            let projection_needed = !session.project_applied.is_some_and(|applied| {
                applied.1 == signature.1
                    && applied.0.session == signature.0.session
                    && applied.0.generation == signature.0.generation
                    && applied.0.query_revision == signature.0.query_revision
            });
            if session
                .project_inflight
                .is_some_and(|inflight| inflight != signature)
            {
                session.stop_projection(&mut self.retired)?;
            }
            if session.selection.is_some()
                && (projection_needed
                    || session.projection.is_some()
                    || state
                        .selection_job
                        .as_ref()
                        .map(WidgetryFileDialogSelectionJob::token)
                        != session.selection_submitted)
            {
                session.stop_selection(&mut self.retired)?;
            }
            if let Some((snapshot, status)) = &session.cached
                && session.project_inflight.is_none()
                && projection_needed
            {
                let manager = world.resource::<JobManager>().clone();
                match crate::projection::spawn(
                    &manager,
                    service.clone(),
                    control.clone(),
                    Arc::new(state.clone()),
                    snapshot.clone(),
                    status.clone(),
                ) {
                    Ok((id, receiver)) => {
                        session.projection = Some(ProjectionJob {
                            manager,
                            id,
                            receiver,
                            pending: None,
                        });
                        session.project_inflight = Some(signature);
                    }
                    Err(error) => {
                        session.failed = true;
                        session.stop(&mut self.retired)?;
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Failed {
                                token: control.token,
                                error: error.to_string(),
                            },
                        )?;
                        continue;
                    }
                }
            }
            if session.project_inflight.is_none()
                && let Some(job) = state.selection_job.as_ref()
                && session.selection_submitted != Some(job.token())
            {
                let manager = world.resource::<JobManager>().clone();
                match crate::selection_task::spawn(
                    &manager,
                    service.clone(),
                    control.clone(),
                    job.clone(),
                ) {
                    Ok((id, receiver)) => {
                        session.selection = Some(PreparedJob {
                            manager,
                            id,
                            receiver,
                            pending: None,
                        });
                        session.selection_submitted = Some(job.token());
                    }
                    Err(error) => {
                        session.failed = true;
                        session.stop(&mut self.retired)?;
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Failed {
                                token: control.token,
                                error: error.to_string(),
                            },
                        )?;
                        continue;
                    }
                }
            }
            if session.validation.is_some()
                && (Some(state.token()) != session.validation_submitted
                    || state
                        .validation_job
                        .as_ref()
                        .map(WidgetryFileDialogValidationJob::token)
                        != session.validation_submitted
                    || state.confirmation() != &WidgetryFileDialogConfirmation::Validating)
            {
                session.stop_validation(&mut self.retired)?;
            }
            if let Some(job) = state.validation_job.as_ref()
                && session.validation_submitted != Some(job.token())
            {
                let manager = world.resource::<JobManager>().clone();
                match crate::candidate::spawn(
                    &manager,
                    world.resource::<WidgetryFileDialogBackend>().clone(),
                    service.clone(),
                    control.clone(),
                    job.clone(),
                ) {
                    Ok((id, receiver)) => {
                        session.validation = Some(PreparedJob {
                            manager,
                            id,
                            receiver,
                            pending: None,
                        });
                        session.validation_submitted = Some(job.token());
                    }
                    Err(error) => {
                        session.validation_submitted = Some(job.token());
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::ValidationFailed {
                                token: job.token(),
                                error: error.to_string(),
                            },
                        )?;
                        continue;
                    }
                }
            }
            if session.folder.is_some()
                && state
                    .folder_request
                    .as_ref()
                    .map(WidgetryFileDialogFolderRequest::token)
                    != session.folder_submitted
            {
                session.stop_folder(&mut self.retired)?;
            }
            if let Some(request) = state.folder_request.as_ref()
                && session.folder_submitted != Some(request.token())
            {
                let manager = world.resource::<JobManager>().clone();
                session.folder_submitted = Some(request.token());
                match crate::folder::spawn(
                    &manager,
                    world.resource::<WidgetryFileDialogBackend>().clone(),
                    request.clone(),
                ) {
                    Ok((id, receiver)) => {
                        session.folder = Some(PreparedJob {
                            manager,
                            id,
                            receiver,
                            pending: None,
                        });
                    }
                    Err(error) => {
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::FolderCreated {
                                request: request.clone(),
                                outcome: Err(error.to_string()),
                            },
                        )?;
                    }
                }
            }
        }
        self.persistence(world)?;
        {
            let mut status = world.resource_mut::<WidgetryFileDialogRuntimeStatus>();
            status.retention_jobs = usize::from(service.active());
            status.active_sessions = self.sessions.len();
            status.queued_retention = service.queued();
            status.retained_snapshots = service.retained.load(Ordering::Relaxed);
            status.applied_last_update = applied;
        }
        self.reap_retired()?;
        // Preference replies can exhaust the shared budget before another Job item is observed.
        // One continuation also covers a time-budget exit without polling idle jobs indefinitely.
        let directory_budget_exit = (applied == options.max_messages_per_update
            || start.elapsed() >= options.update_budget)
            && (self.preference_load.is_some()
                || self.preference_save.is_some()
                || self.sessions.values().any(|session| {
                    session.directory.is_some()
                        || session.projection.is_some()
                        || session.selection.is_some()
                        || session.validation.is_some()
                        || session.folder.is_some()
                        || session.locations.is_some()
                }));
        if directory_budget_exit {
            wake(world)?;
        }
        Ok(())
    }

    fn retain_sessions(&mut self, mut keep: impl FnMut(&Entity, &Session) -> bool) -> Result {
        let expired: Vec<_> = self
            .sessions
            .iter()
            .filter(|(root, session)| !keep(root, session))
            .map(|(root, _)| *root)
            .collect();
        for root in expired {
            if let Some(mut session) = self.sessions.remove(&root) {
                session.stop(&mut self.retired)?;
            }
        }
        Ok(())
    }

    fn reap_retired(&mut self) -> Result {
        let mut index = 0;
        while index < self.retired.len() {
            let (manager, id) = &self.retired[index];
            if manager
                .status(*id)
                .map_err(|error| contract_error(&error.to_string()))?
                .state
                .is_finished()
            {
                manager
                    .remove(*id)
                    .map_err(|error| contract_error(&error.to_string()))?;
                self.retired.swap_remove(index);
            } else {
                index += 1;
            }
        }
        Ok(())
    }

    fn poll_jobs(&mut self, world: &mut World) -> Result<bool> {
        self.prefer_locations = !self.prefer_locations;
        if self.prefer_locations && self.poll_locations(world)? {
            return Ok(true);
        }
        self.prefer_folder = !self.prefer_folder;
        if self.prefer_folder && self.poll_folder(world)? {
            return Ok(true);
        }
        self.prefer_validation = !self.prefer_validation;
        if self.prefer_validation && self.poll_validation(world)? {
            return Ok(true);
        }
        self.prefer_selection = !self.prefer_selection;
        if self.prefer_selection && self.poll_selection(world)? {
            return Ok(true);
        }
        self.prefer_projection = !self.prefer_projection;
        if self.prefer_projection && self.poll_projection(world)? {
            return Ok(true);
        }
        if self.poll_directory(world)? {
            return Ok(true);
        }
        if !self.prefer_projection && self.poll_projection(world)? {
            return Ok(true);
        }
        if !self.prefer_selection && self.poll_selection(world)? {
            return Ok(true);
        }
        if !self.prefer_validation && self.poll_validation(world)? {
            return Ok(true);
        }
        if !self.prefer_folder && self.poll_folder(world)? {
            return Ok(true);
        }
        if !self.prefer_locations {
            return self.poll_locations(world);
        }
        Ok(false)
    }

    fn poll_locations(&mut self, world: &mut World) -> Result<bool> {
        let roots: Vec<_> = self.sessions.keys().copied().collect();
        for _ in 0..roots.len() {
            self.locations_cursor %= roots.len();
            let root = roots[self.locations_cursor];
            self.locations_cursor += 1;
            let Some(session) = self.sessions.get_mut(&root) else {
                continue;
            };
            if session.locations.is_none() {
                continue;
            }
            let token = session.control.token;
            let current = !session.failed
                && session.control.root == root
                && world
                    .get::<WidgetryFileDialogState>(root)
                    .is_some_and(|state| {
                        state.session_state() == WidgetryFileDialogSessionState::Open
                            && state.token().session == token.session
                            && state.token().generation == token.generation
                    });
            if !current {
                session.stop_locations(&mut self.retired)?;
                return Ok(true);
            }
            match session
                .locations
                .as_mut()
                .ok_or_else(|| contract_error("locations job missing"))?
                .poll()?
            {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, pending) => {
                    session.locations = None;
                    let reply =
                        match outcome {
                            JobOutcome::Completed => {
                                pending.unwrap_or_else(|| WidgetryFileDialogReply::Locations {
                                    token,
                                    outcome: Err(
                                        "locations result was canceled before delivery".into()
                                    ),
                                })
                            }
                            JobOutcome::Failed(error) => WidgetryFileDialogReply::Locations {
                                token,
                                outcome: Err(error.to_string()),
                            },
                            JobOutcome::Canceled => WidgetryFileDialogReply::Locations {
                                token,
                                outcome: Err("locations query canceled".into()),
                            },
                        };
                    WidgetryFileDialog::deliver(world, root, reply)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn poll_folder(&mut self, world: &mut World) -> Result<bool> {
        let roots: Vec<_> = self.sessions.keys().copied().collect();
        for _ in 0..roots.len() {
            self.folder_cursor %= roots.len();
            let root = roots[self.folder_cursor];
            self.folder_cursor += 1;
            let Some(session) = self.sessions.get_mut(&root) else {
                continue;
            };
            if session.folder.is_none() {
                continue;
            }
            let request = world
                .get::<WidgetryFileDialogState>(root)
                .and_then(|state| {
                    (!session.failed
                        && state.session_state() == WidgetryFileDialogSessionState::Open
                        && state.token().session == session.control.token.session
                        && state.token().generation == session.control.token.generation)
                        .then(|| state.folder_request.clone())
                        .flatten()
                        .filter(|request| Some(request.token()) == session.folder_submitted)
                });
            let Some(request) = request else {
                session.stop_folder(&mut self.retired)?;
                return Ok(true);
            };
            match session
                .folder
                .as_mut()
                .ok_or_else(|| contract_error("folder job missing"))?
                .poll()?
            {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, pending) => {
                    session.folder = None;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            pending.unwrap_or_else(|| WidgetryFileDialogReply::FolderCreated {
                                request,
                                outcome: Err("folder result was canceled before delivery".into()),
                            })
                        }
                        JobOutcome::Failed(error) => WidgetryFileDialogReply::FolderCreated {
                            request,
                            outcome: Err(error.to_string()),
                        },
                        JobOutcome::Canceled => WidgetryFileDialogReply::FolderCreated {
                            request,
                            outcome: Err("folder creation canceled".into()),
                        },
                    };
                    WidgetryFileDialog::deliver(world, root, reply)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn poll_validation(&mut self, world: &mut World) -> Result<bool> {
        let roots: Vec<_> = self.sessions.keys().copied().collect();
        for _ in 0..roots.len() {
            self.validation_cursor %= roots.len();
            let root = roots[self.validation_cursor];
            self.validation_cursor += 1;
            let Some(session) = self.sessions.get_mut(&root) else {
                continue;
            };
            if session.validation.is_none() {
                continue;
            }
            let current = !session.failed
                && world
                    .get::<WidgetryFileDialogState>(root)
                    .is_some_and(|state| {
                        state.session_state() == WidgetryFileDialogSessionState::Open
                            && Some(state.token()) == session.validation_submitted
                            && state
                                .validation_job
                                .as_ref()
                                .map(WidgetryFileDialogValidationJob::token)
                                == session.validation_submitted
                            && state.confirmation() == &WidgetryFileDialogConfirmation::Validating
                    });
            if !current {
                session.stop_validation(&mut self.retired)?;
                return Ok(true);
            }
            match session
                .validation
                .as_mut()
                .ok_or_else(|| contract_error("candidate validation job missing"))?
                .poll()?
            {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, pending) => {
                    session.validation = None;
                    let token = session
                        .validation_submitted
                        .ok_or_else(|| contract_error("validation token missing"))?;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            pending.unwrap_or_else(|| WidgetryFileDialogReply::ValidationFailed {
                                token,
                                error: "candidate validation result was canceled before delivery"
                                    .into(),
                            })
                        }
                        JobOutcome::Failed(error) => pending
                            .filter(|reply| {
                                matches!(reply, WidgetryFileDialogReply::ValidationFailed { .. })
                            })
                            .unwrap_or_else(|| WidgetryFileDialogReply::ValidationFailed {
                                token,
                                error: error.to_string(),
                            }),
                        JobOutcome::Canceled => WidgetryFileDialogReply::ValidationFailed {
                            token,
                            error: "candidate validation canceled".into(),
                        },
                    };
                    WidgetryFileDialog::deliver(world, root, reply)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn poll_selection(&mut self, world: &mut World) -> Result<bool> {
        let roots: Vec<_> = self.sessions.keys().copied().collect();
        for _ in 0..roots.len() {
            self.selection_cursor %= roots.len();
            let root = roots[self.selection_cursor];
            self.selection_cursor += 1;
            let Some(session) = self.sessions.get_mut(&root) else {
                continue;
            };
            if session.selection.is_none() {
                continue;
            }
            let current = !session.failed
                && world
                    .get::<WidgetryFileDialogState>(root)
                    .is_some_and(|state| {
                        state.session_state() == WidgetryFileDialogSessionState::Open
                            && session.selection_submitted.is_some_and(|token| {
                                state.matches_query(token)
                                    && state.token().selection_revision == token.selection_revision
                            })
                            && state
                                .selection_job
                                .as_ref()
                                .map(WidgetryFileDialogSelectionJob::token)
                                == session.selection_submitted
                    });
            if !current {
                session.stop_selection(&mut self.retired)?;
                return Ok(true);
            }
            match session
                .selection
                .as_mut()
                .ok_or_else(|| contract_error("selection job missing"))?
                .poll()?
            {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, prepared) => {
                    session.selection = None;
                    let token = session
                        .selection_submitted
                        .ok_or_else(|| contract_error("selection token missing"))?;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            WidgetryFileDialogReply::Selection(prepared.ok_or_else(|| {
                                contract_error("selection completed without result")
                            })?)
                        }
                        JobOutcome::Failed(error) => WidgetryFileDialogReply::Failed {
                            token,
                            error: error.to_string(),
                        },
                        JobOutcome::Canceled => WidgetryFileDialogReply::Failed {
                            token,
                            error: "selection job canceled".into(),
                        },
                    };
                    self.receive(world, Output::Reply(root, reply))?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn poll_projection(&mut self, world: &mut World) -> Result<bool> {
        let roots: Vec<_> = self.sessions.keys().copied().collect();
        for _ in 0..roots.len() {
            self.projection_cursor %= roots.len();
            let root = roots[self.projection_cursor];
            self.projection_cursor += 1;
            let Some(session) = self.sessions.get_mut(&root) else {
                continue;
            };
            if session.projection.is_none() {
                continue;
            }
            let signature = session
                .project_inflight
                .ok_or_else(|| contract_error("projection signature missing"))?;
            let current = !session.failed
                && session.version == signature.1
                && world
                    .get::<WidgetryFileDialogState>(root)
                    .is_some_and(|state| {
                        state.session_state() == WidgetryFileDialogSessionState::Open
                            && state.token() == signature.0
                    });
            if !current {
                session.stop_projection(&mut self.retired)?;
                return Ok(true);
            }
            let job = session
                .projection
                .as_mut()
                .ok_or_else(|| contract_error("projection job missing"))?;
            match job.poll()? {
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Empty => continue,
                PreparedPoll::Finished(outcome, pending) => {
                    session.projection = None;
                    session.project_inflight = None;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            let result = pending.ok_or_else(|| {
                                contract_error("projection completed without result")
                            })?;
                            WidgetryFileDialogReply::Snapshot {
                                snapshot: result.snapshot,
                                state: result.state,
                                selection: result.selection,
                            }
                        }
                        JobOutcome::Failed(error) => WidgetryFileDialogReply::Failed {
                            token: signature.0,
                            error: error.to_string(),
                        },
                        JobOutcome::Canceled => WidgetryFileDialogReply::Failed {
                            token: signature.0,
                            error: "projection job canceled".into(),
                        },
                    };
                    if matches!(reply, WidgetryFileDialogReply::Failed { .. }) {
                        self.receive(world, Output::Reply(root, reply))?;
                    } else if WidgetryFileDialog::deliver(world, root, reply)?
                        && let Some(session) = self.sessions.get_mut(&root)
                    {
                        session.project_applied = Some(signature);
                    }
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn poll_directory(&mut self, world: &mut World) -> Result<bool> {
        let roots: Vec<_> = self.sessions.keys().copied().collect();
        for _ in 0..roots.len() {
            self.directory_cursor %= roots.len();
            let root = roots[self.directory_cursor];
            self.directory_cursor += 1;
            let Some(session) = self.sessions.get_mut(&root) else {
                continue;
            };
            let Some(job) = &mut session.directory else {
                continue;
            };
            let message = match job.receiver.try_recv() {
                Ok(message) => message,
                Err(TryRecvError::Empty) => continue,
                Err(error) => return Err(contract_error(&error.to_string())),
            };
            let token = session.control.token;
            // A prior reply can navigate, resolve, or despawn this root within the same budget loop.
            let current = world
                .get::<WidgetryFileDialogState>(root)
                .is_some_and(|state| {
                    state.session_state() == WidgetryFileDialogSessionState::Open
                        && state.token().session == token.session
                        && state.token().generation == token.generation
                });
            if !current {
                session.stop(&mut self.retired)?;
                return Ok(true);
            }
            let output = match message {
                JobMessage::Item(event) => match event.as_ref() {
                    DirectoryEvent::Started(path) => Some(Output::Reply(
                        root,
                        WidgetryFileDialogReply::Started {
                            token,
                            path: path.clone(),
                        },
                    )),
                    DirectoryEvent::Snapshot {
                        snapshot,
                        state,
                        finished,
                    } => {
                        if *finished {
                            job.final_snapshot = Some((snapshot.clone(), state.clone()));
                            None
                        } else {
                            Some(Output::Snapshot(root, snapshot.clone(), state.clone()))
                        }
                    }
                },
                JobMessage::Finished(outcome) => {
                    let job = session
                        .directory
                        .take()
                        .ok_or_else(|| contract_error("directory job missing during completion"))?;
                    job.manager
                        .remove(job.id)
                        .map_err(|error| contract_error(&error.to_string()))?;
                    match outcome {
                        JobOutcome::Completed => {
                            let (snapshot, state) = job.final_snapshot.ok_or_else(|| {
                                contract_error("directory job completed without final snapshot")
                            })?;
                            Some(Output::Snapshot(root, snapshot, state))
                        }
                        JobOutcome::Failed(error) => Some(Output::Reply(
                            root,
                            WidgetryFileDialogReply::Failed {
                                token,
                                error: error.to_string(),
                            },
                        )),
                        JobOutcome::Canceled => Some(Output::Reply(
                            root,
                            WidgetryFileDialogReply::Failed {
                                token,
                                error: "directory job canceled".into(),
                            },
                        )),
                    }
                }
            };
            if let Some(output) = output {
                self.receive(world, output)?;
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn receive(&mut self, world: &mut World, output: Output) -> Result {
        match output {
            Output::Snapshot(root, snapshot, status) => {
                if let Some(session) = self.sessions.get_mut(&root)
                    && !session.failed
                    && snapshot.token().session == session.control.token.session
                    && snapshot.token().generation == session.control.token.generation
                {
                    session.version = session
                        .version
                        .checked_add(1)
                        .ok_or_else(|| contract_error("stream revision exhausted"))?;
                    session.cached = Some((snapshot, status));
                }
            }
            Output::Reply(root, reply) => {
                if let Some(session) = self.sessions.get_mut(&root) {
                    if let WidgetryFileDialogReply::Failed { token, .. } = &reply
                        && token.session == session.control.token.session
                        && token.generation == session.control.token.generation
                    {
                        session.failed = true;
                        session.stop(&mut self.retired)?;
                    }
                    WidgetryFileDialog::deliver(world, root, reply)?;
                }
            }
            Output::Loaded(outcome) => {
                self.loaded = true;
                world
                    .resource_mut::<WidgetryFileDialogStorage>()
                    .file_loaded = true;
                match outcome {
                    Ok(scopes) => {
                        let merged =
                            merge_loaded(world.resource::<WidgetryFileDialogStorage>(), scopes);
                        let storage_state = match merged {
                            Ok(store) => {
                                world.insert_resource(store);
                                WidgetryFileDialogStorageState::Memory
                            }
                            Err(error) => {
                                self.failed_revisions = Some(
                                    world
                                        .resource::<WidgetryFileDialogStorage>()
                                        .scopes
                                        .iter()
                                        .map(|(scope, record)| (scope.clone(), record.revision))
                                        .collect(),
                                );
                                WidgetryFileDialogStorageState::Failed(error.to_string())
                            }
                        };
                        world
                            .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                            .persistence = storage_state;
                    }
                    Err(error) => {
                        self.failed_revisions = Some(
                            world
                                .resource::<WidgetryFileDialogStorage>()
                                .scopes
                                .iter()
                                .map(|(scope, record)| (scope.clone(), record.revision))
                                .collect(),
                        );
                        world
                            .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                            .persistence = WidgetryFileDialogStorageState::Failed(error)
                    }
                }
            }
            Output::Saved(revisions, outcome) => {
                self.saving = false;
                match outcome {
                    Ok(()) => {
                        for (scope, revision) in revisions {
                            world
                                .resource_mut::<WidgetryFileDialogStorage>()
                                .acknowledge_saved(&scope, revision)?;
                        }
                        world
                            .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                            .persistence =
                            WidgetryFileDialogStorageState::Saved(self.disk_revision);
                    }
                    Err(error) => {
                        self.failed_revisions = Some(revisions);
                        world
                            .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                            .persistence = WidgetryFileDialogStorageState::Failed(error);
                    }
                }
            }
        }
        Ok(())
    }

    fn poll_preferences(&mut self, world: &mut World) -> Result<bool> {
        for loading in [true, false] {
            let job = if loading {
                &mut self.preference_load
            } else {
                &mut self.preference_save
            };
            let Some(job) = job.as_mut() else {
                continue;
            };
            match job.poll()? {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, pending) => {
                    if loading {
                        self.preference_load = None;
                    } else {
                        self.preference_save = None;
                    }
                    let error = match outcome {
                        JobOutcome::Completed if pending.is_some() => None,
                        JobOutcome::Completed => {
                            Some("preferences result was canceled before delivery".into())
                        }
                        JobOutcome::Failed(error) => Some(error.to_string()),
                        JobOutcome::Canceled => Some("preferences operation canceled".into()),
                    };
                    let output = match (loading, error, pending) {
                        (true, Some(error), _) => Output::Loaded(Err(error)),
                        (false, Some(error), _) => Output::Saved(
                            self.save_revisions
                                .take()
                                .ok_or_else(|| contract_error("save revisions missing"))?,
                            Err(error),
                        ),
                        (true, None, Some(PreferencesReply::Loaded(result))) => {
                            Output::Loaded(result)
                        }
                        (false, None, Some(PreferencesReply::Saved(revisions, result))) => {
                            self.save_revisions = None;
                            Output::Saved(revisions, result)
                        }
                        _ => {
                            return Err(contract_error(
                                "preferences job returned an invalid reply",
                            ));
                        }
                    };
                    self.receive(world, output)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn persistence(&mut self, world: &mut World) -> Result {
        let Some(path) = world
            .get_resource::<WidgetryFileDialogPersistence>()
            .map(|config| config.path.clone())
        else {
            return Ok(());
        };
        if self.persistence_path.is_none() {
            self.persistence_path = Some(path.clone());
        }
        if self.persistence_path.as_ref() != Some(&path) {
            return Err(contract_error(
                "persistence path cannot change while service is active",
            ));
        }
        if !self.load_admitted {
            self.load_admitted = true;
            let manager = world.resource::<JobManager>().clone();
            match crate::preferences_job::load(
                &manager,
                world.resource::<WidgetryFileDialogBackend>().clone(),
                path.clone(),
            ) {
                Ok((id, receiver)) => {
                    self.preference_load = Some(PreparedJob {
                        manager,
                        id,
                        receiver,
                        pending: None,
                    });
                    world
                        .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                        .persistence = WidgetryFileDialogStorageState::Loading;
                }
                Err(error) => {
                    self.receive(world, Output::Loaded(Err(error.to_string())))?;
                }
            }
        }
        if !self.loaded || self.saving {
            return Ok(());
        }
        let store = world.resource::<WidgetryFileDialogStorage>();
        let revisions: BTreeMap<_, _> = store
            .scopes
            .iter()
            .map(|(scope, record)| (scope.clone(), record.revision))
            .collect();
        let dirty = store
            .scopes
            .values()
            .any(|record| record.revision > record.saved_revision);
        let force_save = world
            .get_resource::<FlushRequest>()
            .is_some_and(|request| request.0);
        if (dirty || force_save)
            && (force_save || self.failed_revisions.as_ref() != Some(&revisions))
        {
            let scopes = store.export();
            let manager = world.resource::<JobManager>().clone();
            self.disk_revision = self
                .disk_revision
                .checked_add(1)
                .ok_or_else(|| contract_error("disk revision exhausted"))?;
            if let Some(mut request) = world.get_resource_mut::<FlushRequest>() {
                request.0 = false;
            }
            match crate::preferences_job::save(
                &manager,
                world.resource::<WidgetryFileDialogBackend>().clone(),
                path,
                scopes,
                revisions.clone(),
            ) {
                Ok((id, receiver)) => {
                    self.preference_save = Some(PreparedJob {
                        manager,
                        id,
                        receiver,
                        pending: None,
                    });
                    self.save_revisions = Some(revisions);
                    self.saving = true;
                    world
                        .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                        .persistence = WidgetryFileDialogStorageState::Saving(self.disk_revision);
                }
                Err(error) => {
                    self.receive(world, Output::Saved(revisions, Err(error.to_string())))?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn cancel(&mut self, root: Entity) -> Result {
        if let Some(session) = self.sessions.get_mut(&root) {
            session.stop(&mut self.retired)?;
        }
        Ok(())
    }
}

fn merge_loaded(
    current: &WidgetryFileDialogStorage,
    scopes: BTreeMap<String, WidgetryFileDialogStorageSnapshot>,
) -> Result<WidgetryFileDialogStorage, BevyError> {
    let mut store = WidgetryFileDialogStorage {
        scopes: current.scopes.clone(),
        file_loaded: current.file_loaded,
    };
    for (scope, snapshot) in scopes {
        match current.scopes.get(&scope) {
            None => {
                store.import(scope.clone(), snapshot)?;
                if let Some(revision) = store.revision(&scope) {
                    store.acknowledge_saved(&scope, revision)?;
                }
            }
            Some(record) => {
                if record.pending_load {
                    let after = record.overlay_loaded(&snapshot);
                    let before = snapshot.clone();
                    store.scopes.insert(
                        scope.clone(),
                        crate::storage::StorageRecord {
                            snapshot,
                            revision: record.revision,
                            saved_revision: 0,
                            pending_load: false,
                            mutations: crate::storage::StorageMutations::default(),
                        },
                    );
                    let merged =
                        store.prepare_merge(&scope, &before, &after, false, false, None)?;
                    store.scopes.insert(scope, merged);
                }
            }
        }
    }
    Ok(store)
}

impl Session {
    fn stop(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        // Publish Job cancellation before the business flag can make an operation return Interrupted.
        if let Some(job) = &self.directory {
            job.manager
                .cancel(job.id)
                .map_err(|error| contract_error(&error.to_string()))?;
        }
        self.stop_projection(retired)?;
        self.stop_selection(retired)?;
        self.stop_validation(retired)?;
        self.stop_folder(retired)?;
        self.stop_locations(retired)?;
        self.control.cancelled.store(true, Ordering::Release);
        if let Some(job) = self.directory.take() {
            retired.push((job.manager, job.id));
        }
        Ok(())
    }

    fn stop_projection(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.projection {
            job.manager
                .cancel(job.id)
                .map_err(|error| contract_error(&error.to_string()))?;
        }
        if let Some(job) = self.projection.take() {
            retired.push((job.manager, job.id));
        }
        self.project_inflight = None;
        Ok(())
    }

    fn stop_selection(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.selection {
            job.manager
                .cancel(job.id)
                .map_err(|error| contract_error(&error.to_string()))?;
        }
        if let Some(job) = self.selection.take() {
            retired.push((job.manager, job.id));
        }
        self.selection_submitted = None;
        Ok(())
    }

    fn stop_validation(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.validation {
            job.manager
                .cancel(job.id)
                .map_err(|error| contract_error(&error.to_string()))?;
        }
        if let Some(job) = self.validation.take() {
            retired.push((job.manager, job.id));
        }
        self.validation_submitted = None;
        Ok(())
    }

    fn stop_folder(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.folder {
            job.manager
                .cancel(job.id)
                .map_err(|error| contract_error(&error.to_string()))?;
        }
        if let Some(job) = self.folder.take() {
            retired.push((job.manager, job.id));
        }
        self.folder_submitted = None;
        Ok(())
    }

    fn stop_locations(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.locations {
            job.manager
                .cancel(job.id)
                .map_err(|error| contract_error(&error.to_string()))?;
        }
        if let Some(job) = self.locations.take() {
            retired.push((job.manager, job.id));
        }
        Ok(())
    }

    fn new(root: Entity, token: WidgetryFileDialogToken) -> Self {
        Self {
            control: Control {
                root,
                token,
                cancelled: Arc::new(AtomicBool::new(false)),
            },
            admitted: false,
            directory: None,
            locations_admitted: false,
            locations: None,
            cached: None,
            version: 0,
            project_inflight: None,
            projection: None,
            project_applied: None,
            selection_submitted: None,
            selection: None,
            validation_submitted: None,
            validation: None,
            folder_submitted: None,
            folder: None,
            failed: false,
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        for job in [&self.preference_load, &self.preference_save]
            .into_iter()
            .flatten()
        {
            if let Err(error) = job.manager.cancel(job.id) {
                widgetry_error!(%error, "FileDialog 退出时取消偏好 Job 失败");
            }
        }
        for session in self.sessions.values_mut() {
            if let Err(error) = session.stop(&mut self.retired) {
                widgetry_error!(%error, "FileDialog 释放会话时取消目录 Job 失败");
            }
        }
        if let Some(service) = &self.retention {
            service.stop();
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    fn cached_projection_fixture() -> Result<(App, Runtime, Entity, Arc<WidgetryFileDialogSnapshot>)>
    {
        cached_projection_fixture_for(WidgetryFileDialogMode::PickFiles)
    }

    fn cached_projection_fixture_for(
        mode: WidgetryFileDialogMode,
    ) -> Result<(App, Runtime, Entity, Arc<WidgetryFileDialogSnapshot>)> {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog { @mode: {mode} } })?
            .id();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let path = PathBuf::from("C:/fixture");
        let snapshot = Arc::new(WidgetryFileDialogSnapshot::prepare(
            state.token(),
            path.clone(),
            ["one.txt", "two.log"]
                .into_iter()
                .map(|name| WidgetryFileDialogEntryData {
                    path: path.join(name),
                    name: name.into(),
                    kind: WidgetryFileDialogEntryKind::File,
                    size: None,
                    modified: None,
                    hidden: Some(false),
                    system: Some(false),
                })
                .collect(),
            &state.query(),
            None,
        )?);
        let token = state.token();
        let selection = state
            .projection_selection_job(snapshot.clone())?
            .prepare()?;
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started { token, path },
        )?;
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Snapshot {
                snapshot: snapshot.clone(),
                state: WidgetryFileDialogDirectoryState::Ready,
                selection,
            },
        )?;
        let mut session = Session::new(root, token);
        session.admitted = true;
        session.locations_admitted = true;
        session.cached = Some((snapshot.clone(), WidgetryFileDialogDirectoryState::Ready));
        session.project_applied = Some((token, 0));
        let mut runtime = Runtime::default();
        runtime.retention =
            Some(Retention::start(app.world().resource::<JobManager>()).map_err(BevyError::error)?);
        runtime.sessions.insert(root, session);
        Ok((app, runtime, root, snapshot))
    }

    fn wait_retention_started(runtime: &Runtime) {
        let retention = runtime.retention.as_ref().unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while retention.live.load(Ordering::Acquire) == 0 {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }

    fn wait_retention_stop(runtime: &Runtime) {
        let retention = runtime.retention.as_ref().unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while !retention.shutdown.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }

    fn wait_job(manager: &JobManager, id: JobId) {
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }

    #[test]
    fn selection_submission_honors_manager_shutdown() -> Result {
        let (mut app, mut runtime, root, _) = cached_projection_fixture()?;
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll)?;
        let token = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        // Isolate the independent selection path from a redundant projection of the same cached view.
        runtime.sessions.get_mut(&root).unwrap().project_applied = Some((token, 0));
        wait_retention_started(&runtime);
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .map_err(BevyError::error)?;
        wait_retention_stop(&runtime);
        runtime.update(app.world_mut(), &Default::default())?;
        assert!(
            matches!(app.world().get::<WidgetryFileDialogState>(root).unwrap().directory_state(),
            WidgetryFileDialogDirectoryState::Failed(error) if error.contains("stopped"))
        );
        assert!(
            !app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .selection_pending()
        );
        Ok(())
    }

    #[test]
    fn folder_item_waits_for_finished_and_retired_session_cannot_refresh_the_new_one() -> Result {
        for reopen in [false, true] {
            let (mut app, mut runtime, root, _) = cached_projection_fixture()?;
            WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::NewFolder("created".into()),
            )?;
            let request = app
                .world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .folder_request()
                .unwrap();
            let token = request.token();
            let manager = app.world().resource::<JobManager>().clone();
            let (id, receiver) = manager
                .spawn_io::<WidgetryFileDialogReply, _, _, String>(
                    Default::default(),
                    |context| async move {
                        context
                            .send(WidgetryFileDialogReply::FolderCreated {
                                request,
                                outcome: Ok(()),
                            })
                            .await
                            .map_err(|error| error.to_string())
                    },
                )
                .map_err(BevyError::error)?;
            let session = runtime.sessions.get_mut(&root).unwrap();
            session.folder_submitted = Some(token);
            session.folder = Some(PreparedJob {
                manager: manager.clone(),
                id,
                receiver,
                pending: None,
            });
            wait_job(&manager, id);
            assert!(runtime.poll_folder(app.world_mut())?);
            assert!(
                app.world()
                    .get::<WidgetryFileDialogState>(root)
                    .unwrap()
                    .folder_request()
                    .is_some()
            );
            if reopen {
                WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)?;
                WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Reopen)?;
            }
            let before = app
                .world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .token();
            assert!(runtime.poll_folder(app.world_mut())?);
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            assert!(state.folder_request().is_none());
            if reopen {
                assert_eq!(state.token(), before);
                assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Completed));
                runtime.reap_retired()?;
            } else {
                assert_ne!(state.token().generation, before.generation);
                assert_eq!(
                    state.directory_state(),
                    &WidgetryFileDialogDirectoryState::Loading
                );
            }
            assert_eq!(
                manager.status(id),
                Err(bevy_widgetry_job::JobAccessError::NotFound)
            );
        }
        Ok(())
    }

    #[test]
    fn locations_wait_for_finished_and_only_commit_to_the_same_directory_session() -> Result {
        for change in 0..5 {
            let (mut app, mut runtime, root, _) = cached_projection_fixture()?;
            let token = app
                .world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .token();
            let old = WidgetryFileDialogLocation {
                name: "Old".into(),
                path: PathBuf::from("C:/old"),
            };
            WidgetryFileDialog::deliver(
                app.world_mut(),
                root,
                WidgetryFileDialogReply::Locations {
                    token,
                    outcome: Ok(vec![old.clone()]),
                },
            )?;
            let manager = app.world().resource::<JobManager>().clone();
            let (id, receiver) = manager
                .spawn_io::<WidgetryFileDialogReply, _, _, String>(
                    Default::default(),
                    move |context| async move {
                        context
                            .send(WidgetryFileDialogReply::Locations {
                                token,
                                outcome: if change == 4 {
                                    Err("controlled location error".into())
                                } else {
                                    Ok(vec![WidgetryFileDialogLocation {
                                        name: "New".into(),
                                        path: PathBuf::from("C:/new"),
                                    }])
                                },
                            })
                            .await
                            .map_err(|error| error.to_string())?;
                        if change == 4 {
                            Err("controlled location error".into())
                        } else {
                            Ok(())
                        }
                    },
                )
                .map_err(BevyError::error)?;
            runtime.sessions.get_mut(&root).unwrap().locations = Some(PreparedJob {
                manager: manager.clone(),
                id,
                receiver,
                pending: None,
            });
            wait_job(&manager, id);
            assert!(runtime.poll_locations(app.world_mut())?);
            assert_eq!(
                app.world()
                    .get::<WidgetryFileDialogState>(root)
                    .unwrap()
                    .locations(),
                std::slice::from_ref(&old)
            );
            match change {
                1 => {
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Search("one".into()),
                    )?;
                }
                2 => {
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Refresh),
                    )?;
                }
                3 => {
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Cancel,
                    )?;
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Reopen,
                    )?;
                }
                _ => {}
            }
            assert!(runtime.poll_locations(app.world_mut())?);
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            if change < 2 {
                assert_eq!(state.locations()[0].name, "New");
            } else {
                assert!(
                    !state
                        .locations()
                        .iter()
                        .any(|location| location.name == "New")
                );
            }
            if change == 4 {
                assert_eq!(state.locations(), &[old]);
                assert_eq!(state.location_error(), Some("controlled location error"));
                assert_eq!(
                    state.directory_state(),
                    &WidgetryFileDialogDirectoryState::Ready
                );
            }
            runtime.reap_retired()?;
            assert_eq!(
                manager.status(id),
                Err(bevy_widgetry_job::JobAccessError::NotFound)
            );
        }
        Ok(())
    }

    #[test]
    fn preferences_wait_for_finished_and_old_save_ack_does_not_clean_a_new_revision() -> Result {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        app.world_mut()
            .resource_mut::<WidgetryFileDialogStorage>()
            .import("open".into(), Default::default())?;
        let revision = app
            .world()
            .resource::<WidgetryFileDialogStorage>()
            .revision("open")
            .unwrap();
        let revisions = BTreeMap::from([("open".into(), revision)]);
        let manager = app.world().resource::<JobManager>().clone();
        let sent = revisions.clone();
        let (id, receiver) = manager
            .spawn_io::<PreferencesReply, _, _, String>(
                Default::default(),
                move |context| async move {
                    context
                        .send(PreferencesReply::Saved(sent, Ok(())))
                        .await
                        .map_err(|error| error.to_string())
                },
            )
            .map_err(BevyError::error)?;
        wait_job(&manager, id);
        let mut runtime = Runtime::default();
        runtime.saving = true;
        runtime.disk_revision = 7;
        runtime.save_revisions = Some(revisions);
        runtime.preference_save = Some(PreparedJob {
            manager: manager.clone(),
            id,
            receiver,
            pending: None,
        });
        let options = WidgetryFileDialogRuntimeOptions {
            max_messages_per_update: 1,
            ..Default::default()
        };
        runtime.update(app.world_mut(), &options)?;
        assert_eq!(
            app.world()
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .applied_last_update,
            1
        );
        assert_eq!(
            app.world()
                .resource::<WidgetryFileDialogStorage>()
                .saved_revision("open"),
            Some(0)
        );
        assert!(runtime.saving);
        app.world_mut()
            .resource_mut::<WidgetryFileDialogStorage>()
            .import(
                "open".into(),
                WidgetryFileDialogStorageSnapshot {
                    show_hidden: true,
                    ..Default::default()
                },
            )?;
        runtime.update(app.world_mut(), &options)?;
        let store = app.world().resource::<WidgetryFileDialogStorage>();
        assert_eq!(store.saved_revision("open"), Some(revision));
        assert!(store.revision("open").unwrap() > revision);
        assert!(store.snapshot("open").unwrap().show_hidden);
        assert!(!runtime.saving);
        assert_eq!(
            app.world()
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .persistence,
            WidgetryFileDialogStorageState::Saved(7)
        );
        assert_eq!(
            manager.status(id),
            Err(bevy_widgetry_job::JobAccessError::NotFound)
        );
        Ok(())
    }

    #[test]
    fn persistence_submission_honors_manager_shutdown() -> Result {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        let directory = tempfile::tempdir().map_err(BevyError::error)?;
        app.insert_resource(WidgetryFileDialogPersistence {
            path: directory.path().join("prefs.json"),
        });
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .map_err(BevyError::error)?;
        let mut runtime = Runtime::default();
        runtime.update(app.world_mut(), &Default::default())?;
        assert!(
            matches!(&app.world().resource::<WidgetryFileDialogRuntimeStatus>().persistence,
            WidgetryFileDialogStorageState::Failed(error) if error.contains("ShuttingDown"))
        );
        Ok(())
    }

    #[test]
    fn locations_submission_honors_manager_shutdown() -> Result {
        let (mut app, mut runtime, root, _) = cached_projection_fixture()?;
        runtime.sessions.get_mut(&root).unwrap().locations_admitted = false;
        wait_retention_started(&runtime);
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .map_err(BevyError::error)?;
        wait_retention_stop(&runtime);
        runtime.update(app.world_mut(), &Default::default())?;
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert!(
            state
                .location_error()
                .is_some_and(|error| error.contains("stopped"))
        );
        assert!(matches!(
            state.directory_state(),
            WidgetryFileDialogDirectoryState::Failed(_)
        ));
        assert!(state.error().is_some_and(|error| error.contains("stopped")));
        Ok(())
    }

    #[test]
    fn folder_submission_honors_manager_shutdown() -> Result {
        let (mut app, mut runtime, root, _) = cached_projection_fixture()?;
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::NewFolder("created".into()),
        )?;
        wait_retention_started(&runtime);
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .map_err(BevyError::error)?;
        wait_retention_stop(&runtime);
        runtime.update(app.world_mut(), &Default::default())?;
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert!(state.folder_request().is_none());
        assert!(state.error().is_some_and(|error| error.contains("stopped")));
        assert!(matches!(
            state.directory_state(),
            WidgetryFileDialogDirectoryState::Failed(_)
        ));
        Ok(())
    }

    #[test]
    fn candidate_validation_submission_honors_manager_shutdown() -> Result {
        let (mut app, mut runtime, root, _) =
            cached_projection_fixture_for(WidgetryFileDialogMode::SaveFile)?;
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Filename("new.txt".into()),
        )?;
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)?;
        wait_retention_started(&runtime);
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .map_err(BevyError::error)?;
        wait_retention_stop(&runtime);
        runtime.update(app.world_mut(), &Default::default())?;
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
        assert!(state.error().is_some_and(|error| error.contains("stopped")));
        assert!(matches!(
            state.directory_state(),
            WidgetryFileDialogDirectoryState::Failed(_)
        ));
        Ok(())
    }

    #[test]
    fn candidate_result_waits_for_finished_and_cannot_overwrite_filename_or_reopened_session()
    -> Result {
        for reopen in [false, true] {
            let (mut app, mut runtime, root, _) =
                cached_projection_fixture_for(WidgetryFileDialogMode::SaveFile)?;
            WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::Filename("old.txt".into()),
            )?;
            WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)?;
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            let token = state.token();
            let candidate = state
                .validation_job
                .clone()
                .unwrap()
                .prepare()
                .map_err(BevyError::error)?;
            let manager = app.world().resource::<JobManager>().clone();
            let (id, receiver) = manager
                .spawn_io::<WidgetryFileDialogReply, _, _, String>(
                    Default::default(),
                    |context| async move {
                        context
                            .send(WidgetryFileDialogReply::Validated {
                                candidate,
                                exists: true,
                            })
                            .await
                            .map_err(|error| error.to_string())
                    },
                )
                .map_err(BevyError::error)?;
            let session = runtime.sessions.get_mut(&root).unwrap();
            session.validation_submitted = Some(token);
            session.validation = Some(PreparedJob {
                manager: manager.clone(),
                id,
                receiver,
                pending: None,
            });
            wait_job(&manager, id);
            assert!(runtime.poll_validation(app.world_mut())?);
            assert_eq!(
                app.world()
                    .get::<WidgetryFileDialogState>(root)
                    .unwrap()
                    .confirmation(),
                &WidgetryFileDialogConfirmation::Validating
            );
            if reopen {
                WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)?;
                WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Reopen)?;
            } else {
                WidgetryFileDialog::apply(
                    app.world_mut(),
                    root,
                    WidgetryFileDialogAction::Filename("new.txt".into()),
                )?;
            }
            assert!(runtime.poll_validation(app.world_mut())?);
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
            assert_eq!(state.session_state(), WidgetryFileDialogSessionState::Open);
            if !reopen {
                assert_eq!(state.filename(), std::ffi::OsStr::new("new.txt"));
            }
            assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Completed));
            runtime.reap_retired()?;
            assert_eq!(
                manager.status(id),
                Err(bevy_widgetry_job::JobAccessError::NotFound)
            );
        }
        Ok(())
    }

    #[test]
    fn validation_without_a_delivered_item_recovers_from_late_cancel_or_panic() -> Result {
        for panicked in [false, true] {
            let (mut app, mut runtime, root, _) =
                cached_projection_fixture_for(WidgetryFileDialogMode::SaveFile)?;
            WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::Filename("new.txt".into()),
            )?;
            WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)?;
            let token = app
                .world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .token();
            let manager = app.world().resource::<JobManager>().clone();
            let (entered, ready) = std::sync::mpsc::channel();
            let (release, released) = std::sync::mpsc::channel();
            let (id, receiver) = manager
                .spawn_thread::<WidgetryFileDialogReply, _, String>(
                    Default::default(),
                    move |_context| {
                        entered.send(()).unwrap();
                        released.recv().unwrap();
                        if panicked {
                            panic!("controlled validation panic");
                        }
                        Ok(())
                    },
                )
                .map_err(BevyError::error)?;
            let session = runtime.sessions.get_mut(&root).unwrap();
            session.validation_submitted = Some(token);
            session.validation = Some(PreparedJob {
                manager: manager.clone(),
                id,
                receiver,
                pending: None,
            });
            ready
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            if !panicked {
                manager.cancel(id).unwrap();
            }
            release.send(()).unwrap();
            wait_job(&manager, id);
            assert!(runtime.poll_validation(app.world_mut())?);
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
            assert!(state.error().is_some());
            assert_eq!(
                state.directory_state(),
                &WidgetryFileDialogDirectoryState::Ready
            );
            assert_eq!(
                manager.status(id),
                Err(bevy_widgetry_job::JobAccessError::NotFound)
            );
        }
        Ok(())
    }

    #[test]
    fn filename_edits_do_not_cancel_pending_savefile_active_preparation() -> Result {
        for movement in [false, true] {
            let (mut app, mut runtime, root, previous) =
                cached_projection_fixture_for(WidgetryFileDialogMode::SaveFile)?;
            let id = previous.visible()[0];
            let action = if movement {
                WidgetryFileDialogAction::MoveActive(WidgetryFileDialogMove::First)
            } else {
                WidgetryFileDialogAction::Active(Some(id))
            };
            WidgetryFileDialog::apply(app.world_mut(), root, action)?;
            runtime.update(app.world_mut(), &Default::default())?;
            let job_id = runtime.sessions[&root].selection.as_ref().unwrap().id;
            let manager = app.world().resource::<JobManager>().clone();
            wait_job(&manager, job_id);
            assert!(runtime.poll_selection(app.world_mut())?);
            WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::Filename("edited.txt".into()),
            )?;
            assert!(runtime.poll_selection(app.world_mut())?);
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            assert!(
                !state.selection_pending(),
                "Filename-only edit must not strand the pending active intent"
            );
            assert_eq!(state.active(), Some(id));
            assert_eq!(state.filename(), std::ffi::OsStr::new("edited.txt"));
            assert!(state.selected().is_empty());
            assert!(runtime.sessions[&root].selection.is_none());
            assert!(WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::Confirm
            )?);
        }
        Ok(())
    }

    #[test]
    fn selection_intents_use_a_cpu_job_and_commit_in_order_after_finished() -> Result {
        let (mut app, mut runtime, root, previous) = cached_projection_fixture()?;
        let first = previous.entries()[0].id();
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll)?;
        let token = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Select {
                id: first,
                token,
                operation: WidgetryFileDialogSelection::Toggle,
            },
        )?;
        runtime.update(app.world_mut(), &Default::default())?;
        assert!(
            runtime.sessions[&root].projection.is_none(),
            "selection changes must not launch a redundant projection"
        );
        let id = runtime.sessions[&root].selection.as_ref().unwrap().id;
        let manager = app.world().resource::<JobManager>().clone();
        assert_eq!(
            manager.status(id).unwrap().kind,
            bevy_widgetry_job::JobKind::Cpu
        );
        wait_job(&manager, id);
        assert!(runtime.poll_selection(app.world_mut())?);
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert!(state.selection_pending());
        assert!(state.selected().is_empty());
        assert!(runtime.poll_selection(app.world_mut())?);
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert!(!state.selection_pending());
        assert_eq!(state.selected().len(), 1);
        assert!(!state.selected().contains(&first));
        assert_eq!(state.active(), Some(first));
        assert!(Arc::strong_count(&state.selected) >= 2);
        assert_eq!(
            manager.status(id),
            Err(bevy_widgetry_job::JobAccessError::NotFound)
        );
        Ok(())
    }

    #[test]
    fn obsolete_selection_intent_query_and_reopened_session_discard_completed_jobs() -> Result {
        for change in 0..3 {
            let (mut app, mut runtime, root, _) = cached_projection_fixture()?;
            WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll)?;
            runtime.update(app.world_mut(), &Default::default())?;
            let id = runtime.sessions[&root].selection.as_ref().unwrap().id;
            let manager = app.world().resource::<JobManager>().clone();
            wait_job(&manager, id);
            match change {
                0 => {
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::ClearSelection,
                    )?;
                }
                1 => {
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Search("one".into()),
                    )?;
                }
                _ => {
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Cancel,
                    )?;
                    assert_eq!(
                        app.world()
                            .get::<WidgetryFileDialogState>(root)
                            .unwrap()
                            .session_state(),
                        WidgetryFileDialogSessionState::Resolved
                    );
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Reopen,
                    )?;
                }
            }
            assert!(runtime.poll_selection(app.world_mut())?);
            assert!(runtime.sessions[&root].selection.is_none());
            assert!(
                app.world()
                    .get::<WidgetryFileDialogState>(root)
                    .unwrap()
                    .selected()
                    .is_empty()
            );
            assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Completed));
            runtime.reap_retired()?;
            assert_eq!(
                manager.status(id),
                Err(bevy_widgetry_job::JobAccessError::NotFound)
            );
        }
        Ok(())
    }

    #[test]
    fn cpu_projection_waits_for_finished_consumption_and_retains_its_output() -> Result {
        let (mut app, mut runtime, root, previous) = cached_projection_fixture()?;
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Search("one".into()),
        )?;
        runtime.update(app.world_mut(), &Default::default())?;
        let id = runtime.sessions[&root].projection.as_ref().unwrap().id;
        let manager = app.world().resource::<JobManager>().clone();
        assert_eq!(
            manager.status(id).unwrap().kind,
            bevy_widgetry_job::JobKind::Cpu
        );
        wait_job(&manager, id);
        assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Completed));
        assert!(runtime.poll_projection(app.world_mut())?);
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert!(state.projection_pending());
        assert!(Arc::ptr_eq(state.snapshot().unwrap(), &previous));
        assert!(runtime.poll_projection(app.world_mut())?);
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert!(!state.projection_pending());
        assert_eq!(state.visible().len(), 1);
        assert!(Arc::ptr_eq(
            &state.snapshot().unwrap().entries,
            &previous.entries
        ));
        assert!(Arc::strong_count(state.snapshot().unwrap()) >= 2);
        assert_eq!(
            manager.status(id),
            Err(bevy_widgetry_job::JobAccessError::NotFound)
        );
        assert!(runtime.sessions[&root].project_inflight.is_none());
        Ok(())
    }

    #[test]
    fn completed_projection_cannot_overwrite_a_new_query_version_or_session() -> Result {
        for change in 0..3 {
            let (mut app, mut runtime, root, previous) = cached_projection_fixture()?;
            WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::Search("one".into()),
            )?;
            runtime.update(app.world_mut(), &Default::default())?;
            let id = runtime.sessions[&root].projection.as_ref().unwrap().id;
            let manager = app.world().resource::<JobManager>().clone();
            wait_job(&manager, id);
            match change {
                0 => {
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Search("two".into()),
                    )?;
                }
                1 => {
                    runtime.sessions.get_mut(&root).unwrap().version += 1;
                }
                _ => {
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Cancel,
                    )?;
                    WidgetryFileDialog::apply(
                        app.world_mut(),
                        root,
                        WidgetryFileDialogAction::Reopen,
                    )?;
                }
            }
            assert!(runtime.poll_projection(app.world_mut())?);
            assert!(runtime.sessions[&root].projection.is_none());
            assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Completed));
            if change < 2 {
                assert!(Arc::ptr_eq(
                    app.world()
                        .get::<WidgetryFileDialogState>(root)
                        .unwrap()
                        .snapshot()
                        .unwrap(),
                    &previous
                ));
            } else {
                assert!(
                    app.world()
                        .get::<WidgetryFileDialogState>(root)
                        .unwrap()
                        .snapshot()
                        .is_none()
                );
            }
            runtime.reap_retired()?;
            assert_eq!(
                manager.status(id),
                Err(bevy_widgetry_job::JobAccessError::NotFound)
            );
        }
        Ok(())
    }

    #[test]
    fn cached_projection_submission_honors_manager_shutdown() -> Result {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog })?
            .id();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let mut session = Session::new(root, state.token());
        session.admitted = true;
        session.locations_admitted = true;
        session.cached = Some((
            Arc::new(WidgetryFileDialogSnapshot::prepare(
                state.token(),
                PathBuf::from("C:/fixture"),
                vec![],
                &state.query(),
                None,
            )?),
            WidgetryFileDialogDirectoryState::Ready,
        ));
        let service =
            Retention::start(app.world().resource::<JobManager>()).map_err(BevyError::error)?;
        let mut runtime = Runtime::default();
        runtime.retention = Some(service);
        runtime.sessions.insert(root, session);
        wait_retention_started(&runtime);
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .map_err(BevyError::error)?;
        wait_retention_stop(&runtime);
        runtime.update(app.world_mut(), &Default::default())?;
        assert!(
            matches!(app.world().get::<WidgetryFileDialogState>(root).unwrap().directory_state(),
            WidgetryFileDialogDirectoryState::Failed(error) if error.contains("stopped"))
        );
        assert!(
            !app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .projection_pending()
        );
        Ok(())
    }

    #[test]
    fn session_stop_registers_job_cancellation_before_publishing_legacy_flag() -> Result {
        struct ObserveCancel {
            control: Arc<AtomicBool>,
            observed: Arc<AtomicBool>,
        }
        impl std::task::Wake for ObserveCancel {
            fn wake(self: Arc<Self>) {
                self.observed
                    .store(self.control.load(Ordering::Acquire), Ordering::Release);
            }
        }

        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let manager = JobManager::default();
        let state = WidgetryFileDialogState::new(WidgetryFileDialogProps::default())?;
        let mut session = Session::new(Entity::PLACEHOLDER, state.token());
        let observed = Arc::new(AtomicBool::new(true));
        let observer = Arc::new(ObserveCancel {
            control: session.control.cancelled.clone(),
            observed: observed.clone(),
        });
        let (registered, ready) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();
        let (id, receiver) = manager
            .spawn_thread::<DirectoryEvent, _, String>(Default::default(), move |context| {
                let mut cancelled = Box::pin(context.cancelled());
                let waker = std::task::Waker::from(observer);
                assert!(
                    std::future::Future::poll(
                        cancelled.as_mut(),
                        &mut std::task::Context::from_waker(&waker),
                    )
                    .is_pending()
                );
                registered.send(()).unwrap();
                released.recv().unwrap();
                bevy::tasks::block_on(cancelled);
                assert!(context.acknowledge_cancel());
                Ok(())
            })
            .map_err(BevyError::error)?;
        ready
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        session.directory = Some(DirectoryJob {
            manager: manager.clone(),
            id,
            receiver,
            final_snapshot: None,
        });
        let mut retired = Vec::new();
        session.stop(&mut retired)?;
        release.send(()).unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(session.control.cancelled.load(Ordering::Acquire));
        assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Canceled));
        assert!(
            !observed.load(Ordering::Acquire),
            "legacy cancellation became visible before Manager cancellation"
        );
        manager.remove(id).unwrap();
        Ok(())
    }

    #[test]
    fn service_fault_cancels_a_directory_with_a_full_result_buffer() -> Result {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog })?
            .id();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let mut session = Session::new(root, state.token());
        let manager = JobManager::with_buffer_capacity(1).unwrap();
        let service =
            Retention::start(app.world().resource::<JobManager>()).map_err(BevyError::error)?;
        let snapshot = Arc::new(WidgetryFileDialogSnapshot::prepare(
            state.token(),
            PathBuf::from("C:/fixture"),
            vec![],
            &state.query(),
            None,
        )?);
        let (blocked, waiting) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();
        let (id, receiver) = manager
            .spawn_thread::<DirectoryEvent, _, String>(Default::default(), move |context| {
                context
                    .send_blocking(DirectoryEvent::Started(PathBuf::from("C:/fixture")))
                    .map_err(|error| error.to_string())?;
                let mut send = Box::pin(context.send(DirectoryEvent::Snapshot {
                    snapshot,
                    state: WidgetryFileDialogDirectoryState::Ready,
                    finished: true,
                }));
                assert!(
                    std::future::Future::poll(
                        send.as_mut(),
                        &mut std::task::Context::from_waker(std::task::Waker::noop())
                    )
                    .is_pending()
                );
                blocked.send(()).unwrap();
                released.recv().unwrap();
                match bevy::tasks::block_on(send) {
                    Err(bevy_widgetry_job::SendError::Canceled) => {
                        assert!(context.acknowledge_cancel());
                        Ok(())
                    }
                    result => result.map_err(|error| error.to_string()),
                }
            })
            .map_err(BevyError::error)?;
        session.admitted = true;
        session.directory = Some(DirectoryJob {
            manager: manager.clone(),
            id,
            receiver,
            final_snapshot: None,
        });
        let mut runtime = Runtime::default();
        runtime.retention = Some(service.clone());
        runtime.sessions.insert(root, session);
        waiting
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(!manager.status(id).unwrap().state.is_finished());
        *service.fault.lock().unwrap() = Some("worker failed".into());
        service.stop();
        assert!(
            runtime
                .update(app.world_mut(), &Default::default())
                .is_err()
        );
        release.send(()).unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Canceled));
        assert!(runtime.sessions[&root].directory.is_none());
        runtime.update(app.world_mut(), &Default::default())?;
        assert_eq!(
            manager.status(id),
            Err(bevy_widgetry_job::JobAccessError::NotFound)
        );
        assert_eq!(
            app.world()
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .error
                .as_deref(),
            Some("worker failed")
        );
        Ok(())
    }

    #[test]
    fn completed_execution_does_not_publish_the_final_snapshot_before_finished_is_consumed()
    -> Result {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog })?
            .id();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let snapshot = Arc::new(WidgetryFileDialogSnapshot::prepare(
            state.token(),
            PathBuf::from("C:/fixture"),
            vec![],
            &state.query(),
            None,
        )?);
        let manager = app.world().resource::<JobManager>().clone();
        let mut session = Session::new(root, state.token());
        let (id, receiver) = manager
            .spawn_io::<DirectoryEvent, _, _, String>(
                Default::default(),
                move |context| async move {
                    context
                        .send(DirectoryEvent::Snapshot {
                            snapshot,
                            state: WidgetryFileDialogDirectoryState::Ready,
                            finished: true,
                        })
                        .await
                        .map_err(|error| error.to_string())?;
                    Ok(())
                },
            )
            .map_err(BevyError::error)?;
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        session.directory = Some(DirectoryJob {
            manager: manager.clone(),
            id,
            receiver,
            final_snapshot: None,
        });
        let mut runtime = Runtime::default();
        runtime.sessions.insert(root, session);
        assert!(runtime.poll_directory(app.world_mut())?);
        assert!(runtime.sessions[&root].cached.is_none());
        assert!(runtime.poll_directory(app.world_mut())?);
        assert_eq!(
            runtime.sessions[&root]
                .cached
                .as_ref()
                .map(|(_, state)| state),
            Some(&WidgetryFileDialogDirectoryState::Ready)
        );
        assert!(runtime.sessions[&root].directory.is_none());
        assert_eq!(
            manager.status(id),
            Err(bevy_widgetry_job::JobAccessError::NotFound)
        );
        Ok(())
    }

    #[test]
    fn late_load_preserves_returned_to_initial_value_and_unpin_intents() -> Result {
        let pinned = PathBuf::from("C:/fixture/pin");
        let base = WidgetryFileDialogStorageSnapshot::default();
        let edited = WidgetryFileDialogStorageSnapshot {
            show_hidden: true,
            pinned: vec![pinned],
            ..Default::default()
        };
        let mut store = WidgetryFileDialogStorage::default();
        let record = store.prepare_merge("open", &base, &edited, false, false, None)?;
        store.scopes.insert("open".into(), record);
        let record = store.prepare_merge("open", &edited, &base, false, false, None)?;
        store.scopes.insert("open".into(), record);
        let merged = merge_loaded(&store, BTreeMap::from([("open".into(), edited)]))?;
        let snapshot = merged
            .snapshot("open")
            .ok_or_else(|| BevyError::error("scope missing"))?;
        assert!(!snapshot.show_hidden);
        assert!(snapshot.pinned.is_empty());
        Ok(())
    }

    #[test]
    fn failed_load_suppresses_automatic_save_of_current_revision() -> Result {
        let mut world = World::new();
        world.init_resource::<WidgetryFileDialogStorage>();
        world.init_resource::<WidgetryFileDialogRuntimeStatus>();
        world.resource_mut::<WidgetryFileDialogStorage>().import(
            "open".into(),
            WidgetryFileDialogStorageSnapshot {
                show_hidden: true,
                ..Default::default()
            },
        )?;
        let mut runtime = Runtime::default();
        runtime.receive(&mut world, Output::Loaded(Err("unsupported schema".into())))?;
        assert_eq!(
            runtime
                .failed_revisions
                .as_ref()
                .and_then(|revisions| revisions.get("open"))
                .copied(),
            world
                .resource::<WidgetryFileDialogStorage>()
                .revision("open")
        );
        assert!(matches!(
            world
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .persistence,
            WidgetryFileDialogStorageState::Failed(_)
        ));
        Ok(())
    }

    #[test]
    fn failed_late_load_merge_preserves_committed_preferences() -> Result {
        let prefix = "C:/fixture";
        let base = WidgetryFileDialogStorageSnapshot::default();
        let local = WidgetryFileDialogStorageSnapshot {
            pinned: vec![PathBuf::from(prefix).join("local")],
            show_hidden: true,
            ..Default::default()
        };
        let mut store = WidgetryFileDialogStorage::default();
        let record = store.prepare_merge("open", &base, &local, false, false, None)?;
        store.scopes.insert("open".into(), record);
        let before = store.export();
        let before_revision = store.revision("open");
        let disk = WidgetryFileDialogStorageSnapshot {
            pinned: (0..1024)
                .map(|index| PathBuf::from(prefix).join(index.to_string()))
                .collect(),
            ..Default::default()
        };
        let mut world = World::new();
        world.insert_resource(store);
        world.init_resource::<WidgetryFileDialogRuntimeStatus>();
        Runtime::default().receive(
            &mut world,
            Output::Loaded(Ok(BTreeMap::from([("open".into(), disk)]))),
        )?;
        assert_eq!(
            world.resource::<WidgetryFileDialogStorage>().export(),
            before
        );
        assert_eq!(
            world
                .resource::<WidgetryFileDialogStorage>()
                .revision("open"),
            before_revision
        );
        assert!(matches!(
            world
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .persistence,
            WidgetryFileDialogStorageState::Failed(_)
        ));
        Ok(())
    }
}
