use super::{Playback, WaveformPlaybackAction, WaveformPlaybackRequested};
use crate::{Waveform, WaveformRuntime};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::{
    diagnostics::FailureState,
    icon::{WidgetryIcon, WidgetryIconPlugin},
    scene::{apply_scene, logged_error, spawn_scene},
    text::WidgetryText,
    ui::WidgetryUiSystems,
};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};

#[derive(Component, Default)]
pub(crate) struct PlaybackUi {
    bar: Option<Entity>,
    generation: Option<u64>,
    icon_playing: Option<bool>,
    init_failure: FailureState,
    project_failure: FailureState,
}

#[derive(Clone, Copy)]
struct Speed(f64);

#[derive(Component)]
struct Owner {
    root: Entity,
    generation: u64,
}

#[derive(Component)]
struct SpeedOwner {
    root: Entity,
    generation: u64,
    projected: Option<WidgetryListItemId>,
}

#[derive(Component, FromTemplate)]
struct Parts {
    model: Entity,
    combo: Entity,
    icon: Entity,
    time: Entity,
}

pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<WidgetryAssetPlugin>() {
        app.add_plugins(WidgetryAssetPlugin);
    }
    if !app.is_plugin_added::<WidgetryButtonPlugin>() {
        app.add_plugins(WidgetryButtonPlugin);
    }
    if !app.is_plugin_added::<WidgetryIconPlugin>() {
        app.add_plugins(WidgetryIconPlugin);
    }
    if let Err(error) = app.register_widgetry_combo_box::<Speed>() {
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result { Err(error) });
    }
    app.add_systems(PostUpdate, initialize.before(WidgetryUiSystems::Build))
        .add_systems(PostUpdate, project.after(WidgetryUiSystems::Build))
        .add_observer(activate)
        .add_observer(speed_changed);
}

fn observe(world: &mut World, root: Entity, result: Result, initializing: bool) -> Result {
    let mut ui = world
        .get_mut::<PlaybackUi>(root)
        .ok_or_else(|| logged_error("Waveform playback UI state missing"))?;
    let failure = if initializing {
        &mut ui.init_failure
    } else {
        &mut ui.project_failure
    };
    failure.observe(
        result,
        |error| widgetry_error!(?root, %error, "Waveform 回放控制栏同步失败"),
        || widgetry_info!(?root, "Waveform 回放控制栏恢复正常"),
    )
}

fn initialize(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<Waveform>>()
        .iter(world)
        .collect();
    let mut failed = None;
    for root in roots {
        let result = initialize_one(world, root);
        if let Err(error) = observe(world, root, result, true)
            && failed.is_none()
        {
            failed = Some(error);
        }
    }
    failed.map_or(Ok(()), Err)
}

