use crate::directory::DirectoryEvent;
use crate::model::contract_error;
use crate::projection::ProjectionResult;
use crate::worker::{Control, CpuWork, IoWork, Output, Service};
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
    pub workers: usize,
    pub active_sessions: usize,
    pub queued: usize,
    pub retained_snapshots: usize,
    pub applied_last_update: usize,
    pub persistence: WidgetryFileDialogStorageState,
    pub error: Option<String>,
}

#[derive(Resource, Default)]
pub(crate) struct FlushRequest(pub(crate) bool);

#[derive(Resource, Default)]
pub(crate) struct Runtime {
    service: Option<Arc<Service>>,
    sessions: BTreeMap<Entity, Session>,
    start_failed: bool,
    persistence_path: Option<PathBuf>,
    load_admitted: bool,
    loaded: bool,
    saving: bool,
    failed_revisions: Option<BTreeMap<String, u64>>,
    disk_revision: u64,
    retired: Vec<(JobManager, JobId)>,
    directory_cursor: usize,
    projection_cursor: usize,
    selection_cursor: usize,
    prefer_directory: bool,
    prefer_projection: bool,
    prefer_selection: bool,
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
    folder_submitted: Option<WidgetryFileDialogToken>,
    candidate: Option<WidgetryFileDialogCandidate>,
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
    let notified = world.get_resource::<Runtime>().is_some_and(Runtime::notify);
    if !notified && let Some(proxy) = world.get_resource::<EventLoopProxyWrapper>() {
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
        if self.service.is_none()
            && !self.start_failed
            && (!states.is_empty() || world.contains_resource::<WidgetryFileDialogPersistence>())
        {
            let proxy = world
                .get_resource::<EventLoopProxyWrapper>()
                .map(|proxy| (**proxy).clone());
            let wake = Box::new(move || match &proxy {
                Some(proxy) => proxy
                    .send_event(WinitUserEvent::WakeUp)
                    .map_err(|error| error.to_string()),
                None => Ok(()),
            });
            match Service::start(world.resource::<WidgetryFileDialogBackend>().clone(), wake) {
                Ok(service) => self.service = Some(service),
                Err(error) => {
                    self.start_failed = true;
                    widgetry_error!(error = %error, "FileDialog 无法启动后台服务");
                    world
                        .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                        .error = Some(error.to_string());
                }
            }
        }
        let Some(service) = self.service.clone() else {
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
                            error: "FileDialog worker startup failed".into(),
                        },
                    )?;
                }
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
        if let Ok(fault) = service.fault.try_lock()
            && let Some(error) = fault.as_ref()
        {
            for session in self.sessions.values_mut() {
                session.stop(&mut self.retired)?;
            }
            let reported = world
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .error
                .as_ref()
                == Some(error);
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
        let start = Instant::now();
        let mut applied = 0;
        while applied < options.max_messages_per_update && start.elapsed() < options.update_budget {
            let directory_first = self.prefer_directory;
            self.prefer_directory = !self.prefer_directory;
            if directory_first && self.poll_jobs(world)? {
                applied += 1;
                continue;
            }
            match service.replies.try_recv() {
                Ok(output) => self.receive(world, output)?,
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    if directory_first || !self.poll_jobs(world)? {
                        break;
                    }
                }
                Err(error) => return Err(contract_error(&error.to_string())),
            }
            applied += 1;
        }
        // Result observers may end or despawn the root, so this reply must release its session and cache in the same frame.
        self.retain_sessions(|root, _| {
            world
                .get::<WidgetryFileDialogState>(*root)
                .is_some_and(|state| state.session_state() == WidgetryFileDialogSessionState::Open)
        })?;
        let mut retry = false;
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
            if service.shutdown.load(Ordering::Acquire) || session.failed {
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
                session.locations_admitted = service
                    .admit((*root, 3), IoWork::Locations(control.clone()))
                    .map_err(|error| contract_error(&error))?;
                if !session.locations_admitted {
                    retry = true;
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
            if let Some(job) = state.validation_job.as_ref()
                && session.validation_submitted != Some(job.token())
            {
                if service
                    .admit_cpu(CpuWork::Candidate(control.clone(), job.clone()))
                    .map_err(|error| contract_error(&error))?
                {
                    session.validation_submitted = Some(job.token());
                } else {
                    retry = true;
                }
            }
            if let Some(candidate) = &session.candidate {
                if service
                    .admit(
                        (*root, 1),
                        IoWork::Validate(control.clone(), candidate.clone()),
                    )
                    .map_err(|error| contract_error(&error))?
                {
                    session.candidate = None;
                } else {
                    retry = true;
                }
            }
            if let Some(request) = state.folder_request.as_ref()
                && session.folder_submitted != Some(request.token())
            {
                if service
                    .admit((*root, 2), IoWork::Folder(control, request.clone()))
                    .map_err(|error| contract_error(&error))?
                {
                    session.folder_submitted = Some(request.token());
                } else {
                    retry = true;
                }
            }
        }
        self.persistence(world, &service, &mut retry)?;
        {
            let mut status = world.resource_mut::<WidgetryFileDialogRuntimeStatus>();
            status.workers = service.live.load(Ordering::Acquire);
            status.active_sessions = self.sessions.len();
            status.queued = service.queued();
            status.retained_snapshots = service.retained.load(Ordering::Relaxed);
            status.applied_last_update = applied;
        }
        self.reap_retired()?;
        service.finish_poll();
        // A legacy reply can exhaust the shared budget before a queued Job item is observed.
        // One continuation also covers a time-budget exit without polling idle jobs indefinitely.
        let directory_budget_exit = (applied == options.max_messages_per_update
            || start.elapsed() >= options.update_budget)
            && self.sessions.values().any(|session| {
                session.directory.is_some()
                    || session.projection.is_some()
                    || session.selection.is_some()
            });
        if retry || directory_budget_exit {
            service.notify();
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
        if !self.prefer_selection {
            return self.poll_selection(world);
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
            Output::Candidate(control, candidate) => {
                if let Some(session) = self.sessions.get_mut(&control.root)
                    && world
                        .get::<WidgetryFileDialogState>(control.root)
                        .is_some_and(|state| {
                            state.token() == candidate.token()
                                && state.confirmation()
                                    == &WidgetryFileDialogConfirmation::Validating
                        })
                {
                    session.candidate = Some(candidate);
                }
            }
            Output::Locations(control, outcome) => {
                WidgetryFileDialog::deliver(
                    world,
                    control.root,
                    WidgetryFileDialogReply::Locations {
                        token: control.token,
                        outcome,
                    },
                )?;
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

    fn persistence(&mut self, world: &mut World, service: &Service, retry: &mut bool) -> Result {
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
            self.load_admitted = service
                .admit((Entity::PLACEHOLDER, 0), IoWork::Load(path.clone()))
                .map_err(|error| contract_error(&error))?;
            if self.load_admitted {
                world
                    .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                    .persistence = WidgetryFileDialogStorageState::Loading;
            } else {
                *retry = true;
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
            if service
                .admit(
                    (Entity::PLACEHOLDER, 1),
                    IoWork::Save(path, scopes, revisions),
                )
                .map_err(|error| contract_error(&error))?
            {
                self.saving = true;
                self.disk_revision = self
                    .disk_revision
                    .checked_add(1)
                    .ok_or_else(|| contract_error("disk revision exhausted"))?;
                if let Some(mut request) = world.get_resource_mut::<FlushRequest>() {
                    request.0 = false;
                }
                world
                    .resource_mut::<WidgetryFileDialogRuntimeStatus>()
                    .persistence = WidgetryFileDialogStorageState::Saving(self.disk_revision);
            } else {
                *retry = true;
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

    fn notify(&self) -> bool {
        if let Some(service) = &self.service {
            service.notify();
            true
        } else {
            false
        }
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
        // Publish the Job request before legacy workers can turn cancellation into Interrupted.
        if let Some(job) = &self.directory {
            job.manager
                .cancel(job.id)
                .map_err(|error| contract_error(&error.to_string()))?;
        }
        self.stop_projection(retired)?;
        self.stop_selection(retired)?;
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
            cached: None,
            version: 0,
            project_inflight: None,
            projection: None,
            project_applied: None,
            selection_submitted: None,
            selection: None,
            validation_submitted: None,
            folder_submitted: None,
            candidate: None,
            failed: false,
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        for session in self.sessions.values_mut() {
            if let Err(error) = session.stop(&mut self.retired) {
                widgetry_error!(%error, "FileDialog 释放会话时取消目录 Job 失败");
            }
        }
        if let Some(service) = &self.service {
            service.stop();
            service.diagnose_shutdown();
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

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
        runtime.service = Some(
            Service::start(WidgetryFileDialogBackend::default(), Box::new(|| Ok(())))
                .map_err(BevyError::error)?,
        );
        runtime.sessions.insert(root, session);
        Ok((app, runtime, root, snapshot))
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
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .map_err(BevyError::error)?;
        runtime.update(app.world_mut(), &Default::default())?;
        assert!(
            matches!(app.world().get::<WidgetryFileDialogState>(root).unwrap().directory_state(),
            WidgetryFileDialogDirectoryState::Failed(error) if error.contains("ShuttingDown"))
        );
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
        let service = Service::start(WidgetryFileDialogBackend::default(), Box::new(|| Ok(())))
            .map_err(BevyError::error)?;
        let mut runtime = Runtime::default();
        runtime.service = Some(service);
        runtime.sessions.insert(root, session);
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(std::time::Duration::ZERO)
            .map_err(BevyError::error)?;
        runtime.update(app.world_mut(), &Default::default())?;
        assert!(
            matches!(app.world().get::<WidgetryFileDialogState>(root).unwrap().directory_state(),
            WidgetryFileDialogDirectoryState::Failed(error) if error.contains("ShuttingDown"))
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
        let service = Service::start(WidgetryFileDialogBackend::default(), Box::new(|| Ok(())))
            .map_err(BevyError::error)?;
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
        runtime.service = Some(service.clone());
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
    fn legacy_reply_using_the_last_budget_slot_rearms_a_waiting_directory() -> Result {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog })?
            .id();
        let token = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        let manager = app.world().resource::<JobManager>().clone();
        let (id, receiver) = manager
            .spawn_io::<DirectoryEvent, _, _, String>(Default::default(), |context| async move {
                context
                    .send(DirectoryEvent::Started(PathBuf::from("C:/fixture")))
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .map_err(BevyError::error)?;
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = wakes.clone();
        let service = Service::start(
            WidgetryFileDialogBackend::default(),
            Box::new(move || {
                counter.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }),
        )
        .map_err(BevyError::error)?;
        let directory = tempfile::tempdir().map_err(BevyError::error)?;
        service
            .admit(
                (Entity::PLACEHOLDER, 0),
                IoWork::Load(directory.path().to_owned()),
            )
            .map_err(BevyError::error)?;
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while service.replies.is_empty() || !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let mut session = Session::new(root, token);
        session.admitted = true;
        session.locations_admitted = true;
        session.directory = Some(DirectoryJob {
            manager,
            id,
            receiver,
            final_snapshot: None,
        });
        let mut runtime = Runtime::default();
        runtime.service = Some(service.clone());
        runtime.sessions.insert(root, session);
        let before = wakes.load(Ordering::Relaxed);
        runtime.update(
            app.world_mut(),
            &WidgetryFileDialogRuntimeOptions {
                max_messages_per_update: 1,
                update_budget: std::time::Duration::from_secs(1),
                ..Default::default()
            },
        )?;
        assert!(service.replies.is_empty());
        assert!(
            wakes.load(Ordering::Relaxed) > before,
            "the directory must be retried after a legacy reply exhausts the shared budget"
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
    fn explicit_flush_wakes_a_dormant_service() -> Result {
        let wakes = Arc::new(AtomicUsize::new(0));
        let observed = wakes.clone();
        let service = Service::start(
            WidgetryFileDialogBackend::default(),
            Box::new(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }),
        )
        .map_err(BevyError::error)?;
        let mut runtime = Runtime::default();
        runtime.service = Some(service.clone());
        let mut world = World::new();
        world.insert_resource(runtime);
        world.init_resource::<FlushRequest>();
        world.insert_resource(WidgetryFileDialogPersistence {
            path: PathBuf::from("preferences.json"),
        });
        WidgetryFileDialog::flush_preferences(&mut world)?;
        assert_eq!(wakes.load(Ordering::Relaxed), 1);
        assert!(world.resource::<FlushRequest>().0);
        service.finish_poll();
        WidgetryFileDialog::flush_preferences(&mut world)?;
        assert_eq!(wakes.load(Ordering::Relaxed), 2);
        Ok(())
    }

    #[test]
    fn late_widget_action_wakes_runtime_and_same_value_does_not() -> Result {
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let service = Service::start(
            WidgetryFileDialogBackend::default(),
            Box::new(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }),
        )
        .map_err(BevyError::error)?;
        let mut runtime = Runtime::default();
        runtime.service = Some(service);
        let mut world = World::new();
        world.insert_resource(runtime);
        world.init_resource::<WidgetryFileDialogRuntimeOptions>();
        let root = world
            .spawn((
                WidgetryFileDialog,
                WidgetryFileDialogState::new(WidgetryFileDialogProps::default())?,
            ))
            .id();
        assert!(!WidgetryFileDialog::apply(
            &mut world,
            root,
            WidgetryFileDialogAction::Search(String::new())
        )?);
        assert_eq!(count.load(Ordering::Relaxed), 0);
        assert!(WidgetryFileDialog::apply(
            &mut world,
            root,
            WidgetryFileDialogAction::Search("query".into())
        )?);
        assert_eq!(count.load(Ordering::Relaxed), 1);
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
