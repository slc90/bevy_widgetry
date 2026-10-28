use super::GalleryTrayState;
use crate::{
    gallery::GalleryPageHost,
    i18n::{CloseKey, LocalizedText, TextKey, localized},
};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ui_widgets::Activate;
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use bevy::{prelude::*, window::RequestRedraw};
use bevy_widgetry::{
    button::WidgetryButton, job::*, scene::spawn_scene, text::WidgetryText,
    theme::WidgetryThemeMode,
};
use std::time::Duration;

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Idle,
    Waiting,
    TimedOut,
    Continuing,
    Completed,
    Forced,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct GalleryShutdownState {
    stage: Stage,
    remaining: usize,
    timeout_ms: u64,
    timeout_notifications: u32,
    exit_sent: bool,
}

#[derive(Resource)]
struct ExitFlow {
    handle: Option<ShutdownHandle>,
    intent: AppExit,
    timeout: Duration,
    windows_prepared: bool,
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct CaptureExitIntents;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct CaptureRemoteExitIntents;

impl Default for ExitFlow {
    fn default() -> Self {
        Self {
            handle: None,
            intent: AppExit::Success,
            timeout: Duration::from_secs(1),
            windows_prepared: false,
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<ExitFlow>()
        .init_resource::<GalleryShutdownState>()
        .register_type::<GalleryShutdownState>()
        .init_schedule(CaptureExitIntents)
        .add_systems(CaptureExitIntents, drive)
        .add_systems(First, prepare_windows)
        .add_systems(Update, show.after(super::close_requests));
    // BRP also has a Last exit listener outside OnAppExitSystems.
    app.world_mut()
        .resource_mut::<bevy::app::MainScheduleOrder>()
        .insert_before(Last, CaptureExitIntents);
    if app
        .world()
        .resource::<bevy::app::MainScheduleOrder>()
        .labels
        .contains(&bevy_remote::RemoteLast.intern())
    {
        // Remote requests run after Last; capture newly received intents before the runner checks AppExit.
        app.init_schedule(CaptureRemoteExitIntents)
            .add_systems(CaptureRemoteExitIntents, capture);
        app.world_mut()
            .resource_mut::<bevy::app::MainScheduleOrder>()
            .insert_after(bevy_remote::RemoteLast, CaptureRemoteExitIntents);
    }
}

pub(super) fn begin(world: &mut World, intent: AppExit) -> Result {
    if !matches!(intent, AppExit::Success) {
        world.resource_mut::<ExitFlow>().intent = intent;
    }
    if world.resource::<ExitFlow>().handle.is_some() {
        return Ok(());
    }
    crate::canvas_experiment::cancel_stage_exports(world)?;
    let timeout = world.resource::<ExitFlow>().timeout;
    let handle = world
        .resource::<JobManager>()
        .begin_shutdown(timeout)
        .map_err(|failure| {
            error!(%failure, "Gallery 无法开始后台任务收尾");
            BevyError::error(failure)
        })?;
    let remaining = handle.status().remaining.len();
    world.resource_mut::<ExitFlow>().handle = Some(handle);
    {
        let mut state = world.resource_mut::<GalleryShutdownState>();
        state.stage = Stage::Waiting;
        state.remaining = remaining;
        state.timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
    }
    world.resource_mut::<GalleryTrayState>().exiting = true;
    info!(
        timeout_ms = timeout.as_millis(),
        "Gallery 开始等待全部后台任务真实退出"
    );
    world.write_message(RequestRedraw);
    if let Some(proxy) = world.get_resource::<EventLoopProxyWrapper>() {
        proxy
            .send_event(WinitUserEvent::WakeUp)
            .map_err(|failure| {
                error!(%failure, "Gallery 无法唤醒关闭界面");
                BevyError::error(failure.to_string())
            })?;
    }
    Ok(())
}

fn prepare_windows(world: &mut World) {
    let flow = world.resource::<ExitFlow>();
    if flow.handle.is_none() || flow.windows_prepared {
        return;
    }
    world.resource_mut::<ExitFlow>().windows_prepared = true;
    // Late remote intents must not make ECS visible before Last can show the native windows.
    // Otherwise winit can wait forever for redraws from windows that are still hidden.
    let (dialog, hidden) = {
        let mut tray = world.resource_mut::<GalleryTrayState>();
        tray.exiting = true;
        tray.focus_after_restore = None;
        (tray.dialog.take(), std::mem::take(&mut tray.hidden_windows))
    };
    if let Some(dialog) = dialog {
        world.despawn(dialog);
        world.flush();
    }
    for entity in hidden {
        if let Some(mut window) = world.get_mut::<Window>(entity) {
            window.visible = true;
            info!(?entity, "Gallery 退出前已提交托盘窗口恢复变更");
        }
    }
    for mut window in world
        .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>()
        .iter_mut(world)
    {
        window.visible = true;
        window.set_minimized(false);
    }
    world.write_message(RequestRedraw);
}

fn capture(world: &mut World) -> Result {
    if world.resource::<GalleryShutdownState>().exit_sent {
        return Ok(());
    }
    let requests: Vec<_> = world.resource_mut::<Messages<AppExit>>().drain().collect();
    for intent in requests {
        begin(world, intent)?;
    }
    Ok(())
}

fn drive(world: &mut World) -> Result {
    if world.resource::<GalleryShutdownState>().exit_sent {
        return Ok(());
    }
    capture(world)?;
    let Some(handle) = world.resource::<ExitFlow>().handle.clone() else {
        return Ok(());
    };
    let remaining = handle.status().remaining.len();
    let mut changed = false;
    {
        let mut state = world.resource_mut::<GalleryShutdownState>();
        if state.remaining != remaining {
            state.remaining = remaining;
            changed = true;
        }
        while let Some(message) = handle.try_recv() {
            match message {
                ShutdownMessage::TimedOut { remaining } => {
                    state.timeout_notifications += 1;
                    if state.stage != Stage::Forced {
                        state.stage = Stage::TimedOut;
                    }
                    warn!(
                        remaining = remaining.len(),
                        timeout_ms = state.timeout_ms,
                        "Gallery 后台任务收尾超时，等待用户决策"
                    );
                }
                ShutdownMessage::Completed => {
                    if state.stage != Stage::Forced {
                        state.stage = Stage::Completed;
                    }
                    state.remaining = 0;
                    info!("Gallery 全部后台任务已真实退出并回收");
                }
            }
            changed = true;
        }
    }
    // Finished Jobs can precede Canvas completion delivery by one App update.
    // Let accepted stages publish or remove their files before ending the runner.
    if world.resource::<GalleryShutdownState>().stage == Stage::Completed
        && crate::canvas_experiment::stage_exports_pending(world)
    {
        world.write_message(RequestRedraw);
        return Ok(());
    }
    if matches!(
        world.resource::<GalleryShutdownState>().stage,
        Stage::Completed | Stage::Forced
    ) {
        let intent = world.resource::<ExitFlow>().intent.clone();
        world.resource_mut::<GalleryShutdownState>().exit_sent = true;
        world.write_message(intent);
        info!("Gallery 已提交最终应用退出通知");
    } else if changed {
        world.write_message(RequestRedraw);
    }
    Ok(())
}

fn decide(world: &mut World, force: bool) {
    let mut state = world.resource_mut::<GalleryShutdownState>();
    if !matches!(state.stage, Stage::TimedOut | Stage::Continuing) {
        return;
    }
    if force {
        state.stage = Stage::Forced;
        warn!(
            remaining = state.remaining,
            "用户选择强制退出 Gallery，后台清理及外部副作用不保证完成"
        );
    } else if state.stage == Stage::TimedOut {
        state.stage = Stage::Continuing;
        info!(
            remaining = state.remaining,
            "用户选择继续等待后台任务收尾，保留原始截止时间"
        );
    }
    world.write_message(RequestRedraw);
}

#[derive(Component, FromTemplate, Clone, Copy)]
struct Panel {
    status: Entity,
    details: Entity,
    actions: Entity,
}

fn panel() -> impl Scene {
    bsn! {
        Name("GalleryShutdown") Panel { status: #Status, details: #Details, actions: #Actions }
        GlobalZIndex(1000)
        Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: percent(100), height: percent(100),
            flex_direction: FlexDirection::Column, padding: UiRect::all(px(32)), row_gap: px(20) }
        Children [
            #Status Name("GalleryShutdownStatus") Text("") WidgetryText--
            #Details Name("GalleryShutdownDetails") Text("") WidgetryText--
            #Actions Node { display: Display::None, flex_direction: FlexDirection::Column, row_gap: px(20) }
            Children [
                @localized(TextKey::GalleryClose(CloseKey::ForceWarning)) WidgetryText--
                Node { column_gap: px(16) } Children [@decision(false, CloseKey::ContinueWaiting)-- @decision(true, CloseKey::ForceExit)]
            ]
        ]
    }
}

fn decision(force: bool, key: CloseKey) -> impl Scene {
    bsn! {
        Name({if force { "GalleryForceExit" } else { "GalleryContinueWaiting" }})
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::GalleryClose(key)) WidgetryText }) as Box<dyn SceneList>)} }
        bevy::input_focus::tab_navigation::TabIndex::default()
        on(move |_: On<Activate>, mut commands: Commands| { commands.queue(move |world: &mut World| decide(world, force)); })
    }
}

