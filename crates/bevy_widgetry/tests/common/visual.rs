//! Each visual_*.rs integration-test target creates only one complete Bevy renderer App.
//! DefaultPlugins may initialize process-global state. For another independent visual
//! scene, add another visual_*.rs target instead of a second App in the same test binary.

use bevy::app::PluginsState;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::{
    app::TerminalCtrlCHandlerPlugin,
    asset::RenderAssetUsages,
    camera::RenderTarget,
    prelude::*,
    render::{
        RenderApp,
        render_resource::{
            Extent3d, PipelineCache, TextureDimension, TextureFormat, TextureUsages,
        },
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use std::path::PathBuf;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub fn setup_offscreen_app() -> App {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            // Offscreen tests have no OS window/event loop.
            .disable::<WinitPlugin>()
            // Drive rendering synchronously with App::update for readback.
            .disable::<PipelinedRenderingPlugin>()
            // The offscreen harness does not need a process-level Ctrl-C handler.
            .disable::<TerminalCtrlCHandlerPlugin>(),
    );

    app
}

pub fn create_render_target(app: &mut App, width: u32, height: u32) -> (Handle<Image>, Entity) {
    // 等 renderer 等插件初始化完成
    while app.plugins_state() == PluginsState::Adding {
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }

    app.finish();
    app.cleanup();

    let size = Extent3d {
        width,
        height,
        ..default()
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );

    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::COPY_SRC
        | TextureUsages::RENDER_ATTACHMENT;

    let image_handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);

    let camera = app
        .world_mut()
        .spawn((Camera2d, RenderTarget::Image(image_handle.clone().into())))
        .id();

    (image_handle, camera)
}

pub fn readback_image(
    app: &mut App,
    image_handle: Handle<Image>,
    width: u32,
    height: u32,
) -> Image {
    // 场景已经准备好，但先不要加 Readback
    // 至少先跑一帧，让 UI 所需 pipeline 被发现并加入 PipelineCache
    app.update();

    let pipeline_deadline = Instant::now() + Duration::from_secs(30);
    loop {
        assert!(
            Instant::now() < pipeline_deadline,
            "timed out waiting for render pipelines"
        );
        let pipelines_ready = app
            .get_sub_app(RenderApp)
            .unwrap()
            .world()
            .resource::<PipelineCache>()
            .waiting_pipelines()
            .next()
            .is_none();

        if pipelines_ready {
            break;
        }

        app.update();
        std::thread::yield_now();
    }

    let pixels = Arc::new(Mutex::new(None));
    let pixels_from_observer = Arc::clone(&pixels);

    let readback_entity = app
        .world_mut()
        .spawn(Readback::texture(image_handle.clone()))
        .observe(move |event: On<ReadbackComplete>, mut commands: Commands| {
            let mut pixels = pixels_from_observer.lock().unwrap();

            if pixels.is_none() {
                *pixels = Some(event.data.clone());
                commands.entity(event.entity).despawn();
            }
        })
        .id();

    // 让这一份 readback 被提交
    app.update();

    app.world_mut()
        .entity_mut(readback_entity)
        .remove::<Readback>();

    // 然后继续 update，纯粹等待已经提交的这一份完成
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        app.update();

        if pixels.lock().unwrap().is_some() {
            break;
        }

        assert!(
            Instant::now() < deadline,
            "timed out waiting for GPU readback"
        );

        std::thread::yield_now();
    }

    let pixels = pixels.lock().unwrap().take().unwrap();

    assert!(!pixels.is_empty());

    assert_eq!(pixels.len(), width as usize * height as usize * 4);

    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    )
}

#[allow(dead_code)] // Shared with the manual generator.
pub fn assert_matches_baseline(actual_image: Image, name: &str) {
    let actual = actual_image.try_into_dynamic().unwrap().to_rgba8();

    let baseline_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("baselines")
        .join(name);

    let baseline_bytes = std::fs::read(&baseline_path).expect("failed to read visual baseline");

    let baseline_image = Image::from_buffer(
        &baseline_bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::default(),
        RenderAssetUsages::MAIN_WORLD,
    )
    .expect("failed to decode visual baseline");

    let baseline = baseline_image.try_into_dynamic().unwrap().to_rgba8();

    assert_eq!(
        actual.dimensions(),
        baseline.dimensions(),
        "rendered image dimensions differ from baseline"
    );

    assert_eq!(
        actual.as_raw(),
        baseline.as_raw(),
        "rendered image differs from baseline"
    );
}
