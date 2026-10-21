use crate::{Waveform, WaveformRuntime};
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_core::scene::{apply_scene, spawn_scene};
use bevy_widgetry_core::text::WidgetryText;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};

#[derive(Component)]
pub(crate) struct TimeChoices(pub Vec<u32>);

#[derive(Component)]
pub(crate) struct TimeBar;

#[derive(Component, Default)]
pub(crate) struct ControlsFailure(FailureState);

#[derive(Component)]
struct ControlsInitialized;

#[derive(Clone, Copy)]
struct DurationChoice(u32);

#[derive(Component)]
struct TimeOwner {
    root: Entity,
    projected: Option<WidgetryListItemId>,
}

#[derive(Component, FromTemplate)]
struct TimeControlParts {
    root: Entity,
    model: Entity,
    combo: Entity,
}

impl TimeChoices {
    pub(crate) fn new(choices: Vec<u32>, duration: u32) -> Result<Self, BevyError> {
        if !choices.is_empty()
            && (!choices.contains(&duration)
                || choices
                    .iter()
                    .enumerate()
                    .any(|(index, &value)| value == 0 || choices[..index].contains(&value)))
        {
            widgetry_error!(?choices, duration, "Waveform 时间范围候选项无效");
            return Err(logged_error("Invalid Waveform time range choices"));
        }
        Ok(Self(choices))
    }
}

pub(crate) fn install(app: &mut App) {
    if let Err(error) = app.register_widgetry_combo_box::<DurationChoice>() {
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result { Err(error) });
    }
    app.add_systems(PostUpdate, initialize.before(WidgetryUiSystems::Build))
        .add_systems(PostUpdate, project.after(WidgetryUiSystems::Build))
        .add_observer(on_duration_choice);
}

