use crate::{
    gallery::{GalleryPage, mount_page, unmount_page},
    i18n::{JobKey, Language, LocalizedText, TextKey, localized, localized_args},
};
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::{
    button::WidgetryButton,
    job::*,
    progress_bar::{WidgetryProgressBar, WidgetryProgressState},
    scene::spawn_scene,
    text::WidgetryText,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};

pub(crate) struct JobDemoPlugin;

impl Plugin for JobDemoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Orphans>()
            .register_type::<JobDemoState>()
            .add_systems(OnEnter(GalleryPage::Job), enter)
            .add_systems(OnExit(GalleryPage::Job), exit)
            .add_systems(Update, (reap_orphans, poll).chain())
            .add_systems(Last, redraw);
    }
}

#[derive(Component, FromTemplate, Clone, Copy)]
struct Parts {
    rows: Entity,
    operation: Entity,
    selection: Entity,
}

#[derive(Component, FromTemplate, Clone, Copy)]
struct Row {
    status: Entity,
    a: Entity,
    b: Entity,
}

#[derive(Resource)]
struct Demo {
    root: Entity,
    parts: Parts,
    selected: Option<JobId>,
    slow_b: bool,
    runs: Vec<Run>,
}

#[derive(Resource, Default)]
struct Orphans(Vec<JobId>);

struct Run {
    id: JobId,
    mode: Mode,
    row: Row,
    total: u32,
    produced: Arc<AtomicU32>,
    status: JobStatus,
    removed: bool,
    a: Consumer,
    b: Consumer,
}

#[derive(Default)]
struct Consumer {
    receiver: Option<JobReceiver<DemoEvent>>,
    items: u32,
    batches: u32,
    first: Option<u32>,
    last: Option<u32>,
    progress: u32,
    finished: Option<JobOutcome>,
    last_batch: Option<Arc<DemoEvent>>,
    next_read: Option<Instant>,
}

enum DemoEvent {
    Batch { sequence: u32, values: Vec<u64> },
    Progress { processed: u32, total: u32 },
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Io,
    Cpu,
    Thread,
    Failure,
    LateComplete,
}

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    StartIo,
    StartCpu,
    StartThread,
    StartFailure,
    StartLateComplete,
    SubscribeB,
    DropA,
    DropB,
    ToggleSlowB,
    Cancel,
    Remove,
}

#[derive(Component)]
struct Select(JobId);

#[derive(Component, Reflect, Default, PartialEq)]
#[reflect(Component)]
struct JobDemoState {
    selected: u64,
    slow_b: bool,
    jobs: Vec<JobSnapshot>,
}

#[derive(Reflect, PartialEq)]
struct JobSnapshot {
    id: u64,
    mode: String,
    backend: String,
    manager_state: String,
    record_removed: bool,
    produced: u32,
    total: u32,
    items_a: u32,
    items_b: u32,
    batches_a: u32,
    batches_b: u32,
    first_a: Option<u32>,
    first_b: Option<u32>,
    last_a: Option<u32>,
    last_b: Option<u32>,
    progress_a: u32,
    progress_b: u32,
    attached_a: bool,
    attached_b: bool,
    finished_a: String,
    finished_b: String,
    shared_last_batch: bool,
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Job, bsn_list! { @scene() })?;
    let (root, parts) = world
        .query::<(Entity, &Parts)>()
        .single(world)
        .map(|(entity, parts)| (entity, *parts))
        .map_err(error)?;
    world.insert_resource(Demo {
        root,
        parts,
        selected: None,
        slow_b: true,
        runs: Vec::new(),
    });
    Ok(())
}

fn exit(world: &mut World) -> Result {
    if let Some(demo) = world.remove_resource::<Demo>() {
        let manager = world.resource::<JobManager>().clone();
        for run in demo.runs {
            if run.removed {
                continue;
            }
            match manager.cancel(run.id) {
                Ok(_) | Err(JobAccessError::NotFound) => {}
                Err(failure) => return Err(error(failure)),
            }
            match manager.remove(run.id) {
                Ok(_) | Err(JobAccessError::NotFound) => {}
                Err(JobAccessError::NotFinished(_)) => {
                    world.resource_mut::<Orphans>().0.push(run.id)
                }
                Err(failure) => return Err(error(failure)),
            }
        }
    }
    unmount_page(world, GalleryPage::Job)
}

