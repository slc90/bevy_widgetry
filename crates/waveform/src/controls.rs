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

#[derive(Clone, Copy, PartialEq)]
enum RangeChoice {
    Duration(u32),
    Amplitude(crate::WaveformAmplitude),
    Channels(crate::WaveformChannelCount),
}
impl RangeChoice {
    fn label(self) -> String {
        match self {
            Self::Duration(ms) => format!("{} s", ms as f64 / 1000.0),
            Self::Amplitude(value) => value.label(),
            Self::Channels(value) => value.label(),
        }
    }
}

#[derive(Clone, Copy)]
enum ControlKind {
    Duration,
    Amplitude,
    Channels,
}

#[derive(Component)]
struct ControlOwner {
    root: Entity,
    projected: Option<WidgetryListItemId>,
    kind: ControlKind,
}

#[derive(Component, FromTemplate)]
struct ControlParts {
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
    if let Err(error) = app.register_widgetry_combo_box::<RangeChoice>() {
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result { Err(error) });
    }
    app.add_systems(PostUpdate, initialize.before(WidgetryUiSystems::Build))
        .add_systems(PostUpdate, project.after(WidgetryUiSystems::Build))
        .add_observer(on_range_choice);
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
                |error| widgetry_error!(?root, %error, "Waveform 范围选择器初始化失败"),
                || widgetry_info!(?root, "Waveform 范围选择器初始化恢复正常"),
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
    let time = world
        .get::<TimeChoices>(root)
        .ok_or_else(|| logged_error("Waveform range choices missing"))?
        .0
        .clone();
    let amplitude = world
        .get::<crate::amplitude::AmplitudeChoices>(root)
        .ok_or_else(|| logged_error("Waveform amplitude choices missing"))?
        .0
        .clone();
    let channels = world
        .get::<crate::channels::ChannelState>(root)
        .ok_or_else(|| logged_error("Waveform channel choices missing"))?
        .choices
        .clone();
    let bar = world
        .get::<Children>(root)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<TimeBar>(child).is_some())
        })
        .ok_or_else(|| logged_error("Waveform control bar missing"))?;
    let mut created = Vec::new();
    for (kind, choices) in [
        (
            ControlKind::Duration,
            time.into_iter()
                .map(RangeChoice::Duration)
                .collect::<Vec<_>>(),
        ),
        (
            ControlKind::Amplitude,
            amplitude.into_iter().map(RangeChoice::Amplitude).collect(),
        ),
        (
            ControlKind::Channels,
            channels.into_iter().map(RangeChoice::Channels).collect(),
        ),
    ] {
        if choices.is_empty() {
            continue;
        }
        match initialize_control(world, root, bar, choices, kind) {
            Ok(control) => created.push(control),
            Err(error) => {
                for control in created {
                    world.despawn(control);
                }
                return Err(error);
            }
        }
    }
    if !created.is_empty() {
        world
            .get_mut::<Node>(bar)
            .ok_or_else(|| logged_error("Waveform control bar Node missing"))?
            .display = Display::Flex;
    }
    world.entity_mut(root).insert(ControlsInitialized);
    Ok(())
}

