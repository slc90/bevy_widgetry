use crate::assets::GalleryIcon;
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry::icon::WidgetryIcon;
use bevy_widgetry::list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};
use bevy_widgetry::style::z_index;
use bevy_widgetry::theme::{WidgetryThemeChanged, WidgetryThemeMode};

#[derive(Component)]
struct GalleryTitle;

#[derive(Component)]
struct ThemeComboBox;

#[derive(Component)]
struct ScaleFactorComboBox {
    target_window: Entity,
}

pub(crate) fn install(app: &mut App) -> Result {
    app.register_widgetry_combo_box::<WidgetryThemeMode>()?
        .register_widgetry_combo_box::<f32>()?
        .add_observer(on_theme_combo_box_changed)
        .add_observer(on_scale_factor_combo_box_changed)
        .add_observer(sync_theme_combo_box)
        .add_systems(PostStartup, initialize_theme_selection);
    Ok(())
}

pub(crate) fn scene(
    commands: &mut Commands,
    target_window: Entity,
) -> Result<impl Scene, BevyError> {
    let theme = theme_selector(commands)?;
    let scale = scale_factor_selector(commands, target_window)?;
    Ok(bsn! {
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
                Node { width: px(16), height: px(16) }--
                Text("Widget Gallery") bevy_widgetry::text::WidgetryText template(|_| Ok(Pickable::IGNORE))
            ]--
            template(|_| Ok(Pickable::IGNORE))
            Node { height: percent(100), align_items: AlignItems::Center, column_gap: px(8) }
            GlobalZIndex({z_index::LOCAL_OVERLAY})
            Children [@{scale}-- @{theme}]
        ]
    })
}

fn theme_selector(commands: &mut Commands) -> Result<impl Scene, BevyError> {
    let mut model = WidgetryListModel::default();
    for mode in [
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::Light,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        model.push(mode)?;
    }
    let source = commands.spawn(model).id();
    Ok(bsn! {
        #ThemeComboBox
        @WidgetryComboBox::<WidgetryThemeMode> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, mode: &WidgetryThemeMode| bsn_list!{Text({match mode {
                WidgetryThemeMode::Dark => "Dark",
                WidgetryThemeMode::Light => "Light",
                WidgetryThemeMode::PinkDream => "Pink Dream",
                WidgetryThemeMode::KamuriViolet => "Kamuri Violet",
            }}) bevy_widgetry::text::WidgetryText})},
        }
        template(|_| Ok(ThemeComboBox))
    })
}

fn scale_factor_selector(
    commands: &mut Commands,
    target_window: Entity,
) -> Result<impl Scene, BevyError> {
    let mut model = WidgetryListModel::default();
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        model.push(scale)?;
    }
    let source = commands.spawn(model).id();
    Ok(bsn! {
        #ScaleFactorComboBox
        @WidgetryComboBox::<f32> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, scale: &f32| bsn_list!{Text({format!("{scale:?}")}) bevy_widgetry::text::WidgetryText})},
        }
        template(move |_| Ok(ScaleFactorComboBox { target_window }))
        Node { width: px(100) }
    })
}

fn on_scale_factor_combo_box_changed(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    combos: Query<(&WidgetryComboBox<f32>, &ScaleFactorComboBox)>,
    models: Query<&WidgetryListModel<f32>>,
    mut windows: Query<&mut Window>,
) -> Result {
    let Ok((combo, scale_combo)) = combos.get(event.source) else {
        return Ok(());
    };
    let Some(selected) = event.value else {
        return Ok(());
    };
    let model = models.get(combo.source()).map_err(|error| {
        error!(source = ?combo.source(), %error, "scale_factor Model 缺失");
        BevyError::error(error)
    })?;
    let scale = *model.get_by_id(selected).ok_or_else(|| {
        error!(?selected, "scale_factor selection 已失效");
        BevyError::error("scale_factor selection 已失效")
    })?;
    let mut window = windows
        .get_mut(scale_combo.target_window)
        .map_err(|error| {
            error!(window = ?scale_combo.target_window, %error, "scale_factor 目标窗口缺失");
            BevyError::error(error)
        })?;
    if window.resolution.scale_factor_override() != Some(scale) {
        window.resolution.set_scale_factor_override(Some(scale));
        info!(scale_factor = scale, "切换窗口缩放");
    }
    Ok(())
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
    select_theme(event.mode, &combos, &models, &mut commands);
}

fn initialize_theme_selection(
    theme_mode: Res<WidgetryThemeMode>,
    combos: Query<(Entity, &WidgetryComboBox<WidgetryThemeMode>), With<ThemeComboBox>>,
    models: Query<&WidgetryListModel<WidgetryThemeMode>>,
    mut commands: Commands,
) {
    select_theme(*theme_mode, &combos, &models, &mut commands);
}

fn select_theme(
    mode: WidgetryThemeMode,
    combos: &Query<(Entity, &WidgetryComboBox<WidgetryThemeMode>), With<ThemeComboBox>>,
    models: &Query<&WidgetryListModel<WidgetryThemeMode>>,
    commands: &mut Commands,
) {
    for (entity, combo) in combos {
        if let Ok(model) = models.get(combo.source())
            && let Some(index) = (0..model.len()).find(|index| model.get(*index) == Some(&mode))
            && let Some(id) = model.id(index)
        {
            WidgetryComboBox::<WidgetryThemeMode>::set_selected(commands, entity, id);
        }
    }
}