fn error(reason: impl std::fmt::Display) -> BevyError {
    error!(%reason, "Job 演示内部操作失败");
    BevyError::error(reason.to_string())
}

fn key(action: Action) -> JobKey {
    match action {
        Action::StartIo => JobKey::StartIo,
        Action::StartCpu => JobKey::StartCpu,
        Action::StartThread => JobKey::StartThread,
        Action::StartFailure => JobKey::StartFailure,
        Action::StartLateComplete => JobKey::StartLateComplete,
        Action::SubscribeB => JobKey::SubscribeB,
        Action::DropA => JobKey::DropA,
        Action::DropB => JobKey::DropB,
        Action::ToggleSlowB => JobKey::ToggleSlowB,
        Action::Cancel => JobKey::Cancel,
        Action::Remove => JobKey::Remove,
    }
}

fn scene() -> impl Scene {
    let buttons: Vec<_> = [Action::StartIo, Action::StartCpu, Action::StartThread, Action::StartFailure,
        Action::StartLateComplete, Action::SubscribeB, Action::DropA, Action::DropB,
        Action::ToggleSlowB, Action::Cancel, Action::Remove].into_iter().map(|action| bsn! {
            Name({format!("Job{action:?}")}) template(move |_| Ok(action))
            @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Job(key(action))) WidgetryText }) as Box<dyn SceneList>)} }
            on(operate)
        }).collect();
    bsn! {
        Name("JobDemo") Parts { rows: #Rows, operation: #Operation, selection: #Selection }
        template(|_| Ok(JobDemoState::default()))
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(12), flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Job(JobKey::Description)) WidgetryText Node { max_width: percent(100) }--
            Node { flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(8), max_width: percent(100) } Children [{buttons}]--
            #Selection Name("JobSelection") @localized(TextKey::Job(JobKey::Ready)) WidgetryText--
            #Operation Name("JobOperationResult") Text("") WidgetryText--
            #Rows Name("JobRows") Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(12) }
        ]
    }
}

fn row(id: JobId, mode: Mode) -> impl Scene {
    bsn! {
        Name({format!("JobRow{}", id.get())}) Row { status: #Status, a: #A, b: #B }
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8), padding: UiRect::all(px(10)), flex_shrink: 0.0 }
        Children [
            Name({format!("JobSelect{}", id.get())}) template(move |_| Ok(Select(id)))
            @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized_args(TextKey::Job(JobKey::Select), vec![("id",id.get().to_string()),("kind",format!("{mode:?}"))]) WidgetryText }) as Box<dyn SceneList>)} }
            on(select)--
            #Status Name({format!("JobState{}", id.get())}) Text("") WidgetryText Node { max_width: percent(100) }--
            #A Name({format!("JobProgressA{}", id.get())}) @WidgetryProgressBar--
            #B Name({format!("JobProgressB{}", id.get())}) @WidgetryProgressBar
        ]
    }
}

fn batch(sequence: u32, cpu: bool) -> DemoEvent {
    let mut value = u64::from(sequence).wrapping_add(0x6a09e667f3bcc909);
    for _ in 0..if cpu { 8_000_000 } else { 128 } {
        value = value.rotate_left(13).wrapping_mul(0x9e3779b97f4a7c15) ^ (value >> 7);
    }
    DemoEvent::Batch {
        sequence,
        values: (0..8192).map(|index| value.wrapping_add(index)).collect(),
    }
}

async fn send(
    context: &JobContext<DemoEvent>,
    item: DemoEvent,
) -> std::result::Result<bool, String> {
    match context.send(item).await {
        Ok(()) => Ok(true),
        Err(SendError::Canceled) => {
            context.acknowledge_cancel();
            Ok(false)
        }
        Err(failure) => Err(failure.to_string()),
    }
}