fn initialize_control(
    world: &mut World,
    root: Entity,
    bar: Entity,
    choices: Vec<RangeChoice>,
    kind: ControlKind,
) -> Result<Entity, BevyError> {
    let control_root = spawn_scene(
        world,
        bsn! {
            ControlParts { root: root, model: #TimeModel, combo: #TimeCombo }
            template(move |_| Ok(ChildOf(bar)))
            Node { display: Display::Flex, flex_shrink: 0.0,
                padding: UiRect::all(px(4)), column_gap: px(8), align_items: AlignItems::Center }
            Children [
                #TimeModel
                template(move |_| {
                    let mut model = WidgetryListModel::<RangeChoice>::default();
                    for &value in &choices { model.push(value)?; }
                    Ok(model)
                })
            --
                #TimeCombo
                Name::new(match kind { ControlKind::Duration => "WaveformTimeRange", ControlKind::Amplitude => "WaveformAmplitudeRange", ControlKind::Channels => "WaveformChannelCount" })
                Node { width: px(112), flex_shrink: 0.0 }
            ]
        },
    )
    .map_err(|error| logged_error(error.to_string()))?;
    let parts = world
        .get::<ControlParts>(control_root)
        .ok_or_else(|| logged_error("Waveform range controls missing"))?;
    let (model_entity, combo) = (parts.model, parts.combo);
    let result = apply_scene(
        &mut world.entity_mut(combo),
        bsn! {
                @WidgetryComboBox::<RangeChoice> {
                    @source: model_entity,
                    @renderer: {WidgetryListViewRenderer::new(|choice: &RangeChoice| {
                        let label = choice.label();
                        bsn_list! { Text(label) WidgetryText }
                    })}
                }
                template(move |_| Ok(ControlOwner { root, projected: None, kind }))
                Node { width: px(112), flex_shrink: 0.0 }
        },
    );
    if let Err(error) = result {
        world.despawn(control_root);
        return Err(logged_error(error.to_string()));
    }
    Ok(control_root)
}

fn project(world: &mut World) -> Result {
    let controls: Vec<_> = world
        .query::<&ControlParts>()
        .iter(world)
        .map(|parts| (parts.root, parts.model, parts.combo))
        .collect();
    let mut failure = None;
    for (root, model, combo) in controls {
        let result = project_one(world, model, combo);
        let result = if let Some(mut state) = world.get_mut::<ControlsFailure>(root) {
            state.0.observe(
                result,
                |error| widgetry_error!(?root, %error, "Waveform 范围选择投影失败"),
                || widgetry_info!(?root, "Waveform 范围选择投影恢复正常"),
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
        .get::<ControlOwner>(combo)
        .ok_or_else(|| logged_error("Waveform range owner missing"))?;
    let config = world
        .get::<WaveformRuntime>(owner.root)
        .ok_or_else(|| logged_error("Waveform range runtime missing"))?
        .config();
    let model = world
        .get::<WidgetryListModel<RangeChoice>>(model_entity)
        .ok_or_else(|| logged_error("Waveform range model missing"))?;
    let current = match owner.kind {
        ControlKind::Duration => RangeChoice::Duration(config.visible_duration_ms),
        ControlKind::Amplitude => RangeChoice::Amplitude(config.amplitude_range),
        ControlKind::Channels => {
            RangeChoice::Channels(Waveform::channels_per_screen(world, owner.root)?)
        }
    };
    let id = (0..model.len())
        .find_map(|index| {
            (*model.get(index)? == current)
                .then(|| model.id(index))
                .flatten()
        })
        .ok_or_else(|| logged_error("Waveform range missing from choices"))?;
    if WidgetryComboBox::<RangeChoice>::selected_id(world, combo)? != Some(id) {
        world
            .get_mut::<ControlOwner>(combo)
            .ok_or_else(|| logged_error("Waveform range owner missing"))?
            .projected = Some(id);
        WidgetryComboBox::<RangeChoice>::select_id(&mut world.commands(), combo, id);
    }
    Ok(())
}

fn on_range_choice(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    mut controls: Query<(&WidgetryComboBox<RangeChoice>, &mut ControlOwner)>,
    models: Query<&WidgetryListModel<RangeChoice>>,
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
        widgetry_error!(%error, "Waveform 范围选择 Model 缺失");
        logged_error(error.to_string())
    })?;
    let value = model
        .get_by_id(id)
        .ok_or_else(|| logged_error("Waveform range choice no longer exists"))?;
    match *value {
        RangeChoice::Duration(value) => {
            Waveform::set_visible_duration(&mut commands, owner.root, value)
        }
        RangeChoice::Amplitude(value) => Waveform::set_amplitude(&mut commands, owner.root, value),
        RangeChoice::Channels(value) => {
            Waveform::set_channels_per_screen(&mut commands, owner.root, value)
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};

    #[test]
    fn amplitude_selector_shares_committed_state_and_rejects_unlisted_values() {
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        app.add_plugins((TransformPlugin, crate::WaveformRenderPlugin));
        let camera = spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
        let uv = crate::WaveformAmplitude {
            value: 100.0,
            unit: crate::WaveformUnit::Microvolts,
        };
        let mv = crate::WaveformAmplitude {
            value: 1.0,
            unit: crate::WaveformUnit::Millivolts,
        };
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @Waveform { @config: {crate::WaveformConfig { amplitude_range: uv, ..default() }},
                    @time_ranges_ms: {vec![1000, 5000]}, @amplitude_ranges: {vec![uv, mv]},
                    @channel_counts: {vec![crate::WaveformChannelCount::Count(8), crate::WaveformChannelCount::All]} }
                template(move |_| Ok(UiTargetCamera(camera)))
                Node { width: px(600), height: px(300) }
            })
            .unwrap()
            .id();
        for _ in 0..3 {
            app.update();
        }
        let (combo, model) = app
            .world_mut()
            .query::<&ControlParts>()
            .iter(app.world())
            .find(|parts| {
                matches!(
                    app.world().get::<ControlOwner>(parts.combo).unwrap().kind,
                    ControlKind::Amplitude
                )
            })
            .map(|parts| (parts.combo, parts.model))
            .unwrap();
        let id = app
            .world()
            .get::<WidgetryListModel<RangeChoice>>(model)
            .unwrap()
            .id(0)
            .unwrap();
        assert_eq!(
            WidgetryComboBox::<RangeChoice>::selected_id(app.world(), combo).unwrap(),
            Some(id)
        );
        assert!(
            Waveform::set_amplitude_in_world(
                app.world_mut(),
                root,
                crate::WaveformAmplitude::default()
            )
            .is_err()
        );
        WidgetryComboBox::<RangeChoice>::select_index(&mut app.world_mut().commands(), combo, 1);
        app.world_mut().flush();
        assert_eq!(
            app.world()
                .get::<WaveformRuntime>(root)
                .unwrap()
                .config()
                .amplitude_range,
            mv
        );
        Waveform::set_amplitude_in_world(app.world_mut(), root, uv).unwrap();
        for _ in 0..2 {
            app.update();
        }
        assert_eq!(
            WidgetryComboBox::<RangeChoice>::selected_id(app.world(), combo).unwrap(),
            Some(id)
        );
        let channel_combo = app
            .world_mut()
            .query::<&ControlParts>()
            .iter(app.world())
            .find(|parts| {
                matches!(
                    app.world().get::<ControlOwner>(parts.combo).unwrap().kind,
                    ControlKind::Channels
                )
            })
            .unwrap()
            .combo;
        WidgetryComboBox::<RangeChoice>::select_index(
            &mut app.world_mut().commands(),
            channel_combo,
            0,
        );
        app.world_mut().flush();
        assert_eq!(
            Waveform::channels_per_screen(app.world(), root).unwrap(),
            crate::WaveformChannelCount::Count(8)
        );
        let controls: Vec<_> = app
            .world_mut()
            .query::<&ControlParts>()
            .iter(app.world())
            .map(|parts| parts.combo)
            .collect();
        assert_eq!(controls.len(), 3);
        let a = app
            .world()
            .get::<UiGlobalTransform>(controls[0])
            .unwrap()
            .translation;
        let b = app
            .world()
            .get::<UiGlobalTransform>(controls[1])
            .unwrap()
            .translation;
        assert!(a.distance(b) >= 112.0);
        app.world_mut().despawn(root);
        app.update();
        assert!(app.world().get_entity(combo).is_err());
        assert!(app.world().get_entity(model).is_err());
    }

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
            .query::<&ControlParts>()
            .single(app.world())
            .unwrap();
        let (combo, model) = (parts.combo, parts.model);
        let id = app
            .world()
            .get::<WidgetryListModel<RangeChoice>>(model)
            .unwrap()
            .id(0)
            .unwrap();
        assert_eq!(
            WidgetryComboBox::<RangeChoice>::selected_id(app.world(), combo).unwrap(),
            Some(id)
        );
        Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap();
        for _ in 0..2 {
            app.update();
        }
        let id = app
            .world()
            .get::<WidgetryListModel<RangeChoice>>(model)
            .unwrap()
            .id(2)
            .unwrap();
        assert_eq!(
            WidgetryComboBox::<RangeChoice>::selected_id(app.world(), combo).unwrap(),
            Some(id)
        );
        WidgetryComboBox::<RangeChoice>::select_index(&mut app.world_mut().commands(), combo, 1);
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
            .query::<&ControlParts>()
            .single(app.world())
            .unwrap();
        assert_eq!((parts.combo, parts.model), (combo, model));
        let plot = app
            .world_mut()
            .query_filtered::<&ComputedNode, With<crate::view::WaveformViewport>>()
            .single(app.world())
            .unwrap();
        assert_eq!(plot.size().x, 468.0);
        assert!(plot.size().y > 200.0 && plot.size().y < 300.0);
        app.world_mut().despawn(root);
        app.update();
        assert!(app.world().get_entity(combo).is_err());
        assert!(app.world().get_entity(model).is_err());
    }
}
