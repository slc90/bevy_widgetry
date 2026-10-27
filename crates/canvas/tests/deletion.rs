#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{add_keyboard_dispatch, pointer_ids, queue_key, queue_pointer};
mod common;
use common::*;

fn delete(app: &mut App, window: Entity) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        queue_key(
            app,
            KeyboardInput {
                key_code: KeyCode::Delete,
                logical_key: Key::Delete,
                state,
                text: None,
                repeat: false,
                window,
            },
        );
        update(app);
    }
}

#[test]
fn focused_delete_removes_model_repairs_both_views_and_releases_capture_and_assets() {
    for id in pointer_ids() {
        let (mut app, a, b, shape, window, at) = fixture(id, 1.0, 1.0);
        add_keyboard_dispatch(&mut app);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let other = CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            CanvasRectangle {
                corner_radius: 0.0,
                size: Vec2::splat(20.0),
            },
            Transform::from_xyz(60.0, 20.0, 0.0),
            CanvasPaint::default(),
        )
        .unwrap();
        CanvasView::set_zoom_in_world(app.world_mut(), b, 0.5).unwrap();
        CanvasView::set_center_in_world(app.world_mut(), b, Vec2::new(10.0, -5.0)).unwrap();
        for view in [a, b] {
            CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
        }
        update(&mut app);
        let meshes = app.world().resource::<Assets<Mesh>>().len();
        #[derive(Resource, Default)]
        struct Notices(Vec<Entity>);
        app.init_resource::<Notices>().add_observer(
            move |e: On<CanvasSelectionChanged>, mut world: bevy::ecs::world::DeferredWorld| {
                if e.selection.is_some() {
                    return;
                }
                assert!(world.get_entity(shape).is_err());
                assert_eq!(e.selection, None);
                assert_eq!(CanvasView::selection(&world, e.entity).unwrap(), None);
                world.resource_mut::<Notices>().0.push(e.entity);
            },
        );
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
        delete(&mut app, window);
        assert!(app.world().get_entity(shape).is_err());
        assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
        for view in [a, b] {
            assert_eq!(CanvasView::selection(app.world(), view).unwrap(), None);
        }
        assert_eq!(app.world().resource::<Notices>().0.len(), 2);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), meshes - 4);
        assert!(
            app.world()
                .get::<CanvasGeometry>(other)
                .and_then(CanvasGeometry::rectangle)
                .is_some()
        );
        assert_eq!(CanvasView::zoom(app.world(), b).unwrap(), 0.5);
        assert_eq!(
            CanvasView::center(app.world(), b).unwrap(),
            Vec2::new(10.0, -5.0)
        );
        delete(&mut app, window);
        assert_eq!(app.world().resource::<Notices>().0.len(), 2);
        CanvasView::set_selection_in_world(app.world_mut(), a, Some(other)).unwrap();
        delete(&mut app, window);
        assert!(app.world().get::<CanvasDocument>(document).is_some());
        assert!(app.world().get::<CanvasView>(a).is_some());
        assert_eq!(
            app.world_mut()
                .query::<&CanvasGeometry>()
                .iter(app.world())
                .count(),
            0
        );
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        let mut at = at.clone();
        moved(&mut app, id, &mut at, Vec2::new(2.0, 2.0));
        scroll(&mut app, id, &at, 1.0);
        assert!(CanvasView::zoom(app.world(), a).unwrap() > 1.0);
        CanvasView::set_mode_in_world(
            app.world_mut(),
            a,
            CanvasMode::Rectangle(CanvasPaint::default()),
        )
        .unwrap();
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        moved(&mut app, id, &mut at, Vec2::splat(20.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(
            app.world_mut()
                .query::<&CanvasGeometry>()
                .iter(app.world())
                .count(),
            1
        );
    }
}

