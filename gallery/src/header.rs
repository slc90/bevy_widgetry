const TITLE_LEFT_PADDING: f32 = 12.0;
const LOGO_SIZE: f32 = 16.0;
const THEME_COMBO_WIDTH: f32 = 160.0;

const CONTROL_GAP: f32 = 8.0;

use crate::assets::GalleryIcon;
use crate::config::{AppConfig, ConfigFile};
use crate::i18n::HeaderKey;
use crate::i18n::{Language, TextKey, localized};
use crate::theme::GalleryTheme;
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::combo_box::{
    WidgetryComboBox, WidgetryComboBoxAppExt, WidgetryComboBoxHeadless,
};
use bevy_widgetry::icon::WidgetryIcon;
use bevy_widgetry::list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};
use bevy_widgetry::style::z_index;
use bevy_widgetry::theme::{WidgetryTheme, WidgetryThemeChanged};
use bevy_widgetry::ui_scale::WidgetryUiScale;

#[derive(Component)]
struct GalleryTitle;

#[derive(Component)]
struct ThemeComboBox;

#[derive(Component)]
struct LanguageComboBox;

#[derive(Component)]
struct ScaleFactorComboBox;

pub(crate) fn install(app: &mut App) -> Result {
    app.register_widgetry_combo_box::<GalleryTheme>()
        .map_err(bevy_widgetry::combo_box::ComboFailure::into_bevy_error)?
        .register_widgetry_combo_box::<f32>()
        .map_err(bevy_widgetry::combo_box::ComboFailure::into_bevy_error)?
        .register_widgetry_combo_box::<Language>()
        .map_err(bevy_widgetry::combo_box::ComboFailure::into_bevy_error)?
        .add_observer(on_language_changed)
        .add_observer(on_theme_combo_box_changed)
        .add_observer(on_scale_factor_combo_box_changed)
        .add_observer(sync_theme_combo_box)
        .add_systems(Update, sync_external_settings)
        .add_systems(
            PostStartup,
            (
                initialize_theme_selection,
                initialize_scale_factor_selection,
                initialize_language_selection,
            ),
        );
    Ok(())
}

fn sync_external_settings(world: &mut World) -> Result {
    let language = *world.resource::<Language>();
    let language_combos = world
        .query_filtered::<(Entity, &WidgetryComboBoxHeadless<Language>), With<LanguageComboBox>>()
        .iter(world)
        .map(|(root, combo)| (root, combo.source()))
        .collect::<Vec<_>>();
    for (root, source) in language_combos {
        let model = world
            .get::<WidgetryListModel<Language>>(source)
            .ok_or_else(|| header_sync_error("语言选择器 Model 缺失"))?;
        let index = (0..model.len())
            .find(|index| model.get(*index) == Some(&language))
            .ok_or_else(|| header_sync_error("当前语言不在选择器选项中"))?;
        if WidgetryComboBox::<Language>::selected_id(world, root)
            .map_err(bevy_widgetry::combo_box::ComboFailure::into_bevy_error)?
            != model.id(index)
        {
            WidgetryComboBox::<Language>::select_index(&mut world.commands(), root, index);
        }
    }
    let scale_combos = world
        .query_filtered::<(Entity, &WidgetryComboBoxHeadless<f32>), With<ScaleFactorComboBox>>()
        .iter(world)
        .map(|(root, combo)| (root, combo.source()))
        .collect::<Vec<_>>();
    for (root, source) in scale_combos {
        let scale = world.resource::<WidgetryUiScale>().get();
        let model = world
            .get::<WidgetryListModel<f32>>(source)
            .ok_or_else(|| header_sync_error("scale_factor Model 缺失"))?;
        let index = (0..model.len())
            .find(|index| model.get(*index) == Some(&scale))
            .ok_or_else(|| header_sync_error("当前 scale_factor 不在选择器选项中"))?;
        if WidgetryComboBox::<f32>::selected_id(world, root)
            .map_err(bevy_widgetry::combo_box::ComboFailure::into_bevy_error)?
            != model.id(index)
        {
            WidgetryComboBox::<f32>::select_index(&mut world.commands(), root, index);
        }
    }
    Ok(())
}

fn header_sync_error(reason: &str) -> BevyError {
    crate::diagnostics::at(
        reason.to_owned(),
        bevy_widgetry::diagnostics::ErrorFrame::new("Header ComboBox 同步失败"),
    )
}

