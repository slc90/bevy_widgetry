#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{
    picking::pointer::{PointerAction, PointerButton},
    prelude::*,
};
use bevy_widgetry_canvas::*;
use bevy_widgetry_test_utils::{LogCapture, pointer_ids};

mod common;
use common::*;

fn rectangle(world: &mut World, document: Entity, layer: CanvasLayerId) -> Entity {
    CanvasDocument::create_rectangle_in_layer(
        world,
        document,
        layer,
        CanvasRectangle {
            size: Vec2::splat(20.0),
            corner_radius: 0.0,
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap()
}

fn proxies(app: &mut App) -> usize {
    let world = app.world_mut();
    world
        .query::<&Name>()
        .iter(world)
        .filter(|name| name.as_str() == "CanvasShapeProxy")
        .count()
}

#[test]
fn layer_lifecycle_preserves_ids_members_and_explicit_removal_policy() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let base = CanvasDocument::default_layer(&world, document).unwrap();
    let foreground = CanvasDocument::create_layer(&mut world, document, 1, "Foreground").unwrap();
    let accent = CanvasDocument::create_layer(&mut world, document, 1, "Accent").unwrap();
    let a = rectangle(&mut world, document, base);
    let b = rectangle(&mut world, document, foreground);
    CanvasDocument::rename_layer(&mut world, document, foreground, "Details").unwrap();
    CanvasDocument::reorder_layer(&mut world, document, foreground, 0).unwrap();
    assert_eq!(
        CanvasDocument::layer_order(&world, document).unwrap(),
        [foreground, base, accent],
    );
    assert_eq!(
        CanvasDocument::layers(&world, document)
            .unwrap()
            .first()
            .unwrap()
            .name(),
        "Details"
    );
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [b, a]
    );

    CanvasDocument::set_layer_locked(&mut world, document, accent, true).unwrap();
    CanvasDocument::move_item_to_layer(&mut world, document, b, accent).unwrap();
    assert_eq!(world.get::<ChildOf>(b).unwrap().parent(), document);
    assert_eq!(
        CanvasDocument::item_order(&world, document, CanvasContainer::Layer(accent)).unwrap(),
        [b],
    );
    let logs = LogCapture::default();
    for removal in [
        CanvasLayerRemoval::OnlyIfEmpty,
        CanvasLayerRemoval::MoveContentsTo(accent),
    ] {
        assert!(
            logs.run(|| CanvasDocument::remove_layer(&mut world, document, base, removal))
                .is_err()
        );
        assert_eq!(
            CanvasDocument::item_order(&world, document, CanvasContainer::Layer(base)).unwrap(),
            [a],
        );
    }
    CanvasDocument::set_layer_locked(&mut world, document, accent, false).unwrap();
    CanvasDocument::remove_layer(
        &mut world,
        document,
        base,
        CanvasLayerRemoval::MoveContentsTo(accent),
    )
    .unwrap();
    assert_eq!(
        CanvasDocument::default_layer(&world, document).unwrap(),
        foreground
    );
    assert_eq!(
        CanvasDocument::item_order(&world, document, CanvasContainer::Layer(accent)).unwrap(),
        [b, a],
    );
    CanvasDocument::remove_layer(
        &mut world,
        document,
        accent,
        CanvasLayerRemoval::DeleteContents,
    )
    .unwrap();
    assert!(world.get_entity(a).is_ok() && world.get::<ChildOf>(a).is_none());
    assert!(world.get_entity(b).is_ok() && world.get::<ChildOf>(b).is_none());
    assert!(
        logs.run(|| CanvasDocument::remove_layer(
            &mut world,
            document,
            foreground,
            CanvasLayerRemoval::OnlyIfEmpty
        ))
        .is_err()
    );
}

