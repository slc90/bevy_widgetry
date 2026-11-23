#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;

const OFFSET: Vec2 = Vec2::splat(16.0);

fn rectangle(world: &mut World, document: Entity, x: f32) -> Entity {
    CanvasDocument::create_rectangle(
        world,
        document,
        CanvasRectangle {
            size: Vec2::new(20.0, 12.0),
            corner_radius: 2.0,
        },
        Transform::from_xyz(x, 5.0, 0.0),
        CanvasPaint {
            fill: Some(Color::srgb(0.4, 0.7, 0.9)),
            ..default()
        },
    )
    .unwrap()
}

fn connect(world: &mut World, doc: Entity, source: Entity, destination: Entity) -> Entity {
    let connector = CanvasDocument::create_connector(
        world,
        doc,
        CanvasConnectorRoute {
            start: Vec2::ZERO,
            waypoints: vec![Vec2::new(20.0, 8.0)],
            end: Vec2::new(90.0, 5.0),
        },
        CanvasConnectorStyle {
            end_arrow: CanvasArrow::Triangle,
            ..default()
        },
        Some(CanvasConnectorLabel {
            text: "Connection".into(),
            font_size: 14.0,
            color: Color::BLACK,
            position: 0.5,
        }),
    )
    .unwrap();
    for (item, end, anchor) in [
        (source, CanvasConnectorEnd::Start, CanvasAnchor::East),
        (destination, CanvasConnectorEnd::End, CanvasAnchor::West),
    ] {
        CanvasDocument::set_connector_endpoint(
            world,
            doc,
            connector,
            end,
            CanvasConnectorEndpoint::Attached {
                target: item,
                anchor,
            },
        )
        .unwrap();
    }
    connector
}

#[test]
fn clipboard_pastes_nested_items_into_another_document_and_remaps_only_internal_anchors() {
    let mut world = World::new();
    let source = world.spawn(CanvasDocument).id();
    let dest = world.spawn(CanvasDocument).id();
    let layer = CanvasDocument::create_layer(&mut world, dest, 1, "Paste destination").unwrap();
    CanvasDocument::clear_history(&mut world, dest).unwrap();
    let left = rectangle(&mut world, source, 0.0);
    let right = rectangle(&mut world, source, 50.0);
    let inner = CanvasDocument::group_items(&mut world, source, &[left, right], "Inner").unwrap();
    let text = CanvasDocument::create_text(
        &mut world,
        source,
        CanvasTextStyle {
            content: "Caption".into(),
            ..default()
        },
        Transform::from_xyz(20.0, 35.0, 0.0),
    )
    .unwrap();
    let outer = CanvasDocument::group_items(&mut world, source, &[inner, text], "Outer").unwrap();
    let mut pose = *world.get::<Transform>(outer).unwrap();
    pose.rotation = Quat::from_rotation_z(0.45);
    pose.scale = Vec3::new(1.5, 1.5, 1.0);
    CanvasDocument::update_transforms(&mut world, source, &[(outer, pose)]).unwrap();
    let external = rectangle(&mut world, source, 120.0);
    let internal = connect(&mut world, source, left, right);
    let mixed = connect(&mut world, source, right, external);
    CanvasDocument::clear_history(&mut world, source).unwrap();
    assert_eq!(
        CanvasClipboard::copy(&mut world, source, &[left, mixed, outer, internal]).unwrap(),
        3
    );
    assert!(!CanvasDocument::can_undo(&world, source).unwrap());
    let pasted = CanvasClipboard::paste(&mut world, dest, Some(layer)).unwrap();
    assert_eq!(pasted.len(), 3);
    assert_eq!(
        CanvasDocument::item_order(&world, dest, CanvasContainer::Layer(layer)).unwrap(),
        pasted
    );
    let children =
        CanvasDocument::item_order(&world, dest, CanvasContainer::Group(pasted[0])).unwrap();
    let copies =
        CanvasDocument::item_order(&world, dest, CanvasContainer::Group(children[0])).unwrap();
    assert_eq!(
        world.get::<CanvasGroup>(children[0]).unwrap().name(),
        "Inner"
    );
    assert_eq!(
        world.get::<CanvasText>(children[1]).unwrap().style(),
        world.get::<CanvasText>(text).unwrap().style()
    );
    assert_eq!(
        world.get::<CanvasPaint>(copies[0]),
        world.get::<CanvasPaint>(left)
    );
    let relative = CanvasDocument::document_transform(&world, dest, copies[0])
        .unwrap()
        .translation
        .truncate()
        - CanvasDocument::document_transform(&world, source, left)
            .unwrap()
            .translation
            .truncate();
    assert!(relative.abs_diff_eq(OFFSET, 0.001));
    let endpoint = |world: &World, item, end| {
        CanvasDocument::connector_endpoint(world, dest, item, end).unwrap()
    };
    let assert_endpoints = |world: &World| {
        for (item, end, expected) in [
            (
                pasted[1],
                CanvasConnectorEnd::Start,
                CanvasConnectorEndpoint::Attached {
                    target: copies[0],
                    anchor: CanvasAnchor::East,
                },
            ),
            (
                pasted[1],
                CanvasConnectorEnd::End,
                CanvasConnectorEndpoint::Attached {
                    target: copies[1],
                    anchor: CanvasAnchor::West,
                },
            ),
            (
                pasted[2],
                CanvasConnectorEnd::Start,
                CanvasConnectorEndpoint::Attached {
                    target: copies[1],
                    anchor: CanvasAnchor::East,
                },
            ),
            (
                pasted[2],
                CanvasConnectorEnd::End,
                CanvasConnectorEndpoint::Free(Vec2::new(110.0, 5.0) + OFFSET),
            ),
        ] {
            assert_eq!(endpoint(world, item, end), expected);
        }
    };
    assert_endpoints(&world);
    assert_eq!(
        world.get::<CanvasConnector>(pasted[2]).unwrap().label(),
        world.get::<CanvasConnector>(mixed).unwrap().label()
    );
    assert!(CanvasDocument::undo(&mut world, dest).unwrap());
    assert!(CanvasDocument::draw_order(&world, dest).unwrap().is_empty());
    assert!(!CanvasDocument::can_undo(&world, dest).unwrap());
    assert!(CanvasDocument::redo(&mut world, dest).unwrap());
    assert_eq!(
        CanvasDocument::item_order(&world, dest, CanvasContainer::Layer(layer)).unwrap(),
        pasted
    );
    assert_eq!(
        CanvasDocument::item_order(&world, dest, CanvasContainer::Group(pasted[0])).unwrap(),
        children
    );
    assert_eq!(
        CanvasDocument::item_order(&world, dest, CanvasContainer::Group(children[0])).unwrap(),
        copies
    );
    assert_endpoints(&world);
    let source_pose = CanvasDocument::document_transform(&world, source, left).unwrap();
    let flattened = CanvasClipboard::duplicate(&mut world, source, None, &[left]).unwrap();
    let copied_pose = CanvasDocument::document_transform(&world, source, flattened[0]).unwrap();
    assert!(
        (copied_pose.translation.truncate() - source_pose.translation.truncate())
            .abs_diff_eq(OFFSET, 0.001)
    );
    assert!(copied_pose.matrix3.abs_diff_eq(source_pose.matrix3, 0.001));
}

