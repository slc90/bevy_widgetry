use crate::directory::DirectoryEvent;
use crate::preferences_job::Reply as PreferencesReply;
use crate::projection::ProjectionResult;
use crate::retention::{Control, Retention};
use crate::*;
use bevy::prelude::*;
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use bevy_widgetry_job::{JobId, JobManager, JobOutcome, JobRead, JobReceiver};
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
    start_failure: Option<bevy_widgetry_job::JobFailure>,
    persistence_path: Option<PathBuf>,
    load_admitted: bool,
    loaded: bool,
    saving: bool,
    preference_load: Option<PreparedJob<PreferencesReply>>,
    preference_save: Option<PreparedJob<PreferencesReply>>,
    save_revisions: Option<BTreeMap<String, u64>>,
    failed_revisions: Option<BTreeMap<String, u64>>,
    preference_failure:
        bevy_widgetry_diagnostics::bevy::FailureState<bevy_widgetry_diagnostics::ErrorId>,
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
    Loaded(Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>, FileDialogRequestError>),
    Saved(BTreeMap<String, u64>, Result<(), FileDialogRequestError>),
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

fn outcome_failure(outcome: &JobOutcome) -> Option<bevy_widgetry_job::JobFailure> {
    match outcome {
        JobOutcome::Failed(error) => Some(error.clone()),
        _ => None,
    }
}

fn combine_failure(
    failures: &mut Option<bevy_widgetry_job::JobFailure>,
    failure: impl Into<bevy_widgetry_job::JobFailure>,
) {
    let failure = failure.into();
    *failures = Some(match failures.take() {
        Some(previous) => previous.combine(failure),
        None => failure,
    });
}

fn finish_job_delivery(result: Result, failure: Option<bevy_widgetry_job::JobFailure>) -> Result {
    match (result, failure) {
        (Ok(()), Some(error)) => {
            error.record_handled(
                "终止 FileDialog 本次任务并消费失败结果",
                "本次后台请求已结束",
            );
            Ok(())
        }
        (Err(delivery), Some(error)) => Err(error.combine(delivery.into()).into_bevy_error()),
        (result, None) => result,
    }
}

