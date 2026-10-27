#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    input_focus::InputFocus,
    picking::{hover::PointerCaptureMap, pointer::*},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, pointer_ids};
mod common;
use common::*;

#[path = "common/coordinates.rs"]
mod coordinates;
use coordinates::at_document;

fn click(app: &mut App, id: PointerId, at: &Location) {
    input(app, id, at, PointerAction::Move { delta: Vec2::ONE });
    input(app, id, at, PointerAction::Press(PointerButton::Primary));
    input(app, id, at, PointerAction::Release(PointerButton::Primary));
}

#[test]
fn pointer_hit_testing_passes_hollow_interior_and_selects_rotated_scaled_stroke_without_moving_model()
 {
    for id in pointer_ids() {
        for (scale, ui_scale) in [(1.0, 1.0), (1.5, 1.0), (2.0, 1.5)] {
            let (mut app, a, _, shape, _, base) = fixture(id, scale, ui_scale);
            let document = CanvasView::document(app.world(), a).unwrap().unwrap();
            let transform = Transform::from_xyz(-15.0, 10.0, 0.0)
                .with_rotation(Quat::from_rotation_z(0.4))
                .with_scale(Vec3::new(-1.2, 0.8, 0.0));
            let rectangle = CanvasRectangle {
                size: Vec2::new(80.0, 40.0),
            };
            let paint = CanvasPaint {
                fill: None,
                stroke: Some(CanvasStroke {
                    color: Color::BLACK,
                    width: 8.0,
                    ..Default::default()
                }),
                ..Default::default()
            };
            CanvasDocument::update_rectangle(
                app.world_mut(),
                document,
                shape,
                rectangle,
                transform,
                paint,
            )
            .unwrap();
            CanvasView::set_center_in_world(app.world_mut(), a, Vec2::new(-10.0, 5.0)).unwrap();
            CanvasView::set_zoom_in_world(app.world_mut(), a, 0.75).unwrap();
            update(&mut app);
            for (local, expected) in [
                (Vec2::ZERO, None),
                (Vec2::new(43.0, 0.0), Some(shape)),
                (Vec2::new(54.0, 0.0), None),
            ] {
                let point = transform.transform_point(local.extend(0.0)).truncate();
                let at = at_document(&app, a, &base, point);
                click(&mut app, id, &at);
                assert_eq!(CanvasView::selection(app.world(), a).unwrap(), expected);
                assert_eq!(app.world().resource::<InputFocus>().get(), Some(a));
            }
            let at = at_document(&app, a, &base, transform.translation.truncate());
            click(&mut app, id, &at);
            scroll(&mut app, id, &at, 1.0);
            CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap();
            let mut at = at_document(&app, a, &base, transform.translation.truncate());
            input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            moved(&mut app, id, &mut at, Vec2::new(15.0, 5.0) * ui_scale);
            input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert_eq!(*app.world().get::<Transform>(shape).unwrap(), transform);
            assert_eq!(
                *app.world()
                    .get::<CanvasGeometry>(shape)
                    .and_then(CanvasGeometry::rectangle)
                    .unwrap(),
                rectangle
            );
            assert_eq!(*app.world().get::<CanvasPaint>(shape).unwrap(), paint);
        }
    }
}