async fn async_work(
    context: JobContext<DemoEvent>,
    mode: Mode,
    total: u32,
    produced: Arc<AtomicU32>,
) -> std::result::Result<(), String> {
    if matches!(mode, Mode::Failure) {
        futures_timer::Delay::new(Duration::from_millis(500)).await;
        return Err("Requested deterministic demo failure".into());
    }
    for sequence in 1..=total {
        if context.is_cancelled() {
            context.acknowledge_cancel();
            return Ok(());
        }
        if matches!(mode, Mode::Io) {
            futures_timer::Delay::new(Duration::from_millis(80)).await;
        }
        if !send(&context, batch(sequence, matches!(mode, Mode::Cpu))).await? {
            return Ok(());
        }
        produced.store(sequence, Ordering::Release);
        if !send(
            &context,
            DemoEvent::Progress {
                processed: sequence,
                total,
            },
        )
        .await?
        {
            return Ok(());
        }
    }
    Ok(())
}

fn thread_work(
    context: JobContext<DemoEvent>,
    mode: Mode,
    total: u32,
    produced: Arc<AtomicU32>,
) -> std::result::Result<(), String> {
    if matches!(mode, Mode::LateComplete) {
        std::thread::sleep(Duration::from_secs(3));
        return match context.send_blocking(DemoEvent::Progress {
            processed: 1,
            total,
        }) {
            Ok(()) | Err(SendError::Canceled) => Ok(()),
            Err(failure) => Err(failure.to_string()),
        };
    }
    for sequence in 1..=total {
        let canceled = bevy::tasks::block_on(futures_util::future::select(
            Box::pin(context.cancelled()),
            Box::pin(futures_timer::Delay::new(Duration::from_millis(50))),
        ));
        if matches!(canceled, futures_util::future::Either::Left(_)) {
            std::thread::sleep(Duration::from_millis(200));
            context.acknowledge_cancel();
            return Ok(());
        }
        for item in [
            batch(sequence, false),
            DemoEvent::Progress {
                processed: sequence,
                total,
            },
        ] {
            let is_batch = matches!(item, DemoEvent::Batch { .. });
            match context.send_blocking(item) {
                Ok(()) => {
                    if is_batch {
                        produced.store(sequence, Ordering::Release);
                    }
                }
                Err(SendError::Canceled) => {
                    std::thread::sleep(Duration::from_millis(200));
                    context.acknowledge_cancel();
                    return Ok(());
                }
                Err(failure) => return Err(failure.to_string()),
            }
        }
    }
    Ok(())
}

fn start(world: &mut World, demo: &mut Demo, mode: Mode) -> std::result::Result<String, String> {
    if demo.runs.len() >= 8 {
        return Err(
            crate::i18n::tr(*world.resource::<Language>(), TextKey::Job(JobKey::Limit)).into(),
        );
    }
    let total = match mode {
        Mode::Io => 8,
        Mode::Cpu => 64,
        Mode::Thread => 32,
        _ => 1,
    };
    let produced = Arc::new(AtomicU32::new(0));
    let worker = produced.clone();
    let options = JobOptions {
        label: Some(format!("Gallery {mode:?} demo")),
        buffer_capacity: Some(4),
    };
    let manager = world.resource::<JobManager>();
    let submitted = match mode {
        Mode::Thread | Mode::LateComplete => manager.spawn_thread(options, move |context| {
            thread_work(context, mode, total, worker)
        }),
        Mode::Cpu => manager.spawn_cpu(options, move |context| {
            async_work(context, mode, total, worker)
        }),
        _ => manager.spawn_io(options, move |context| {
            async_work(context, mode, total, worker)
        }),
    }
    .map_err(|failure| failure.to_string())?;
    let (id, receiver) = submitted;
    let status = manager.status(id).map_err(|failure| failure.to_string())?;
    let rows = demo.parts.rows;
    let entity = match spawn_scene(world, bsn! { ChildOf(rows) @row(id, mode) }) {
        Ok(entity) => entity,
        Err(failure) => {
            world
                .resource::<JobManager>()
                .cancel(id)
                .map_err(|failure| failure.to_string())?;
            world.resource_mut::<Orphans>().0.push(id);
            return Err(failure.to_string());
        }
    };
    let row = *world
        .get::<Row>(entity)
        .ok_or_else(|| "Job row parts missing".to_owned())?;
    demo.runs.push(Run {
        id,
        mode,
        row,
        total,
        produced,
        status,
        removed: false,
        a: Consumer {
            receiver: Some(receiver),
            ..default()
        },
        b: default(),
    });
    demo.selected = Some(id);
    Ok(format!("#{}, {:?}", id.get(), mode))
}

