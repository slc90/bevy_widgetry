mod assets;
mod gallery;
mod logging;
mod pages;
mod renderer;

use crate::assets::{GalleryAssetPlugin, GalleryIcon};
use crate::gallery::GalleryPlugin;
use bevy::app::Propagate;
use bevy::ui_widgets::ValueChange;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowPosition, WindowResolution};
use bevy::winit::WinitSettings;
use bevy::{prelude::*, render::RenderPlugin, tasks::block_on};
use bevy_widgetry::button::StyledButtonPlugin;
use bevy_widgetry::combo_box::{SetComboBoxSelected, StyledComboBoxPlugin, spawn_styled_combo_box};
use bevy_widgetry::icon::Icon;
use bevy_widgetry::style::ForegroundColor;
use bevy_widgetry::style::{ThemeChanged, ThemeMode};
use bevy_widgetry::text_field::StyledTextFieldPlugin;
use bevy_widgetry::window::{
    WindowControlsConfig, WindowPlugin as WidgetryWindowPlugin, widgetry_window, window,
};

/// 标记应用自有标题颜色，避免刷新其他控件的前景色。
#[derive(Component)]
struct GalleryTitle;

/// 标记应用的主题选择器，避免其他下拉框触发全局主题切换。
#[derive(Component)]
struct ThemeComboBox;

/// 装配 Gallery 的窗口、渲染后端及控件插件并启动应用。
fn main() -> Result {
    let logging = logging::GalleryLogging::new()?;
    let mut app = App::new();
    let _log_guard = logging.install(&mut app);
    // continuous模式每个窗口都疯狂刷新，会导致很卡
    // 设置成这样
    app.insert_resource(WinitSettings::desktop_app());
    app.add_plugins(
        DefaultPlugins
            .set(logging::log_plugin())
            .set(WindowPlugin {
                primary_window: Some(gallery_window()),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: block_on(renderer::transparent_renderer())?,
                ..default()
            }),
    )
    .add_plugins((
        GalleryAssetPlugin,
        WidgetryWindowPlugin,
        StyledButtonPlugin,
        StyledComboBoxPlugin,
        StyledTextFieldPlugin,
        GalleryPlugin,
    ))
    .add_observer(on_theme_combo_box_changed)
    .add_observer(refresh_title_theme)
    .add_systems(Startup, setup);
    app.run();
    Ok(())
}

/// 集中声明 Gallery 的桌面窗口配置。
fn gallery_window() -> Window {
    widgetry_window(Window {
        title: "Widget Gallery".into(),
        // 固定 Gallery 的窗口缩放因子，避免跟随系统 DPI 缩放
        resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
        position: WindowPosition::Centered(MonitorSelection::Primary),
        ..default()
    })
}

/// 为主窗口指定相机与 BSN 内容，主题下拉框沿用已有控件入口。
fn setup(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    theme_mode: Res<ThemeMode>,
) -> Result {
    let target = primary_window.single()?;
    let camera = commands.spawn(Camera2d).id();
    let theme_combo = spawn_styled_combo_box(&mut commands, vec!["Dark".into(), "Light".into()]);
    commands.entity(theme_combo).insert(ThemeComboBox);
    commands.spawn_scene(bsn! {
        window(target, camera, WindowControlsConfig::default(), bsn_list![title_content(theme_combo)], bsn_list![gallery::scene()])
    });
    commands.trigger(SetComboBoxSelected {
        entity: theme_combo,
        selected: if *theme_mode == ThemeMode::Dark { 0 } else { 1 },
    });
    Ok(())
}

/// 应用标题与 Logo 使用普通内容插槽，主题选择器保持独立拾取。
/// SVG 的 currentColor 使用白色遮罩，使 Icon 的继承前景色能够直接调色。
fn title_content(theme_combo: Entity) -> impl Scene {
    bsn! {
        template(|_| Ok(Pickable::IGNORE))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        template(|_| Ok(GalleryTitle))
        Node {
            width: percent(100), height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::left(px(12)),
        }
        Children [
            (
                template(|_| Ok(Pickable::IGNORE))
                Node { align_items: AlignItems::Center, column_gap: px(8) }
                Children [
                    (
                        @Icon {
                            @path: {GalleryIcon::Logo.path()},
                            @max_size: { Some(UVec2::new(16, 16)) },
                        }
                        template(|_| Ok(Pickable::IGNORE))
                        Node { width: px(16), height: px(16) }
                    ),
                    (Text("Widget Gallery") template(|_| Ok(Pickable::IGNORE))),
                ]
            ),
            (
                template(move |context| {
                    context.entity.add_child(theme_combo);
                    Ok(Pickable::IGNORE)
                })
                Node { height: percent(100), align_items: AlignItems::Center }
                GlobalZIndex(10)
            ),
        ]
    }
}

/// 跟随主题刷新应用自有标题前景色，不改变 Window 系统按钮配色。
fn refresh_title_theme(
    event: On<ThemeChanged>,
    mut titles: Query<&mut Propagate<ForegroundColor>, With<GalleryTitle>>,
) {
    for mut foreground in &mut titles {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
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
