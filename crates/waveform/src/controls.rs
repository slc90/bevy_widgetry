use crate::view_error::ViewResult;
use crate::view_error::{ViewErrors, ViewFailureState};
use crate::{Waveform, WaveformRuntime};
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::scene::{apply_scene, spawn_scene};
use bevy_widgetry_core::text::WidgetryText;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_job::JobFailure;
use bevy_widgetry_list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};

#[derive(Component)]
pub(crate) struct TimeChoices(pub Vec<u32>);

#[derive(Component)]
pub(crate) struct TimeBar;

#[derive(Component, Default)]
pub(crate) struct ControlsFailure(ViewFailureState);

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
    failure: ViewFailureState,
}

#[derive(Component, FromTemplate)]
struct ControlParts {
    root: Entity,
    model: Entity,
    combo: Entity,
}

impl TimeChoices {
    pub(crate) fn new(choices: Vec<u32>, duration: u32) -> Result<Self, JobFailure> {
        if !choices.is_empty()
            && (!choices.contains(&duration)
                || choices
                    .iter()
                    .enumerate()
                    .any(|(index, &value)| value == 0 || choices[..index].contains(&value)))
        {
            return Err(crate::error::request(
                crate::WaveformError::TimeChoices { choices, duration },
                None,
                "waveform.time.choices",
            ));
        }
        Ok(Self(choices))
    }
}

pub(crate) fn install(app: &mut App) {
    if let Err(error) = app.register_widgetry_combo_box::<RangeChoice>() {
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result { Err(error.into_bevy_error()) });
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
    let mut failure = ViewErrors::default();
    for root in roots {
        let result = initialize_root(world, root);
        let result = if let Some(mut state) = world.get_mut::<ControlsFailure>(root) {
            state
                .0
                .observe(result, root, "waveform.controls.initialize")
        } else {
            result.map_err(|error| error.into_bevy_error(Some(root), "waveform.controls"))
        };
        if let Err(error) = result {
            failure.push(error);
        }
    }
    failure.finish(world.commands())
}

fn initialize_root(world: &mut World, root: Entity) -> ViewResult {
    let time = world
        .get::<TimeChoices>(root)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::RangeChoices).at(root))?
        .0
        .clone();
    let amplitude = world
        .get::<crate::amplitude::AmplitudeChoices>(root)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::AmplitudeChoices).at(root))?
        .0
        .clone();
    let channels = world
        .get::<crate::channels::ChannelState>(root)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::ChannelChoices).at(root))?
        .choices
        .clone();
    let bar = world
        .get::<Children>(root)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<TimeBar>(child).is_some())
        })
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::ControlBar).at(root))?;
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
            .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::ControlBarNode).at(bar))?
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
) -> ViewResult<Entity> {
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
                    for &value in &choices { model.push(value).map_err(bevy_widgetry_list_view::ListFailure::into_bevy_error)?; }
                    Ok(model)
                })
            --
                #TimeCombo
                Name::new(match kind { ControlKind::Duration => "WaveformTimeRange", ControlKind::Amplitude => "WaveformAmplitudeRange", ControlKind::Channels => "WaveformChannelCount" })
                Node { width: px(112), flex_shrink: 0.0 }
            ]
        },
    )
    .map_err(|error| error.into_bevy_error())?;
    let parts = world.get::<ControlParts>(control_root).ok_or_else(|| {
        crate::view_error::missing(crate::WaveformPart::RangeControls).at(control_root)
    })?;
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
                template(move |_| Ok(ControlOwner { root, projected: None, kind, failure: default() }))
                Node { width: px(112), flex_shrink: 0.0 }
        },
    );
    if let Err(error) = result {
        world.despawn(control_root);
        return Err(error.into_bevy_error().into());
    }
    Ok(control_root)
}

fn project(world: &mut World) -> Result {
    let controls: Vec<_> = world
        .query::<&ControlParts>()
        .iter(world)
        .map(|parts| (parts.root, parts.model, parts.combo))
        .collect();
    let mut failure = ViewErrors::default();
    for (root, model, combo) in controls {
        let result = project_one(world, model, combo);
        let result = if let Some(mut state) = world.get_mut::<ControlOwner>(combo) {
            state
                .failure
                .observe(result, root, "waveform.controls.project")
        } else {
            result.map_err(|error| error.into_bevy_error(Some(root), "waveform.controls"))
        };
        if let Err(error) = result {
            failure.push(error);
        }
    }
    failure.finish(world.commands())
}

