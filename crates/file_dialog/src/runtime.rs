use crate::model::contract_error;
use crate::worker::{Control, CpuWork, IoWork, Output, Service};
use crate::*;
use bevy::prelude::*;
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use bevy_widgetry_log::widgetry_error;
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
}

struct Session {
    control: Control,
    admitted: bool,
    locations_admitted: bool,
    cached: Option<(
        Arc<WidgetryFileDialogSnapshot>,
        WidgetryFileDialogDirectoryState,
    )>,
    version: u64,
    project_inflight: Option<(WidgetryFileDialogToken, u64)>,
    project_applied: Option<(WidgetryFileDialogToken, u64)>,
    selection_submitted: Option<WidgetryFileDialogToken>,
    validation_submitted: Option<WidgetryFileDialogToken>,
    folder_submitted: Option<WidgetryFileDialogToken>,
    candidate: Option<WidgetryFileDialogCandidate>,
    failed: bool,
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
        self.sessions.retain(|root, session| {
            let keep = states.iter().any(|(entity, token, _)| {
                entity == root
                    && token.session == session.control.token.session
                    && token.generation == session.control.token.generation
            });
            if !keep {
                session.control.cancelled.store(true, Ordering::Release);
            }
            keep
        });
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
            let output = match service.replies.try_recv() {
                Ok(output) => output,
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(error) => return Err(contract_error(&error.to_string())),
            };
            self.receive(world, output)?;
            applied += 1;
        }
        // Result observer 可以结束或销毁 root；本次回复必须在同帧释放 session/cache。
        self.sessions.retain(|root, session| {
            let keep = world
                .get::<WidgetryFileDialogState>(*root)
                .is_some_and(|state| state.session_state() == WidgetryFileDialogSessionState::Open);
            if !keep {
                session.control.cancelled.store(true, Ordering::Release);
            }
            keep
        });
        let mut retry = false;
        for (root, session) in &mut self.sessions {
            let Some(state) = world.get::<WidgetryFileDialogState>(*root) else {
                continue;
            };
            if state.session_state() != WidgetryFileDialogSessionState::Open {
                session.control.cancelled.store(true, Ordering::Release);
                continue;
            }
            // 回复（例如 FolderCreated → Refresh）可以在本轮推进 generation。
            // 新请求必须从全新的 cache/admission 开始，不能携带旧目录 projection。
            if state.token().session != session.control.token.session
                || state.token().generation != session.control.token.generation
            {
                session.control.cancelled.store(true, Ordering::Release);
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
                session.admitted = service
                    .admit(
                        (*root, 0),
                        IoWork::Directory(control.clone(), path, state.query()),
                    )
                    .map_err(|error| contract_error(&error))?;
                if !session.admitted {
                    retry = true;
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
            if let Some((snapshot, status)) = &session.cached {
                let signature = (state.token(), session.version);
                if session.project_inflight.is_none() && session.project_applied != Some(signature)
                {
                    if service
                        .admit_cpu(CpuWork::Projection(
                            control.clone(),
                            Arc::new(state.clone()),
                            snapshot.clone(),
                            status.clone(),
                            session.version,
                        ))
                        .map_err(|error| contract_error(&error))?
                    {
                        session.project_inflight = Some(signature);
                    } else {
                        retry = true;
                    }
                }
            }
            if session.project_inflight.is_none()
                && let Some(job) = state.selection_job.as_ref()
                && session.selection_submitted != Some(job.token())
            {
                if service
                    .admit_cpu(CpuWork::Selection(control.clone(), job.clone()))
                    .map_err(|error| contract_error(&error))?
                {
                    session.selection_submitted = Some(job.token());
                } else {
                    retry = true;
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
        service.finish_poll();
        if retry {
            service.notify();
        }
        Ok(())
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
                        session.control.cancelled.store(true, Ordering::Release);
                    }
                    WidgetryFileDialog::deliver(world, root, reply)?;
                }
            }
            Output::Projection(control, version, reply) => {
                if let Some(session) = self.sessions.get_mut(&control.root)
                    && session.project_inflight == Some((control.token, version))
                {
                    let signature = session.project_inflight.take();
                    if !session.failed && WidgetryFileDialog::deliver(world, control.root, reply)? {
                        session.project_applied = signature;
                    }
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
                        let unavailable = scopes
                            .values()
                            .any(|snapshot| snapshot.unavailable_paths > 0);
                        let merged =
                            merge_loaded(world.resource::<WidgetryFileDialogStorage>(), scopes);
                        let storage_state = match merged {
                            Ok(store) => {
                                world.insert_resource(store);
                                if unavailable {
                                    WidgetryFileDialogStorageState::Failed(
                                        "stored paths from another platform are unavailable".into(),
                                    )
                                } else {
                                    WidgetryFileDialogStorageState::Memory
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

    pub(crate) fn cancel(&mut self, root: Entity) {
        if let Some(session) = self.sessions.get(&root) {
            session.control.cancelled.store(true, Ordering::Release);
        }
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
    fn new(root: Entity, token: WidgetryFileDialogToken) -> Self {
        Self {
            control: Control {
                root,
                token,
                cancelled: Arc::new(AtomicBool::new(false)),
            },
            admitted: false,
            locations_admitted: false,
            cached: None,
            version: 0,
            project_inflight: None,
            project_applied: None,
            selection_submitted: None,
            validation_submitted: None,
            folder_submitted: None,
            candidate: None,
            failed: false,
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        for session in self.sessions.values() {
            session.control.cancelled.store(true, Ordering::Release);
        }
        if let Some(service) = &self.service {
            service.stop();
            service.diagnose_shutdown();
        }
    }
}

#[cfg(test)]
// 测试断言保护 reactive wake contract，不适用生产 macro 禁令。
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

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
        let pinned = PathBuf::from(if cfg!(windows) {
            "C:/fixture/pin"
        } else {
            "/fixture/pin"
        });
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
        let prefix = if cfg!(windows) {
            "C:/fixture"
        } else {
            "/fixture"
        };
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
