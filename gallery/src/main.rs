mod gallery;
mod pages;

use bevy::ui_widgets::ValueChange;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowPosition, WindowResolution};
use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, WgpuSettings},
    },
};
use bevy_widgetry::button::StyledButtonPlugin;
use bevy_widgetry::combo_box::{SetComboBoxSelected, StyledComboBoxPlugin, spawn_styled_combo_box};
use bevy_widgetry::style::{ThemeChanged, ThemeMode};
use bevy_widgetry::text_field::StyledTextFieldPlugin;
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
                    primary_window: Some(gallery_window()),
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

/// 集中声明 Gallery 的桌面窗口配置。
fn gallery_window() -> Window {
    Window {
        title: "Widget Gallery".into(),
        // 固定 Gallery 的窗口缩放因子，避免跟随系统 DPI 缩放
        resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
        position: WindowPosition::Centered(MonitorSelection::Primary),
        decorations: false,
        ..default()
    }
}

/// 覆盖完整窗口的纯视觉边框场景。
fn window_border() -> impl Scene {
    bsn! {
        #WindowBorder
        template(|_| Ok(Pickable::IGNORE))
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            border: UiRect::all(px(1)),
        }
        template(|_| Ok(BorderColor::all(Color::WHITE)))
        GlobalZIndex(100)
    }
}

/// 在主窗口构建演示控件；主窗口查询失败时将错误交给系统错误处理器。
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    theme_mode: Res<ThemeMode>,
) -> Result {
    let window = primary_window.single()?;
    commands.spawn(Camera2d);
    let theme_combo = spawn_styled_combo_box(&mut commands, vec!["Dark".into(), "Light".into()]);
    // 主题弹层会伸入页面区，必须高于后创建的页面容器以保持选项可点击。
    commands.entity(theme_combo).insert(ThemeComboBox);

    let window_root = spawn_window(
        &mut commands,
        &asset_server,
        window,
        // 在标题栏中安排应用标题与主题选择器。
        |commands, title_bar| {
            let bar = commands
                .spawn_scene(bsn! {
                    template(|_| Ok(Pickable::IGNORE))
                    ChildOf(title_bar)
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                    }
                    Children [Text("Widget Gallery")]
                })
                .id();
            let theme_slot = commands
                .spawn_scene(bsn! {
                    ChildOf(bar)
                    template(|_| Ok(Pickable::IGNORE))
                    Node {
                        height: percent(100),
                        align_items: AlignItems::Center,
                    }
                })
                .id();
            commands.entity(theme_combo).insert(ChildOf(theme_slot));
        },
        // 在窗口内容区展示各控件。
        |commands, content| {
            commands.spawn_scene(bsn! {
                gallery::scene()
                ChildOf(content)
            });
        },
    );

    commands.spawn_scene(bsn! {
        window_border()
        ChildOf(window_root)
    });

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
    info!("theme_combo_boxes: {:?}", theme_combo_boxes);

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
