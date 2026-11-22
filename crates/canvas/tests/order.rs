#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::LogCapture;

fn create(world: &mut World, document: Entity, z: f32) -> Entity {
    CanvasDocument::create_rectangle(
        world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::from_xyz(0.0, 0.0, z),
        CanvasPaint::default(),
    )
    .unwrap()
}

#[test]
fn a_headless_document_has_one_default_layer_and_shapes_are_items() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let layers = CanvasDocument::layer_order(&world, document).unwrap();
    let [layer] = layers.as_slice() else {
        panic!("expected one default layer");
    };
    let container = CanvasContainer::Layer(*layer);
    let stack = world.get::<CanvasLayerStack>(document).unwrap();
    let default_layer = stack.layers().first().unwrap();
    assert_eq!(default_layer.id(), *layer);
    assert_eq!(default_layer.name(), "Layer 1");
    assert!(default_layer.visible());
    assert!(!default_layer.locked());
    assert!(
        CanvasDocument::item_order(&world, document, container)
            .unwrap()
            .is_empty()
    );

    let bottom = create(&mut world, document, 10.0);
    let top = create(&mut world, document, -10.0);
    for entity in [bottom, top] {
        assert!(world.get::<CanvasItem>(entity).is_some());
        assert_eq!(world.get::<ChildOf>(entity).unwrap().parent(), document);
    }
    assert_eq!(
        CanvasDocument::item_order(&world, document, container).unwrap(),
        [bottom, top]
    );
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [bottom, top]
    );
    assert!(
        CanvasDocument::reorder_item(&mut world, document, container, top, CanvasOrder::Back)
            .unwrap()
    );
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [top, bottom]
    );
    CanvasDocument::remove_shapes(&mut world, document, &[bottom]).unwrap();
    assert_eq!(
        CanvasDocument::item_order(&world, document, container).unwrap(),
        [top]
    );
    world.despawn(document);
    assert!(world.get_entity(top).is_err());
}

#[test]
fn document_has_one_total_order_with_atomic_reordering_and_removal() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let a = create(&mut world, document, 9000.0);
    let b = create(&mut world, document, -9000.0);
    let c = create(&mut world, document, 0.0);
    let foreign = create(&mut world, other, 0.0);
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [a, b, c]
    );
    for (shape, operation, expected, changed) in [
        (c, CanvasOrder::Raise, [a, b, c], false),
        (a, CanvasOrder::Lower, [a, b, c], false),
        (a, CanvasOrder::Raise, [b, a, c], true),
        (c, CanvasOrder::Lower, [b, c, a], true),
        (b, CanvasOrder::Front, [c, a, b], true),
        (b, CanvasOrder::Front, [c, a, b], false),
        (a, CanvasOrder::Back, [a, c, b], true),
        (a, CanvasOrder::Back, [a, c, b], false),
    ] {
        assert_eq!(
            CanvasDocument::reorder_shape(&mut world, document, shape, operation).unwrap(),
            changed
        );
        assert_eq!(
            CanvasDocument::shape_order(&mut world, document).unwrap(),
            expected
        );
    }
    let logs = LogCapture::default();
    for invalid in [foreign, document, Entity::PLACEHOLDER] {
        let error = logs
            .run(|| {
                CanvasDocument::reorder_shape(&mut world, document, invalid, CanvasOrder::Front)
            })
            .unwrap_err();
        assert!(
            error
                .diagnostics()
                .next()
                .unwrap()
                .source()
                .is_some_and(|cause| cause.is::<CanvasError>())
        );
        assert_eq!(
            CanvasDocument::shape_order(&mut world, document).unwrap(),
            [a, c, b]
        );
    }
    assert_eq!(world.get::<Transform>(a).unwrap().translation.z, 9000.0);
    CanvasDocument::remove_shape(&mut world, document, c).unwrap();
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [a, b]
    );
    world.despawn(a);
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [b]
    );
    let svg = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(&mut world, document, svg.path(), None).unwrap();
    world.entity_mut(b).insert(ChildOf(other));
    assert!(
        CanvasDocument::shape_order(&mut world, document)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        CanvasDocument::shape_order(&mut world, other).unwrap(),
        [foreign]
    );
}

#[test]
fn reordering_skips_and_repairs_incomplete_and_despawned_neighbors() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let a = create(&mut world, document, 0.0);
    let incomplete = create(&mut world, document, 0.0);
    let despawned = create(&mut world, document, 0.0);
    let b = create(&mut world, document, 0.0);
    world.entity_mut(incomplete).remove::<CanvasPaint>();
    world.despawn(despawned);

    assert!(CanvasDocument::reorder_shape(&mut world, document, a, CanvasOrder::Raise).unwrap());
    let svg = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(&mut world, document, svg.path(), None).unwrap();
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [b, a]
    );
}

#[test]
fn shape_order_repairs_entries_that_lost_both_shape_components() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let keep = create(&mut world, document, 0.0);
    let incomplete = create(&mut world, document, 0.0);
    world
        .entity_mut(incomplete)
        .remove::<(CanvasShape, CanvasGeometry)>();

    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [keep]
    );
    let svg = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(&mut world, document, svg.path(), None).unwrap();
}
