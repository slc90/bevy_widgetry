use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{ListBox, ListItem},
};
use bevy_widgetry::combo_box::{StyledComboBoxPlugin, spawn_styled_combo_box};
pub const WIDTH: u32 = 704;
pub const HEIGHT: u32 = 180;
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

pub fn setup(app: &mut App) {
    app.add_plugins(StyledComboBoxPlugin);
    app.add_systems(Startup, spawn_combo_box_visual_scene);
}
pub fn spawn(app: &mut App, camera: Entity) {
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
}
