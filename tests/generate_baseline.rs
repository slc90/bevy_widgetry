use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use bevy::{
    app::PluginsState,
    asset::RenderAssetUsages,
    camera::RenderTarget,
    picking::hover::Hovered,
    prelude::*,
    render::{
        RenderPlugin,
        pipelined_rendering::PipelinedRenderingPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, save_to_disk},
    },
    ui::{InteractionDisabled, Pressed},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};

use bevy_widgetry::styled_button::{StyledButton, StyledButtonPlugin};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

#[test]
#[ignore]
fn generate_styled_button_baseline() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<PipelinedRenderingPlugin>(),
    );

    app.add_plugins(StyledButtonPlugin);

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

    // 先跑一帧，让 UI layout / extraction / render 准备完成。
    app.update();

    let baseline_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("baselines")
        .join("styled_button.png");

    std::fs::create_dir_all(baseline_path.parent().unwrap()).unwrap();

    let screenshot_entity = app
        .world_mut()
        .spawn(Screenshot::image(image_handle))
        .observe(save_to_disk(baseline_path.clone()))
        .id();

    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        app.update();

        // Screenshot 在完成 capture 并触发 observer 后会自动 despawn。
        if app.world().get_entity(screenshot_entity).is_err() {
            break;
        }

        assert!(
            Instant::now() < deadline,
            "timed out generating styled button baseline"
        );

        std::thread::yield_now();
    }

    assert!(baseline_path.exists(), "baseline PNG was not created");

    println!("baseline written to {}", baseline_path.display());
}
