#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_collapsible::{
    WidgetryCollapsible, WidgetryCollapsibleChanged, WidgetryCollapsiblePlugin,
};
use bevy_widgetry_core::disabled::WidgetryEffectiveDisabled;
use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

#[derive(Resource, Default)]
struct Changes(usize);

fn fixture(initial: bool) -> (App, Entity, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin)
        .init_resource::<Changes>();
    app.add_observer(
        |_: On<WidgetryCollapsibleChanged>, mut changes: ResMut<Changes>| changes.0 += 1,
    );
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryCollapsible {
                @header_disabled: initial,
                @header: {Some(Box::new(bsn! { Node Children [Node] }) as Box<dyn Scene>)},
                @children: {Some(Box::new(bsn! { Node }) as Box<dyn SceneList>)},
            }
        })
        .unwrap()
        .id();
    app.update();
    let header = app.world().get::<Children>(root).unwrap()[0];
    let content = app.world().get::<Children>(root).unwrap()[1];
    let button = app.world().get::<Children>(header).unwrap()[1];
    (app, root, header, content, button)
}

fn assert_disabled(world: &World, entity: Entity, disabled: bool) {
    assert_eq!(world.get::<InteractionDisabled>(entity).is_some(), disabled);
    assert_eq!(
        world
            .get::<WidgetryEffectiveDisabled>(entity)
            .unwrap()
            .is_disabled(),
        disabled
    );
    if let Some(children) = world.get::<Children>(entity) {
        for child in children {
            assert_disabled(world, *child, disabled);
        }
    }
}

#[test]
fn header_request_survives_root_disable_and_never_disables_content() {
    let (mut app, root, header, content, _) = fixture(false);
    assert_disabled(app.world(), root, false);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    assert_disabled(app.world(), root, true);
    assert!(
        WidgetryCollapsible::set_header_disabled_in_world(app.world_mut(), root, true).unwrap()
    );
    assert!(
        !WidgetryCollapsible::set_header_disabled_in_world(app.world_mut(), root, true).unwrap()
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    assert_disabled(app.world(), header, true);
    assert_disabled(app.world(), content, false);
    assert!(WidgetryCollapsible::header_disabled(app.world(), root).unwrap());
    assert!(
        WidgetryCollapsible::set_header_disabled_in_world(app.world_mut(), root, false).unwrap()
    );
    assert_disabled(app.world(), root, false);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    WidgetryCollapsible::set_header_disabled_in_world(app.world_mut(), root, true).unwrap();
    WidgetryCollapsible::set_header_disabled_in_world(app.world_mut(), root, false).unwrap();
    assert_disabled(app.world(), root, true);
    assert!(app.world().resource::<Changes>().0 == 0);
}

#[test]
fn initial_request_queued_updates_and_disabled_programmatic_collapse_preserve_contracts() {
    let (mut app, root, header, content, _) = fixture(true);
    assert_disabled(app.world(), header, true);
    assert_disabled(app.world(), content, false);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(content, FocusCause::Navigated);
    assert!(WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap());
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert!(WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, true).unwrap());
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryCollapsible::set_header_disabled(
        &mut Commands::new(&mut queue, app.world()),
        root,
        false,
    );
    assert!(WidgetryCollapsible::header_disabled(app.world(), root).unwrap());
    queue.apply(app.world_mut());
    assert!(!WidgetryCollapsible::header_disabled(app.world(), root).unwrap());
    assert_disabled(app.world(), header, false);
    assert_eq!(app.world().resource::<Changes>().0, 2);
}

#[test]
fn invalid_requests_are_rejected_at_execution_and_preserve_request_and_expansion() {
    let (mut app, root, header, _, _) = fixture(false);
    app.set_error_handler(ErrorCapture::handler());
    let wrong = app.world_mut().spawn_empty().id();
    let error = WidgetryCollapsible::set_header_disabled_in_world(app.world_mut(), wrong, true)
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryCollapsible::set_header_disabled(
        &mut Commands::new(&mut queue, app.world()),
        root,
        true,
    );
    app.world_mut().entity_mut(header).remove::<Node>();
    let errors = ErrorCapture::default();
    errors.run(|| queue.apply(app.world_mut()));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    app.world_mut().entity_mut(header).insert(Node::default());
    assert!(!WidgetryCollapsible::header_disabled(app.world(), root).unwrap());
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<Changes>().0, 0);
}

#[test]
fn header_disabled_preserves_content_text_input_but_root_disabled_blocks_it() {
    use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.add_plugins((WidgetryCollapsiblePlugin, WidgetryTextFieldPlugin));
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryCollapsible {
                @header_disabled: true,
                @children: {Some(Box::new(bsn! { @WidgetryTextField }) as Box<dyn SceneList>)},
            }
        })
        .unwrap()
        .id();
    app.update();
    let content = app.world().get::<Children>(root).unwrap()[1];
    let field = app.world().get::<Children>(content).unwrap()[0];
    WidgetryTextField::focus_in_world(app.world_mut(), field).unwrap();
    let key = || bevy::input::keyboard::KeyboardInput {
        key_code: KeyCode::KeyX,
        logical_key: bevy::input::keyboard::Key::Character("x".into()),
        text: Some("x".into()),
        state: bevy::input::ButtonState::Pressed,
        repeat: false,
        window,
    };
    app.world_mut().write_message(key());
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "x");
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().write_message(key());
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "x");
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().write_message(key());
    app.update();
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "xx");
    assert!(WidgetryCollapsible::header_disabled(app.world(), root).unwrap());
}

#[test]
fn initial_header_disable_is_applied_before_an_immediate_programmatic_collapse() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin);
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryCollapsible { @header_disabled: true, @children: {Some(Box::new(bsn! { Node }) as Box<dyn SceneList>)} } }).unwrap().id();
    let content = app.world().get::<Children>(root).unwrap()[1];
    let field = app.world().get::<Children>(content).unwrap()[0];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
    WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}
