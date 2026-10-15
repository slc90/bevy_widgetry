use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
    diagnostic::{
        DiagnosticsRecorder, RecordDiagnostics, RenderDiagnosticsPlugin, resolve_encoder,
    },
    renderer::{PendingCommandBuffers, RenderDevice, RenderGraph, RenderGraphSystems},
    sync_world::MainEntity,
    view::window::ExtractedWindow,
};
use bevy::window::PrimaryWindow;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;

#[derive(Resource, Clone, Default)]
pub(super) struct BenchmarkPresent {
    pub(super) count: Arc<AtomicU64>,
    pub(super) interval_ns: Arc<AtomicU64>,
}

#[derive(Resource, Default)]
struct PendingPresent {
    window: Option<Entity>,
    acquired: bool,
    previous: Option<Instant>,
}

pub(super) fn install(app: &mut App) -> Result {
    let channel = BenchmarkPresent::default();
    app.insert_resource(channel.clone())
        .add_plugins(RenderDiagnosticsPlugin);
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        error!("Splash benchmark 需要 RenderApp");
        return Err(BevyError::error("Splash benchmark 需要 RenderApp"));
    };
    render_app
        .insert_resource(channel)
        .init_resource::<PendingPresent>()
        .add_systems(ExtractSchedule, extract_window)
        .add_systems(
            Render,
            acquired
                .after(RenderSystems::Prepare)
                .before(RenderSystems::Render),
        )
        .add_systems(
            Render,
            presented
                .after(RenderSystems::Render)
                .before(RenderSystems::Cleanup),
        )
        .add_systems(
            RenderGraph,
            gpu_begin
                .after(RenderGraphSystems::Begin)
                .before(RenderGraphSystems::Render),
        )
        .add_systems(
            RenderGraph,
            gpu_end
                .after(RenderGraphSystems::Render)
                .before(resolve_encoder)
                .before(RenderGraphSystems::Submit),
        );
    Ok(())
}

fn extract_window(
    windows: Extract<Query<Entity, With<PrimaryWindow>>>,
    mut pending: ResMut<PendingPresent>,
) {
    pending.window = windows.iter().next();
    pending.acquired = false;
}

fn acquired(windows: Query<(MainEntity, &ExtractedWindow)>, mut pending: ResMut<PendingPresent>) {
    pending.acquired = windows
        .iter()
        .any(|(main, window)| Some(main) == pending.window && window.swap_chain_texture.is_some());
}

fn presented(
    windows: Query<(MainEntity, &ExtractedWindow)>,
    mut pending: ResMut<PendingPresent>,
    channel: Res<BenchmarkPresent>,
) {
    if pending.acquired
        && windows.iter().any(|(main, window)| {
            Some(main) == pending.window && window.swap_chain_texture.is_none()
        })
    {
        let now = Instant::now();
        if let Some(previous) = pending.previous.replace(now) {
            channel.interval_ns.store(
                now.duration_since(previous).as_nanos() as u64,
                Ordering::Relaxed,
            );
        }
        channel.count.fetch_add(1, Ordering::Release);
    }
}

fn gpu_begin(world: &mut World) {
    // Identically named UI and upscaling diagnostics overwrite samples from different Cameras, so the last sample cannot represent total cost.
    // Queue two timestamps in submission order around the full Camera driver to include every offscreen and main-window pass.
    // Recorder pairs spans per thread and panics if a span ends on another worker.
    // Both exclusive systems run on the same scheduling thread as the Camera driver.
    let mut encoder = world
        .resource::<RenderDevice>()
        .create_command_encoder(&default());
    world
        .resource::<DiagnosticsRecorder>()
        .begin_time_span(&mut encoder, "splash_total".into());
    world
        .resource_mut::<PendingCommandBuffers>()
        .push_encoder(encoder, "splash_benchmark_begin");
}

fn gpu_end(world: &mut World) {
    let mut encoder = world
        .resource::<RenderDevice>()
        .create_command_encoder(&default());
    world
        .resource::<DiagnosticsRecorder>()
        .end_time_span(&mut encoder);
    world
        .resource_mut::<PendingCommandBuffers>()
        .push_encoder(encoder, "splash_benchmark_end");
}