pub(crate) fn scene(commands: &mut Commands) -> Result<impl Scene, BevyError> {
    let theme = theme_selector(commands)?;
    let scale = scale_factor_selector(commands)?;
    let language = language_selector(commands)?;
    Ok(bsn! {
        template(|_| Ok(Pickable::IGNORE))
        template(|_| Ok(GalleryTitle))
        Node {
            width: percent(100), height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::left(px(TITLE_LEFT_PADDING)),
        }
        Children [
            template(|_| Ok(Pickable::IGNORE))
            Node { align_items: AlignItems::Center, column_gap: px(CONTROL_GAP) }
            Children [
                @WidgetryIcon {
                    @path: {GalleryIcon::Logo.path()},
                    @max_size: { Some(UVec2::new(24, 24)) },
                }
                template(|_| Ok(Pickable::IGNORE))
                Node { width: px(LOGO_SIZE), height: px(LOGO_SIZE) }--
                bevy_widgetry::window::WidgetryWindowTitle bevy_widgetry::text::WidgetryText template(|_| Ok(Pickable::IGNORE))
            ]--
            template(|_| Ok(Pickable::IGNORE))
            Node { height: percent(100), align_items: AlignItems::Center, column_gap: px(CONTROL_GAP) }
            GlobalZIndex({z_index::LOCAL_OVERLAY})
            Children [@{language}-- @{scale}-- @{theme}]
        ]
    })
}

fn theme_selector(commands: &mut Commands) -> Result<impl Scene, BevyError> {
    let mut model = WidgetryListModel::default();
    for mode in [
        GalleryTheme::Dark,
        GalleryTheme::Light,
        GalleryTheme::PinkDream,
        GalleryTheme::KamuriViolet,
    ] {
        model
            .push(mode)
            .map_err(bevy_widgetry::list_view::ListFailure::into_bevy_error)?;
    }
    let source = commands.spawn(model).id();
    Ok(bsn! {
        #ThemeComboBox
        @WidgetryComboBox::<GalleryTheme> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|mode: &GalleryTheme| bsn_list!{@localized(match mode {
                GalleryTheme::Dark => TextKey::GalleryHeader(HeaderKey::Dark),
                GalleryTheme::Light => TextKey::GalleryHeader(HeaderKey::Light),
                GalleryTheme::PinkDream => TextKey::GalleryHeader(HeaderKey::PinkDream),
                GalleryTheme::KamuriViolet => TextKey::GalleryHeader(HeaderKey::KamuriViolet),
            }) bevy_widgetry::text::WidgetryText})},
        }
        template(|_| Ok(ThemeComboBox))
    })
}

fn scale_factor_selector(commands: &mut Commands) -> Result<impl Scene, BevyError> {
    let mut model = WidgetryListModel::default();
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        model
            .push(scale)
            .map_err(bevy_widgetry::list_view::ListFailure::into_bevy_error)?;
    }
    let source = commands.spawn(model).id();
    Ok(bsn! {
        #ScaleFactorComboBox
        @WidgetryComboBox::<f32> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|scale: &f32| bsn_list!{Text({format!("{scale:?}")}) bevy_widgetry::text::WidgetryText})},
        }
        template(|_| Ok(ScaleFactorComboBox))
        Node { width: px(100) }
    })
}

fn initialize_scale_factor_selection(
    combos: Query<(Entity, &WidgetryComboBoxHeadless<f32>), With<ScaleFactorComboBox>>,
    models: Query<&WidgetryListModel<f32>>,
    scale: Res<WidgetryUiScale>,
    mut commands: Commands,
) -> Result {
    for (entity, combo) in &combos {
        let model = models.get(combo.source()).map_err(|error| {
            crate::diagnostics::at(
                error,
                bevy_widgetry::diagnostics::ErrorFrame::new("scale_factor Model 缺失")
                    .field("source", format_args!("{:?}", combo.source())),
            )
        })?;
        let scale = scale.get();
        let id = (0..model.len())
            .find(|index| model.get(*index) == Some(&scale))
            .and_then(|index| model.id(index))
            .ok_or_else(|| {
                crate::diagnostics::at(
                    "初始 scale_factor 不在选择器选项中",
                    bevy_widgetry::diagnostics::ErrorFrame::new(
                        "初始 scale_factor 不在选择器选项中",
                    )
                    .field("scale_factor", scale),
                )
            })?;
        WidgetryComboBox::<f32>::select_id(&mut commands, entity, id);
    }
    Ok(())
}