#[test]
fn selection_api_is_view_owned_validated_and_committed_before_notifications() {
    let (mut app, a, b, shape, _, _) = fixture(pointer_ids()[1], 1.0, 1.0);
    #[derive(Resource, Default)]
    struct Seen(Vec<Option<Entity>>);
    app.init_resource::<Seen>().add_observer(
        |e: On<CanvasSelectionChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                CanvasView::selection(&world, e.entity).unwrap(),
                e.selection
            );
            world.resource_mut::<Seen>().0.push(e.selection);
        },
    );
    assert!(!CanvasView::set_selection_in_world(app.world_mut(), a, None).unwrap());
    assert!(CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap());
    assert!(!CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap());
    assert_eq!(CanvasView::selection(app.world(), b).unwrap(), None);
    let other_doc = app.world_mut().spawn(CanvasDocument).id();
    let other = CanvasDocument::create_rectangle(
        app.world_mut(),
        other_doc,
        CanvasRectangle {
            size: Vec2::splat(20.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        let error =
            CanvasView::set_selection_in_world(app.world_mut(), a, Some(other)).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    });
    assert_eq!(logs.records().len(), 1);
    assert_eq!(app.world().resource::<Seen>().0, [Some(shape)]);
    let mut commands = app.world_mut().commands();
    CanvasView::set_selection(&mut commands, a, None);
    app.world_mut().flush();
    assert_eq!(CanvasView::selection(app.world(), a).unwrap(), None);
    let mut commands = app.world_mut().commands();
    CanvasView::set_selection(&mut commands, a, Some(other));
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    assert_eq!(app.world().resource::<Seen>().0, [Some(shape), None]);
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    CanvasView::set_document_in_world(app.world_mut(), a, Some(other_doc)).unwrap();
    assert_eq!(CanvasView::selection(app.world(), a).unwrap(), None);
    assert_eq!(CanvasView::selection(app.world(), b).unwrap(), None);
}

#[test]
fn views_select_independently_and_space_pan_does_not_change_selection() {
    for id in pointer_ids() {
        let (mut app, a, b, shape, _, base) = fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), a).unwrap().unwrap();
        let other = CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            CanvasRectangle {
                size: Vec2::splat(20.0),
            },
            Transform::from_xyz(65.0, 0.0, 0.0),
            CanvasPaint::default(),
        )
        .unwrap();
        update(&mut app);
        let at = at_document(&app, a, &base, Vec2::ZERO);
        click(&mut app, id, &at);
        let at = at_document(&app, b, &base, Vec2::new(65.0, 0.0));
        click(&mut app, id, &at);
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), Some(shape));
        assert_eq!(CanvasView::selection(app.world(), b).unwrap(), Some(other));
        let mut at = at_document(&app, a, &base, Vec2::new(70.0, 50.0));
        input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        input(
            &mut app,
            id,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
        moved(&mut app, id, &mut at, Vec2::new(10.0, 5.0));
        input(
            &mut app,
            id,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), Some(shape));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Space);
        let at = at_document(&app, a, &base, Vec2::new(70.0, 50.0));
        click(&mut app, id, &at);
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), None);
        assert_eq!(CanvasView::selection(app.world(), b).unwrap(), Some(other));
    }
}

#[test]
fn nested_widget_owns_focus_and_input_without_changing_canvas_selection_or_starting_pan() {
    let id = pointer_ids()[1];
    let (mut app, a, _, shape, _, base) = fixture(id, 1.0, 1.0);
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    let nested = app
        .world_mut()
        .spawn_scene(bsn! {
            bevy::ui_widgets::Button bevy::input_focus::tab_navigation::TabIndex(0)
            Node {width:percent(100),height:percent(100)}
            template(move |_|Ok(ChildOf(a)))
        })
        .unwrap()
        .id();
    update(&mut app);
    let at = at_document(&app, a, &base, Vec2::new(60.0, 40.0));
    click(&mut app, id, &at);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(nested));
    assert_eq!(CanvasView::selection(app.world(), a).unwrap(), Some(shape));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    assert_eq!(CanvasView::center(app.world(), a).unwrap(), Vec2::ZERO);
}

#[test]
fn unavailable_views_reject_pointer_selection_and_stale_selection_does_not_escape_queries() {
    for cause in 0..6 {
        let id = pointer_ids()[1];
        let (mut app, a, _, shape, window, base) = fixture(id, 1.0, 1.0);
        CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
        let at = at_document(&app, a, &base, Vec2::new(60.0, 40.0));
        match cause {
            0 => {
                app.world_mut()
                    .entity_mut(a)
                    .insert(bevy::ui::InteractionDisabled);
            }
            1 => {
                app.world_mut().entity_mut(a).insert(Visibility::Hidden);
            }
            2 => {
                app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
            }
            3 => {
                app.world_mut().get_mut::<Node>(a).unwrap().width = px(0);
            }
            4 => {
                app.world_mut().get_mut::<Node>(a).unwrap().display = Display::None;
            }
            _ => {
                CanvasView::set_document_in_world(app.world_mut(), a, None).unwrap();
            }
        }
        update(&mut app);
        click(&mut app, id, &at);
        if cause != 5 {
            assert_eq!(CanvasView::selection(app.world(), a).unwrap(), Some(shape));
        }
        let document = app.world_mut().spawn(CanvasDocument).id();
        app.world_mut().entity_mut(shape).insert(ChildOf(document));
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), None);
        update(&mut app);
        assert_eq!(CanvasView::selection(app.world(), a).unwrap(), None);
    }
}