#[test]
fn document_add_observers_see_the_stable_default_layer_id() {
    #[derive(Resource, Default)]
    struct ObservedLayer(Option<CanvasLayerId>);

    let mut world = World::new();
    world.init_resource::<ObservedLayer>();
    world.add_observer(
        |event: On<Add<CanvasDocument>>,
         stacks: Query<&CanvasLayerStack>,
         mut observed: ResMut<ObservedLayer>| {
            observed.0 = stacks
                .get(event.entity)
                .unwrap()
                .layers()
                .first()
                .map(CanvasLayer::id);
        },
    );
    let document = world.spawn(CanvasDocument).id();
    let layer = world.resource::<ObservedLayer>().0.unwrap();
    assert_eq!(
        layer,
        CanvasDocument::default_layer(&world, document).unwrap()
    );
    let item = rectangle(&mut world, document, layer);
    assert_eq!(
        CanvasDocument::item_order(&world, document, CanvasContainer::Layer(layer)).unwrap(),
        [item],
    );
}

#[test]
fn moving_a_malformed_item_does_not_pollute_a_visible_layer() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let base = CanvasDocument::default_layer(&world, document).unwrap();
    let hidden = CanvasDocument::create_layer(&mut world, document, 1, "Hidden").unwrap();
    let visible = rectangle(&mut world, document, base);
    let invalid = rectangle(&mut world, document, hidden);
    CanvasDocument::set_layer_visible(&mut world, document, hidden, false).unwrap();
    world
        .entity_mut(invalid)
        .remove::<(CanvasShape, CanvasGeometry)>();

    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::move_item_to_layer(&mut world, document, invalid, base))
            .is_err()
    );
    assert_eq!(
        CanvasDocument::item_order(&world, document, CanvasContainer::Layer(base)).unwrap(),
        [visible],
    );
    let svg = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(&mut world, document, svg.path(), None).unwrap();
}

#[test]
fn foreign_layer_ids_cannot_move_contents_between_local_layers() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let foreign = world.spawn(CanvasDocument).id();
    let base = CanvasDocument::default_layer(&world, document).unwrap();
    let foreign_base = CanvasDocument::default_layer(&world, foreign).unwrap();
    let source = CanvasDocument::create_layer(&mut world, document, 1, "Source").unwrap();
    let item = rectangle(&mut world, document, source);
    let logs = LogCapture::default();

    assert!(
        logs.run(|| CanvasDocument::remove_layer(
            &mut world,
            document,
            source,
            CanvasLayerRemoval::MoveContentsTo(foreign_base),
        ))
        .is_err()
    );
    assert_eq!(
        CanvasDocument::layer_order(&world, document).unwrap(),
        [base, source]
    );
    assert_eq!(
        CanvasDocument::item_order(&world, document, CanvasContainer::Layer(source)).unwrap(),
        [item],
    );
}