fn on_scale_factor_combo_box_changed(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    combos: Query<&WidgetryComboBoxHeadless<f32>, With<ScaleFactorComboBox>>,
    models: Query<&WidgetryListModel<f32>>,
    mut current: ResMut<WidgetryUiScale>,
    mut config: ResMut<AppConfig>,
    config_file: Res<ConfigFile>,
) -> Result {
    let Ok(combo) = combos.get(event.source) else {
        return Ok(());
    };
    let Some(selected) = event.value else {
        return Ok(());
    };
    let model = models.get(combo.source()).map_err(|error| {
        crate::diagnostics::at(
            error,
            bevy_widgetry::diagnostics::ErrorFrame::new("scale_factor Model 缺失")
                .field("source", format_args!("{:?}", combo.source())),
        )
    })?;
    let scale = *model.get_by_id(selected).ok_or_else(|| {
        crate::diagnostics::at(
            "scale_factor selection 已失效",
            bevy_widgetry::diagnostics::ErrorFrame::new("scale_factor selection 已失效")
                .field("selected", format_args!("{:?}", selected)),
        )
    })?;
    let previous = current.get();
    if current
        .set(scale)
        .map_err(|failure| failure.into_bevy_error())?
    {
        info!(
            setting = "scale_factor",
            requested = scale,
            previous,
            "用户选择 Gallery 设置"
        );
        debug!(
            setting = "scale_factor",
            applied = scale,
            "Gallery 全局缩放设置已更新"
        );
        config.scale_factor = scale;
        config.save(&config_file);
    }
    Ok(())
}

fn on_theme_combo_box_changed(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    theme_combo_boxes: Query<&WidgetryComboBoxHeadless<GalleryTheme>, With<ThemeComboBox>>,
    models: Query<&WidgetryListModel<GalleryTheme>>,
    mut commands: Commands,
) -> Result {
    let Ok(combo) = theme_combo_boxes.get(event.source) else {
        return Ok(());
    };
    let Some(selected) = event.value else {
        return Ok(());
    };
    let model = models
        .get(combo.source())
        .map_err(|source| crate::diagnostics::error(source))?;
    let mode = *model
        .get_by_id(selected)
        .ok_or_else(|| header_sync_error("主题选择器选中项失效"))?;

    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        let previous = world
            .get_resource::<WidgetryTheme>()
            .and_then(GalleryTheme::from_colors);
        if previous != Some(mode) {
            info!(setting = "theme", requested = ?mode, ?previous, "用户选择 Gallery 设置");
        }
        if WidgetryTheme::set_in_world(world, mode.colors())
            .map_err(|error| error.into_bevy_error())?
        {
            debug!(setting = "theme", applied = ?mode, "Gallery 设置已生效");
            world.resource_mut::<AppConfig>().theme = mode;
            world
                .resource::<AppConfig>()
                .save(world.resource::<ConfigFile>());
        }
        Ok(())
    });
    Ok(())
}

fn sync_theme_combo_box(
    _: On<WidgetryThemeChanged>,
    theme: Res<WidgetryTheme>,
    combos: Query<(Entity, &WidgetryComboBoxHeadless<GalleryTheme>), With<ThemeComboBox>>,
    models: Query<&WidgetryListModel<GalleryTheme>>,
    mut commands: Commands,
) -> Result {
    select_theme(
        GalleryTheme::from_colors(&theme),
        &combos,
        &models,
        &mut commands,
    )
}

fn initialize_theme_selection(
    theme_mode: Res<WidgetryTheme>,
    combos: Query<(Entity, &WidgetryComboBoxHeadless<GalleryTheme>), With<ThemeComboBox>>,
    models: Query<&WidgetryListModel<GalleryTheme>>,
    mut commands: Commands,
) -> Result {
    select_theme(
        GalleryTheme::from_colors(&theme_mode),
        &combos,
        &models,
        &mut commands,
    )
}

fn select_theme(
    mode: Option<GalleryTheme>,
    combos: &Query<(Entity, &WidgetryComboBoxHeadless<GalleryTheme>), With<ThemeComboBox>>,
    models: &Query<&WidgetryListModel<GalleryTheme>>,
    commands: &mut Commands,
) -> Result {
    for (entity, combo) in combos {
        let model = models
            .get(combo.source())
            .map_err(|source| crate::diagnostics::error(source))?;
        let id = (0..model.len())
            .find(|index| model.get(*index).copied() == mode)
            .and_then(|index| model.id(index));
        if let Some(id) = id {
            WidgetryComboBox::<GalleryTheme>::select_id(commands, entity, id);
        } else {
            WidgetryComboBox::<GalleryTheme>::clear_selection(commands, entity);
        }
    }
    Ok(())
}

