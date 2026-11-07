#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    camera::NormalizedRenderTarget,
    ecs::entity::EntityHashMap,
    picking::{
        backend::HitData,
        hover::HoverMap,
        pointer::{Location, PointerId, PointerInput, PointerLocation},
    },
    prelude::*,
    time::TimeUpdateStrategy,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_tooltip::*;
use std::time::Duration;

#[derive(Resource, Default)]
struct Lifecycle {
    shown: Vec<ShowTooltip>,
    hidden: Vec<HideTooltip>,
}
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<bevy::scene::ScenePatch>();
    app.init_resource::<bevy::picking::pointer::PointerMap>()
        .init_resource::<HoverMap>()
        .add_message::<PointerInput>()
        .init_resource::<Lifecycle>()
        .add_plugins(WidgetryTooltipHeadlessPlugin)
        .add_observer(|event: On<ShowTooltip>, mut events: ResMut<Lifecycle>| {
            events.shown.push(*event)
        })
        .add_observer(|event: On<HideTooltip>, mut events: ResMut<Lifecycle>| {
            events.hidden.push(*event)
        });
    app.world_mut().spawn((
        PointerId::Mouse,
        PointerLocation::new(Location {
            target: NormalizedRenderTarget::None {
                width: 100,
                height: 100,
            },
            position: Vec2::ZERO,
        }),
    ));
    app
}
fn advance(app: &mut App, millis: u64) {
    *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
        TimeUpdateStrategy::ManualDuration(Duration::from_millis(millis));
    app.update();
}
fn hover(app: &mut App, anchor: Option<Entity>) {
    let mut map = app.world_mut().resource_mut::<HoverMap>();
    map.clear();
    if let Some(anchor) = anchor {
        let mut hits = EntityHashMap::default();
        hits.insert(anchor, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
        map.insert(PointerId::Mouse, hits);
    }
}
fn show(app: &mut App, anchor: Entity) -> ShowTooltip {
    hover(app, Some(anchor));
    advance(app, 0);
    advance(app, 200);
    *app.world().resource::<Lifecycle>().shown.last().unwrap()
}

#[test]
fn external_popup_cleanup_does_not_depend_on_anchor_parenting_or_local_observers() {
    for despawn in [false, true] {
        let mut app = app();
        let anchor = spawn_scene(app.world_mut(), bsn! {@WidgetryTooltipHeadless}).unwrap();
        let event = show(&mut app, anchor);
        assert!(event.is_current(app.world()));
        let popup = spawn_scene(app.world_mut(), bsn! {Node Children[Text("external")]}).unwrap();
        assert!(event.register_popup(app.world_mut(), popup).unwrap());
        assert_eq!(event.popup(app.world()), Some(popup));
        assert!(app.world().get::<ChildOf>(popup).is_none());
        assert!(app.world().get::<WidgetryTooltip>(anchor).is_none());
        advance(&mut app, 500);
        assert_eq!(app.world().resource::<Lifecycle>().shown.len(), 1);
        if despawn {
            app.world_mut().despawn(anchor);
        } else {
            app.world_mut()
                .entity_mut(anchor)
                .remove::<WidgetryTooltipHeadless>();
        }
        app.world_mut().flush();
        assert!(app.world().get_entity(popup).is_err());
        assert!(!event.is_current(app.world()));
    }
}

#[test]
fn duplicate_and_expired_presentations_cannot_replace_or_hide_a_new_popup() {
    let mut app = app();
    app.add_plugins(WidgetryTooltipPlugin);
    let anchor = spawn_scene(app.world_mut(), bsn! {@WidgetryTooltipHeadless}).unwrap();
    let first = show(&mut app, anchor);
    let first_popup = spawn_scene(app.world_mut(), bsn! {Node}).unwrap();
    assert!(first.register_popup(app.world_mut(), first_popup).unwrap());
    let duplicate = spawn_scene(app.world_mut(), bsn! {Node}).unwrap();
    assert!(!first.register_popup(app.world_mut(), duplicate).unwrap());
    assert!(app.world().get_entity(duplicate).is_err());
    assert_eq!(first.popup(app.world()), Some(first_popup));
    hover(&mut app, None);
    advance(&mut app, 0);
    let old_hide = *app.world().resource::<Lifecycle>().hidden.last().unwrap();
    let second = show(&mut app, anchor);
    let second_popup = spawn_scene(app.world_mut(), bsn! {Node}).unwrap();
    assert!(
        second
            .register_popup(app.world_mut(), second_popup)
            .unwrap()
    );
    app.world_mut().trigger(old_hide);
    app.world_mut().flush();
    assert_eq!(second.popup(app.world()), Some(second_popup));
    assert!(app.world().get_entity(second_popup).is_ok());
    let stale = spawn_scene(app.world_mut(), bsn! {Node}).unwrap();
    assert!(!first.register_popup(app.world_mut(), stale).unwrap());
    assert!(app.world().get_entity(stale).is_err());
    assert_eq!(second.popup(app.world()), Some(second_popup));
    assert!(!first.register_popup(app.world_mut(), second_popup).unwrap());
    assert!(app.world().get_entity(second_popup).is_ok());
}

#[test]
fn invalid_popup_registration_preserves_the_current_presentation_and_reports_its_targets() {
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
    let mut app = app();
    let anchor = spawn_scene(app.world_mut(), bsn! {@WidgetryTooltipHeadless}).unwrap();
    let event = show(&mut app, anchor);
    let logs = LogCapture::default();
    logs.run(|| {
        for popup in [anchor, Entity::PLACEHOLDER] {
            let error = event.register_popup(app.world_mut(), popup).unwrap_err();
            assert!(matches!(error.kind(), TooltipError::InvalidPopup {anchor:actual, popup:target} if *actual==anchor && *target==popup));
            assert_eq!(logs.diagnostic_records(error.origin().id(),DiagnosticPhase::Source).len(),1);
            assert!(event.is_current(app.world()));
            assert_eq!(event.popup(app.world()), None);
        }
    });
}

#[test]
fn independent_plugin_cancels_native_mouse_tooltip_on_window_focus_loss() {
    use bevy::{
        picking::{
            PickingSystems,
            input::{PointerInputSettings, mouse_pick_events},
        },
        window::{WindowEvent, WindowFocused},
    };
    let mut app = app();
    app.add_message::<WindowEvent>()
        .init_resource::<PointerInputSettings>()
        .add_systems(First, mouse_pick_events.in_set(PickingSystems::Input));
    let window = app.world_mut().spawn(Window::default()).id();
    let pointer = app
        .world_mut()
        .query_filtered::<Entity, With<PointerId>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(pointer)
        .insert(PointerLocation::new(Location {
            target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                .normalize(None)
                .unwrap(),
            position: Vec2::ZERO,
        }));
    let anchor = spawn_scene(app.world_mut(), bsn! {@WidgetryTooltipHeadless}).unwrap();
    let event = show(&mut app, anchor);
    let popup = spawn_scene(app.world_mut(), bsn! {Node}).unwrap();
    assert!(event.register_popup(app.world_mut(), popup).unwrap());
    app.world_mut()
        .write_message(WindowEvent::WindowFocused(WindowFocused {
            window,
            focused: false,
        }));
    advance(&mut app, 0);
    assert!(app.world().get_entity(popup).is_err());
    assert_eq!(app.world().resource::<Lifecycle>().hidden.len(), 1);
    advance(&mut app, 500);
    assert_eq!(app.world().resource::<Lifecycle>().shown.len(), 1);
}

#[test]
fn deferred_external_construction_checks_the_presentation_and_scene_failure_cleans_its_root() {
    use bevy::ecs::world::CommandQueue;
    use bevy_widgetry_test_utils::ErrorCapture;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    let anchor = spawn_scene(app.world_mut(), bsn! {@WidgetryTooltipHeadless}).unwrap();
    let event = show(&mut app, anchor);
    let calls = Arc::new(AtomicUsize::new(0));
    let mut queue = CommandQueue::default();
    let sink = calls.clone();
    Commands::new(&mut queue, app.world()).queue(move |world: &mut World| -> Result {
        if !event.is_current(world) || event.popup(world).is_some() {
            return Ok(());
        }
        sink.fetch_add(1, Ordering::Relaxed);
        let popup = spawn_scene(world, bsn! {Node}).map_err(|error| error.into_bevy_error())?;
        event
            .register_popup(world, popup)
            .map_err(|error| error.into_bevy_error())?;
        Ok(())
    });
    app.world_mut()
        .entity_mut(anchor)
        .remove::<WidgetryTooltipHeadless>();
    queue.apply(app.world_mut());
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    app.world_mut()
        .entity_mut(anchor)
        .insert(WidgetryTooltipHeadless);
    let event = show(&mut app, anchor);
    let before = app.world().entities().len();
    let errors = ErrorCapture::default();
    app.world_mut().commands().queue(move |world: &mut World| -> Result {
        if !event.is_current(world) { return Ok(()); }
        let popup=spawn_scene(world,bsn!{Node Children[template(|_|Err::<Node,_>(BevyError::error("external content failed")))]}).map_err(|error|error.into_bevy_error())?;
        event.register_popup(world,popup).map_err(|error|error.into_bevy_error())?;
        Ok(())
    });
    errors.run(|| app.world_mut().flush());
    let records = errors.take();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(app.world().entities().len(), before);
    assert_eq!(event.popup(app.world()), None);
}