fn operate(event: On<Activate>, actions: Query<&Action>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    commands.queue(move |world: &mut World| -> Result {
        if !world.contains_resource::<Demo>() {
            return Ok(());
        }
        world.resource_scope(|world, mut demo: Mut<Demo>| -> Result {
            let result = apply(world, &mut demo, action);
            match &result {
                Ok(result) => info!(?action, %result, "执行 Job 演示操作"),
                Err(failure) => warn!(?action, %failure, "Job 演示操作被拒绝"),
            }
            bind(
                world,
                demo.parts.operation,
                LocalizedText::with_args(
                    TextKey::Job(JobKey::Operation),
                    vec![("result", result.unwrap_or_else(|failure| failure))],
                )
                .with_key_arg("action", TextKey::Job(key(action))),
            )
        })
    });
}

fn apply(
    world: &mut World,
    demo: &mut Demo,
    action: Action,
) -> std::result::Result<String, String> {
    let mode = match action {
        Action::StartIo => Some(Mode::Io),
        Action::StartCpu => Some(Mode::Cpu),
        Action::StartThread => Some(Mode::Thread),
        Action::StartFailure => Some(Mode::Failure),
        Action::StartLateComplete => Some(Mode::LateComplete),
        _ => None,
    };
    if let Some(mode) = mode {
        return start(world, demo, mode);
    }
    if matches!(action, Action::ToggleSlowB) {
        demo.slow_b = !demo.slow_b;
        return Ok(format!("slow_b={}", demo.slow_b));
    }
    let run = demo
        .runs
        .iter_mut()
        .find(|run| Some(run.id) == demo.selected)
        .ok_or("Select a Job first")?;
    let manager = world.resource::<JobManager>();
    match action {
        Action::SubscribeB => {
            if run.b.receiver.is_some() {
                return Err("B is already subscribed".into());
            }
            let receiver = manager
                .subscribe(run.id)
                .map_err(|failure| failure.to_string())?;
            run.b = Consumer {
                receiver: Some(receiver),
                ..default()
            };
            Ok(format!("#{} B", run.id.get()))
        }
        Action::DropA => {
            run.a.receiver = None;
            run.a.last_batch = None;
            Ok("A".into())
        }
        Action::DropB => {
            run.b.receiver = None;
            run.b.last_batch = None;
            Ok("B".into())
        }
        Action::Cancel => manager
            .cancel(run.id)
            .map(|result| format!("{result:?}"))
            .map_err(|failure| failure.to_string()),
        Action::Remove => manager
            .remove(run.id)
            .map(|status| {
                run.status = status;
                run.removed = true;
                "removed".into()
            })
            .map_err(|failure| failure.to_string()),
        _ => Err("Unexpected Job action".into()),
    }
}

fn select(event: On<Activate>, choices: Query<&Select>, mut demo: ResMut<Demo>) {
    if let Ok(choice) = choices.get(event.entity) {
        demo.selected = Some(choice.0);
        info!(job_id = choice.0.get(), "选择 Job 演示任务");
    }
}