fn language_selector(commands: &mut Commands) -> Result<impl Scene, BevyError> {
    let mut model = WidgetryListModel::default();
    model
        .push(Language::English)
        .map_err(bevy_widgetry::list_view::ListFailure::into_bevy_error)?;
    model
        .push(Language::SimplifiedChinese)
        .map_err(bevy_widgetry::list_view::ListFailure::into_bevy_error)?;
    let source = commands.spawn(model).id();
    Ok(bsn! {
        #LanguageComboBox
        @WidgetryComboBox::<Language> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|language: &Language| bsn_list! {
                Text({match language {
                    Language::English => "English",
                    Language::SimplifiedChinese => "简体中文",
                }}) bevy_widgetry::text::WidgetryText
            })},
        }
        template(|_| Ok(LanguageComboBox))
        Node { width: px(THEME_COMBO_WIDTH) }
    })
}

fn initialize_language_selection(
    language: Res<Language>,
    combos: Query<(Entity, &WidgetryComboBoxHeadless<Language>), With<LanguageComboBox>>,
    models: Query<&WidgetryListModel<Language>>,
    mut commands: Commands,
) {
    for (entity, combo) in &combos {
        if let Ok(model) = models.get(combo.source())
            && let Some(index) =
                (0..model.len()).find(|index| model.get(*index) == Some(&*language))
            && let Some(id) = model.id(index)
        {
            WidgetryComboBox::<Language>::select_id(&mut commands, entity, id);
        }
    }
}

