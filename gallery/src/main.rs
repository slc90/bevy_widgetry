//! 提供可运行的 Widget Gallery，用于浏览 Widgetry 的功能、体验输入交互和检查组合场景的界面表现。
//! 当前展示常驻窗口、Sidebar、theme 选择器和页面承载区域。
//!
//! Sidebar 提供 Button 到 Waveform 共 11 个页面的导航入口，点击后切换 GalleryPage State。
//! theme 选择器提供 Dark 与 Light 配色切换，同时更新窗口内容与 Sidebar 边框。
//! 提供 BRP runtime 接入，便于外部工具检查运行时 state、执行交互和获取截图。
//!
//! 启动后从 Initializing 转入 Button State，主窗口、Camera、Sidebar 和 theme 选择器保持常驻。
//! 每次只挂载当前页面及演示内容，颜色区域按需展开，切走后销毁，重进重新折叠。
//! 页面 UI 的实例 state 与 ComboBox、ListView、Tree 演示数据在重进时重新创建，Header 的 theme Model 保持常驻。
//! Table 的演示 Model 与 Waveform producer 当前仍在应用期间保留。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod assets;
mod color_showcase;
mod file_dialog_benchmark;
mod gallery;
mod page_lifecycle_benchmark;
mod pages;
mod waveform_benchmark;
mod waveform_data;

use crate::assets::{GalleryAssetPlugin, GalleryIcon};
use crate::gallery::{GalleryPage, GalleryPlugin};
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
use bevy_widgetry::style::z_index;
use bevy_widgetry::text_field::WidgetryTextFieldPlugin;
use bevy_widgetry::theme::{WidgetryThemeChanged, WidgetryThemeMode};
use bevy_widgetry::tooltip::WidgetryTooltipPlugin;
use bevy_widgetry::window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
    prepare_native_window, transparent_render_creation, widgetry_window,
};
use bevy_widgetry_app_logging::{AppLogging, file_layer, terminal_layer};
use std::path::PathBuf;

#[derive(Component)]
struct GalleryTitle;

#[derive(Component)]
struct ThemeComboBox;

fn main() -> Result {
    let log_dir = match std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT") {
        Some(output) => PathBuf::from(output).join("logs"),
        None => std::env::current_exe()
            .map_err(|error| {
                error!(error = %error, "无法确定 Gallery executable 路径");
                BevyError::error(error)
            })?
            .with_file_name("logs"),
    };
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
                render_creation: block_on(transparent_render_creation())?,
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
    .register_widgetry_combo_box::<WidgetryThemeMode>()?
    .add_observer(on_theme_combo_box_changed)
    .add_observer(sync_theme_combo_box)
    .add_systems(Startup, setup)
    .add_plugins(bevy_widgetry::window::taskbar_icon!(
        "src/assets/icons/widget_gallery_taskbar.png"
    )?);
    waveform_benchmark::install(&mut app)?;
    file_dialog_benchmark::install(&mut app)?;
    page_lifecycle_benchmark::install(&mut app)?;
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
    theme_mode: Res<WidgetryThemeMode>,
    mut next_page: ResMut<NextState<GalleryPage>>,
) -> Result {
    let target = primary_window.single()?;
    let camera = commands.spawn(Camera2d).id();
    let mut model = WidgetryListModel::default();
    let dark = model.push(WidgetryThemeMode::Dark)?;
    let light = model.push(WidgetryThemeMode::Light)?;
    let source = commands.spawn(model).id();
    let theme_combo = commands
        .spawn_scene_with_error_handler(bsn! {
            #ThemeComboBox
            @WidgetryComboBox::<WidgetryThemeMode> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_, mode: &WidgetryThemeMode| bsn_list!{Text({if *mode == WidgetryThemeMode::Dark { "Dark" } else { "Light" }}) bevy_widgetry::text::WidgetryText})},
            }
            template(|_| Ok(ThemeComboBox))
        })
        .id();
    commands.spawn_scene_with_error_handler(bsn! {
        @widgetry_window(target, camera, WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{@title_content(theme_combo)}, bsn_list!{@gallery::scene()})
    });
    WidgetryComboBox::<WidgetryThemeMode>::set_selected(
        &mut commands,
        theme_combo,
        if *theme_mode == WidgetryThemeMode::Dark {
            dark
        } else {
            light
        },
    );
    // 默认 State 的 OnEnter 早于 Startup，先完成 deferred 外壳构造再进入 Button。
    next_page.set_if_different(GalleryPage::Button);
    Ok(())
}

fn title_content(theme_combo: Entity) -> impl Scene {
    bsn! {
        template(|_| Ok(Pickable::IGNORE))
        template(|_| Ok(GalleryTitle))
        Node {
            width: percent(100), height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::left(px(12)),
        }
        Children [

                template(|_| Ok(Pickable::IGNORE))
                Node { align_items: AlignItems::Center, column_gap: px(8) }
                Children [

                        @WidgetryIcon {
                            @path: {GalleryIcon::Logo.path()},
                            @max_size: { Some(UVec2::new(24, 24)) },
                        }
                        template(|_| Ok(Pickable::IGNORE))
                        Node { width: px(16), height: px(16) }
                    --
                    Text("Widget Gallery") bevy_widgetry::text::WidgetryText template(|_| Ok(Pickable::IGNORE))
                ]
            --

                template(move |context| {
                    context.entity.add_child(theme_combo);
                    Ok(Pickable::IGNORE)
                })
                Node { height: percent(100), align_items: AlignItems::Center }
                GlobalZIndex({z_index::LOCAL_OVERLAY})

        ]
    }
}

fn on_theme_combo_box_changed(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    theme_combo_boxes: Query<&WidgetryComboBox<WidgetryThemeMode>, With<ThemeComboBox>>,
    models: Query<&WidgetryListModel<WidgetryThemeMode>>,
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

    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if WidgetryThemeMode::set_in_world(world, mode)? {
            info!(?mode, "切换主题");
        }
        Ok(())
    });
}

fn sync_theme_combo_box(
    event: On<WidgetryThemeChanged>,
    combos: Query<(Entity, &WidgetryComboBox<WidgetryThemeMode>), With<ThemeComboBox>>,
    models: Query<&WidgetryListModel<WidgetryThemeMode>>,
    mut commands: Commands,
) {
    for (entity, combo) in &combos {
        if let Ok(model) = models.get(combo.source())
            && let Some(index) =
                (0..model.len()).find(|index| model.get(*index) == Some(&event.mode))
            && let Some(id) = model.id(index)
        {
            WidgetryComboBox::<WidgetryThemeMode>::set_selected(&mut commands, entity, id);
        }
    }
}
