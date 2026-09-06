use bevy::ui_widgets::{ListBox, ListItem};
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
    ui::InteractionDisabled,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use bevy_widgetry::combo_box::{StyledComboBoxPlugin, spawn_styled_combo_box};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Resource)]
struct ComboBoxBaselineCamera(Entity);

#[derive(Resource)]
struct ComboBoxBaselineEntities {
    open: Entity,
    disabled: Entity,
}

const COMBO_BOX_BASELINE_WIDTH: u32 = 704;
const COMBO_BOX_BASELINE_HEIGHT: u32 = 180;

fn spawn_combo_box_baseline_scene(mut commands: Commands, camera: Res<ComboBoxBaselineCamera>) {
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

    let default_combo = spawn_styled_combo_box(
        &mut commands,
        vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Orange".to_string(),
        ],
    );

    let open_combo = spawn_styled_combo_box(
        &mut commands,
        vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Orange".to_string(),
        ],
    );

    let disabled_combo = spawn_styled_combo_box(
        &mut commands,
        vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Orange".to_string(),
        ],
    );

    commands.entity(default_combo).insert(ChildOf(container));
    commands.entity(open_combo).insert(ChildOf(container));
    commands.entity(disabled_combo).insert(ChildOf(container));

    commands.insert_resource(ComboBoxBaselineEntities {
        open: open_combo,
        disabled: disabled_combo,
    });
}

#[test]
#[ignore]
fn generate_styled_combo_box_baseline() {
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

    app.add_plugins(StyledComboBoxPlugin);
    app.add_systems(Startup, spawn_combo_box_baseline_scene);

    while app.plugins_state() == PluginsState::Adding {
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }

    app.finish();
    app.cleanup();

    let size = Extent3d {
        width: COMBO_BOX_BASELINE_WIDTH,
        height: COMBO_BOX_BASELINE_HEIGHT,
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
        .insert_resource(ComboBoxBaselineCamera(camera));

    // Startup + StyledComboBox setup
    app.update();

    let (open_combo, disabled_combo) = {
        let entities = app.world().resource::<ComboBoxBaselineEntities>();
        (entities.open, entities.disabled)
    };

    let (popup, hovered_option) = {
        let world = app.world();

        let popup = world
            .get::<Children>(open_combo)
            .unwrap()
            .iter()
            .find(|&child| world.get::<ListBox>(child).is_some())
            .expect("open ComboBox should contain a ListBox");

        let hovered_option = world
            .get::<Children>(popup)
            .unwrap()
            .iter()
            .filter(|&child| world.get::<ListItem>(child).is_some())
            .nth(1)
            .expect("open ComboBox should contain a second option");

        (popup, hovered_option)
    };

    // 第二个 ComboBox：打开 Popup，同时 hover Banana
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

    app.world_mut()
        .entity_mut(hovered_option)
        .insert(Hovered(true));

    // 第三个 ComboBox：Disabled
    app.world_mut()
        .entity_mut(disabled_combo)
        .insert(InteractionDisabled);

    // 让样式系统处理这些状态变化
    app.update();

    let baseline_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("baselines")
        .join("styled_combo_box.png");

    std::fs::create_dir_all(baseline_path.parent().unwrap()).unwrap();

    let screenshot_entity = app
        .world_mut()
        .spawn(Screenshot::image(image_handle))
        .observe(save_to_disk(baseline_path.clone()))
        .id();

    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        app.update();

        if app.world().get_entity(screenshot_entity).is_err() {
            break;
        }

        assert!(
            Instant::now() < deadline,
            "timed out generating styled ComboBox baseline"
        );

        std::thread::yield_now();
    }

    assert!(baseline_path.exists(), "baseline PNG was not created");

    println!("baseline written to {}", baseline_path.display());
}