#[test]
fn layers_share_visibility_and_locking_while_active_layers_belong_to_each_view() {
    let pointer = pointer_ids()[0];
    let (mut app, a, b, shape, _, mut at) = fixture(pointer, 1.0, 1.0);
    let doc = CanvasView::document(app.world(), a).unwrap().unwrap();
    let base = CanvasDocument::default_layer(app.world(), doc).unwrap();
    let second = CanvasDocument::create_layer(app.world_mut(), doc, 1, "Second").unwrap();
    CanvasView::set_active_layer_in_world(app.world_mut(), a, second).unwrap();
    assert_eq!(
        CanvasView::active_layer(app.world(), a).unwrap(),
        Some(second)
    );
    assert_eq!(
        CanvasView::active_layer(app.world(), b).unwrap(),
        Some(base)
    );

    CanvasDocument::move_item_to_layer(app.world_mut(), doc, shape, second).unwrap();
    for view in [a, b] {
        CanvasView::set_selection_in_world(app.world_mut(), view, [shape]).unwrap();
    }
    CanvasDocument::set_layer_locked(app.world_mut(), doc, second, true).unwrap();
    for view in [a, b] {
        assert!(CanvasView::selection(app.world(), view).unwrap().is_empty());
        assert_eq!(
            CanvasView::active_layer(app.world(), view).unwrap(),
            Some(base)
        );
    }
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert!(CanvasView::selection(app.world(), a).unwrap().is_empty());
    assert_eq!(proxies(&mut app), 2);
    let svg = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(app.world_mut(), doc, svg.path(), None).unwrap();
    CanvasDocument::set_layer_visible(app.world_mut(), doc, second, false).unwrap();
    update(&mut app);
    assert_eq!(proxies(&mut app), 0);
    let logs = LogCapture::default();
    assert!(
        logs.run(|| CanvasDocument::export_svg(app.world_mut(), doc, svg.path(), None))
            .is_err()
    );
    CanvasDocument::set_layer_visible(app.world_mut(), doc, base, false).unwrap();
    assert_eq!(CanvasView::active_layer(app.world(), a).unwrap(), None);
    CanvasView::set_mode_in_world(
        app.world_mut(),
        a,
        CanvasMode::Rectangle(CanvasPaint::default()),
    )
    .unwrap();
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, Vec2::splat(20.0));
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(
        CanvasDocument::draw_order(app.world(), doc).unwrap(),
        [shape]
    );
    CanvasDocument::set_layer_locked(app.world_mut(), doc, second, false).unwrap();
    assert_eq!(CanvasView::active_layer(app.world(), a).unwrap(), None);
    CanvasDocument::set_layer_visible(app.world_mut(), doc, second, true).unwrap();
    assert_eq!(
        CanvasView::active_layer(app.world(), a).unwrap(),
        Some(second)
    );
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Press(PointerButton::Primary),
    );
    moved(&mut app, pointer, &mut at, Vec2::splat(20.0));
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Release(PointerButton::Primary),
    );
    let drawn = CanvasView::single_selection(app.world(), a)
        .unwrap()
        .unwrap();
    assert_eq!(
        CanvasDocument::item_order(app.world(), doc, CanvasContainer::Layer(second)).unwrap(),
        [shape, drawn],
    );
    assert!(
        CanvasDocument::item_order(app.world(), doc, CanvasContainer::Layer(base))
            .unwrap()
            .is_empty()
    );
    scroll(&mut app, pointer, &at, 1.0);
    assert!(CanvasView::zoom(app.world(), a).unwrap() > 1.0);

    let replacement = app.world_mut().spawn(CanvasDocument).id();
    CanvasDocument::create_layer(app.world_mut(), replacement, 1, "Another foreground").unwrap();
    CanvasView::set_document_in_world(app.world_mut(), a, Some(replacement)).unwrap();
    assert_eq!(
        CanvasView::active_layer(app.world(), a).unwrap(),
        Some(CanvasDocument::default_layer(app.world(), replacement).unwrap()),
    );
    assert_eq!(
        CanvasView::active_layer(app.world(), b).unwrap(),
        Some(second)
    );
}

#[test]
fn removing_hidden_layers_ignores_despawned_order_entries() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let base = CanvasDocument::default_layer(&world, doc).unwrap();
    for removal in [
        CanvasLayerRemoval::OnlyIfEmpty,
        CanvasLayerRemoval::MoveContentsTo(base),
    ] {
        let layer = CanvasDocument::create_layer(&mut world, doc, 1, "Transient").unwrap();
        let stale = rectangle(&mut world, doc, layer);
        CanvasDocument::set_layer_visible(&mut world, doc, layer, false).unwrap();
        world.despawn(stale);
        let kept =
            (removal != CanvasLayerRemoval::OnlyIfEmpty).then(|| rectangle(&mut world, doc, layer));
        CanvasDocument::remove_layer(&mut world, doc, layer, removal).unwrap();
        if let Some(kept) = kept {
            assert_eq!(
                CanvasDocument::item_order(&world, doc, CanvasContainer::Layer(base)).unwrap(),
                [kept]
            );
            let svg = tempfile::NamedTempFile::new().unwrap();
            CanvasDocument::export_svg(&mut world, doc, svg.path(), None).unwrap();
        }
    }
}

#[test]
fn showing_a_layer_prepares_its_surviving_items_for_immediate_export() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let layer = CanvasDocument::create_layer(&mut world, doc, 1, "Details").unwrap();
    let stale = rectangle(&mut world, doc, layer);
    let keep = rectangle(&mut world, doc, layer);
    CanvasDocument::set_layer_visible(&mut world, doc, layer, false).unwrap();
    world.despawn(stale);
    CanvasDocument::set_layer_visible(&mut world, doc, layer, true).unwrap();

    let svg = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(&mut world, doc, svg.path(), None).unwrap();
    assert_eq!(CanvasDocument::draw_order(&world, doc).unwrap(), [keep]);
}

