use bevy::text::EditableText;
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
use bevy_widgetry::text_field::{StyledTextField, StyledTextFieldPlugin};
use bevy_widgetry::window::{TitleBarPlugin, spawn_window};

/// 标记应用的主题选择器，避免其他下拉框触发全局主题切换。
#[derive(Component)]
struct ThemeComboBox;

/// 装配 Gallery 的窗口、渲染后端及控件插件并启动应用。
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
        .add_plugins((
            TitleBarPlugin,
            StyledButtonPlugin,
            StyledComboBoxPlugin,
            StyledTextFieldPlugin,
        ))
        .add_observer(on_theme_combo_box_changed)
        .add_systems(Startup, setup)
        .run();
}

/// 在主窗口构建演示控件；主窗口查询失败时将错误交给系统错误处理器。
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    theme_mode: Res<ThemeMode>,
) -> Result {
    let theme_slot = commands.spawn_empty().id();
    let window = primary_window.single()?;
    commands.spawn(Camera2d);

    spawn_window(
        &mut commands,
        &asset_server,
        window,
        // 在标题栏中安排应用标题与主题选择器。
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

                    let parent = bar.target_entity();
                    bar.commands().entity(theme_slot).insert((
                        ChildOf(parent),
                        Pickable::IGNORE,
                        Node {
                            height: percent(100),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ));
                });
        },
        // 在窗口内容区展示各控件。
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
            commands.spawn((StyledTextField, EditableText::new("A"), ChildOf(content)));
            commands.spawn((StyledTextField, EditableText::new("B"), ChildOf(content)));
        },
    );

    let theme_combo = spawn_styled_combo_box(&mut commands, vec!["Dark".into(), "Light".into()]);

    commands
        .entity(theme_combo)
        .insert((ThemeComboBox, ChildOf(theme_slot)));

    let selected = match *theme_mode {
        ThemeMode::Dark => 0,
        ThemeMode::Light => 1,
    };

    commands.trigger(SetComboBoxSelected {
        entity: theme_combo,
        selected,
    });
    Ok(())
}

/// 只接受主题选择器的有效索引，资源改变后再通知控件刷新。
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