fn on_language_changed(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    combos: Query<&WidgetryComboBoxHeadless<Language>, With<LanguageComboBox>>,
    models: Query<&WidgetryListModel<Language>>,
    mut language: ResMut<Language>,
    mut config: ResMut<AppConfig>,
    config_file: Res<ConfigFile>,
) -> Result {
    let Ok(combo) = combos.get(event.source) else {
        return Ok(());
    };
    let Some(id) = event.value else {
        return Ok(());
    };
    let model = models.get(combo.source()).map_err(|error| {
        crate::diagnostics::at(
            error,
            bevy_widgetry::diagnostics::ErrorFrame::new("语言选择器 Model 缺失")
                .field("source", format_args!("{:?}", combo.source())),
        )
    })?;
    let selected = model.get_by_id(id).copied().ok_or_else(|| {
        crate::diagnostics::at(
            "语言选择器选中项失效",
            bevy_widgetry::diagnostics::ErrorFrame::new("语言选择器选中项失效")
                .field("id", format_args!("{:?}", id)),
        )
    })?;
    if *language != selected {
        info!(setting = "language", requested = ?selected, previous = ?*language, "用户选择 Gallery 设置");
    }
    if language.set_if_neq(selected) {
        debug!(setting = "language", applied = ?selected, "Gallery 设置已生效");
        config.language = selected;
        config.save(&config_file);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

    #[test]
    fn configured_scale_applies_to_the_gallery_logical_design_before_the_first_update() {
        let mut app = App::new();
        let config = AppConfig::default();
        app.insert_resource(WidgetryUiScale::new(config.scale_factor).unwrap())
            .add_plugins(bevy_widgetry::ui_scale::WidgetryUiScalePlugin);
        let window = app.world_mut().spawn(crate::gallery_window(false)).id();
        let window = app.world().get::<Window>(window).unwrap();
        assert_eq!(
            window.resolution.size(),
            crate::STARTUP_LOGICAL_SIZE.as_vec2()
        );
        assert_eq!(window.physical_size(), UVec2::new(2400, 1350));
    }

    #[test]
    fn scale_selection_updates_open_windows_and_subsequent_popups() {
        let directory = tempfile::tempdir().unwrap();
        let config_path = directory.path().join("config.ron");
        let loaded = crate::config::load(config_path.clone());
        let mut app = scene_app();
        app.insert_resource(WidgetryUiScale::new(loaded.config.scale_factor).unwrap())
            .add_plugins(bevy_widgetry::ui_scale::WidgetryUiScalePlugin)
            .insert_resource(loaded.config.language)
            .insert_resource(loaded.config)
            .insert_resource(loaded.file);
        install(&mut app).unwrap();
        let primary = app.world_mut().spawn(Window::default()).id();
        let independent = app.world_mut().spawn(Window::default()).id();
        let selector = scale_factor_selector(&mut app.world_mut().commands()).unwrap();
        app.world_mut().flush();
        let root = bevy_widgetry::scene::spawn_scene(app.world_mut(), selector).unwrap();
        let source = app
            .world()
            .get::<WidgetryComboBoxHeadless<f32>>(root)
            .unwrap()
            .source();
        for (index, expected) in [(0, 1.0), (2, 1.5)] {
            let selected = app
                .world()
                .get::<WidgetryListModel<f32>>(source)
                .unwrap()
                .id(index)
                .unwrap();
            app.world_mut().trigger(ValueChange {
                source: root,
                value: Some(selected),
                is_final: true,
            });
            let popup = app.world_mut().spawn(Window::default()).id();
            app.update();
            for entity in [primary, independent, popup] {
                assert_eq!(
                    app.world()
                        .get::<Window>(entity)
                        .unwrap()
                        .resolution
                        .scale_factor_override(),
                    Some(expected)
                );
            }
            assert_eq!(app.world().resource::<AppConfig>().scale_factor, expected);
            assert_eq!(
                crate::config::load(config_path.clone()).config.scale_factor,
                expected
            );
        }
    }

    #[test]
    fn external_palette_clears_preset_selection_and_known_palette_restores_it() {
        let mut app = scene_app();
        crate::i18n::install(&mut app);
        app.register_widgetry_combo_box::<GalleryTheme>()
            .unwrap()
            .add_observer(sync_theme_combo_box);
        let selector = theme_selector(&mut app.world_mut().commands()).unwrap();
        app.world_mut().flush();
        let root = bevy_widgetry::scene::spawn_scene(app.world_mut(), selector).unwrap();
        app.update();
        WidgetryTheme::set_in_world(app.world_mut(), GalleryTheme::Light.colors()).unwrap();
        app.update();
        assert!(
            WidgetryComboBox::<GalleryTheme>::selected_id(app.world(), root)
                .unwrap()
                .is_some()
        );
        let mut external = GalleryTheme::Light.colors();
        external.button.normal.background = Color::srgb(0.1, 0.2, 0.3);
        WidgetryTheme::set_in_world(app.world_mut(), external).unwrap();
        app.update();
        assert_eq!(
            WidgetryComboBox::<GalleryTheme>::selected_id(app.world(), root).unwrap(),
            None
        );
        WidgetryTheme::set_in_world(app.world_mut(), GalleryTheme::Dark.colors()).unwrap();
        app.update();
        let source = app
            .world()
            .get::<WidgetryComboBoxHeadless<GalleryTheme>>(root)
            .unwrap()
            .source();
        let id = app
            .world()
            .get::<WidgetryListModel<GalleryTheme>>(source)
            .unwrap()
            .id(0)
            .unwrap();
        assert_eq!(
            WidgetryComboBox::<GalleryTheme>::selected_id(app.world(), root).unwrap(),
            Some(id)
        );
    }

    #[test]
    fn missing_theme_reaches_the_command_error_handler_without_logging_success() {
        let mut app = scene_app();
        app.set_error_handler(ErrorCapture::handler());
        app.register_widgetry_combo_box::<GalleryTheme>()
            .unwrap()
            .add_observer(on_theme_combo_box_changed);
        let selector = theme_selector(&mut app.world_mut().commands()).unwrap();
        app.world_mut().flush();
        let root = bevy_widgetry::scene::spawn_scene(app.world_mut(), selector).unwrap();
        let source = app
            .world()
            .get::<WidgetryComboBoxHeadless<GalleryTheme>>(root)
            .unwrap()
            .source();
        let selected = app
            .world()
            .get::<WidgetryListModel<GalleryTheme>>(source)
            .unwrap()
            .id(1)
            .unwrap();
        app.world_mut().remove_resource::<WidgetryTheme>();
        let errors = ErrorCapture::default();
        let logs = LogCapture::with_targets(["bevy_widgetry", "widget_gallery::header"]);
        logs.run(|| {
            errors.run(|| {
                app.world_mut().trigger(ValueChange {
                    source: root,
                    value: Some(selected),
                    is_final: true,
                });
                app.world_mut().flush();
            });
        });
        let errors = errors.take_records();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostic = errors[0].managed().unwrap().diagnostics().next().unwrap();
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.matches_field("message", "用户选择 Gallery 设置"))
                .count(),
            1
        );
        assert!(logs.records().iter().all(|record| {
            record
                .fields
                .get("message")
                .is_none_or(|message| !message.contains("Gallery 设置已生效"))
        }));
    }
}