#[test]
fn delete_requires_exact_canvas_focus_and_an_available_window_and_view() {
    let (mut app, a, _, shape, window, _) = fixture(pointer_ids()[1], 1.0, 1.0);
    add_keyboard_dispatch(&mut app);
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    let child = app
        .world_mut()
        .spawn_scene(bsn! { Node template(move |_| Ok(ChildOf(a))) })
        .unwrap()
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(child, FocusCause::Navigated);
    delete(&mut app, window);
    assert!(
        app.world()
            .get::<CanvasGeometry>(shape)
            .and_then(CanvasGeometry::rectangle)
            .is_some()
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(a, FocusCause::Navigated);
    app.world_mut().entity_mut(a).insert(Visibility::Hidden);
    delete(&mut app, window);
    assert!(
        app.world()
            .get::<CanvasGeometry>(shape)
            .and_then(CanvasGeometry::rectangle)
            .is_some()
    );
    app.world_mut().entity_mut(a).insert(Visibility::Inherited);
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    delete(&mut app, window);
    assert!(
        app.world()
            .get::<CanvasGeometry>(shape)
            .and_then(CanvasGeometry::rectangle)
            .is_some()
    );
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    delete(&mut app, window);
    assert!(app.world().get_entity(shape).is_err());
}

#[test]
fn text_field_inside_canvas_receives_delete_without_removing_the_selected_shape() {
    use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.add_plugins((TransformPlugin, CanvasPlugin, WidgetryTextFieldPlugin));
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(bevy_widgetry_asset::BuiltinFont::Default.path());
    bevy_widgetry_test_utils::advance_until(
        &mut app,
        std::time::Duration::from_secs(10),
        "TextField font",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    bevy_widgetry_core::WidgetryAppExt::set_default_font(
        &mut app,
        bevy::text::FontSource::Handle(font),
    );
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let document = app.world_mut().spawn(CanvasDocument).id();
    let shape = CanvasDocument::create_rectangle(
        app.world_mut(),
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(40.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let view = app
        .world_mut()
        .spawn_scene(bsn! {
            @CanvasView { @document: {Some(document)} }
            Node { width: px(400), height: px(300) }
        })
        .unwrap()
        .id();
    let field = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField { @value: {"abc".to_owned()} }
            template(move |_| Ok(ChildOf(view))) Node { width: px(200) }
        })
        .unwrap()
        .id();
    CanvasView::set_selection_in_world(app.world_mut(), view, Some(shape)).unwrap();
    update(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
    queue_key(
        &mut app,
        KeyboardInput {
            key_code: KeyCode::Home,
            logical_key: Key::Home,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    update(&mut app);
    delete(&mut app, window);
    assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "bc");
    assert!(
        app.world()
            .get::<CanvasGeometry>(shape)
            .and_then(CanvasGeometry::rectangle)
            .is_some()
    );
    assert_eq!(
        CanvasView::selection(app.world(), view).unwrap(),
        Some(shape)
    );
}

#[test]
fn same_frame_delete_does_not_cancel_pan_final_motion_or_rectangle_completion() {
    for id in pointer_ids() {
        for drawing in [false, true] {
            let (mut app, a, _, shape, window, mut at) = fixture(id, 1.0, 1.0);
            add_keyboard_dispatch(&mut app);
            app.configure_sets(
                PreUpdate,
                bevy::input_focus::InputFocusSystems::Dispatch
                    .after(bevy::picking::PickingSystems::ProcessInput)
                    .before(bevy::picking::PickingSystems::Hover),
            );
            CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
            if drawing {
                CanvasView::set_mode_in_world(
                    app.world_mut(),
                    a,
                    CanvasMode::Rectangle(CanvasPaint::default()),
                )
                .unwrap();
            } else {
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::Space);
            }
            input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            let delta = Vec2::new(30.0, 20.0);
            at.position += delta;
            queue_pointer(&mut app, id, at.clone(), PointerAction::Move { delta });
            queue_pointer(
                &mut app,
                id,
                at.clone(),
                PointerAction::Release(PointerButton::Primary),
            );
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code: KeyCode::Delete,
                    logical_key: Key::Delete,
                    state: ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            );
            update(&mut app);
            assert!(app.world().get_entity(shape).is_err());
            assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
            if drawing {
                let selected = CanvasView::selection(app.world(), a).unwrap().unwrap();
                assert_eq!(
                    app.world()
                        .get::<CanvasGeometry>(selected)
                        .and_then(CanvasGeometry::rectangle)
                        .unwrap()
                        .size,
                    delta
                );
            } else {
                assert_eq!(CanvasView::center(app.world(), a).unwrap(), -delta);
            }
        }
    }
}