impl<T: Clone> PreparedJob<T> {
    fn poll(
        &mut self,
        frame: impl FnOnce() -> bevy_widgetry_diagnostics::ErrorFrame,
    ) -> Result<PreparedPoll<T>> {
        let result = (|| match self.receiver.try_recv() {
            Ok(JobRead::Item(result)) => {
                self.pending = Some((*result).clone());
                Ok(PreparedPoll::Item)
            }
            Ok(JobRead::Finished(outcome)) => {
                if let Err(error) = self.manager.remove(self.id) {
                    let access = bevy_widgetry_job::JobFailure::from(error);
                    let failure = match outcome {
                        JobOutcome::Failed(error) => error.combine(access),
                        _ => access,
                    };
                    return Err(failure.into_bevy_error());
                }
                Ok(PreparedPoll::Finished(outcome, self.pending.take()))
            }
            Ok(JobRead::Empty) => Ok(PreparedPoll::Empty),
            Ok(JobRead::Closed) => Err(crate::error::internal_error(
                FileDialogInternalFault::ReceiverClosed {
                    operation: "prepared",
                },
            )),
            Err(error) => Err(error.into_bevy_error()),
        })();
        result.map_err(|error| {
            crate::error::boundary_context(
                error,
                frame().field("job", format_args!("{:?}", self.id)),
            )
        })
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

pub(crate) fn wake(world: &World) -> Result<(), FileDialogFailure> {
    if world
        .get_resource::<WidgetryFileDialogRuntimeOptions>()
        .is_some_and(|options| !options.automatic)
    {
        return Ok(());
    }
    if let Some(proxy) = world.get_resource::<EventLoopProxyWrapper>() {
        proxy.send_event(WinitUserEvent::WakeUp).map_err(|source| {
            crate::error::failure(
                FileDialogError::Wake {
                    source: Box::new(source),
                },
                "file_dialog.wake",
            )
        })?;
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
        return Err(crate::error::internal_error(
            FileDialogInternalFault::RuntimeOptions {
                max_sessions: options.max_sessions,
                max_messages: options.max_messages_per_update,
                budget: options.update_budget,
            },
        ));
    }
    let mut runtime = world.remove_resource::<Runtime>().ok_or_else(|| {
        crate::error::internal_error(FileDialogInternalFault::Missing { part: "runtime" })
    })?;
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
        if self.retention.is_none() && self.start_failure.is_none() && !states.is_empty() {
            match Retention::start(world.resource::<JobManager>()) {
                Ok(service) => self.retention = Some(service),
                Err(error) => {
                    let error = bevy_widgetry_job::JobFailure::from(error).with_context(
                        bevy_widgetry_diagnostics::ErrorFrame::new("file_dialog.retention.start"),
                    );
                    world
                        .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                        .error = Some(error.to_string());
                    self.start_failure = Some(error);
                }
            }
        }
        let Some(service) = self.retention.clone() else {
            if let Some(error) = &self.start_failure {
                let mut failures = None;
                for (root, token, failed) in states {
                    if failed {
                        continue;
                    }
                    if let Err(delivery) = WidgetryFileDialog::deliver(
                        world,
                        root,
                        WidgetryFileDialogReply::Failed {
                            token,
                            error: error.to_string(),
                        },
                    ) {
                        combine_failure(&mut failures, delivery);
                    }
                }
                if let Some(failures) = failures {
                    return Err(error.clone().combine(failures).into_bevy_error());
                }
                error.record_handled(
                    "停止后台服务启动并提交所有会话失败状态",
                    "FileDialog 后台不可用，保留取消入口",
                );
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
                wake(world).map_err(FileDialogFailure::into_bevy_error)?;
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
                        let failure: bevy_widgetry_job::JobFailure = crate::error::failure(
                            FileDialogError::Capacity {
                                resource: "active sessions",
                                actual: self.sessions.len() + 1,
                                limit: options.max_sessions,
                            },
                            "file_dialog.session.admit",
                        )
                        .with_context(crate::error::owner_frame(
                            world,
                            *root,
                            *root,
                            "file_dialog.session.admit",
                        ))
                        .into();
                        let delivered = WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Failed {
                                token: *token,
                                error: failure.to_string(),
                            },
                        );
                        finish_job_delivery(
                            delivered
                                .map(|_| ())
                                .map_err(FileDialogOperationFailure::into_bevy_error),
                            Some(failure),
                        )?;
                    }
                    continue;
                }
                self.sessions.insert(*root, Session::new(*root, *token));
            }
        }
        if let Some(error) = service.error()? {
            let message = error.to_string();
            let mut failures = None;
            for session in self.sessions.values_mut() {
                if let Err(stop) = session.stop(&mut self.retired) {
                    combine_failure(&mut failures, stop);
                }
            }
            world
                .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                .error = Some(message.clone());
            for (root, token, failed) in &states {
                if !failed
                    && let Err(delivery) = WidgetryFileDialog::deliver(
                        world,
                        *root,
                        WidgetryFileDialogReply::Failed {
                            token: *token,
                            error: message.clone(),
                        },
                    )
                {
                    combine_failure(&mut failures, delivery);
                }
            }
            if let Some(failures) = failures {
                return Err(error.combine(failures).into_bevy_error());
            }
            error.record_handled(
                "停止所有会话任务并提交后台服务失败",
                "FileDialog 保留当前数据与取消入口",
            );
            return Ok(());
        }
        if service.shutdown.load(Ordering::Acquire) || !service.active() {
            let mut failures = None;
            for (root, token, _) in &states {
                if let Some(session) = self.sessions.get_mut(root) {
                    if let Err(error) = session.stop(&mut self.retired) {
                        combine_failure(&mut failures, error);
                    }
                    session.failed = true;
                }
                if let Err(error) = WidgetryFileDialog::deliver(
                    world,
                    *root,
                    WidgetryFileDialogReply::OperationsStopped {
                        token: *token,
                        error: "FileDialog background operations stopped during shutdown".into(),
                    },
                ) {
                    combine_failure(&mut failures, error);
                }
            }
            if let Some(error) = failures {
                return Err(error.into_bevy_error());
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
                        let error = bevy_widgetry_job::JobFailure::from(error)
                            .with_context(control.frame("file_dialog.directory.submit"));
                        let delivered = WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Failed {
                                token: control.token,
                                error: error.to_string(),
                            },
                        );
                        finish_job_delivery(
                            delivered
                                .map(|_| ())
                                .map_err(FileDialogOperationFailure::into_bevy_error),
                            Some(error),
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
                        let error = bevy_widgetry_job::JobFailure::from(error);
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Locations {
                                token: control.token,
                                outcome: Err(FileDialogRequestError::Failed(error.clone())),
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
                        let error = bevy_widgetry_job::JobFailure::from(error)
                            .with_context(control.frame("file_dialog.projection.submit"));
                        if let Err(stop) = session.stop(&mut self.retired) {
                            return Err(error.combine(stop.into()).into_bevy_error());
                        }
                        let delivered = WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Failed {
                                token: control.token,
                                error: error.to_string(),
                            },
                        );
                        finish_job_delivery(
                            delivered
                                .map(|_| ())
                                .map_err(FileDialogOperationFailure::into_bevy_error),
                            Some(error),
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
                        let error = bevy_widgetry_job::JobFailure::from(error)
                            .with_context(control.frame("file_dialog.selection.submit"));
                        if let Err(stop) = session.stop(&mut self.retired) {
                            return Err(error.combine(stop.into()).into_bevy_error());
                        }
                        let delivered = WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::Failed {
                                token: control.token,
                                error: error.to_string(),
                            },
                        );
                        finish_job_delivery(
                            delivered
                                .map(|_| ())
                                .map_err(FileDialogOperationFailure::into_bevy_error),
                            Some(error),
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
                        let error = bevy_widgetry_job::JobFailure::from(error);
                        session.validation_submitted = Some(job.token());
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::ValidationFailed {
                                token: job.token(),
                                error: FileDialogRequestError::Failed(error.clone()),
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
                        let error = bevy_widgetry_job::JobFailure::from(error);
                        WidgetryFileDialog::deliver(
                            world,
                            *root,
                            WidgetryFileDialogReply::FolderCreated {
                                request: request.clone(),
                                outcome: Err(FileDialogRequestError::Failed(error.clone())),
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
            wake(world).map_err(FileDialogFailure::into_bevy_error)?;
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
        let mut failures = None;
        for root in expired {
            if let Some(session) = self.sessions.get_mut(&root) {
                match session.stop(&mut self.retired) {
                    Ok(()) => {
                        self.sessions.remove(&root);
                    }
                    Err(error) => combine_failure(&mut failures, error),
                }
            }
        }
        failures.map_or(Ok(()), |error| Err(error.into_bevy_error()))
    }

    fn reap_retired(&mut self) -> Result {
        let mut failures = None;
        let mut index = 0;
        while index < self.retired.len() {
            let (manager, id) = &self.retired[index];
            let status = match manager.status(*id) {
                Ok(status) => status,
                Err(error) => {
                    combine_failure(
                        &mut failures,
                        bevy_widgetry_job::JobFailure::from(error).with_context(
                            bevy_widgetry_diagnostics::ErrorFrame::new(
                                "file_dialog.retired.status",
                            )
                            .field("job", format_args!("{:?}", id)),
                        ),
                    );
                    index += 1;
                    continue;
                }
            };
            if status.state.is_finished() {
                let status = match manager.remove(*id) {
                    Ok(status) => status,
                    Err(error) => {
                        let error = bevy_widgetry_job::JobFailure::from(error).with_context(
                            bevy_widgetry_diagnostics::ErrorFrame::new(
                                "file_dialog.retired.remove",
                            )
                            .field("job", format_args!("{:?}", id)),
                        );
                        combine_failure(
                            &mut failures,
                            match status.outcome {
                                Some(JobOutcome::Failed(failure)) => failure.combine(error),
                                _ => error,
                            },
                        );
                        index += 1;
                        continue;
                    }
                };
                if let Some(JobOutcome::Failed(error)) = status.outcome {
                    error.record_handled("回收已过期任务并放弃其结果", "FileDialog 旧请求已停止");
                }
                self.retired.swap_remove(index);
            } else {
                index += 1;
            }
        }
        failures.map_or(Ok(()), |error| Err(error.into_bevy_error()))
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
                .ok_or_else(|| {
                    crate::error::internal_error(FileDialogInternalFault::Missing {
                        part: "locations job",
                    })
                })?
                .poll(|| {
                    crate::error::owner_frame(world, root, root, "file_dialog.locations.poll")
                })? {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, pending) => {
                    let failure = outcome_failure(&outcome);
                    session.locations = None;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            pending.unwrap_or_else(|| WidgetryFileDialogReply::Locations {
                                token,
                                outcome: Err(FileDialogRequestError::Canceled(
                                    "locations result was canceled before delivery",
                                )),
                            })
                        }
                        JobOutcome::Failed(error) => WidgetryFileDialogReply::Locations {
                            token,
                            outcome: Err(FileDialogRequestError::Failed(error.clone())),
                        },
                        JobOutcome::Canceled => WidgetryFileDialogReply::Locations {
                            token,
                            outcome: Err(FileDialogRequestError::Canceled(
                                "locations query canceled",
                            )),
                        },
                    };
                    finish_job_delivery(
                        WidgetryFileDialog::deliver(world, root, reply)
                            .map(|_| ())
                            .map_err(FileDialogOperationFailure::into_bevy_error),
                        failure,
                    )?;
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
                .ok_or_else(|| {
                    crate::error::internal_error(FileDialogInternalFault::Missing {
                        part: "folder job",
                    })
                })?
                .poll(|| crate::error::owner_frame(world, root, root, "file_dialog.folder.poll"))?
            {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, pending) => {
                    let failure = outcome_failure(&outcome);
                    session.folder = None;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            pending.unwrap_or_else(|| WidgetryFileDialogReply::FolderCreated {
                                request,
                                outcome: Err(FileDialogRequestError::Canceled(
                                    "folder result was canceled before delivery",
                                )),
                            })
                        }
                        JobOutcome::Failed(error) => WidgetryFileDialogReply::FolderCreated {
                            request,
                            outcome: Err(FileDialogRequestError::Failed(error.clone())),
                        },
                        JobOutcome::Canceled => WidgetryFileDialogReply::FolderCreated {
                            request,
                            outcome: Err(FileDialogRequestError::Canceled(
                                "folder creation canceled",
                            )),
                        },
                    };
                    finish_job_delivery(
                        WidgetryFileDialog::deliver(world, root, reply)
                            .map(|_| ())
                            .map_err(FileDialogOperationFailure::into_bevy_error),
                        failure,
                    )?;
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
                .ok_or_else(|| {
                    crate::error::internal_error(FileDialogInternalFault::Missing {
                        part: "candidate validation job",
                    })
                })?
                .poll(|| {
                    crate::error::owner_frame(world, root, root, "file_dialog.validation.poll")
                })? {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, pending) => {
                    let failure = outcome_failure(&outcome);
                    session.validation = None;
                    let token = session.validation_submitted.ok_or_else(|| {
                        crate::error::internal_error(FileDialogInternalFault::Missing {
                            part: "validation token",
                        })
                    })?;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            pending.unwrap_or_else(|| WidgetryFileDialogReply::ValidationFailed {
                                token,
                                error: FileDialogRequestError::Canceled(
                                    "candidate validation result was canceled before delivery",
                                ),
                            })
                        }
                        JobOutcome::Failed(error) => pending
                            .filter(|reply| {
                                matches!(reply, WidgetryFileDialogReply::ValidationFailed { .. })
                            })
                            .unwrap_or_else(|| WidgetryFileDialogReply::ValidationFailed {
                                token,
                                error: FileDialogRequestError::Failed(error.clone()),
                            }),
                        JobOutcome::Canceled => WidgetryFileDialogReply::ValidationFailed {
                            token,
                            error: FileDialogRequestError::Canceled(
                                "candidate validation canceled",
                            ),
                        },
                    };
                    finish_job_delivery(
                        WidgetryFileDialog::deliver(world, root, reply)
                            .map(|_| ())
                            .map_err(FileDialogOperationFailure::into_bevy_error),
                        failure,
                    )?;
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
                .ok_or_else(|| {
                    crate::error::internal_error(FileDialogInternalFault::Missing {
                        part: "selection job",
                    })
                })?
                .poll(|| {
                    crate::error::owner_frame(world, root, root, "file_dialog.selection.poll")
                })? {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, prepared) => {
                    let failure = outcome_failure(&outcome);
                    session.selection = None;
                    let token = session.selection_submitted.ok_or_else(|| {
                        crate::error::internal_error(FileDialogInternalFault::Missing {
                            part: "selection token",
                        })
                    })?;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            WidgetryFileDialogReply::Selection(prepared.ok_or_else(|| {
                                crate::error::internal_error(FileDialogInternalFault::Missing {
                                    part: "selection result",
                                })
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
                    finish_job_delivery(self.receive(world, Output::Reply(root, reply)), failure)?;
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
            let signature = session.project_inflight.ok_or_else(|| {
                crate::error::internal_error(FileDialogInternalFault::Missing {
                    part: "projection signature",
                })
            })?;
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
            let job = session.projection.as_mut().ok_or_else(|| {
                crate::error::internal_error(FileDialogInternalFault::Missing {
                    part: "projection job",
                })
            })?;
            match job.poll(|| {
                crate::error::owner_frame(world, root, root, "file_dialog.projection.poll")
            })? {
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Empty => continue,
                PreparedPoll::Finished(outcome, pending) => {
                    let failure = outcome_failure(&outcome);
                    session.projection = None;
                    session.project_inflight = None;
                    let reply = match outcome {
                        JobOutcome::Completed => {
                            let result = pending.ok_or_else(|| {
                                crate::error::internal_error(FileDialogInternalFault::Missing {
                                    part: "projection result",
                                })
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
                        finish_job_delivery(
                            self.receive(world, Output::Reply(root, reply)),
                            failure,
                        )?;
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
                Ok(JobRead::Empty) => continue,
                Ok(message) => message,
                Err(error) => {
                    return Err(bevy_widgetry_job::JobFailure::from(error)
                        .with_context(
                            crate::error::owner_frame(
                                world,
                                root,
                                root,
                                "file_dialog.directory.poll",
                            )
                            .field("job", format_args!("{:?}", job.id)),
                        )
                        .into_bevy_error());
                }
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
            let failure = match &message {
                JobRead::Finished(outcome) => outcome_failure(outcome),
                _ => None,
            };
            let output = match message {
                JobRead::Empty => continue,
                JobRead::Closed => {
                    return Err(crate::error::internal_error(
                        FileDialogInternalFault::ReceiverClosed {
                            operation: "directory",
                        },
                    ));
                }
                JobRead::Item(event) => match event.as_ref() {
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
                JobRead::Finished(outcome) => {
                    let job = session.directory.take().ok_or_else(|| {
                        crate::error::internal_error(FileDialogInternalFault::Missing {
                            part: "directory job during completion",
                        })
                    })?;
                    if let Err(error) = job.manager.remove(job.id) {
                        let error = bevy_widgetry_job::JobFailure::from(error).with_context(
                            session
                                .control
                                .frame("file_dialog.directory.remove")
                                .field("job", format_args!("{:?}", job.id)),
                        );
                        return Err(match failure {
                            Some(failure) => failure.combine(error),
                            None => error,
                        }
                        .into_bevy_error());
                    }
                    match outcome {
                        JobOutcome::Completed => {
                            let (snapshot, state) = job.final_snapshot.ok_or_else(|| {
                                crate::error::internal_error(FileDialogInternalFault::Missing {
                                    part: "directory final snapshot",
                                })
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
                finish_job_delivery(self.receive(world, output), failure)?;
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
                    session.version = session.version.checked_add(1).ok_or_else(|| {
                        crate::error::failure(
                            FileDialogError::RevisionExhausted {
                                counter: "stream",
                                value: session.version,
                                token: Some(Box::new(snapshot.token())),
                                scope: None,
                            },
                            "file_dialog.directory",
                        )
                        .into_bevy_error()
                    })?;
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
                        let mut merge_failure = None;
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
                                let message = error.to_string();
                                merge_failure = Some(error);
                                WidgetryFileDialogStorageState::Failed(message)
                            }
                        };
                        world
                            .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                            .persistence = storage_state;
                        if let Some(failure) = merge_failure {
                            self.handle_preference_failure(
                                &failure.into(),
                                "放弃本次加载后的偏好合并",
                                "已保留现有偏好、抑制当前 revision 自动保存并提交加载失败状态",
                            );
                        } else {
                            self.recover_preferences();
                        }
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
                            .persistence =
                            WidgetryFileDialogStorageState::Failed(error.to_string());
                        if let Some(failure) = error.failure() {
                            self.handle_preference_failure(
                                failure,
                                "保留当前内存偏好并抑制当前 Revision 自动保存",
                                "已提交偏好加载失败状态",
                            );
                        }
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
                                .acknowledge_saved(&scope, revision)
                                .map_err(FileDialogFailure::into_bevy_error)?;
                        }
                        world
                            .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                            .persistence =
                            WidgetryFileDialogStorageState::Saved(self.disk_revision);
                        self.recover_preferences();
                    }
                    Err(error) => {
                        self.failed_revisions = Some(revisions);
                        world
                            .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                            .persistence =
                            WidgetryFileDialogStorageState::Failed(error.to_string());
                        if let Some(failure) = error.failure() {
                            self.handle_preference_failure(
                                failure,
                                "保留当前内存偏好与未确认的保存 Revision",
                                "已提交保存失败状态并保留重试规则",
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_preference_failure(
        &mut self,
        failure: &bevy_widgetry_job::JobFailure,
        action: &str,
        outcome: &str,
    ) {
        if let Some(first) = failure.diagnostics().next() {
            let _ = self
                .preference_failure
                .failure_batch(first.origin().id().clone(), || {
                    failure.clone().into_report()
                });
        }
        failure.record_handled(action, outcome);
    }

    fn recover_preferences(&mut self) {
        self.preference_failure.recover(|| {
            bevy_widgetry_diagnostics::ErrorFrame::new("file_dialog.preferences.recovered")
                .field("path", format_args!("{:?}", self.persistence_path))
                .field("disk_revision", self.disk_revision)
        });
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
            match job.poll(|| {
                bevy_widgetry_diagnostics::ErrorFrame::new("file_dialog.preferences.poll")
                    .field("path", format_args!("{:?}", self.persistence_path))
                    .field("loading", loading)
            })? {
                PreparedPoll::Empty => continue,
                PreparedPoll::Item => return Ok(true),
                PreparedPoll::Finished(outcome, pending) => {
                    let failure = outcome_failure(&outcome);
                    if loading {
                        self.preference_load = None;
                    } else {
                        self.preference_save = None;
                    }
                    let error = match outcome {
                        JobOutcome::Completed if pending.is_some() => None,
                        JobOutcome::Completed => Some(FileDialogRequestError::Canceled(
                            "preferences result was canceled before delivery",
                        )),
                        JobOutcome::Failed(error) => Some(FileDialogRequestError::Failed(error)),
                        JobOutcome::Canceled => Some(FileDialogRequestError::Canceled(
                            "preferences operation canceled",
                        )),
                    };
                    let output = match (loading, error, pending) {
                        (true, Some(error), _) => Output::Loaded(Err(error)),
                        (false, Some(error), _) => Output::Saved(
                            self.save_revisions.take().ok_or_else(|| {
                                crate::error::internal_error(FileDialogInternalFault::Missing {
                                    part: "save revisions",
                                })
                            })?,
                            Err(error),
                        ),
                        (true, None, Some(PreferencesReply::Loaded(result))) => {
                            Output::Loaded(result.map_err(FileDialogRequestError::Failed))
                        }
                        (false, None, Some(PreferencesReply::Saved(revisions, result))) => {
                            self.save_revisions = None;
                            Output::Saved(revisions, result.map_err(FileDialogRequestError::Failed))
                        }
                        _ => {
                            return Err(crate::error::internal_error(
                                FileDialogInternalFault::PreferenceReply,
                            ));
                        }
                    };
                    finish_job_delivery(self.receive(world, output), failure)?;
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
            return Err(crate::error::internal_error(
                FileDialogInternalFault::PersistencePath {
                    original: self.persistence_path.clone().unwrap_or_default(),
                    requested: path,
                },
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
                    let error = crate::error::preference_io(
                        FileDialogPreferenceContext {
                            path: path.clone(),
                            resource: None,
                            operation: FileDialogPreferenceOperation::Load,
                            scope: None,
                            revisions: Default::default(),
                        },
                        std::io::Error::other(bevy_widgetry_job::JobFailure::from(error)),
                    );
                    self.receive(
                        world,
                        Output::Loaded(Err(FileDialogRequestError::Failed(error))),
                    )?;
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
            self.disk_revision = self.disk_revision.checked_add(1).ok_or_else(|| {
                crate::error::failure(
                    FileDialogError::RevisionExhausted {
                        counter: "disk",
                        value: self.disk_revision,
                        token: None,
                        scope: None,
                    },
                    "file_dialog.persistence",
                )
                .into_bevy_error()
            })?;
            if let Some(mut request) = world.get_resource_mut::<FlushRequest>() {
                request.0 = false;
            }
            match crate::preferences_job::save(
                &manager,
                world.resource::<WidgetryFileDialogBackend>().clone(),
                path.clone(),
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
                    let error = crate::error::preference_io(
                        FileDialogPreferenceContext {
                            path,
                            resource: None,
                            operation: FileDialogPreferenceOperation::Save,
                            scope: None,
                            revisions: Arc::new(revisions.clone()),
                        },
                        std::io::Error::other(bevy_widgetry_job::JobFailure::from(error)),
                    );
                    self.receive(
                        world,
                        Output::Saved(revisions, Err(FileDialogRequestError::Failed(error))),
                    )?;
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
) -> Result<WidgetryFileDialogStorage, FileDialogFailure> {
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
        let mut failures = None;
        // Publish Job cancellation before the business flag can make an operation return Interrupted.
        if let Some(job) = &self.directory {
            match job.manager.cancel(job.id) {
                Ok(_) => {}
                Err(error) => combine_failure(
                    &mut failures,
                    bevy_widgetry_job::JobFailure::from(error).with_context(
                        self.control
                            .frame("file_dialog.directory.cancel")
                            .field("job", format_args!("{:?}", job.id)),
                    ),
                ),
            }
        }
        for result in [
            self.stop_projection(retired),
            self.stop_selection(retired),
            self.stop_validation(retired),
            self.stop_folder(retired),
            self.stop_locations(retired),
        ] {
            if let Err(error) = result {
                combine_failure(&mut failures, error);
            }
        }
        self.control.cancelled.store(true, Ordering::Release);
        if failures.is_none()
            && let Some(job) = self.directory.take()
        {
            retired.push((job.manager, job.id));
        }
        failures.map_or(Ok(()), |error| Err(error.into_bevy_error()))
    }

    fn stop_projection(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.projection {
            job.manager.cancel(job.id).map_err(|error| {
                bevy_widgetry_job::JobFailure::from(error)
                    .with_context(
                        self.control
                            .frame("file_dialog.projection.cancel")
                            .field("job", format_args!("{:?}", job.id)),
                    )
                    .into_bevy_error()
            })?;
        }
        if let Some(job) = self.projection.take() {
            retired.push((job.manager, job.id));
        }
        self.project_inflight = None;
        Ok(())
    }

    fn stop_selection(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.selection {
            job.manager.cancel(job.id).map_err(|error| {
                bevy_widgetry_job::JobFailure::from(error)
                    .with_context(
                        self.control
                            .frame("file_dialog.selection.cancel")
                            .field("job", format_args!("{:?}", job.id)),
                    )
                    .into_bevy_error()
            })?;
        }
        if let Some(job) = self.selection.take() {
            retired.push((job.manager, job.id));
        }
        self.selection_submitted = None;
        Ok(())
    }

    fn stop_validation(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.validation {
            job.manager.cancel(job.id).map_err(|error| {
                bevy_widgetry_job::JobFailure::from(error)
                    .with_context(
                        self.control
                            .frame("file_dialog.validation.cancel")
                            .field("job", format_args!("{:?}", job.id)),
                    )
                    .into_bevy_error()
            })?;
        }
        if let Some(job) = self.validation.take() {
            retired.push((job.manager, job.id));
        }
        self.validation_submitted = None;
        Ok(())
    }

    fn stop_folder(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.folder {
            job.manager.cancel(job.id).map_err(|error| {
                bevy_widgetry_job::JobFailure::from(error)
                    .with_context(
                        self.control
                            .frame("file_dialog.folder.cancel")
                            .field("job", format_args!("{:?}", job.id)),
                    )
                    .into_bevy_error()
            })?;
        }
        if let Some(job) = self.folder.take() {
            retired.push((job.manager, job.id));
        }
        self.folder_submitted = None;
        Ok(())
    }

    fn stop_locations(&mut self, retired: &mut Vec<(JobManager, JobId)>) -> Result {
        if let Some(job) = &self.locations {
            job.manager.cancel(job.id).map_err(|error| {
                bevy_widgetry_job::JobFailure::from(error)
                    .with_context(
                        self.control
                            .frame("file_dialog.locations.cancel")
                            .field("job", format_args!("{:?}", job.id)),
                    )
                    .into_bevy_error()
            })?;
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
        let mut retired = std::mem::take(&mut self.retired);
        for job in [&self.preference_load, &self.preference_save]
            .into_iter()
            .flatten()
        {
            retired.push((job.manager.clone(), job.id));
        }
        for session in self.sessions.values() {
            retired.extend(
                session
                    .directory
                    .as_ref()
                    .map(|job| (job.manager.clone(), job.id)),
            );
            retired.extend(
                session
                    .projection
                    .as_ref()
                    .map(|job| (job.manager.clone(), job.id)),
            );
            retired.extend(
                session
                    .selection
                    .as_ref()
                    .map(|job| (job.manager.clone(), job.id)),
            );
            retired.extend(
                session
                    .validation
                    .as_ref()
                    .map(|job| (job.manager.clone(), job.id)),
            );
            retired.extend(
                session
                    .folder
                    .as_ref()
                    .map(|job| (job.manager.clone(), job.id)),
            );
            retired.extend(
                session
                    .locations
                    .as_ref()
                    .map(|job| (job.manager.clone(), job.id)),
            );
        }
        if let Some(service) = &self.retention {
            retired.extend(service.job_identity());
        }
        // Request all Job cancellations before business flags or receiver destruction
        // can turn a normal stop into an Interrupted/Closed failure.
        for (manager, id) in &retired {
            let _ = manager.cancel_if_present(*id);
        }
        for session in self.sessions.values() {
            session.control.cancelled.store(true, Ordering::Release);
        }
        if let Some(service) = &self.retention {
            service.stop();
        }
        if retired.is_empty() {
            return;
        }
        // Retired jobs have heterogeneous result types and no remaining receivers.
        // Drive their real exit on the existing pool, without requiring another UI update
        // or submitting a managed job after shutdown has closed admission.
        bevy::tasks::IoTaskPool::get()
            .spawn(async move {
                loop {
                    retired.retain(|(manager, id)| {
                        let Some(status) = manager.find_status(*id) else {
                            return false;
                        };
                        if !status.state.is_finished() {
                            return true;
                        }
                        if let Err(error) = manager.remove_if_present(*id) {
                            error.record_handled(
                                "保留无法移除的 FileDialog 任务记录",
                                "Runtime 已销毁，结果已放弃",
                            );
                        }
                        if let Some(JobOutcome::Failed(error)) = status.outcome {
                            error.record_handled(
                                "放弃已销毁 FileDialog Runtime 的后台结果",
                                "过期任务已真实退出",
                            );
                        }
                        false
                    });
                    if retired.is_empty() {
                        break;
                    }
                    futures_timer::Delay::new(std::time::Duration::from_millis(1)).await;
                }
            })
            .detach();
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn retired_access_failures_do_not_block_other_cleanup() {
        use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let manager = JobManager::default();
        let mut runtime = Runtime::default();
        for _ in 0..2 {
            let (id, _) = manager
                .spawn_thread::<(), _>(Default::default(), |_| Ok(()))
                .unwrap();
            wait_job(&manager, id);
            manager.remove(id).unwrap();
            runtime.retired.push((manager.clone(), id));
        }
        let logs = LogCapture::default();
        let failure = logs
            .run(|| bevy_widgetry_job::JobFailure::from(std::io::Error::other("retired failed")));
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        let (id, _) = manager
            .spawn_thread::<(), _>(Default::default(), move |_| Err(failure))
            .unwrap();
        wait_job(&manager, id);
        runtime.retired.push((manager.clone(), id));
        let error = logs.run(|| runtime.reap_retired()).unwrap_err();
        let report = error.downcast_ref::<WidgetryEcsError>().unwrap();
        assert_eq!(report.diagnostics().count(), 2);
        assert!(manager.find_status(id).is_none());
        assert_eq!(runtime.retired.len(), 2);
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .len(),
            1
        );
        for diagnostic in report.diagnostics() {
            assert!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
    }

    #[test]
    fn prepared_cleanup_failure_retains_terminal_diagnostic_and_request_context() {
        use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let logs = LogCapture::default();
        let failure = logs
            .run(|| bevy_widgetry_job::JobFailure::from(std::io::Error::other("prepared failed")));
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        let manager = JobManager::default();
        let (id, receiver) = manager
            .spawn_thread::<(), _>(Default::default(), move |_| Err(failure))
            .unwrap();
        wait_job(&manager, id);
        manager.remove(id).unwrap();
        let mut job = PreparedJob {
            manager,
            id,
            receiver,
            pending: None,
        };
        let frame = bevy_widgetry_diagnostics::ErrorFrame::new("file_dialog.fixture.poll")
            .field("root", "fixture-root");
        let error = logs.run(|| job.poll(|| frame)).err().unwrap();
        let report = error.downcast_ref::<WidgetryEcsError>().unwrap();
        assert_eq!(report.diagnostics().count(), 2);
        let original = report
            .diagnostics()
            .find(|diagnostic| diagnostic.origin().id() == origin.id())
            .unwrap();
        assert!(Arc::ptr_eq(original.origin(), &origin));
        assert!(original.contexts().iter().any(|context| {
            context.operation() == "file_dialog.fixture.poll"
                && context
                    .fields()
                    .iter()
                    .any(|(key, value)| key == "root" && value == "fixture-root")
        }));
        for diagnostic in report.diagnostics() {
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
    }

    #[test]
    fn service_fault_retains_cleanup_failures_from_every_session_and_job() {
        use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        let manager = app.world().resource::<JobManager>().clone();
        let mut runtime = Runtime::default();
        let service = Retention::start(&manager).unwrap();
        runtime.retention = Some(service.clone());
        for _ in 0..2 {
            let state = WidgetryFileDialogState::new(Default::default()).unwrap();
            let token = state.token();
            let root = app.world_mut().spawn((WidgetryFileDialog, state)).id();
            let mut session = Session::new(root, token);
            for slot in [&mut session.folder, &mut session.locations] {
                let (id, receiver) = manager
                    .spawn_thread::<WidgetryFileDialogReply, _>(Default::default(), |_| Ok(()))
                    .unwrap();
                wait_job(&manager, id);
                manager.remove(id).unwrap();
                *slot = Some(PreparedJob {
                    manager: manager.clone(),
                    id,
                    receiver,
                    pending: None,
                });
            }
            runtime.sessions.insert(root, session);
        }
        let logs = LogCapture::default();
        let failure = logs
            .run(|| bevy_widgetry_job::JobFailure::from(std::io::Error::other("service failed")));
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        *service.fault.lock().unwrap() = Some(failure);
        let error = logs
            .run(|| runtime.update(app.world_mut(), &Default::default()))
            .unwrap_err();
        let report = error.downcast_ref::<WidgetryEcsError>().unwrap();
        assert_eq!(report.diagnostics().count(), 5);
        assert!(
            report
                .diagnostics()
                .any(|diagnostic| Arc::ptr_eq(diagnostic.origin(), &origin))
        );
        for diagnostic in report.diagnostics() {
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
        for session in runtime.sessions.values() {
            assert!(session.folder.is_some() && session.locations.is_some());
        }
        service.stop();
    }

    #[test]
    fn startup_failure_keeps_all_failed_deliveries_and_runtime_until_safe_degradation() {
        use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        let roots: Vec<_> = (0..2)
            .map(|_| {
                let state = WidgetryFileDialogState::new(WidgetryFileDialogProps {
                    storage_scope: Some("open".into()),
                    ..Default::default()
                })
                .unwrap();
                app.world_mut().spawn((WidgetryFileDialog, state)).id()
            })
            .collect();
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .unwrap();
        app.world_mut()
            .remove_resource::<WidgetryFileDialogStorage>();
        let logs = LogCapture::default();
        let error = logs.run(|| update(app.world_mut())).unwrap_err();
        assert!(app.world().contains_resource::<Runtime>());
        let report = error.downcast_ref::<WidgetryEcsError>().unwrap();
        let diagnostics: Vec<_> = report.diagnostics().collect();
        assert_eq!(diagnostics.len(), 3);
        for diagnostic in &diagnostics {
            assert!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
        let startup = diagnostics
            .iter()
            .find(|diagnostic| {
                let mut cause = diagnostic.source();
                while let Some(error) = cause {
                    if error.is::<bevy_widgetry_job::SubmitFailure>() {
                        return true;
                    }
                    cause = error.source();
                }
                false
            })
            .unwrap()
            .origin()
            .clone();
        app.world_mut().init_resource::<WidgetryFileDialogStorage>();
        logs.run(|| update(app.world_mut())).unwrap();
        for root in roots {
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            assert!(matches!(
                state.directory_state(),
                WidgetryFileDialogDirectoryState::Failed(_)
            ));
            assert_eq!(state.session_state(), WidgetryFileDialogSessionState::Open);
        }
        assert_eq!(
            logs.diagnostic_records(startup.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(
            logs.diagnostic_records(startup.id(), DiagnosticPhase::Handled)
                .len(),
            1
        );
        logs.run(|| update(app.world_mut())).unwrap();
        assert_eq!(
            logs.diagnostic_records(startup.id(), DiagnosticPhase::Handled)
                .len(),
            1
        );
    }

    #[test]
    fn dropped_runtime_consumes_late_failures_from_preferences_retired_and_session_jobs() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        use std::sync::{Mutex, mpsc};
        use std::time::Duration;
        const CHILD: &str = "WIDGETRY_FILE_DIALOG_DROPPED_RUNTIME_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .env(CHILD, "1")
                .args(["--exact", "runtime::tests::dropped_runtime_consumes_late_failures_from_preferences_retired_and_session_jobs", "--nocapture"])
                .output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let logs = LogCapture::default();
        bevy::log::tracing::dispatcher::set_global_default(logs.dispatcher()).unwrap();
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        for mode in 0..3 {
            for panics in [false, true] {
                let directory = tempfile::tempdir().unwrap();
                let missing = directory.path().join("missing-file");
                let manager = JobManager::default();
                let (entered, ready) = mpsc::channel();
                let (release, gate) = mpsc::channel();
                let gate = Mutex::new(gate);
                let (id, receiver) = manager
                    .spawn_thread::<PreferencesReply, _>(Default::default(), move |_| {
                        entered.send(()).unwrap();
                        gate.lock().unwrap().recv().unwrap();
                        assert!(!panics, "controlled abandoned FileDialog panic");
                        Err(crate::retention::work_error(
                            std::fs::File::open(missing).unwrap_err(),
                        ))
                    })
                    .unwrap();
                let mut observer = manager.subscribe::<PreferencesReply>(id).unwrap();
                let mut runtime = Runtime::default();
                let job = PreparedJob {
                    manager: manager.clone(),
                    id,
                    receiver,
                    pending: None,
                };
                match mode {
                    0 => runtime.preference_load = Some(job),
                    1 => {
                        runtime.retired.push((manager.clone(), id));
                        drop(job);
                    }
                    _ => runtime.preference_save = Some(job),
                }
                ready.recv_timeout(Duration::from_secs(5)).unwrap();
                let shutdown = manager.begin_shutdown(Duration::from_secs(5)).unwrap();
                drop(runtime);
                release.send(()).unwrap();
                let failure = match observer.recv_blocking().unwrap() {
                    JobRead::Finished(JobOutcome::Failed(error)) => error,
                    _ => panic!("expected late failure"),
                };
                let diagnostic = failure.diagnostics().next().unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while logs
                    .diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
                    || manager.find_status(id).is_some()
                {
                    assert!(
                        Instant::now() < deadline,
                        "Runtime destroyed without terminal failure owner"
                    );
                    std::thread::yield_now();
                }
                assert_eq!(
                    logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                        .len(),
                    1
                );
                assert_eq!(
                    logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                        .len(),
                    1
                );
                while !matches!(
                    shutdown.try_recv(),
                    Some(bevy_widgetry_job::ShutdownMessage::Completed)
                ) {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
            }
        }
        let (mut app, mut runtime, root, _) = cached_projection_fixture().unwrap();
        let manager = app.world().resource::<JobManager>().clone();
        let (entered, ready) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let (id, receiver) = manager
            .spawn_thread::<WidgetryFileDialogReply, _>(Default::default(), move |_| {
                entered.send(()).unwrap();
                gate.recv().unwrap();
                panic!("controlled session failure after App destruction");
            })
            .unwrap();
        let mut observer = manager.subscribe::<WidgetryFileDialogReply>(id).unwrap();
        runtime.sessions.get_mut(&root).unwrap().folder = Some(PreparedJob {
            manager: manager.clone(),
            id,
            receiver,
            pending: None,
        });
        app.insert_resource(runtime);
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(app);
        assert_eq!(
            manager.status(id).unwrap().state,
            bevy_widgetry_job::JobState::Stopping
        );
        release.send(()).unwrap();
        let failure = match observer.recv_blocking().unwrap() {
            JobRead::Finished(JobOutcome::Failed(error)) => error,
            _ => panic!("expected late session failure"),
        };
        let diagnostic = failure.diagnostics().next().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while logs
            .diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
            .is_empty()
            || manager.find_status(id).is_some()
        {
            assert!(
                Instant::now() < deadline,
                "App destruction lost the session terminal owner"
            );
            std::thread::yield_now();
        }
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .len(),
            1
        );
    }

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
                .spawn_io::<WidgetryFileDialogReply, _, _>(
                    Default::default(),
                    |context| async move {
                        context
                            .send(WidgetryFileDialogReply::FolderCreated {
                                request,
                                outcome: Ok(()),
                            })
                            .await
                            .map(|status| assert_eq!(status, bevy_widgetry_job::SendStatus::Sent))
                            .map_err(bevy_widgetry_job::JobFailure::from)
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
            assert!(manager.find_status(id).is_none());
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
                .spawn_io::<WidgetryFileDialogReply, _, _>(
                    Default::default(),
                    move |context| async move {
                        context
                            .send(WidgetryFileDialogReply::Locations {
                                token,
                                outcome: if change == 4 {
                                    Err(FileDialogRequestError::Failed(
                                        std::io::Error::other("controlled location error").into(),
                                    ))
                                } else {
                                    Ok(vec![WidgetryFileDialogLocation {
                                        name: "New".into(),
                                        path: PathBuf::from("C:/new"),
                                    }])
                                },
                            })
                            .await
                            .map(|status| assert_eq!(status, bevy_widgetry_job::SendStatus::Sent))
                            .map_err(bevy_widgetry_job::JobFailure::from)?;
                        if change == 4 {
                            Err(std::io::Error::other("controlled location error").into())
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
                assert!(
                    state
                        .location_error()
                        .is_some_and(|error| error.contains("controlled location error"))
                );
                assert_eq!(
                    state.directory_state(),
                    &WidgetryFileDialogDirectoryState::Ready
                );
            }
            runtime.reap_retired()?;
            assert!(manager.find_status(id).is_none());
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
            .spawn_io::<PreferencesReply, _, _>(Default::default(), move |context| async move {
                context
                    .send(PreferencesReply::Saved(sent, Ok(())))
                    .await
                    .map(|status| assert_eq!(status, bevy_widgetry_job::SendStatus::Sent))
                    .map_err(bevy_widgetry_job::JobFailure::from)
            })
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
        assert!(manager.find_status(id).is_none());
        Ok(())
    }

    #[test]
    fn rejected_preference_jobs_keep_request_context_and_submission_origin() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        for saving in [false, true] {
            let mut world = World::new();
            world.init_resource::<WidgetryFileDialogStorage>();
            world.init_resource::<WidgetryFileDialogRuntimeStatus>();
            world.init_resource::<WidgetryFileDialogBackend>();
            world.insert_resource(WidgetryFileDialogPersistence {
                path: "C:/fixture/preferences.json".into(),
            });
            world.insert_resource(JobManager::default());
            for scope in ["open", "save"] {
                world
                    .resource_mut::<WidgetryFileDialogStorage>()
                    .import(scope.into(), Default::default())
                    .unwrap();
            }
            let mut runtime = Runtime::default();
            if saving {
                runtime
                    .receive(&mut world, Output::Loaded(Ok(BTreeMap::new())))
                    .unwrap();
                runtime.load_admitted = true;
            }
            let expected: BTreeMap<_, _> = world
                .resource::<WidgetryFileDialogStorage>()
                .scopes
                .iter()
                .map(|(scope, record)| (scope.clone(), record.revision))
                .collect();
            world
                .resource::<JobManager>()
                .begin_shutdown(std::time::Duration::ZERO)
                .unwrap();
            let logs = LogCapture::default();
            logs.run(|| runtime.persistence(&mut world)).unwrap();
            let diagnostic = runtime.preference_failure.active().unwrap();
            let mut cause = diagnostic.source();
            let mut typed = false;
            let mut original = false;
            while let Some(error) = cause {
                if let Some(FileDialogError::PreferenceManaged { context, .. }) =
                    error.downcast_ref::<FileDialogError>()
                {
                    assert_eq!(context.path, PathBuf::from("C:/fixture/preferences.json"));
                    assert_eq!(
                        context.operation,
                        if saving {
                            FileDialogPreferenceOperation::Save
                        } else {
                            FileDialogPreferenceOperation::Load
                        }
                    );
                    if saving {
                        assert_eq!(*context.revisions, expected);
                    } else {
                        assert!(context.revisions.is_empty());
                    }
                    typed = true;
                }
                if let Some(error) = error.downcast_ref::<bevy_widgetry_job::SubmitFailure>() {
                    assert!(matches!(
                        error.kind(),
                        bevy_widgetry_job::SubmitError::ShuttingDown { .. }
                    ));
                    assert!(Arc::ptr_eq(error.origin(), diagnostic.origin()));
                    original = true;
                }
                cause = error.source();
            }
            assert!(typed, "typed preference request context missing");
            assert!(original);
            assert_eq!(runtime.failed_revisions.as_ref(), Some(&expected));
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .len(),
                1
            );
        }
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
            let candidate = state.validation_job.clone().unwrap().prepare().unwrap();
            let manager = app.world().resource::<JobManager>().clone();
            let (id, receiver) = manager
                .spawn_io::<WidgetryFileDialogReply, _, _>(
                    Default::default(),
                    |context| async move {
                        context
                            .send(WidgetryFileDialogReply::Validated {
                                candidate,
                                exists: true,
                            })
                            .await
                            .map(|status| assert_eq!(status, bevy_widgetry_job::SendStatus::Sent))
                            .map_err(bevy_widgetry_job::JobFailure::from)
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
            assert!(manager.find_status(id).is_none());
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
                .spawn_thread::<WidgetryFileDialogReply, _>(Default::default(), move |_context| {
                    entered.send(()).unwrap();
                    released.recv().unwrap();
                    if panicked {
                        panic!("controlled validation panic");
                    }
                    Ok(())
                })
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
            assert!(manager.find_status(id).is_none());
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
        assert!(manager.find_status(id).is_none());
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
            assert!(manager.find_status(id).is_none());
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
        assert!(manager.find_status(id).is_none());
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
            assert!(manager.find_status(id).is_none());
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
            .spawn_thread::<DirectoryEvent, _>(Default::default(), move |context| {
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
            .spawn_thread::<DirectoryEvent, _>(Default::default(), move |context| {
                assert_eq!(
                    context
                        .send_blocking(DirectoryEvent::Started(PathBuf::from("C:/fixture")))
                        .map_err(bevy_widgetry_job::JobFailure::from)?,
                    bevy_widgetry_job::SendStatus::Sent
                );
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
                    Ok(bevy_widgetry_job::SendStatus::Canceled) => {
                        assert!(context.acknowledge_cancel());
                        Ok(())
                    }
                    result => result
                        .map(|status| assert_eq!(status, bevy_widgetry_job::SendStatus::Sent))
                        .map_err(bevy_widgetry_job::JobFailure::from),
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
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        let failure = logs
            .run(|| bevy_widgetry_job::JobFailure::from(std::io::Error::other("worker failed")));
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        *service.fault.lock().unwrap() = Some(failure);
        service.stop();
        logs.run(|| runtime.update(app.world_mut(), &Default::default()))?;
        assert_eq!(
            logs.diagnostic_records(
                origin.id(),
                bevy_widgetry_test_utils::DiagnosticPhase::Handled
            )
            .len(),
            1
        );
        release.send(()).unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(manager.outcome(id).unwrap(), Some(JobOutcome::Canceled));
        assert!(runtime.sessions[&root].directory.is_none());
        runtime.update(app.world_mut(), &Default::default())?;
        assert!(manager.find_status(id).is_none());
        assert!(
            app.world()
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .error
                .as_deref()
                .is_some_and(|error| error.contains("worker failed"))
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
            .spawn_io::<DirectoryEvent, _, _>(Default::default(), move |context| async move {
                context
                    .send(DirectoryEvent::Snapshot {
                        snapshot,
                        state: WidgetryFileDialogDirectoryState::Ready,
                        finished: true,
                    })
                    .await
                    .map(|status| assert_eq!(status, bevy_widgetry_job::SendStatus::Sent))
                    .map_err(bevy_widgetry_job::JobFailure::from)?;
                Ok(())
            })
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
        assert!(manager.find_status(id).is_none());
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
    fn preference_handling_and_recovery_are_distinct_and_cancel_does_not_recover() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        let mut world = World::new();
        world.init_resource::<WidgetryFileDialogStorage>();
        world.init_resource::<WidgetryFileDialogRuntimeStatus>();
        world
            .resource_mut::<WidgetryFileDialogStorage>()
            .import("open".into(), Default::default())
            .unwrap();
        let revision = world
            .resource::<WidgetryFileDialogStorage>()
            .revision("open")
            .unwrap();
        let revisions = BTreeMap::from([("open".into(), revision)]);
        let mut runtime = Runtime::default();
        let logs = LogCapture::default();
        let failure = logs.run(|| {
            bevy_widgetry_job::JobFailure::from(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "load denied",
            ))
        });
        let origin = failure.diagnostics().next().unwrap().origin().clone();
        assert!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        logs.run(|| {
            runtime.receive(
                &mut world,
                Output::Loaded(Err(FileDialogRequestError::Failed(failure))),
            )
        })
        .unwrap();
        assert_eq!(runtime.failed_revisions.as_ref(), Some(&revisions));
        assert_eq!(
            world.resource::<WidgetryFileDialogStorage>().scopes["open"].saved_revision,
            0
        );
        assert!(matches!(
            world
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .persistence,
            WidgetryFileDialogStorageState::Failed(_)
        ));
        let handled = logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled);
        assert_eq!(handled.len(), 1);
        assert_eq!(handled[0].level, bevy::log::tracing::Level::DEBUG);
        logs.run(|| {
            runtime.receive(
                &mut world,
                Output::Saved(
                    revisions.clone(),
                    Err(FileDialogRequestError::Canceled("save canceled")),
                ),
            )
        })
        .unwrap();
        assert!(
            logs.records_for_error(origin.id())
                .iter()
                .all(|record| !record.matches_field("phase", "recovered"))
        );
        logs.run(|| runtime.receive(&mut world, Output::Saved(revisions.clone(), Ok(()))))
            .unwrap();
        assert_eq!(
            world.resource::<WidgetryFileDialogStorage>().scopes["open"].saved_revision,
            revision
        );
        let recovered: Vec<_> = logs
            .records_for_error(origin.id())
            .into_iter()
            .filter(|record| record.matches_field("phase", "recovered"))
            .collect();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].level, bevy::log::tracing::Level::INFO);
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .len(),
            1
        );
        let count = logs.records().len();
        logs.run(|| runtime.receive(&mut world, Output::Saved(revisions, Ok(()))))
            .unwrap();
        assert_eq!(logs.records().len(), count);
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
        runtime.receive(
            &mut world,
            Output::Loaded(Err(FileDialogRequestError::Failed(
                std::io::Error::other("unsupported schema").into(),
            ))),
        )?;
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
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        logs.run(|| {
            Runtime::default().receive(
                &mut world,
                Output::Loaded(Ok(BTreeMap::from([("open".into(), disk)]))),
            )
        })?;
        let records = logs.records();
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].phase(),
            Some(bevy_widgetry_test_utils::DiagnosticPhase::Source)
        );
        assert_eq!(
            records[1].phase(),
            Some(bevy_widgetry_test_utils::DiagnosticPhase::Handled)
        );
        assert_eq!(records[0].fields["error_id"], records[1].fields["error_id"]);
        assert!(
            !records
                .iter()
                .any(|record| record.fields.contains_key("backtrace"))
        );
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