fn show(world: &mut World) -> Result {
    let state = world.resource::<GalleryShutdownState>();
    if matches!(state.stage, Stage::Idle | Stage::Completed | Stage::Forced) {
        return Ok(());
    }
    let (stage, remaining, timeout) = (state.stage, state.remaining, state.timeout_ms);
    let parts = world.query::<&Panel>().iter(world).next().copied();
    if parts.is_some()
        && !world.is_resource_changed::<GalleryShutdownState>()
        && !world.is_resource_changed::<WidgetryThemeMode>()
    {
        return Ok(());
    }
    let parts = if let Some(parts) = parts {
        parts
    } else {
        let Some(host) = world
            .query_filtered::<Entity, With<GalleryPageHost>>()
            .iter(world)
            .next()
        else {
            return Ok(());
        };
        let root = spawn_scene(world, bsn! { ChildOf(host) @panel() })?;
        *world.get::<Panel>(root).ok_or_else(|| {
            error!("Gallery 关闭界面缺少必要结构");
            BevyError::error("Gallery shutdown panel parts missing")
        })?
    };
    let status = match stage {
        Stage::TimedOut => CloseKey::TimedOut,
        Stage::Continuing => CloseKey::Continuing,
        _ => CloseKey::Waiting,
    };
    world
        .entity_mut(parts.status)
        .insert(LocalizedText::new(TextKey::GalleryClose(status)));
    world
        .entity_mut(parts.details)
        .insert(LocalizedText::with_args(
            TextKey::GalleryClose(CloseKey::ShutdownDetails),
            vec![
                ("remaining", remaining.to_string()),
                ("timeout", timeout.to_string()),
            ],
        ));
    world
        .get_mut::<Node>(parts.actions)
        .ok_or_else(|| {
            error!("Gallery 关闭界面缺少决策区域");
            BevyError::error("Gallery shutdown actions missing")
        })?
        .display = if matches!(stage, Stage::TimedOut | Stage::Continuing) {
        Display::Flex
    } else {
        Display::None
    };
    let color = world
        .resource::<WidgetryThemeMode>()
        .colors()
        .window
        .frame
        .normal
        .background;
    let root = world
        .query::<(Entity, &Panel)>()
        .iter(world)
        .next()
        .map(|(entity, _)| entity);
    if let Some(root) = root {
        world.entity_mut(root).insert(BackgroundColor(color));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::app::OnAppExitSystems;
    use std::sync::mpsc;
    use std::time::Instant;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, JobManagerPlugin))
            .init_resource::<GalleryTrayState>()
            .add_message::<RequestRedraw>();
        install(&mut app);
        app
    }

    fn controlled(app: &App, acknowledge: bool) -> (JobId, JobReceiver<()>, mpsc::Sender<()>) {
        let (release, wait) = mpsc::channel();
        let (started, running) = mpsc::channel();
        let result = app
            .world()
            .resource::<JobManager>()
            .spawn_thread(JobOptions::default(), move |context| {
                started.send(()).unwrap();
                wait.recv().unwrap();
                if acknowledge && context.is_cancelled() {
                    context.acknowledge_cancel();
                }
                Ok::<_, String>(())
            })
            .unwrap();
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        (result.0, result.1, release)
    }

    fn await_exit(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.should_exit().is_none() {
            assert!(Instant::now() < deadline);
            app.update();
            std::thread::yield_now();
        }
    }

    #[test]
    fn empty_programmatic_exit_is_completed_and_preserves_error_code() {
        let mut app = app();
        app.world_mut().write_message(AppExit::from_code(7));
        app.update();
        let state = app.world().resource::<GalleryShutdownState>();
        assert_eq!(state.stage, Stage::Completed);
        assert!(state.exit_sent);
        assert_eq!(app.should_exit(), Some(AppExit::from_code(7)));
        assert!(matches!(
            app.world()
                .resource::<JobManager>()
                .spawn_thread::<(), _, String>(JobOptions::default(), |_| Ok(())),
            Err(SubmitError::ShuttingDown)
        ));
    }

    #[test]
    fn exit_is_intercepted_before_exit_hooks_and_waits_for_real_thread_reaping() {
        #[derive(Resource, Default)]
        struct ExitHookCalls(usize);
        #[derive(Resource, Default)]
        struct TransportExitCalls(usize);
        let mut app = app();
        app.init_resource::<TransportExitCalls>().add_systems(
            Last,
            (|mut exits: MessageReader<AppExit>, mut calls: ResMut<TransportExitCalls>| {
                calls.0 += exits.read().count();
            })
            .before(drive),
        );
        app.init_resource::<ExitHookCalls>().add_systems(
            Last,
            (|mut exits: MessageReader<AppExit>, mut calls: ResMut<ExitHookCalls>| {
                calls.0 += exits.read().count();
            })
            .in_set(OnAppExitSystems),
        );
        let (id, mut receiver, release) = controlled(&app, true);
        app.world_mut().write_message(AppExit::Success);
        app.update();
        assert_eq!(app.should_exit(), None);
        assert_eq!(app.world().resource::<ExitHookCalls>().0, 0);
        assert_eq!(app.world().resource::<TransportExitCalls>().0, 0);
        assert_eq!(
            app.world()
                .resource::<JobManager>()
                .status(id)
                .unwrap()
                .state,
            JobState::Stopping
        );
        assert_eq!(app.world().resource::<GalleryShutdownState>().remaining, 1);
        release.send(()).unwrap();
        await_exit(&mut app);
        assert!(matches!(
            receiver.recv_blocking().unwrap(),
            JobMessage::Finished(JobOutcome::Canceled)
        ));
        assert_eq!(app.world().resource::<ExitHookCalls>().0, 1);
        assert_eq!(app.world().resource::<TransportExitCalls>().0, 1);
        assert_eq!(app.world().resource::<GalleryShutdownState>().remaining, 0);
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Completed
        );
    }

    #[test]
    fn remote_last_exit_cannot_bypass_shutdown_at_the_end_of_an_update() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, JobManagerPlugin))
            .init_resource::<GalleryTrayState>()
            .add_message::<RequestRedraw>()
            .init_schedule(bevy_remote::RemoteLast);
        app.world_mut()
            .resource_mut::<bevy::app::MainScheduleOrder>()
            .insert_after(Last, bevy_remote::RemoteLast);
        install(&mut app);
        app.add_systems(
            bevy_remote::RemoteLast,
            (|mut exits: MessageWriter<AppExit>| {
                exits.write(AppExit::Success);
            })
            .run_if(bevy::ecs::schedule::common_conditions::run_once),
        );
        let (_, _, release) = controlled(&app, true);
        app.update();
        assert!(app.should_exit().is_none());
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Waiting
        );
        release.send(()).unwrap();
        await_exit(&mut app);
    }

    #[test]
    fn remote_exit_leaves_hidden_windows_unchanged_until_the_next_frame() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, JobManagerPlugin))
            .init_resource::<GalleryTrayState>()
            .add_message::<RequestRedraw>()
            .init_schedule(bevy_remote::RemoteLast);
        app.world_mut()
            .resource_mut::<bevy::app::MainScheduleOrder>()
            .insert_after(Last, bevy_remote::RemoteLast);
        install(&mut app);
        app.add_systems(
            bevy_remote::RemoteLast,
            (|mut exits: MessageWriter<AppExit>| {
                exits.write(AppExit::Success);
            })
            .run_if(bevy::ecs::schedule::common_conditions::run_once),
        );
        let primary = app
            .world_mut()
            .spawn((
                Window {
                    visible: false,
                    ..default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();
        app.world_mut()
            .resource_mut::<GalleryTrayState>()
            .hidden_windows
            .push(primary);
        let (_, _, release) = controlled(&app, true);
        app.update();
        assert!(!app.world().get::<Window>(primary).unwrap().visible);
        app.update();
        assert!(app.world().get::<Window>(primary).unwrap().visible);
        assert!(app.should_exit().is_none());
        release.send(()).unwrap();
        await_exit(&mut app);
    }

    #[test]
    fn timeout_is_once_and_continue_preserves_the_handle_until_natural_completion() {
        let mut app = app();
        app.world_mut().resource_mut::<ExitFlow>().timeout = Duration::ZERO;
        let (id, mut receiver, release) = controlled(&app, false);
        begin(app.world_mut(), AppExit::Success).unwrap();
        app.update();
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::TimedOut
        );
        assert_eq!(
            app.world()
                .resource::<GalleryShutdownState>()
                .timeout_notifications,
            1
        );
        decide(app.world_mut(), false);
        for _ in 0..3 {
            app.world_mut().write_message(AppExit::Success);
            begin(app.world_mut(), AppExit::Success).unwrap();
            app.update();
            assert_eq!(app.should_exit(), None);
        }
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Continuing
        );
        assert_eq!(
            app.world()
                .resource::<GalleryShutdownState>()
                .timeout_notifications,
            1
        );
        assert_eq!(
            app.world()
                .resource::<JobManager>()
                .status(id)
                .unwrap()
                .state,
            JobState::Stopping
        );
        release.send(()).unwrap();
        await_exit(&mut app);
        assert!(matches!(
            receiver.recv_blocking().unwrap(),
            JobMessage::Finished(JobOutcome::Completed)
        ));
        assert_eq!(
            app.world()
                .resource::<GalleryShutdownState>()
                .timeout_notifications,
            1
        );
    }

    #[test]
    fn force_requires_timeout_and_sends_exit_only_once_with_a_live_job() {
        let mut app = app();
        app.world_mut().resource_mut::<ExitFlow>().timeout = Duration::ZERO;
        let (_, _receiver, release) = controlled(&app, false);
        decide(app.world_mut(), true);
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Idle
        );
        begin(app.world_mut(), AppExit::Success).unwrap();
        app.update();
        decide(app.world_mut(), true);
        app.update();
        assert_eq!(app.should_exit(), Some(AppExit::Success));
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Forced
        );
        assert_eq!(app.world().resource::<GalleryShutdownState>().remaining, 1);
        let messages = app.world().resource::<Messages<AppExit>>().len();
        decide(app.world_mut(), true);
        app.update();
        assert_eq!(app.world().resource::<Messages<AppExit>>().len(), messages);
        release.send(()).unwrap();
    }

    fn window_app() -> App {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins((
            JobManagerPlugin,
            bevy_widgetry::window::WidgetryWindowPlugin,
            bevy_widgetry::button::WidgetryButtonPlugin,
            bevy_widgetry::radio_group::WidgetryRadioGroupPlugin,
        ))
        .init_resource::<GalleryTrayState>()
        .add_message::<RequestRedraw>()
        .add_systems(Update, super::super::close_requests);
        crate::i18n::install(&mut app);
        install(&mut app);
        app
    }

    fn owned(app: &mut App) -> (Entity, Entity) {
        let root = spawn_scene(app.world_mut(), bsn! {
            @bevy_widgetry::window::owned_widgetry_window(Window::default(), default(), bevy_widgetry::window::WidgetryWindowBackground::Theme,
                default(), bsn_list!{}, bsn_list!{})
        }).unwrap();
        let native = bevy_widgetry::window::widgetry_window_target(app.world(), root).unwrap();
        (root, native)
    }

    #[test]
    fn repeated_primary_close_keeps_one_confirmation_and_cancel_and_hide_accept_jobs() {
        let mut app = window_app();
        let (_, native) = owned(&mut app);
        app.world_mut()
            .entity_mut(native)
            .insert(bevy::window::PrimaryWindow);
        for _ in 0..2 {
            app.world_mut()
                .write_message(bevy::window::WindowCloseRequested { window: native });
        }
        app.update();
        let dialog = app.world().resource::<GalleryTrayState>().dialog.unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&super::super::CloseDialog>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Idle
        );
        super::super::finish(app.world_mut(), dialog, None).unwrap();
        assert!(app.world().get::<Window>(native).unwrap().visible);
        super::super::open_confirmation(app.world_mut(), native).unwrap();
        let dialog = app.world().resource::<GalleryTrayState>().dialog.unwrap();
        super::super::finish(app.world_mut(), dialog, Some(super::super::Choice::Hide)).unwrap();
        assert!(!app.world().get::<Window>(native).unwrap().visible);
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Idle
        );
        let (_, _, release) = controlled(&app, true);
        super::super::open_confirmation(app.world_mut(), native).unwrap();
        let dialog = app.world().resource::<GalleryTrayState>().dialog.unwrap();
        super::super::finish(app.world_mut(), dialog, Some(super::super::Choice::Exit)).unwrap();
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Waiting
        );
        app.update();
        assert!(app.world().get::<Window>(native).unwrap().visible);
        assert!(app.should_exit().is_none());
        release.send(()).unwrap();
        await_exit(&mut app);
    }

    #[test]
    fn programmatic_exit_restores_hidden_modal_windows_before_disabling_tray_restore() {
        use bevy_widgetry::message_box::*;
        let mut app = window_app();
        app.add_plugins(WidgetryMessageBoxPlugin);
        let (_, primary) = owned(&mut app);
        app.world_mut()
            .entity_mut(primary)
            .insert(bevy::window::PrimaryWindow);
        let message = spawn_scene(app.world_mut(), bsn! {
            @widgetry_message_box(primary, "Hidden modal", WidgetryMessageBoxButtons::YesNoCancel, default(), bsn_list!{})
        }).unwrap();
        let modal = bevy_widgetry::window::widgetry_window_target(app.world(), message).unwrap();
        super::super::open_confirmation(app.world_mut(), primary).unwrap();
        let confirmation = app.world().resource::<GalleryTrayState>().dialog.unwrap();
        super::super::finish(
            app.world_mut(),
            confirmation,
            Some(super::super::Choice::Hide),
        )
        .unwrap();
        for window in [primary, modal] {
            assert!(!app.world().get::<Window>(window).unwrap().visible);
        }
        let (_, _, release) = controlled(&app, true);
        app.world_mut().resource_mut::<ExitFlow>().timeout = Duration::ZERO;
        app.world_mut().write_message(AppExit::Success);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::TimedOut
        );
        for window in [primary, modal] {
            assert!(app.world().get::<Window>(window).unwrap().visible);
        }
        assert!(
            app.world()
                .resource::<GalleryTrayState>()
                .hidden_windows
                .is_empty()
        );
        assert!(app.world().resource::<GalleryTrayState>().exiting);
        assert!(
            WidgetryMessageBox::resolve(app.world_mut(), message, WidgetryMessageBoxResult::Cancel)
                .unwrap()
        );
        assert!(app.world().get_entity(message).is_err());
        decide(app.world_mut(), false);
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Continuing
        );
        release.send(()).unwrap();
        await_exit(&mut app);
    }

    #[test]
    fn independent_windows_close_without_exiting_and_still_close_during_shutdown() {
        let mut app = window_app();
        let (_, primary) = owned(&mut app);
        app.world_mut()
            .entity_mut(primary)
            .insert(bevy::window::PrimaryWindow);
        let (a, a_native) = owned(&mut app);
        let (b, b_native) = owned(&mut app);
        for root in [a, b] {
            app.world_mut()
                .entity_mut(root)
                .insert(super::super::GalleryIndependentWindow);
        }
        app.world_mut()
            .write_message(bevy::window::WindowCloseRequested { window: a_native });
        app.update();
        assert!(app.world().get_entity(a).is_err());
        assert!(app.world().get_entity(b).is_ok());
        assert_eq!(
            app.world().resource::<GalleryShutdownState>().stage,
            Stage::Idle
        );
        let (_, _, release) = controlled(&app, true);
        begin(app.world_mut(), AppExit::Success).unwrap();
        for window in [primary, b_native, primary] {
            app.world_mut()
                .write_message(bevy::window::WindowCloseRequested { window });
        }
        app.update();
        assert!(app.world().get_entity(b).is_err());
        assert!(app.world().get::<Window>(primary).is_some());
        assert!(app.world().resource::<GalleryTrayState>().dialog.is_none());
        assert!(app.should_exit().is_none());
        release.send(()).unwrap();
        await_exit(&mut app);
    }

    #[test]
    fn file_dialog_and_message_box_keep_cancel_and_deferred_cleanup_during_shutdown() {
        use bevy_widgetry::file_dialog::*;
        use bevy_widgetry::message_box::*;
        use bevy_widgetry::window::widgetry_window_target;
        #[derive(Resource, Default)]
        struct Results {
            files: Vec<WidgetryFileDialogResult>,
            boxes: Vec<WidgetryMessageBoxResult>,
        }
        let mut app = window_app();
        bevy_widgetry_test_utils::add_ui_plugins(&mut app);
        app.insert_resource(WidgetryFileDialogRuntimeOptions {
            automatic: false,
            ..default()
        })
        .add_plugins((
            WidgetryFileDialogPlugin,
            bevy::input_focus::InputDispatchPlugin,
            bevy::ui_widgets::TextInputPlugin,
        ))
        .init_resource::<Results>()
        .add_observer(
            |event: On<WidgetryFileDialogResultEvent>, mut results: ResMut<Results>| {
                results.files.push(event.result.clone());
            },
        )
        .add_observer(
            |event: On<WidgetryMessageBoxResultEvent>, mut results: ResMut<Results>| {
                results.boxes.push(event.result);
            },
        );
        let (_, primary) = owned(&mut app);
        app.world_mut()
            .entity_mut(primary)
            .insert(bevy::window::PrimaryWindow);
        let file = spawn_scene(
            app.world_mut(),
            bsn! { @WidgetryFileDialog { @window: {Some(WidgetryFileDialogWindow::default())} } },
        )
        .unwrap();
        let message = spawn_scene(app.world_mut(), bsn! {
            @widgetry_message_box(primary, "Close", WidgetryMessageBoxButtons::YesNoCancel, default(), bsn_list!{})
        }).unwrap();
        for _ in 0..3 {
            app.update();
        }
        let file_native = widgetry_window_target(app.world(), file).unwrap();
        let file_camera = app.world().get::<UiTargetCamera>(file).unwrap().0;
        let box_native = widgetry_window_target(app.world(), message).unwrap();
        let (_, _, release) = controlled(&app, true);
        begin(app.world_mut(), AppExit::Success).unwrap();
        for window in [file_native, box_native] {
            app.world_mut()
                .write_message(bevy::window::WindowCloseRequested { window });
        }
        app.update();
        assert_eq!(
            app.world().resource::<Results>().files,
            [WidgetryFileDialogResult::Cancelled]
        );
        assert_eq!(
            app.world().resource::<Results>().boxes,
            [WidgetryMessageBoxResult::Cancel]
        );
        assert!(app.world().get_entity(message).is_err());
        for entity in [file, file_native, file_camera] {
            assert!(app.world().get_entity(entity).is_ok());
        }
        let messages = app
            .world()
            .resource::<Messages<bevy::window::WindowClosing>>();
        assert!(
            messages
                .get_cursor()
                .read(messages)
                .any(|event| event.window == file_native)
        );
        app.update();
        for entity in [file, file_native, file_camera] {
            assert!(app.world().get_entity(entity).is_err());
        }
        assert!(app.world().get::<Window>(primary).is_some());
        assert!(app.should_exit().is_none());
        release.send(()).unwrap();
        await_exit(&mut app);
    }
}
