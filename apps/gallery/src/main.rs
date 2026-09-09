use bevy::ui_widgets::ValueChange;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, WgpuSettings},
    },
};
use bevy_widgetry::button::{StyledButton, StyledButtonPlugin};
use bevy_widgetry::combo_box::{SetComboBoxSelected, StyledComboBoxPlugin, spawn_styled_combo_box};
use bevy_widgetry::style::{ThemeChanged, ThemeMode};
use bevy_widgetry::window::{TitleBarPlugin, spawn_window};

#[derive(Component)]
struct ThemeComboBox;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Widget Gallery".into(),
                        resolution: WindowResolution::new(1200, 800),
                        decorations: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        // Windows上选择Vulkan时拉伸有黑色
                        backends: Some(Backends::DX12),
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        )
        .add_plugins((TitleBarPlugin, StyledButtonPlugin, StyledComboBoxPlugin))
        .add_observer(on_theme_combo_box_changed)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    theme_mode: Res<ThemeMode>,
) {
    let mut theme_slot = None;
    let window = primary_window.single().unwrap();
    commands.spawn(Camera2d);

    spawn_window(
        &mut commands,
        &asset_server,
        window,
        // title bar content
        |commands, title_bar| {
            commands
                .spawn((
                    Pickable::IGNORE,
                    ChildOf(title_bar),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                ))
                .with_children(|bar| {
                    bar.spawn(Text::new("Widget Gallery"));

                    theme_slot = Some(
                        bar.spawn((
                            Pickable::IGNORE,
                            Node {
                                height: percent(100),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .id(),
                    );
                });
        },
        // window content
        |commands, content| {
            commands
                .spawn((
                    StyledButton,
                    ChildOf(content),
                    Node {
                        width: px(160),
                        height: px(40),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ))
                .with_child(Text::new("Button"));

            let combo = spawn_styled_combo_box(
                commands,
                vec!["Apple".into(), "Banana".into(), "Orange".into()],
            );

            commands.entity(combo).insert(ChildOf(content));
        },
    );

    let theme_combo = spawn_styled_combo_box(&mut commands, vec!["Dark".into(), "Light".into()]);

    commands
        .entity(theme_combo)
        .insert((ThemeComboBox, ChildOf(theme_slot.unwrap())));

    let selected = match *theme_mode {
        ThemeMode::Dark => 0,
        ThemeMode::Light => 1,
    };

    commands.trigger(SetComboBoxSelected {
        entity: theme_combo,
        selected,
    });
}

fn on_theme_combo_box_changed(
    event: On<ValueChange<usize>>,
    theme_combo_boxes: Query<(), With<ThemeComboBox>>,
    mut theme_mode: ResMut<ThemeMode>,
    mut commands: Commands,
) {
    // 只处理标题栏里的 Theme ComboBox
    if !theme_combo_boxes.contains(event.source) {
        return;
    }

    let mode = match event.value {
        0 => ThemeMode::Dark,
        1 => ThemeMode::Light,
        _ => return,
    };

    if *theme_mode == mode {
        return;
    }

    *theme_mode = mode;

    commands.trigger(ThemeChanged { mode });
}
