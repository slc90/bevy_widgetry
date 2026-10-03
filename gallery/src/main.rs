//! 提供可运行的 Widget Gallery，用于浏览 Widgetry 的功能、体验输入交互和检查组合场景的界面表现。
//! 各示例展示实际 Widget 与业务内容的组合，可用于确认 theme、layout 和 state 变化后的可视效果。
//!
//! 提供 Button、CheckBox、RadioGroup、TextField、ComboBox 和 Tooltip 等基础交互示例。
//! 提供 ScrollArea、ListView、Tree 和 Table 示例，展示滚动、selection、navigation、数据更新与自定义内容。
//! 提供多 channel Waveform 示例，展示连续数据、可见时间范围和绘制配置变化。
//! Window 页面提供独立窗口、Theme/Stretch/Cover 图片背景、modal MessageBox 和文件 dialog 等组合场景。
//! 可通过导航切换示例页面，并通过 theme 选择器切换 Dark 与 Light 配色。
//! 提供 BRP runtime 接入，便于外部工具检查运行时 state、执行交互和获取截图。
//! 提供 startup readiness 与 Waveform GUI 性能测量入口，按启用的测量模式输出结果和截图证据。
//!
//! Gallery 使用桌面 App 的交互方式，可直接操作各页面中的 Widget。
//! theme 切换同时更新示例与窗口内容，可观察同一场景在两种配色下的表现。
//! 性能测量通过 GALLERY_STARTUP_BENCH_OUTPUT 或 GALLERY_WAVEFORM_BENCH_OUTPUT 指定输出目录后启用。
//! startup readiness 检查初始页面、可交互导航以及实际截图中的文字和 Icon 是否准备完成。
//! 各页面的交互和测量范围由当前示例提供的场景决定。

mod assets;
mod gallery;
mod pages;
mod renderer;
mod startup_benchmark;
mod waveform_benchmark;
mod waveform_data;

use crate::assets::{GalleryAssetPlugin, GalleryIcon};
use crate::gallery::GalleryPlugin;
use bevy::app::Propagate;
use bevy::log::LogPlugin;
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
use bevy_widgetry::scene::WidgetrySceneCommandsExt;
use bevy_widgetry::style::{ForegroundColor, z_index};
use bevy_widgetry::style::{ThemeChanged, ThemeMode};
use bevy_widgetry::text_field::WidgetryTextFieldPlugin;
use bevy_widgetry::tooltip::WidgetryTooltipPlugin;
use bevy_widgetry::window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
    prepare_native_window, widgetry_window,
};
use bevy_widgetry_app_logging::{AppLogging, file_layer, terminal_layer};
use std::path::{Path, PathBuf};

#[derive(Component)]
struct GalleryTitle;

#[derive(Component)]
struct ThemeComboBox;

fn main() -> Result {
    let log_dir = std::env::var_os("GALLERY_STARTUP_BENCH_STATE")
        .map(|state| PathBuf::from(state).join("logs"))
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("logs"));
    let logging = AppLogging::new(log_dir)?;
    let mut app = App::new();
    let _log_guard = logging.install(&mut app);
    app.insert_resource(WinitSettings::desktop_app());
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                fmt_layer: terminal_layer,
                custom_layer: file_layer,
                ..default()
            })
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
        renderer::GalleryRenderPlugin,
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
    startup_benchmark::install(&mut app)?;
    waveform_benchmark::install(&mut app)?;
    app.run();
    Ok(())
}

fn gallery_window() -> Window {
    prepare_native_window(Window {
        title: "Widget Gallery".into(),
        resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
        position: WindowPosition::Centered(MonitorSelection::Primary),
        ..default()
    })
}

fn setup(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    theme_mode: Res<ThemeMode>,
    list_sources: Res<pages::ListViewDemoSources>,
    combo_sources: Res<pages::ComboBoxDemoSources>,
    tree_sources: Res<pages::TreeDemoSources>,
    table_sources: Res<pages::TableDemoSources>,
    waveform_sources: Res<pages::WaveformDemoSources>,
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
        widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![title_content(theme_combo)], bsn_list![gallery::scene(list_sources.0, combo_sources.0, tree_sources.0, table_sources.clone(), Box::new(bsn_list![pages::waveform(&waveform_sources)]))])
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

fn refresh_title_theme(
    event: On<ThemeChanged>,
    mut titles: Query<&mut Propagate<ForegroundColor>, With<GalleryTitle>>,
) {
    for mut foreground in &mut titles {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

fn on_theme_combo_box_changed(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
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
        .and_then(|model| model.get_by_id(event.value?))
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
