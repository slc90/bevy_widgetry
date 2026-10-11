use bevy::camera::NormalizedRenderTarget;
use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
    camera::ExtractedCamera,
    render_phase::ViewSortedRenderPhases,
    render_resource::PipelineCache,
    sync_world::MainEntity,
    view::{ExtractedView, ViewTarget, window::ExtractedWindow},
};
use bevy::ui_render::{ImageNodeBindGroups, TransparentUi, UiBatch, UiCameraView};
use bevy::window::RequestRedraw;
use bevy_widgetry::cursor::WidgetryCursorTheme;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Resource, Clone)]
pub(crate) struct StartupCursor {
    window: Entity,
    camera: Entity,
    background: Entity,
    presented: Arc<AtomicBool>,
}

impl StartupCursor {
    pub(crate) fn new(window: Entity, camera: Entity, background: Entity) -> Self {
        Self {
            window,
            camera,
            background,
            presented: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Resource, Default)]
struct PendingCursor {
    startup: Option<StartupCursor>,
    acquired: bool,
}

pub(crate) fn install(app: &mut App) -> Result {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        error!("Gallery Cursor 启动检查需要 RenderApp");
        return Err(BevyError::error("Gallery Cursor 启动检查需要 RenderApp"));
    };
    render_app
        .init_resource::<PendingCursor>()
        .add_systems(ExtractSchedule, extract_startup)
        .add_systems(
            Render,
            observe_acquired
                .after(RenderSystems::Prepare)
                .before(RenderSystems::Render),
        )
        .add_systems(
            Render,
            observe_presented
                .after(RenderSystems::Render)
                .before(RenderSystems::Cleanup),
        );
    app.add_systems(
        Update,
        enable_cursor.run_if(resource_exists::<StartupCursor>),
    );
    Ok(())
}

fn enable_cursor(
    mut commands: Commands,
    startup: Res<StartupCursor>,
    mut theme: ResMut<WidgetryCursorTheme>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    if startup.presented.load(Ordering::Acquire) {
        *theme = WidgetryCursorTheme::Kamuri;
        commands.remove_resource::<StartupCursor>();
    } else {
        redraw.write(RequestRedraw);
    }
}

fn extract_startup(
    startup: Extract<Option<Res<StartupCursor>>>,
    mut pending: ResMut<PendingCursor>,
) {
    pending.startup = startup.as_ref().map(|startup| (*startup).clone());
    pending.acquired = false;
}

fn observe_acquired(
    windows: Query<(MainEntity, &ExtractedWindow)>,
    mut pending: ResMut<PendingCursor>,
) {
    pending.acquired = pending.startup.as_ref().is_some_and(|startup| {
        windows
            .iter()
            .any(|(main, window)| main == startup.window && window.swap_chain_texture.is_some())
    });
}

fn observe_presented(
    pending: Res<PendingCursor>,
    windows: Query<(MainEntity, &ExtractedWindow)>,
    cameras: Query<(&MainEntity, &ViewTarget, &UiCameraView, &ExtractedCamera)>,
    ui_views: Query<&ExtractedView>,
    phases: Res<ViewSortedRenderPhases<TransparentUi>>,
    pipelines: Res<PipelineCache>,
    batches: Query<&UiBatch>,
    images: Res<ImageNodeBindGroups>,
) -> Result {
    let Some(startup) = pending.startup.as_ref().filter(|_| pending.acquired) else {
        return Ok(());
    };
    if startup.presented.load(Ordering::Acquire) {
        return Ok(());
    }
    // 透明窗口的 visible/focused 不能证明已有画面，需确认本帧 back buffer 已 present 且背景 draw 可用。
    if !windows
        .iter()
        .any(|(main, window)| main == startup.window && window.swap_chain_texture.is_none())
    {
        return Ok(());
    }
    let Some((_, target, ui_camera, camera)) = cameras
        .iter()
        .find(|(main, _, _, _)| main.id() == startup.camera)
    else {
        return Ok(());
    };
    if !target.needs_present()
        || !matches!(camera.target, Some(NormalizedRenderTarget::Window(window)) if window.entity() == startup.window)
    {
        return Ok(());
    }
    let Ok(view) = ui_views.get(ui_camera.0) else {
        return Ok(());
    };
    let Some(phase) = phases.get(&view.retained_view_entity) else {
        return Ok(());
    };
    if phase.items.values().any(|item| {
        item.entity.1.id() == startup.background
            && !item.batch_range.is_empty()
            && pipelines.get_render_pipeline(item.pipeline).is_some()
            && batches.get(item.entity.0).is_ok_and(|batch| {
                !batch.range.is_empty() && images.values.contains_key(&batch.image)
            })
    }) {
        // present 仅安排呈现；DwmFlush 等本进程已排队的 DirectX 画面显示后再放行 Cursor。
        winsafe::DwmFlush().map_err(|error| {
            error!(error = %error, "等待 Gallery 首帧桌面呈现失败");
            BevyError::error(error)
        })?;
        startup.presented.store(true, Ordering::Release);
    }
    Ok(())
}