impl Consumer {
    fn drain(&mut self, slow: bool) -> Result {
        let now = Instant::now();
        if slow && self.next_read.is_some_and(|next| now < next) {
            return Ok(());
        }
        let Some(receiver) = &mut self.receiver else {
            return Ok(());
        };
        let mut finished = None;
        for _ in 0..if slow { 1 } else { 8 } {
            match receiver.try_recv() {
                Ok(JobMessage::Item(item)) => {
                    self.items += 1;
                    match &*item {
                        DemoEvent::Batch { sequence, values } => {
                            std::hint::black_box(values.first());
                            self.batches += 1;
                            self.first.get_or_insert(*sequence);
                            self.last = Some(*sequence);
                            self.last_batch = Some(item);
                        }
                        DemoEvent::Progress { processed, total } => {
                            if processed > total {
                                return Err(error("Job progress exceeds its total"));
                            }
                            self.progress = *processed;
                        }
                    }
                }
                Ok(JobMessage::Finished(outcome)) => {
                    finished = Some(outcome);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(failure) => return Err(error(failure)),
            }
        }
        if let Some(outcome) = finished {
            self.finished = Some(outcome);
            self.receiver = None;
        }
        self.next_read = slow.then_some(now + Duration::from_millis(200));
        Ok(())
    }

    fn flow(&self, language: Language) -> String {
        match &self.finished {
            Some(outcome) => format!("Finished({outcome:?})"),
            None => crate::i18n::tr(
                language,
                TextKey::Job(if self.receiver.is_some() {
                    JobKey::Receiving
                } else {
                    JobKey::Detached
                }),
            )
            .into(),
        }
    }
}

fn bind(world: &mut World, entity: Entity, binding: LocalizedText) -> Result {
    world.get_entity_mut(entity).map_err(error)?.insert(binding);
    Ok(())
}

fn poll(world: &mut World) -> Result {
    if !world.contains_resource::<Demo>() {
        return Ok(());
    }
    world.resource_scope(|world, mut demo: Mut<Demo>| -> Result {
        let manager = world.resource::<JobManager>().clone();
        let language = *world.resource::<Language>();
        let slow_b = demo.slow_b;
        let selected = demo.selected;
        let selection = demo.parts.selection;
        let mut snapshots = Vec::new();
        for run in &mut demo.runs {
            run.a.drain(false)?; run.b.drain(slow_b)?;
            if !run.removed {
                match manager.status(run.id) {
                    Ok(status) => {
                        if !run.status.state.is_finished() && status.state.is_finished() {
                            if matches!(status.state, JobState::Failed) {
                                warn!(job_id=run.id.get(), outcome=?status.outcome, "Job 演示任务执行失败");
                            } else {
                                info!(job_id=run.id.get(), outcome=?status.outcome, "Job 演示任务执行结束");
                            }
                        }
                        run.status = status;
                    }
                    Err(JobAccessError::NotFound) => run.removed = true,
                    Err(failure) => return Err(error(failure)),
                }
            }
            let shared = match (&run.a.last_batch, &run.b.last_batch) { (Some(a), Some(b)) => Arc::ptr_eq(a,b), _ => false };
            let state = if run.removed { format!("{:?} (record removed)", run.status.state) } else { format!("{:?}", run.status.state) };
            let produced = run.produced.load(Ordering::Acquire);
            let sequence = |value: Option<u32>| value.map_or_else(|| "—".into(), |value| value.to_string());
            bind(world, run.row.status, LocalizedText::with_args(TextKey::Job(JobKey::Stats), vec![
                ("state",state.clone()), ("produced",produced.to_string()), ("total",run.total.to_string()), ("shared",shared.to_string()),
                ("items_a",run.a.items.to_string()), ("items_b",run.b.items.to_string()),
                ("a",run.a.batches.to_string()), ("b",run.b.batches.to_string()),
                ("first_a",sequence(run.a.first)), ("first_b",sequence(run.b.first)), ("last_a",sequence(run.a.last)), ("last_b",sequence(run.b.last)),
                ("progress_a",run.a.progress.to_string()), ("progress_b",run.b.progress.to_string()), ("flow_a",run.a.flow(language)), ("flow_b",run.b.flow(language)),
            ]))?;
            for (entity, progress) in [(run.row.a, run.a.progress), (run.row.b, run.b.progress)] {
                WidgetryProgressBar::set_state_in_world(world, entity, WidgetryProgressState::Determinate { value: progress as f32 / run.total as f32 })?;
            }
            if selected == Some(run.id) {
                bind(world, selection, LocalizedText::with_args(TextKey::Job(JobKey::Selection), vec![
                    ("id",run.id.get().to_string()), ("kind",format!("{:?}/{:?}",run.mode,run.status.kind)), ("state",state),
                ]).with_key_arg("speed", TextKey::Job(if slow_b { JobKey::Slow } else { JobKey::Fast })))?;
            }
            snapshots.push(JobSnapshot { id:run.id.get(), mode:format!("{:?}",run.mode), backend:format!("{:?}",run.status.kind),
                manager_state:format!("{:?}",run.status.state), record_removed:run.removed, produced, total:run.total,
                items_a:run.a.items,items_b:run.b.items,batches_a:run.a.batches,batches_b:run.b.batches,
                first_a:run.a.first,first_b:run.b.first,last_a:run.a.last,last_b:run.b.last,
                progress_a:run.a.progress,progress_b:run.b.progress,attached_a:run.a.receiver.is_some(),attached_b:run.b.receiver.is_some(),
                finished_a:format!("{:?}",run.a.finished),finished_b:format!("{:?}",run.b.finished), shared_last_batch:shared });
        }
        world.get_mut::<JobDemoState>(demo.root).ok_or_else(|| error("Job diagnostics missing"))?
            .set_if_neq(JobDemoState { selected:demo.selected.map_or(0,JobId::get), slow_b, jobs:snapshots });
        Ok(())
    })
}

fn reap_orphans(manager: Res<JobManager>, mut orphans: ResMut<Orphans>) -> Result {
    let mut remaining = Vec::new();
    for id in std::mem::take(&mut orphans.0) {
        match manager.remove(id) {
            Ok(_) | Err(JobAccessError::NotFound) => {}
            Err(JobAccessError::NotFinished(_)) => remaining.push(id),
            Err(failure) => return Err(error(failure)),
        }
    }
    orphans.0 = remaining;
    Ok(())
}

fn redraw(
    demo: Option<Res<Demo>>,
    orphans: Res<Orphans>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    if !orphans.0.is_empty()
        || demo.is_some_and(|demo| {
            demo.runs.iter().any(|run| {
                !run.status.state.is_finished()
                    || run.a.receiver.is_some()
                    || run.b.receiver.is_some()
            })
        })
    {
        redraw.write(bevy::window::RequestRedraw);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn sent_thread_batch_is_counted_when_cancellation_interrupts_its_progress_broadcast() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let manager = JobManager::default();
        let produced = Arc::new(AtomicU32::new(0));
        let worker = produced.clone();
        let (start, ready) = std::sync::mpsc::channel();
        let (id, mut a) = manager
            .spawn_thread(
                JobOptions {
                    buffer_capacity: Some(1),
                    ..default()
                },
                move |context| {
                    ready.recv().unwrap();
                    thread_work(context, Mode::Thread, 1, worker)
                },
            )
            .unwrap();
        let _b = manager.subscribe::<DemoEvent>(id).unwrap();
        start.send(()).unwrap();
        assert!(matches!(a.recv_blocking().unwrap(), JobMessage::Item(item)
            if matches!(&*item, DemoEvent::Batch { sequence: 1, .. })));
        manager.cancel(id).unwrap();
        assert!(matches!(
            a.recv_blocking().unwrap(),
            JobMessage::Finished(JobOutcome::Canceled)
        ));
        assert_eq!(produced.load(Ordering::Acquire), 1);
    }

    #[test]
    fn progress_without_a_batch_is_not_counted_as_a_produced_batch() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let manager = JobManager::default();
        let produced = Arc::new(AtomicU32::new(0));
        let worker = produced.clone();
        let (_, mut receiver) = manager
            .spawn_thread(JobOptions::default(), move |context| {
                thread_work(context, Mode::LateComplete, 1, worker)
            })
            .unwrap();
        assert!(
            matches!(receiver.recv_blocking().unwrap(), JobMessage::Item(item)
            if matches!(&*item, DemoEvent::Progress { processed: 1, total: 1 }))
        );
        assert!(matches!(
            receiver.recv_blocking().unwrap(),
            JobMessage::Finished(JobOutcome::Completed)
        ));
        assert_eq!(produced.load(Ordering::Acquire), 0);
    }
}