fn initialize_one(world: &mut World, root: Entity) -> Result {
    let generation = Waveform::playback(world, root)?.map(|state| state.session_generation);
    let ui = world
        .get::<PlaybackUi>(root)
        .ok_or_else(|| logged_error("Waveform playback UI state missing"))?;
    if generation.is_some() && ui.generation == generation && ui.bar.is_some() {
        return Ok(());
    }
    if let Some(bar) = ui.bar {
        world.despawn(bar);
    }
    let mut ui = world
        .get_mut::<PlaybackUi>(root)
        .ok_or_else(|| logged_error("Waveform playback UI state missing"))?;
    ui.bar = None;
    ui.generation = None;
    ui.icon_playing = None;
    let Some(generation) = generation else {
        return Ok(());
    };
    let speeds = world
        .get::<Playback>(root)
        .ok_or_else(|| logged_error("Waveform playback missing"))?
        .speeds
        .clone();
    let bar = spawn_scene(world, bsn! {
        Parts { model: #SpeedModel, combo: #SpeedCombo, icon: #PlayIcon, time: #Time }
        ChildOf(root) Name::new("WaveformReplayBar")
        Node { width: percent(100), height: px(40), flex_shrink: 0.0, align_items: AlignItems::Center, column_gap: px(8), padding: UiRect::all(px(4)) }
        Children [
            @WidgetryButton Name::new("WaveformReplayPlay") template(move |_| Ok(Owner { root, generation }))
            Node { width: px(32), height: px(32), padding: UiRect::all(px(4)), flex_shrink: 0.0 }
            Children [#PlayIcon @WidgetryIcon { @path: {BuiltinIcon::PlaybackPlay.path()}, @max_size: {Some(UVec2::splat(24))} } Node { width: px(24), height: px(24) }]
        --
            Name::new("WaveformReplayProgressSlot") Node { min_width: px(0), flex_grow: 1.0 }
        --
            #Time Text("") WidgetryText Name::new("WaveformReplayTime") Node { flex_shrink: 0.0 }
        --
            #SpeedModel template(move |_| {
                let mut model = WidgetryListModel::default();
                for speed in &speeds { model.push(Speed(*speed))?; }
                Ok(model)
            })
        --
            #SpeedCombo Name::new("WaveformReplaySpeed") Node { width: px(88), flex_shrink: 0.0 }
        ]
    }).map_err(|error| logged_error(error.to_string()))?;
    let parts = world
        .get::<Parts>(bar)
        .ok_or_else(|| logged_error("Waveform playback parts missing"))?;
    let (model, combo) = (parts.model, parts.combo);
    let result = apply_scene(&mut world.entity_mut(combo), bsn! {
        @WidgetryComboBox::<Speed> { @source: model, @renderer: {WidgetryListViewRenderer::new(|speed: &Speed| { let label = format!("{}×", speed.0); bsn_list! { Text(label) WidgetryText } })} }
        template(move |_| Ok(SpeedOwner { root, generation, projected: None }))
        Node { width: px(88), flex_shrink: 0.0 }
    }).map_err(|error| logged_error(error.to_string()));
    if let Err(error) = result {
        world.despawn(bar);
        return Err(error);
    }
    let mut ui = world
        .get_mut::<PlaybackUi>(root)
        .ok_or_else(|| logged_error("Waveform playback UI state missing"))?;
    ui.bar = Some(bar);
    ui.generation = Some(generation);
    Ok(())
}

fn project(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query::<(Entity, &PlaybackUi)>()
        .iter(world)
        .filter_map(|(root, ui)| ui.bar.map(|bar| (root, bar)))
        .collect();
    let mut failed = None;
    for (root, bar) in roots {
        let result = project_one(world, root, bar);
        if let Err(error) = observe(world, root, result, false)
            && failed.is_none()
        {
            failed = Some(error);
        }
    }
    failed.map_or(Ok(()), Err)
}

fn project_one(world: &mut World, root: Entity, bar: Entity) -> Result {
    let Some(playback) = Waveform::playback(world, root)? else {
        return Ok(());
    };
    let parts = world
        .get::<Parts>(bar)
        .ok_or_else(|| logged_error("Waveform playback parts missing"))?;
    let (model, combo, icon, time) = (parts.model, parts.combo, parts.icon, parts.time);
    if world
        .get::<PlaybackUi>(root)
        .is_none_or(|ui| ui.icon_playing != Some(playback.playing))
    {
        WidgetryIcon::set_svg_in_world(
            world,
            icon,
            if playback.playing {
                BuiltinIcon::PlaybackPause
            } else {
                BuiltinIcon::PlaybackPlay
            }
            .path(),
        )?;
        world
            .get_mut::<PlaybackUi>(root)
            .ok_or_else(|| logged_error("Waveform playback UI state missing"))?
            .icon_playing = Some(playback.playing);
    }
    let total = world
        .get::<WaveformRuntime>(root)
        .and_then(WaveformRuntime::metadata)
        .ok_or_else(|| logged_error("Waveform metadata missing"))?
        .total_duration()?
        .ok_or_else(|| logged_error("Replay duration missing"))?;
    let label = format!(
        "{} / {}",
        format_time(playback.position),
        format_time(total)
    );
    let mut text = world
        .get_mut::<Text>(time)
        .ok_or_else(|| logged_error("Replay time Text missing"))?;
    if text.0 != label {
        text.0 = label;
    }
    let model = world
        .get::<WidgetryListModel<Speed>>(model)
        .ok_or_else(|| logged_error("Replay speed model missing"))?;
    let id = (0..model.len())
        .find_map(|index| {
            (model.get(index)?.0 == playback.speed)
                .then(|| model.id(index))
                .flatten()
        })
        .ok_or_else(|| logged_error("Replay speed not in choices"))?;
    if WidgetryComboBox::<Speed>::selected_id(world, combo)? != Some(id) {
        world
            .get_mut::<SpeedOwner>(combo)
            .ok_or_else(|| logged_error("Replay speed owner missing"))?
            .projected = Some(id);
        WidgetryComboBox::<Speed>::select_id(&mut world.commands(), combo, id);
    }
    Ok(())
}