fn initialize(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query_filtered::<Entity, (With<Waveform>, Without<ControlsInitialized>)>()
        .iter(world)
        .collect();
    let mut failure = None;
    for root in roots {
        let result = initialize_root(world, root);
        let result = if let Some(mut state) = world.get_mut::<ControlsFailure>(root) {
            state.0.observe(
                result,
                |error| widgetry_error!(?root, %error, "Waveform 时间选择器初始化失败"),
                || widgetry_info!(?root, "Waveform 时间选择器初始化恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn initialize_root(world: &mut World, root: Entity) -> Result {
    let choices = world
        .get::<TimeChoices>(root)
        .ok_or_else(|| logged_error("Waveform time choices missing"))?
        .0
        .clone();
    if choices.is_empty() {
        world.entity_mut(root).insert(ControlsInitialized);
        return Ok(());
    }
    let bar = world
        .get::<Children>(root)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<TimeBar>(child).is_some())
        })
        .ok_or_else(|| logged_error("Waveform time bar missing"))?;
    let control_root = spawn_scene(
        world,
        bsn! {
            TimeControlParts { root: root, model: #TimeModel, combo: #TimeCombo }
            template(move |_| Ok(ChildOf(bar)))
            Node { display: Display::Flex, width: percent(100), flex_shrink: 0.0,
                padding: UiRect::all(px(4)), column_gap: px(8), align_items: AlignItems::Center }
            Children [
                #TimeModel
                template(move |_| {
                    let mut model = WidgetryListModel::<DurationChoice>::default();
                    for &value in &choices { model.push(DurationChoice(value))?; }
                    Ok(model)
                })
            --
                #TimeCombo
                Name::new("WaveformTimeRange")
                Node { width: px(112), flex_shrink: 0.0 }
            ]
        },
    )
    .map_err(|error| logged_error(error.to_string()))?;
    let parts = world
        .get::<TimeControlParts>(control_root)
        .ok_or_else(|| logged_error("Waveform time controls missing"))?;
    let (model_entity, combo) = (parts.model, parts.combo);
    let result = apply_scene(
        &mut world.entity_mut(combo),
        bsn! {
                @WidgetryComboBox::<DurationChoice> {
                    @source: model_entity,
                    @renderer: {WidgetryListViewRenderer::new(|choice: &DurationChoice| {
                        let label = format!("{} s", choice.0 as f64 / 1000.0);
                        bsn_list! { Text(label) WidgetryText }
                    })}
                }
                template(move |_| Ok(TimeOwner { root, projected: None }))
                Node { width: px(112), flex_shrink: 0.0 }
        },
    );
    if let Err(error) = result {
        world.despawn(control_root);
        return Err(logged_error(error.to_string()));
    }
    world
        .get_mut::<Node>(bar)
        .ok_or_else(|| logged_error("Waveform time bar Node missing"))?
        .display = Display::Flex;
    world.entity_mut(root).insert(ControlsInitialized);
    Ok(())
}

fn project(world: &mut World) -> Result {
    let controls: Vec<_> = world
        .query::<&TimeControlParts>()
        .iter(world)
        .map(|parts| (parts.root, parts.model, parts.combo))
        .collect();
    let mut failure = None;
    for (root, model, combo) in controls {
        let result = project_one(world, model, combo);
        let result = if let Some(mut state) = world.get_mut::<ControlsFailure>(root) {
            state.0.observe(
                result,
                |error| widgetry_error!(?root, %error, "Waveform 时间选择投影失败"),
                || widgetry_info!(?root, "Waveform 时间选择投影恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn project_one(world: &mut World, model_entity: Entity, combo: Entity) -> Result {
    let owner = world
        .get::<TimeOwner>(combo)
        .ok_or_else(|| logged_error("Waveform time owner missing"))?;
    let duration = world
        .get::<WaveformRuntime>(owner.root)
        .ok_or_else(|| logged_error("Waveform time runtime missing"))?
        .config()
        .visible_duration_ms;
    let model = world
        .get::<WidgetryListModel<DurationChoice>>(model_entity)
        .ok_or_else(|| logged_error("Waveform time model missing"))?;
    let id = (0..model.len())
        .find_map(|index| {
            (model.get(index)?.0 == duration)
                .then(|| model.id(index))
                .flatten()
        })
        .ok_or_else(|| logged_error("Waveform duration missing from choices"))?;
    if WidgetryComboBox::<DurationChoice>::selected_id(world, combo)? != Some(id) {
        world
            .get_mut::<TimeOwner>(combo)
            .ok_or_else(|| logged_error("Waveform time owner missing"))?
            .projected = Some(id);
        WidgetryComboBox::<DurationChoice>::select_id(&mut world.commands(), combo, id);
    }
    Ok(())
}

fn on_duration_choice(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    mut controls: Query<(&WidgetryComboBox<DurationChoice>, &mut TimeOwner)>,
    models: Query<&WidgetryListModel<DurationChoice>>,
    mut commands: Commands,
) -> Result {
    let Ok((combo, mut owner)) = controls.get_mut(event.source) else {
        return Ok(());
    };
    if owner.projected == event.value && event.value.is_some() {
        owner.projected = None;
        return Ok(());
    }
    let Some(id) = event.value else {
        return Ok(());
    };
    let model = models.get(combo.source()).map_err(|error| {
        widgetry_error!(%error, "Waveform 时间选择 Model 缺失");
        logged_error(error.to_string())
    })?;
    let value = model
        .get_by_id(id)
        .ok_or_else(|| logged_error("Waveform duration choice no longer exists"))?
        .0;
    Waveform::set_visible_duration(&mut commands, owner.root, value);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};

    #[test]
    fn time_selector_projects_api_and_commits_selection_without_rebuilding_root() {
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        app.add_plugins((TransformPlugin, crate::WaveformRenderPlugin));
        let camera = spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @Waveform { @time_ranges_ms: {vec![1000, 10000, 30000]} }
                template(move |_| Ok(UiTargetCamera(camera)))
                Node { width: px(600), height: px(300) }
            })
            .unwrap()
            .id();
        for _ in 0..3 {
            app.update();
        }
        let parts = app
            .world_mut()
            .query::<&TimeControlParts>()
            .single(app.world())
            .unwrap();
        let (combo, model) = (parts.combo, parts.model);
        let id = app
            .world()
            .get::<WidgetryListModel<DurationChoice>>(model)
            .unwrap()
            .id(0)
            .unwrap();
        assert_eq!(
            WidgetryComboBox::<DurationChoice>::selected_id(app.world(), combo).unwrap(),
            Some(id)
        );
        Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap();
        for _ in 0..2 {
            app.update();
        }
        let id = app
            .world()
            .get::<WidgetryListModel<DurationChoice>>(model)
            .unwrap()
            .id(2)
            .unwrap();
        assert_eq!(
            WidgetryComboBox::<DurationChoice>::selected_id(app.world(), combo).unwrap(),
            Some(id)
        );
        WidgetryComboBox::<DurationChoice>::select_index(&mut app.world_mut().commands(), combo, 1);
        app.world_mut().flush();
        assert_eq!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .config()
                .visible_duration_ms,
            10000
        );
        app.update();
        let parts = app
            .world_mut()
            .query::<&TimeControlParts>()
            .single(app.world())
            .unwrap();
        assert_eq!((parts.combo, parts.model), (combo, model));
        let plot = app
            .world_mut()
            .query_filtered::<&ComputedNode, With<crate::view::WaveformViewport>>()
            .single(app.world())
            .unwrap();
        assert_eq!(plot.size().x, 600.0);
        assert!(plot.size().y > 200.0 && plot.size().y < 300.0);
        app.world_mut().despawn(root);
        app.update();
        assert!(app.world().get_entity(combo).is_err());
        assert!(app.world().get_entity(model).is_err());
    }
}
