use bevy::app::PluginsState;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::picking::hover::Hovered;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::{ListBox, ListItem};
use bevy::{
    app::TerminalCtrlCHandlerPlugin,
    asset::RenderAssetUsages,
    camera::RenderTarget,
    log::LogPlugin,
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
use bevy_widgetry::button::{StyledButton, StyledButtonPlugin};
use bevy_widgetry::combo_box::{StyledComboBoxPlugin, spawn_styled_combo_box};
use std::path::PathBuf;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const BYTES_PER_PIXEL: usize = 4;

#[test]
fn styled_button_can_render_offscreen() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<PipelinedRenderingPlugin>()
            .disable::<LogPlugin>()
            .disable::<TerminalCtrlCHandlerPlugin>(),
    );

    app.add_plugins(StyledButtonPlugin);

    // 等 renderer 等插件初始化完成
    while app.plugins_state() == PluginsState::Adding {
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }

    app.finish();
    app.cleanup();

    let size = Extent3d {
        width: WIDTH,
        height: HEIGHT,
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

    app.world_mut()
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(8),
                ..default()
            },
            UiTargetCamera(camera),
        ))
        .with_children(|parent| {
            // Default
            parent.spawn((
                StyledButton,
                Node {
                    width: px(120),
                    height: px(40),
                    ..default()
                },
            ));

            // Hover
            parent.spawn((
                StyledButton,
                Hovered(true),
                Node {
                    width: px(120),
                    height: px(40),
                    ..default()
                },
            ));

            // Pressed
            parent.spawn((
                StyledButton,
                Hovered(true),
                Pressed,
                Node {
                    width: px(120),
                    height: px(40),
                    ..default()
                },
            ));

            // Disabled
            parent.spawn((
                StyledButton,
                InteractionDisabled,
                Node {
                    width: px(120),
                    height: px(40),
                    ..default()
                },
            ));
        });

    // 场景已经准备好，但先不要加 Readback
    // 至少先跑一帧，让 UI 所需 pipeline 被发现并加入 PipelineCache
    app.update();

    loop {
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

    assert_eq!(
        pixels.len(),
        WIDTH as usize * HEIGHT as usize * BYTES_PER_PIXEL
    );

    let actual_image = Image::new(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    );

    let actual = actual_image.try_into_dynamic().unwrap().to_rgba8();

    let baseline_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("baselines")
        .join("styled_button.png");

    let baseline_bytes =
        std::fs::read(&baseline_path).expect("failed to read styled button baseline");

    let baseline_image = Image::from_buffer(
        &baseline_bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::default(),
        RenderAssetUsages::MAIN_WORLD,
    )
    .expect("failed to decode styled button baseline");

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

const COMBO_BOX_WIDTH: u32 = 704;
const COMBO_BOX_HEIGHT: u32 = 180;

#[derive(Resource)]
struct ComboBoxVisualCamera(Entity);

#[derive(Resource)]
struct ComboBoxVisualEntities {
    open: Entity,
    disabled: Entity,
}

fn spawn_combo_box_visual_scene(mut commands: Commands, camera: Res<ComboBoxVisualCamera>) {
    let container = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                column_gap: px(20),
                padding: UiRect {
                    top: px(20),
                    ..default()
                },
                ..default()
            },
            UiTargetCamera(camera.0),
        ))
        .id();

    let options = || {
        vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Orange".to_string(),
        ]
    };

    let default_combo = spawn_styled_combo_box(&mut commands, options());
    let open_combo = spawn_styled_combo_box(&mut commands, options());
    let disabled_combo = spawn_styled_combo_box(&mut commands, options());

    commands.entity(default_combo).insert(ChildOf(container));
    commands.entity(open_combo).insert(ChildOf(container));
    commands.entity(disabled_combo).insert(ChildOf(container));

    commands.insert_resource(ComboBoxVisualEntities {
        open: open_combo,
        disabled: disabled_combo,
    });
}

#[test]
fn styled_combo_box_can_render_offscreen() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<PipelinedRenderingPlugin>()
            .disable::<LogPlugin>()
            .disable::<TerminalCtrlCHandlerPlugin>(),
    );

    app.add_plugins(StyledComboBoxPlugin);
    app.add_systems(Startup, spawn_combo_box_visual_scene);

    while app.plugins_state() == PluginsState::Adding {
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }

    app.finish();
    app.cleanup();

    let size = Extent3d {
        width: COMBO_BOX_WIDTH,
        height: COMBO_BOX_HEIGHT,
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

    app.world_mut()
        .insert_resource(ComboBoxVisualCamera(camera));

    // Startup 创建三个 ComboBox，Style setup 也跑一遍
    app.update();

    let (open_combo, disabled_combo) = {
        let entities = app.world().resource::<ComboBoxVisualEntities>();
        (entities.open, entities.disabled)
    };

    let (popup, hovered_option) = {
        let world = app.world();

        let popup = world
            .get::<Children>(open_combo)
            .unwrap()
            .iter()
            .find(|&child| world.get::<ListBox>(child).is_some())
            .unwrap();

        let hovered_option = world
            .get::<Children>(popup)
            .unwrap()
            .iter()
            .filter(|&child| world.get::<ListItem>(child).is_some())
            .nth(1)
            .unwrap();

        (popup, hovered_option)
    };

    // 中间：Open + Banana Hovered
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

    app.world_mut()
        .entity_mut(hovered_option)
        .insert(Hovered(true));

    // 右边：Disabled
    app.world_mut()
        .entity_mut(disabled_combo)
        .insert(InteractionDisabled);

    // 场景已经准备好，但先不要加 Readback
    // 至少先跑一帧，让 UI 所需 pipeline 被发现并加入 PipelineCache
    app.update();

    loop {
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

    assert_eq!(
        pixels.len(),
        COMBO_BOX_WIDTH as usize * COMBO_BOX_HEIGHT as usize * BYTES_PER_PIXEL
    );

    let actual_image = Image::new(
        Extent3d {
            width: COMBO_BOX_WIDTH,
            height: COMBO_BOX_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    );

    let actual = actual_image.try_into_dynamic().unwrap().to_rgba8();

    let baseline_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("baselines")
        .join("styled_combo_box.png");

    let baseline_bytes =
        std::fs::read(&baseline_path).expect("failed to read styled combo_box baseline");

    let baseline_image = Image::from_buffer(
        &baseline_bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::default(),
        RenderAssetUsages::MAIN_WORLD,
    )
    .expect("failed to decode styled combo_box baseline");

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
