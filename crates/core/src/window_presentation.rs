use bevy::{
    camera::{CameraOutputMode, NormalizedRenderTarget},
    core_pipeline::{Core2d, upscaling::upscaling},
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        camera::SortedCameras,
        renderer::FlushCommands,
        sync_world::MainEntity,
        view::window::{ExtractedWindow, prepare_windows},
    },
};

#[derive(Resource)]
struct Installed;

pub fn install(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    if render_app.world().contains_resource::<Installed>() {
        return;
    }
    render_app
        .insert_resource(Installed)
        .add_systems(
            Core2d,
            // Core2d does not insert ApplyDeferred automatically.
            // DX12 allows only one swap chain in each submission.
            (ApplyDeferred, submit_window_commands)
                .chain()
                .after(upscaling),
        )
        .add_systems(
            Render,
            defer_initial_present_without_camera
                .in_set(RenderSystems::PrepareViews)
                .before(prepare_windows),
        );
}

fn submit_window_commands(mut commands: FlushCommands) {
    commands.flush();
}

// Without this guard, no_camera_clear_pass can write multiple swap chains in one command list.
fn defer_initial_present_without_camera(
    mut windows: Query<(MainEntity, &mut ExtractedWindow)>,
    cameras: Res<SortedCameras>,
) {
    for (main, mut window) in &mut windows {
        let has_camera = cameras.0.iter().any(|camera| {
            matches!(camera.target, Some(NormalizedRenderTarget::Window(target)) if target.entity() == main)
                && matches!(camera.output_mode, CameraOutputMode::Write { .. })
        });
        if !has_camera {
            window.needs_initial_present = false;
        }
    }
}