fn format_time(value: std::time::Duration) -> String {
    let seconds = value.as_secs();
    if seconds < 3600 {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    } else {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    }
}

fn activate(
    event: On<Activate>,
    owners: Query<(&Owner, Has<InteractionDisabled>)>,
    runtimes: Query<&WaveformRuntime>,
    mut commands: Commands,
) {
    let Ok((owner, disabled)) = owners.get(event.entity) else {
        return;
    };
    if disabled {
        return;
    }
    let root = owner.root;
    let Ok(runtime) = runtimes.get(root) else {
        return;
    };
    let generation = owner.generation;
    if runtime.session_generation() != generation {
        return;
    }
    commands.queue(move |world: &mut World| -> Result {
        if world.get::<Waveform>(root).is_none() {
            return Ok(());
        }
        if let Some(state) = Waveform::playback(world, root)?
            && state.session_generation == generation
        {
            world.trigger(WaveformPlaybackRequested {
                entity: root,
                session_generation: generation,
                action: WaveformPlaybackAction::Playing(!state.playing),
            });
        }
        Ok(())
    });
}

fn speed_changed(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    mut combos: Query<(
        &WidgetryComboBox<Speed>,
        &mut SpeedOwner,
        Has<InteractionDisabled>,
    )>,
    models: Query<&WidgetryListModel<Speed>>,
    runtimes: Query<&WaveformRuntime>,
    mut commands: Commands,
) -> Result {
    let Ok((combo, mut owner, disabled)) = combos.get_mut(event.source) else {
        return Ok(());
    };
    if owner.projected == event.value && event.value.is_some() {
        owner.projected = None;
        return Ok(());
    }
    if disabled {
        return Ok(());
    }
    let Some(id) = event.value else {
        return Ok(());
    };
    let speed = models
        .get(combo.source())?
        .get_by_id(id)
        .ok_or_else(|| logged_error("Replay speed choice missing"))?
        .0;
    let root = owner.root;
    let generation = owner.generation;
    if runtimes.get(root)?.session_generation() != generation {
        return Ok(());
    }
    commands.trigger(WaveformPlaybackRequested {
        entity: root,
        session_generation: generation,
        action: WaveformPlaybackAction::Speed(speed),
    });
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{
        PlanarBuffer, WaveformMetadata, WaveformReadError, WaveformReadRequest, WaveformReadStatus,
        WaveformSessionKind, WaveformSource, WaveformUnit,
    };
    use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};
    use std::sync::{Arc, Mutex};

    struct Source(WaveformSessionKind);
    impl WaveformSource for Source {
        fn metadata(&self) -> std::result::Result<WaveformMetadata, WaveformReadError> {
            Ok(WaveformMetadata {
                channel_count: 1,
                channel_labels: vec!["A".into()],
                sample_rate: 1000,
                unit: WaveformUnit::Volts,
                kind: self.0,
            })
        }
        fn read(
            &self,
            request: &WaveformReadRequest,
            out: &mut PlanarBuffer,
        ) -> std::result::Result<WaveformReadStatus, WaveformReadError> {
            for channel in out.channels_mut() {
                channel.resize((request.range.end - request.range.start) as usize, 0.1);
            }
            Ok(WaveformReadStatus::Ready(request.clone()))
        }
    }

    #[test]
    fn replay_only_bar_projects_committed_state_and_rejected_intents_revert() {
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        app.add_plugins((TransformPlugin, crate::WaveformRenderPlugin));
        let camera = spawn_ui_camera(&mut app, UVec2::new(800, 400), 1.0);
        let root = app.world_mut().spawn_scene(bsn! {
            @Waveform { @source: {Some(Arc::new(Source(WaveformSessionKind::Live)) as Arc<dyn WaveformSource>)} }
            UiTargetCamera(camera) Node { width: px(780), height: px(320) }
        }).unwrap().id();
        for _ in 0..4 {
            app.update();
        }
        assert!(app.world().get::<PlaybackUi>(root).unwrap().bar.is_none());
        Waveform::set_session_in_world(
            app.world_mut(),
            root,
            Arc::new(Source(WaveformSessionKind::Replay {
                total_frames: 100_000,
            })),
        )
        .unwrap();
        for _ in 0..4 {
            app.update();
        }
        let bar = app.world().get::<PlaybackUi>(root).unwrap().bar.unwrap();
        let parts = app.world().get::<Parts>(bar).unwrap();
        let (model, combo, time) = (parts.model, parts.combo, parts.time);
        assert_eq!(app.world().get::<Text>(time).unwrap().0, "00:00 / 01:40");
        let button = app
            .world_mut()
            .query::<(Entity, &Owner)>()
            .iter(app.world())
            .find(|(_, owner)| owner.root == root)
            .unwrap()
            .0;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        app.world_mut()
            .add_observer(move |event: On<WaveformPlaybackRequested>| {
                seen.lock().unwrap().push(event.action);
            });
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert!(
            !Waveform::playback(app.world(), root)
                .unwrap()
                .unwrap()
                .playing
        );
        assert_eq!(
            requests.lock().unwrap().as_slice(),
            &[WaveformPlaybackAction::Playing(true)]
        );
        WidgetryComboBox::<Speed>::select_index(&mut app.world_mut().commands(), combo, 4);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            requests.lock().unwrap().as_slice(),
            &[
                WaveformPlaybackAction::Playing(true),
                WaveformPlaybackAction::Speed(2.0)
            ]
        );
        let default_id = app
            .world()
            .get::<WidgetryListModel<Speed>>(model)
            .unwrap()
            .id(2)
            .unwrap();
        assert_eq!(
            WidgetryComboBox::<Speed>::selected_id(app.world(), combo).unwrap(),
            Some(default_id)
        );
        let mut state = Waveform::playback(app.world(), root).unwrap().unwrap();
        state.position = std::time::Duration::from_secs(15);
        state.speed = 2.0;
        state.playing = true;
        Waveform::set_playback_in_world(app.world_mut(), root, state).unwrap();
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().get::<Text>(time).unwrap().0, "00:15 / 01:40");
        assert_eq!(requests.lock().unwrap().len(), 2);
        let selected = app
            .world()
            .get::<WidgetryListModel<Speed>>(model)
            .unwrap()
            .id(4)
            .unwrap();
        assert_eq!(
            WidgetryComboBox::<Speed>::selected_id(app.world(), combo).unwrap(),
            Some(selected)
        );
        Waveform::set_session_in_world(
            app.world_mut(),
            root,
            Arc::new(Source(WaveformSessionKind::Replay {
                total_frames: 200_000,
            })),
        )
        .unwrap();
        app.world_mut().trigger(Activate { entity: button });
        WidgetryComboBox::<Speed>::select_index(&mut app.world_mut().commands(), combo, 5);
        app.world_mut().flush();
        assert_eq!(requests.lock().unwrap().len(), 2);
        for _ in 0..3 {
            app.update();
        }
        let replacement_bar = app.world().get::<PlaybackUi>(root).unwrap().bar.unwrap();
        assert_ne!(replacement_bar, bar);
        assert!(app.world().get_entity(bar).is_err());
        let state = Waveform::playback(app.world(), root).unwrap().unwrap();
        assert!(!state.playing);
        assert_eq!(state.speed, 2.0);
        assert_eq!(state.position, std::time::Duration::ZERO);
        Waveform::set_session_in_world(
            app.world_mut(),
            root,
            Arc::new(Source(WaveformSessionKind::Live)),
        )
        .unwrap();
        app.update();
        assert!(app.world().get::<PlaybackUi>(root).unwrap().bar.is_none());
        assert!(app.world().get_entity(bar).is_err());
        assert!(app.world().get_entity(replacement_bar).is_err());
    }
}