#[test]
fn views_and_picking_follow_document_order_and_keep_each_shapes_paints_together() {
    use bevy::picking::pointer::*;
    use bevy_widgetry_test_utils::pointer_ids;
    for id in pointer_ids() {
        let (mut app, view, other_view, back, _, at) = common::fixture(id, 1.0, 1.0);
        let document = CanvasView::document(app.world(), view).unwrap().unwrap();
        CanvasDocument::update_rectangle(
            app.world_mut(),
            document,
            back,
            CanvasRectangle {
                size: Vec2::splat(20.0),
            },
            Transform::from_xyz(0.0, 0.0, 9000.0),
            CanvasPaint {
                fill: Some(Color::srgb(1.0, 0.0, 0.0)),
                stroke: Some(CanvasStroke {
                    color: Color::srgb(0.0, 1.0, 0.0),
                    width: 4.0,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
        let front = CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            CanvasRectangle {
                size: Vec2::splat(30.0),
            },
            Transform::from_xyz(0.0, 0.0, -9000.0),
            CanvasPaint {
                fill: Some(Color::srgb(0.0, 0.0, 1.0)),
                stroke: None,
                ..Default::default()
            },
        )
        .unwrap();
        CanvasView::set_selection_in_world(app.world_mut(), other_view, Some(back)).unwrap();
        common::update(&mut app);
        for (operation, top) in [(None, front), (Some(CanvasOrder::Back), back)] {
            if let Some(operation) = operation {
                CanvasDocument::reorder_shape(app.world_mut(), document, front, operation).unwrap();
            }
            common::update(&mut app);
            let mut depths = [Vec::new(), Vec::new(), Vec::new()];
            for (name, material, transform) in app
                .world_mut()
                .query::<(&Name, &MeshMaterial2d<ColorMaterial>, &GlobalTransform)>()
                .iter(app.world())
            {
                if !["CanvasFill", "CanvasStroke"].contains(&name.as_str()) {
                    continue;
                }
                let color = app
                    .world()
                    .resource::<Assets<ColorMaterial>>()
                    .get(&material.0)
                    .unwrap()
                    .color;
                for (index, expected) in [
                    Color::srgb(1.0, 0.0, 0.0),
                    Color::srgb(0.0, 1.0, 0.0),
                    Color::srgb(0.0, 0.0, 1.0),
                ]
                .into_iter()
                .enumerate()
                {
                    if color == expected {
                        depths[index].push(transform.translation().z);
                    }
                }
            }
            assert!(depths.iter().all(|values| values.len() == 2));
            for ((red, green), blue) in depths[0].iter().zip(&depths[1]).zip(&depths[2]) {
                assert!(red < green);
                if top == front {
                    assert!(green < blue);
                } else {
                    assert!(blue < red);
                }
            }
            CanvasView::set_selection_in_world(app.world_mut(), view, None).unwrap();
            common::input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
            common::input(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            common::input(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert_eq!(CanvasView::selection(app.world(), view).unwrap(), Some(top));
            assert_eq!(
                CanvasView::selection(app.world(), other_view).unwrap(),
                Some(back)
            );
        }
        assert_eq!(
            app.world().get::<Transform>(back).unwrap().translation.z,
            9000.0
        );
        assert_eq!(
            app.world().get::<Transform>(front).unwrap().translation.z,
            -9000.0
        );
    }
}