fn project_one(world: &mut World, model_entity: Entity, combo: Entity) -> ViewResult {
    let owner = world
        .get::<ControlOwner>(combo)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::RangeOwner).at(combo))?;
    let config = world
        .get::<WaveformRuntime>(owner.root)
        .ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::RangeRuntime).at(owner.root)
        })?
        .config();
    let model = world
        .get::<WidgetryListModel<RangeChoice>>(model_entity)
        .ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::RangeModel).at(model_entity)
        })?;
    let current = match owner.kind {
        ControlKind::Duration => RangeChoice::Duration(config.visible_duration_ms),
        ControlKind::Amplitude => RangeChoice::Amplitude(config.amplitude_range),
        ControlKind::Channels => {
            RangeChoice::Channels(crate::channels::channels_per_screen(world, owner.root)?)
        }
    };
    let id = (0..model.len())
        .find_map(|index| {
            (*model.get(index)? == current)
                .then(|| model.id(index))
                .flatten()
        })
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::RangeMissingFromChoices))?;
    if world.get::<WidgetryComboBox<RangeChoice>>(combo).is_none() {
        return Err(crate::view_error::missing(crate::WaveformPart::RangeCombo).at(combo));
    }
    if WidgetryComboBox::<RangeChoice>::selected_id(world, combo)? != Some(id) {
        world
            .get_mut::<ControlOwner>(combo)
            .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::RangeOwner).at(combo))?
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
        crate::error::request(
            crate::WaveformError::Query {
                entity: combo.source(),
                part: crate::WaveformPart::RangeModel,
                source: error,
            },
            Some(owner.root),
            "waveform.controls.choice",
        )
        .into_bevy_error()
    })?;
    let value = model.get_by_id(id).ok_or_else(|| {
        crate::view_error::missing(crate::WaveformPart::RangeChoiceNoLongerExists)
            .into_bevy_error(None, "waveform.on_range_choice")
    })?;
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
    fn a_broken_selector_does_not_recover_when_its_siblings_project_successfully() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        app.add_plugins((TransformPlugin, crate::WaveformRenderPlugin));
        app.set_error_handler(ErrorCapture::handler());
        let camera = spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
        let duration = crate::WaveformConfig::default().visible_duration_ms;
        let amplitude = crate::WaveformAmplitude::default();
        let root = app.world_mut().spawn_scene(bsn! {
            @Waveform {
                @time_ranges_ms: {vec![duration, duration * 2]},
                @amplitude_ranges: {vec![amplitude, crate::WaveformAmplitude { value: amplitude.value * 2.0, ..amplitude }]},
            }
            template(move |_| Ok(UiTargetCamera(camera)))
            Node { width: px(800), height: px(320) }
        }).unwrap().id();
        app.update();
        app.update();
        let model = app
            .world_mut()
            .query::<&ControlParts>()
            .iter(app.world())
            .find_map(|parts| {
                (parts.root == root
                    && matches!(
                        app.world().get::<ControlOwner>(parts.combo).unwrap().kind,
                        ControlKind::Duration
                    ))
                .then_some(parts.model)
            })
            .unwrap();
        app.world_mut().despawn(model);
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        let records = errors.take_records();
        let ids: std::collections::HashSet<_> = records
            .iter()
            .flat_map(|r| r.managed().into_iter().flat_map(|m| m.diagnostics()))
            .filter_map(|d| {
                let mut source = d.source();
                while let Some(error) = source {
                    if matches!(
                        error.downcast_ref::<crate::WaveformError>(),
                        Some(crate::WaveformError::Structure {
                            part: crate::WaveformPart::RangeModel,
                            entity: Some(_),
                        })
                    ) {
                        if let Some(crate::WaveformError::Structure { entity, .. }) =
                            error.downcast_ref::<crate::WaveformError>()
                        {
                            assert_eq!(*entity, Some(model));
                        }
                        return Some(d.origin().id());
                    }
                    source = error.source();
                }
                None
            })
            .collect();
        assert_eq!(ids.len(), 1);
        let id = *ids.iter().next().unwrap();
        assert_eq!(
            logs.diagnostic_records(id, DiagnosticPhase::Source).len(),
            1
        );
        assert!(!logs.records().iter().any(|r| {
            r.matches_field("error_id", &id.to_string()) && r.matches_field("phase", "recovered")
        }));
    }

    #[test]
    fn range_choice_preserves_the_missing_model_query_error() {
        use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
        let mut app = scene_app();
        app.add_plugins(bevy_widgetry_core::icon::WidgetryIconPlugin);
        app.set_error_handler(ErrorCapture::handler());
        app.add_observer(on_range_choice);
        let mut model = WidgetryListModel::default();
        let id = model.push(RangeChoice::Duration(1000)).unwrap();
        let source = app.world_mut().spawn(model).id();
        let combo = app.world_mut().spawn_scene(bsn! {
            @WidgetryComboBox::<RangeChoice> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_: &RangeChoice| bsn_list! { Text("range") })},
            }
            template(|_| Ok(ControlOwner { root: Entity::PLACEHOLDER, projected: None, kind: ControlKind::Duration, failure: default() }))
        }).unwrap().id();
        app.world_mut().despawn(source);
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        errors.run(|| {
            logs.run(|| {
                app.world_mut().trigger(ValueChange {
                    source: combo,
                    value: Some(id),
                    is_final: true,
                })
            })
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        let mut error = diagnostic.source();
        let mut typed = false;
        while let Some(source) = error {
            typed |= matches!(
                source.downcast_ref::<bevy::ecs::query::QueryEntityError>(),
                Some(bevy::ecs::query::QueryEntityError::NotSpawned(_))
            );
            error = source.source();
        }
        assert!(typed);
    }

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