#[test]
fn undo_restores_an_empty_layer_whose_stale_item_already_belonged_elsewhere() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let base = CanvasDocument::default_layer(&world, doc).unwrap();
    let keep = rectangle(&mut world, doc, base);
    let layer = CanvasDocument::create_layer(&mut world, doc, 1, "Details").unwrap();
    let stale = rectangle(&mut world, doc, layer);
    world.entity_mut(stale).insert(ChildOf(other));

    CanvasDocument::remove_layer(&mut world, doc, layer, CanvasLayerRemoval::OnlyIfEmpty).unwrap();
    assert!(CanvasDocument::undo(&mut world, doc).unwrap());
    assert_eq!(
        CanvasDocument::layer_order(&world, doc).unwrap(),
        [base, layer]
    );
    assert_eq!(CanvasDocument::draw_order(&world, doc).unwrap(), [keep]);
    assert_eq!(world.get::<ChildOf>(stale).unwrap().parent(), other);
    assert!(CanvasDocument::redo(&mut world, doc).unwrap());
    assert_eq!(CanvasDocument::layer_order(&world, doc).unwrap(), [base]);
}

#[test]
fn replay_showing_a_layer_prepares_its_surviving_items_for_immediate_export() {
    for redo in [false, true] {
        let mut world = World::new();
        let doc = world.spawn(CanvasDocument).id();
        let layer = CanvasDocument::create_layer(&mut world, doc, 1, "Details").unwrap();
        let stale = rectangle(&mut world, doc, layer);
        let keep = rectangle(&mut world, doc, layer);
        CanvasDocument::set_layer_visible(&mut world, doc, layer, false).unwrap();
        if redo {
            CanvasDocument::set_layer_visible(&mut world, doc, layer, true).unwrap();
            assert!(CanvasDocument::undo(&mut world, doc).unwrap());
        }
        world.despawn(stale);
        if redo {
            assert!(CanvasDocument::redo(&mut world, doc).unwrap());
        } else {
            assert!(CanvasDocument::undo(&mut world, doc).unwrap());
        }
        let svg = tempfile::NamedTempFile::new().unwrap();
        CanvasDocument::export_svg(&mut world, doc, svg.path(), None).unwrap();
        assert_eq!(CanvasDocument::draw_order(&world, doc).unwrap(), [keep]);
    }
}

#[test]
fn replay_cleans_layer_order_before_and_after_queued_transform_observers() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let layer = CanvasDocument::default_layer(&world, doc).unwrap();
    let stale_before = rectangle(&mut world, doc, layer);
    let stale_during = rectangle(&mut world, doc, layer);
    let keep = rectangle(&mut world, doc, layer);
    CanvasDocument::edit(&mut world, doc, |world, token| {
        CanvasDocument::set_layer_visible(world, token, layer, false)?;
        CanvasDocument::update_transform(world, token, keep, Transform::from_xyz(30.0, 0.0, 0.0))
    })
    .unwrap();
    world.despawn(stale_before);
    let observer_svg = tempfile::NamedTempFile::new().unwrap();
    let observer_path = observer_svg.path().to_owned();
    world.add_observer(
        move |event: On<Insert<Transform>>, mut commands: Commands| {
            if event.entity == keep {
                let path = observer_path.clone();
                commands.queue(move |world: &mut World| {
                    CanvasDocument::export_svg(world, doc, &path, None).unwrap();
                });
                commands.entity(stale_during).despawn();
            }
        },
    );

    assert!(CanvasDocument::undo(&mut world, doc).unwrap());
    assert!(!std::fs::read(observer_svg.path()).unwrap().is_empty());
    assert!(world.get_entity(stale_during).is_err());
    let svg = tempfile::NamedTempFile::new().unwrap();
    CanvasDocument::export_svg(&mut world, doc, svg.path(), None).unwrap();
    assert_eq!(CanvasDocument::draw_order(&world, doc).unwrap(), [keep]);
}

