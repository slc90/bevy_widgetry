#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_canvas::*;

fn fixture() -> (World, Entity, Entity, CanvasPath) {
    use CanvasPathCommand::*;
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let path = CanvasPath {
        commands: vec![
            MoveTo(Vec2::ZERO),
            LineTo(Vec2::new(30.0, 0.0)),
            LineTo(Vec2::new(30.0, 20.0)),
            Close,
            MoveTo(Vec2::new(50.0, 0.0)),
            LineTo(Vec2::new(60.0, 10.0)),
        ],
    };
    let shape = CanvasDocument::create_path(
        &mut world,
        document,
        path.clone(),
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    (world, document, shape, path)
}
#[allow(clippy::unreachable)]
fn read(world: &World, shape: Entity) -> CanvasPath {
    match world.get::<CanvasGeometry>(shape).unwrap() {
        CanvasGeometry::Path(p) => p.clone(),
        _ => unreachable!(),
    }
}

#[test]
fn removing_first_middle_last_or_an_entire_single_anchor_contour_keeps_a_valid_path() {
    use CanvasPathCommand::*;
    for index in [0, 1, 2, 4, 5] {
        let (mut world, document, shape, original) = fixture();
        assert!(CanvasDocument::remove_path_anchor(&mut world, document, shape, index).unwrap());
        let actual = read(&world, shape);
        actual
            .validate(&Transform::default(), &CanvasPaint::default())
            .unwrap();
        if index < 3 {
            assert_eq!(
                &actual.commands[actual.commands.len() - 2..],
                &original.commands[4..]
            );
        }
        if index == 0 {
            assert_eq!(actual.commands[0], MoveTo(Vec2::new(30.0, 0.0)));
        }
        if index == 4 {
            assert_eq!(actual.commands.last(), Some(&MoveTo(Vec2::new(60.0, 10.0))));
        }
    }
    let (mut world, document, shape, _) = fixture();
    let p = CanvasPath {
        commands: vec![
            MoveTo(Vec2::ZERO),
            Close,
            MoveTo(Vec2::ONE),
            LineTo(Vec2::splat(2.0)),
        ],
    };
    CanvasDocument::update_path(
        &mut world,
        document,
        shape,
        p,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    CanvasDocument::remove_path_anchor(&mut world, document, shape, 0).unwrap();
    assert_eq!(
        read(&world, shape).commands,
        vec![MoveTo(Vec2::ONE), LineTo(Vec2::splat(2.0))]
    );
}

#[test]
fn opening_a_closed_contour_preserves_the_independent_implicit_contour_after_close() {
    use CanvasPathCommand::*;
    let (mut world, document, shape, _) = fixture();
    let commands = vec![
        MoveTo(Vec2::ZERO),
        LineTo(Vec2::new(30.0, 0.0)),
        Close,
        LineTo(Vec2::new(10.0, 20.0)),
        LineTo(Vec2::new(20.0, 30.0)),
        Close,
    ];
    CanvasDocument::update_path(
        &mut world,
        document,
        shape,
        CanvasPath {
            commands: commands.clone(),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    assert!(
        CanvasDocument::set_path_subpath_closed(&mut world, document, shape, 1, false).unwrap()
    );
    assert_eq!(
        read(&world, shape).commands,
        vec![
            commands[0],
            commands[1],
            MoveTo(Vec2::ZERO),
            commands[3],
            commands[4],
            Close
        ]
    );
    assert!(
        CanvasDocument::set_path_subpath_closed(&mut world, document, shape, 4, false).unwrap()
    );
    assert!(!read(&world, shape).commands.contains(&Close));
}

#[test]
fn rejected_edits_report_errors_without_partial_model_writes() {
    use bevy_widgetry_test_utils::LogCapture;
    for case in 0..7 {
        let (mut world, document, shape, original) = fixture();
        let log = LogCapture::default();
        let result = log.run(|| match case {
            0 => CanvasDocument::insert_path_anchor(
                &mut world,
                document,
                shape,
                usize::MAX,
                Vec2::ZERO,
            ),
            1 => CanvasDocument::insert_path_anchor(&mut world, document, shape, 1, Vec2::NAN),
            2 => CanvasDocument::remove_path_anchor(&mut world, document, shape, 3),
            3 => CanvasDocument::set_path_segment(
                &mut world,
                document,
                shape,
                0,
                CanvasPathSegment::Line,
            ),
            4 => CanvasDocument::set_path_segment(
                &mut world,
                document,
                shape,
                1,
                CanvasPathSegment::Quadratic {
                    control: Vec2::INFINITY,
                },
            ),
            5 => CanvasDocument::set_path_subpath_closed(
                &mut world,
                Entity::PLACEHOLDER,
                shape,
                0,
                false,
            ),
            _ => CanvasDocument::remove_path_anchor(&mut world, document, Entity::PLACEHOLDER, 0),
        });
        assert_eq!(
            result.unwrap_err().severity(),
            bevy::ecs::error::Severity::Error
        );
        assert_eq!(read(&world, shape), original);
        assert_eq!(
            CanvasDocument::shape_order(&mut world, document).unwrap(),
            [shape]
        );
        assert_eq!(log.records().len(), 1);
    }
}

#[allow(dead_code)]
mod common;
#[path = "common/coordinates.rs"]
mod coordinates;
#[test]
fn topology_updates_refresh_node_picking_and_both_views_and_cancel_stale_node_capture() {
    use bevy::picking::{hover::PointerCaptureMap, pointer::*};
    use bevy_widgetry_test_utils::pointer_ids;
    use common::*;
    let id = pointer_ids()[1];
    let (mut app, a, b, old, _, base) = fixture_for_view(id);
    let document = CanvasView::document(app.world(), a).unwrap().unwrap();
    let p = CanvasPath {
        commands: vec![
            CanvasPathCommand::MoveTo(Vec2::new(-30.0, -20.0)),
            CanvasPathCommand::LineTo(Vec2::new(0.0, -20.0)),
            CanvasPathCommand::LineTo(Vec2::new(30.0, 10.0)),
        ],
    };
    let shape = CanvasDocument::create_path(
        app.world_mut(),
        document,
        p,
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), a, Some(shape)).unwrap();
    CanvasView::set_selection_in_world(app.world_mut(), b, Some(old)).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::PathNodeEdit).unwrap();
    update(&mut app);
    let count = app.world().entities().count_spawned();
    let assets = app
        .world()
        .resource::<Assets<Mesh>>()
        .ids()
        .collect::<Vec<_>>();
    CanvasDocument::set_path_segment(
        app.world_mut(),
        document,
        shape,
        1,
        CanvasPathSegment::Quadratic {
            control: Vec2::new(-10.0, -40.0),
        },
    )
    .unwrap();
    update(&mut app);
    let mut at = coordinates::at_document(&app, a, &base, Vec2::new(-10.0, -40.0));
    input(&mut app, id, &at, PointerAction::Move { delta: Vec2::ONE });
    input(
        &mut app,
        id,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
    moved(&mut app, id, &mut at, Vec2::new(3.0, 4.0));
    assert!(
        matches!(read(app.world(),shape).commands[1],CanvasPathCommand::QuadTo {control,..} if control.distance(Vec2::new(-7.0,-36.0))<0.001)
    );
    let CanvasPathCommand::QuadTo { control, .. } = read(app.world(), shape).commands[1] else {
        panic!("Quad expected")
    };
    assert!(
        !CanvasDocument::set_path_segment(
            app.world_mut(),
            document,
            shape,
            1,
            CanvasPathSegment::Quadratic { control }
        )
        .unwrap()
    );
    assert!(app.world().resource::<PointerCaptureMap>().is_captured(&id));
    CanvasDocument::set_path_segment(
        app.world_mut(),
        document,
        shape,
        1,
        CanvasPathSegment::Cubic {
            control1: Vec2::new(-5.0, -20.0),
            control2: Vec2::new(-5.0, -30.0),
        },
    )
    .unwrap();
    assert!(!app.world().resource::<PointerCaptureMap>().is_captured(&id));
    let saved = read(app.world(), shape);
    moved(&mut app, id, &mut at, Vec2::ONE);
    assert_eq!(read(app.world(), shape), saved);
    assert_eq!(
        CanvasView::single_selection(app.world(), b).unwrap(),
        Some(old)
    );
    assert_eq!(app.world().entities().count_spawned(), count);
    assert_eq!(
        app.world()
            .resource::<Assets<Mesh>>()
            .ids()
            .collect::<Vec<_>>(),
        assets
    );
}

fn fixture_for_view(
    id: bevy::picking::pointer::PointerId,
) -> (
    App,
    Entity,
    Entity,
    Entity,
    Entity,
    bevy::picking::pointer::Location,
) {
    common::fixture(id, 1.0, 1.0)
}
#[test]
fn anchor_edits_and_segment_conversions_preserve_the_other_subpath_and_common_identity() {
    use CanvasPathCommand::*;
    let (mut world, document, shape, path) = fixture();
    assert!(
        CanvasDocument::insert_path_anchor(&mut world, document, shape, 1, Vec2::new(15.0, 0.0))
            .unwrap()
    );
    let mut expected = path.clone();
    expected.commands.insert(1, LineTo(Vec2::new(15.0, 0.0)));
    assert_eq!(read(&world, shape), expected);
    for segment in [
        CanvasPathSegment::Quadratic {
            control: Vec2::new(15.0, -10.0),
        },
        CanvasPathSegment::Cubic {
            control1: Vec2::new(5.0, -10.0),
            control2: Vec2::new(20.0, -10.0),
        },
        CanvasPathSegment::Line,
    ] {
        assert!(CanvasDocument::set_path_segment(&mut world, document, shape, 2, segment).unwrap());
        let current = read(&world, shape);
        assert_eq!(&current.commands[3..], &expected.commands[3..]);
        assert_eq!(current.commands[0], expected.commands[0]);
        assert_eq!(current.commands[1], expected.commands[1]);
    }
    assert!(CanvasDocument::remove_path_anchor(&mut world, document, shape, 1).unwrap());
    assert_eq!(read(&world, shape), path);
    assert!(CanvasDocument::set_path_subpath_closed(&mut world, document, shape, 5, true).unwrap());
    assert_eq!(read(&world, shape).commands.last(), Some(&Close));
    assert!(
        !CanvasDocument::set_path_subpath_closed(&mut world, document, shape, 4, true).unwrap()
    );
    assert!(
        CanvasDocument::set_path_subpath_closed(&mut world, document, shape, 2, false).unwrap()
    );
    assert_eq!(
        read(&world, shape)
            .commands
            .iter()
            .filter(|c| **c == Close)
            .count(),
        1
    );
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [shape]
    );
    assert_eq!(world.get::<Transform>(shape), Some(&Transform::default()));
    assert_eq!(
        world.get::<CanvasPaint>(shape),
        Some(&CanvasPaint::default())
    );
}