#[test]
fn cuts_a_child_from_a_group_with_a_large_but_representable_scale() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let cut = rectangle(&mut world, document, 0.0);
    let sibling = rectangle(&mut world, document, 0.0);
    let group = CanvasDocument::group_items(&mut world, document, &[cut, sibling], "Wide").unwrap();
    for item in [cut, sibling] {
        let mut transform = *world.get::<Transform>(item).unwrap();
        transform.scale = Vec3::new(1e-20, 1e-20, 1.0);
        CanvasDocument::update_transforms(&mut world, document, &[(item, transform)]).unwrap();
    }
    let mut transform = *world.get::<Transform>(group).unwrap();
    transform.scale = Vec3::new(1e20, 1e20, 1.0);
    CanvasDocument::update_group_transform(&mut world, document, group, transform).unwrap();
    let source = CanvasDocument::document_transform(&world, document, cut).unwrap();
    assert_eq!(
        CanvasClipboard::cut(&mut world, document, &[cut]).unwrap(),
        1
    );
    let inserted = CanvasClipboard::paste(&mut world, document, None).unwrap();
    let pasted = CanvasDocument::document_transform(&world, document, inserted[0]).unwrap();
    assert!(pasted.matrix3.abs_diff_eq(source.matrix3, 0.001));
    assert!(
        (pasted.translation.truncate() - source.translation.truncate()).abs_diff_eq(OFFSET, 0.001)
    );
}

#[test]
fn failed_cut_and_paste_keep_their_inputs_and_duplicate_uses_one_history_entry() {
    let mut world = World::new();
    let a = world.spawn(CanvasDocument).id();
    let b = world.spawn(CanvasDocument).id();
    let first = rectangle(&mut world, a, 5.0);
    let foreign = rectangle(&mut world, b, 90.0);
    let layer = CanvasDocument::default_layer(&world, b).unwrap();
    assert_eq!(CanvasClipboard::copy(&mut world, a, &[first]).unwrap(), 1);
    assert!(CanvasClipboard::cut(&mut world, a, &[first, foreign]).is_err());
    assert_eq!(CanvasDocument::draw_order(&world, a).unwrap(), [first]);
    CanvasDocument::set_layer_locked(&mut world, b, layer, true).unwrap();
    assert!(CanvasClipboard::paste(&mut world, b, Some(layer)).is_err());
    assert_eq!(CanvasDocument::draw_order(&world, b).unwrap(), [foreign]);
    CanvasDocument::set_layer_locked(&mut world, b, layer, false).unwrap();
    let foreign_layer = CanvasDocument::default_layer(&world, a).unwrap();
    let copies = CanvasClipboard::paste(&mut world, b, Some(foreign_layer)).unwrap();
    assert_eq!(copies.len(), 1);
    assert_eq!(
        world
            .get::<Transform>(copies[0])
            .unwrap()
            .translation
            .truncate(),
        Vec2::splat(5.0) + OFFSET
    );
    CanvasDocument::clear_history(&mut world, a).unwrap();
    let duplicate = CanvasClipboard::duplicate(&mut world, a, None, &[first]).unwrap();
    assert_eq!(duplicate.len(), 1);
    assert!(CanvasDocument::undo(&mut world, a).unwrap());
    assert!(!CanvasDocument::can_undo(&world, a).unwrap());
    assert_eq!(CanvasDocument::draw_order(&world, a).unwrap(), [first]);
    assert_eq!(CanvasClipboard::cut(&mut world, a, &[first]).unwrap(), 1);
    assert!(CanvasDocument::draw_order(&world, a).unwrap().is_empty());
    assert!(CanvasDocument::undo(&mut world, a).unwrap());
    assert_eq!(CanvasDocument::draw_order(&world, a).unwrap(), [first]);
}