#[test]
fn deleting_the_draft_layer_cannot_commit_pen_to_the_fallback_layer() {
    #[derive(Resource, Default)]
    struct CheckedLiveDraft(bool);

    let pointer = pointer_ids()[0];
    let (mut app, view, _, original, _, mut at) = fixture(pointer, 1.0, 1.0);
    let document = CanvasView::document(app.world(), view).unwrap().unwrap();
    let removed = CanvasDocument::create_layer(app.world_mut(), document, 1, "Draft").unwrap();
    let shape = rectangle(app.world_mut(), document, removed);
    CanvasView::set_active_layer_in_world(app.world_mut(), view, removed).unwrap();
    CanvasView::set_mode_in_world(
        app.world_mut(),
        view,
        CanvasMode::Pen(CanvasPaint::default()),
    )
    .unwrap();
    input(
        &mut app,
        pointer,
        &at,
        PointerAction::Move { delta: Vec2::ONE },
    );
    for _ in 0..2 {
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        moved(&mut app, pointer, &mut at, Vec2::splat(20.0));
    }
    assert!(CanvasView::pen_draft(app.world(), view).unwrap().is_some());
    app.init_resource::<CheckedLiveDraft>();
    app.add_observer(move |event: On<Remove<ChildOf>>, mut commands: Commands| {
        if event.entity == shape {
            commands.queue(move |world: &mut World| {
                assert!(CanvasView::pen_draft(world, view).unwrap().is_some());
                assert!(matches!(
                    CanvasView::finish_pen_in_world(world, view),
                    Ok(None)
                ));
                world.resource_mut::<CheckedLiveDraft>().0 = true;
            });
        }
    });

    CanvasDocument::remove_layer(
        app.world_mut(),
        document,
        removed,
        CanvasLayerRemoval::DeleteContents,
    )
    .unwrap();
    assert!(app.world().resource::<CheckedLiveDraft>().0);
    assert_eq!(
        CanvasDocument::draw_order(app.world(), document).unwrap(),
        [original]
    );
    assert!(CanvasView::pen_draft(app.world(), view).unwrap().is_none());
}

#[test]
fn locking_a_layer_from_the_shape_add_observer_preserves_the_successful_tool_commit() {
    let paint = CanvasPaint::default();
    for mode in [CanvasMode::Rectangle(paint), CanvasMode::Pen(paint)] {
        let pointer = pointer_ids()[0];
        let (mut app, view, _, _, _, mut at) = fixture(pointer, 1.0, 1.0);
        let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
        let layer = CanvasDocument::default_layer(app.world(), doc).unwrap();
        app.add_observer(move |_: On<Add<CanvasGeometry>>, mut commands: Commands| {
            commands.queue(move |world: &mut World| -> bevy::prelude::Result {
                CanvasDocument::set_layer_locked(world, doc, layer, true)
                    .map(|_| ())
                    .map_err(|failure| failure.into_bevy_error())
            });
        });
        CanvasView::set_mode_in_world(app.world_mut(), view, mode).unwrap();
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Move { delta: Vec2::ONE },
        );
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Press(PointerButton::Primary),
        );
        if matches!(mode, CanvasMode::Rectangle(_)) {
            moved(&mut app, pointer, &mut at, Vec2::splat(20.0));
        }
        input(
            &mut app,
            pointer,
            &at,
            PointerAction::Release(PointerButton::Primary),
        );
        if matches!(mode, CanvasMode::Pen(_)) {
            moved(&mut app, pointer, &mut at, Vec2::splat(20.0));
            input(
                &mut app,
                pointer,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            input(
                &mut app,
                pointer,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(
                CanvasView::finish_pen_in_world(app.world_mut(), view)
                    .unwrap()
                    .is_some()
            );
        }
        assert_eq!(
            CanvasDocument::draw_order(app.world(), doc).unwrap().len(),
            2
        );
        assert!(CanvasView::selection(app.world(), view).unwrap().is_empty());
        assert_eq!(CanvasView::active_layer(app.world(), view).unwrap(), None);
    }
}
