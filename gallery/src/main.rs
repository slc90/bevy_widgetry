use bevy_widgetry::scene::WidgetrySceneCommandsExt;
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
use bevy_brp_runtime::BrpRuntimePlugin;
use bevy_widgetry::button::WidgetryButtonPlugin;
use bevy_widgetry::check_box::WidgetryCheckBoxPlugin;
use bevy_widgetry::combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt, WidgetryComboBoxPlugin};
use bevy_widgetry::icon::WidgetryIcon;
use bevy_widgetry::list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};
use bevy_widgetry::radio_group::WidgetryRadioGroupPlugin;
use bevy_widgetry::style::{ForegroundColor, z_index};
use bevy_widgetry::style::{ThemeChanged, ThemeMode};
use bevy_widgetry::text_field::WidgetryTextFieldPlugin;
use bevy_widgetry::tooltip::WidgetryTooltipPlugin;
use bevy_widgetry::window::{
    WidgetryWindowControlsConfig, WidgetryWindowPlugin, prepare_native_window, widgetry_window,
};

/// 标记应用自有标题颜色，避免刷新其他 Widget 的 foreground color。
#[derive(Component)]
struct GalleryTitle;

/// 标记应用的 theme selector，避免其他 ComboBox 触发全局 theme 切换。
#[derive(Component)]
struct ThemeComboBox;

/// 装配 Gallery 的 window、render backend 及 Widget plugin 并启动应用。
fn main() -> Result {
    let logging = logging::GalleryLogging::new()?;
    let mut app = App::new();
    let _log_guard = logging.install(&mut app);
    app.insert_resource(WinitSettings::desktop_app());
    app.add_plugins(
        DefaultPlugins
            .set(logging::log_plugin())
            // Gallery 只演示 pointer/keyboard，无需初始化手柄 backend 和设备 mapping。
            .disable::<bevy::gilrs::GilrsPlugin>()
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
        BrpRuntimePlugin::default(),
        GalleryAssetPlugin,
        WidgetryWindowPlugin,
        WidgetryButtonPlugin,
        WidgetryCheckBoxPlugin,
        WidgetryComboBoxPlugin,
        WidgetryRadioGroupPlugin,
        WidgetryTextFieldPlugin,
        WidgetryTooltipPlugin,
        GalleryPlugin,
    ))
    .register_widgetry_combo_box::<ThemeMode>()?
    .add_observer(on_theme_combo_box_changed)
    .add_observer(refresh_title_theme)
    .add_systems(Startup, setup);
    app.run();
    Ok(())
}

/// 集中声明 Gallery 的桌面 window 配置。
fn gallery_window() -> Window {
    prepare_native_window(Window {
        title: "Widget Gallery".into(),
        // 固定 Gallery 的 window scale factor，避免跟随系统 DPI 缩放。
        resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
        position: WindowPosition::Centered(MonitorSelection::Primary),
        ..default()
    })
}

/// 为主 window 指定 camera 与 BSN 内容，并静默初始化 theme ComboBox。
fn setup(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    theme_mode: Res<ThemeMode>,
    list_sources: Res<pages::ListViewDemoSources>,
    combo_sources: Res<pages::ComboBoxDemoSources>,
    tree_sources: Res<pages::TreeDemoSources>,
    table_sources: Res<pages::TableDemoSources>,
) -> Result {
    let target = primary_window.single()?;
    let camera = commands.spawn(Camera2d).id();
    let mut model = WidgetryListModel::default();
    let dark = model.push(ThemeMode::Dark)?;
    let light = model.push(ThemeMode::Light)?;
    let source = commands.spawn(model).id();
    let theme_combo = commands
        .spawn_scene_with_error_handler(bsn! {
            @WidgetryComboBox::<ThemeMode> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_, mode: &ThemeMode| bsn_list![(Text({if *mode == ThemeMode::Dark { "Dark" } else { "Light" }}))])},
            }
            template(|_| Ok(ThemeComboBox))
        })
        .id();
    commands.spawn_scene_with_error_handler(bsn! {
        widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), bsn_list![title_content(theme_combo)], bsn_list![gallery::scene(list_sources.0, combo_sources.0, tree_sources.0, table_sources.clone())])
    });
    WidgetryComboBox::<ThemeMode>::set_selected(
        &mut commands,
        theme_combo,
        if *theme_mode == ThemeMode::Dark {
            dark
        } else {
            light
        },
    );
    Ok(())
}

/// 应用标题与 Logo 使用普通内容 slot，theme selector 保持独立 picking。
/// SVG 的 currentColor 使用白色 mask，使 WidgetryIcon 继承的 foreground color 能够直接调色。
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
                        @WidgetryIcon {
                            @path: {GalleryIcon::Logo.path()},
                            @max_size: { Some(UVec2::new(24, 24)) },
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
                GlobalZIndex({z_index::LOCAL_OVERLAY})
            ),
        ]
    }
}

/// 跟随 theme 刷新应用自有标题的 foreground color，不改变 Window 系统按钮配色。
fn refresh_title_theme(
    event: On<ThemeChanged>,
    mut titles: Query<&mut Propagate<ForegroundColor>, With<GalleryTitle>>,
) {
    for mut foreground in &mut titles {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

/// 只接受 theme selector source 中的有效 stable id，resource 改变后再通知 Widget 刷新。
fn on_theme_combo_box_changed(
    event: On<ValueChange<WidgetryListItemId>>,
    theme_combo_boxes: Query<&WidgetryComboBox<ThemeMode>, With<ThemeComboBox>>,
    models: Query<&WidgetryListModel<ThemeMode>>,
    mut theme_mode: ResMut<ThemeMode>,
    mut commands: Commands,
) {
    let Ok(combo) = theme_combo_boxes.get(event.source) else {
        return;
    };
    let Some(mode) = models
        .get(combo.source())
        .ok()
        .and_then(|model| model.get_by_id(event.value))
        .copied()
    else {
        return;
    };

    if *theme_mode == mode {
        return;
    }

    *theme_mode = mode;
    info!(mode = ?mode, "切换主题");

    commands.trigger(ThemeChanged { mode });
}
