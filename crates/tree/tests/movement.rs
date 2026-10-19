#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{prelude::*, ui::InteractionDisabled};
use bevy_widgetry_tree::*;
use std::collections::HashSet;

fn fixture() -> (App, Entity, [Entity; 2], [Entity; 5]) {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryTreePlugin);
    let root = app.world_mut().spawn_empty().id();
    let a = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let b = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let c = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let leaf = app.world_mut().spawn((WidgetryTreeNode, ChildOf(a))).id();
    let deep = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(leaf)))
        .id();
    let model = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let mut spawn = || {
        app.world_mut()
            .spawn_scene(bsn! { @WidgetryTreeView { @source: model } })
            .unwrap()
            .id()
    };
    let views = [spawn(), spawn()];
    app.update();
    (app, root, views, [a, b, c, leaf, deep])
}

fn movement(
    moving_entity: Entity,
    target_entity: Entity,
    placement: WidgetryTreeDropPlacement,
) -> WidgetryTreeMove {
    WidgetryTreeMove {
        moving_entity,
        target_entity,
        placement,
    }
}

fn children(world: &World, parent: Entity) -> Vec<Entity> {
    world
        .get::<Children>(parent)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

#[test]
fn before_inside_after_write_real_hierarchy_and_preserve_view_local_state() {
    use WidgetryTreeDropPlacement::*;
    let (mut app, root, [view, other], [a, b, c, leaf, deep]) = fixture();
    WidgetryTreeView::expand_all(app.world_mut(), view).unwrap();
    WidgetryTreeView::replace_selection(app.world_mut(), view, [a, c, deep]).unwrap();
    WidgetryTreeView::select(app.world_mut(), other, Some(leaf)).unwrap();
    let expanded = WidgetryTreeView::state(app.world(), view)
        .unwrap()
        .expanded_entities()
        .clone();
    assert!(WidgetryTreeView::move_node(app.world_mut(), view, movement(c, a, Before)).unwrap());
    assert_eq!(children(app.world(), root), vec![c, a, b]);
    assert!(WidgetryTreeView::move_node(app.world_mut(), view, movement(c, b, After)).unwrap());
    assert_eq!(children(app.world(), root), vec![a, b, c]);
    assert!(WidgetryTreeView::move_node(app.world_mut(), view, movement(a, b, Inside)).unwrap());
    assert_eq!(children(app.world(), root), vec![b, c]);
    assert_eq!(children(app.world(), b), vec![a]);
    assert_eq!(app.world().get::<ChildOf>(deep).unwrap().parent(), leaf);
    app.update();
    assert_eq!(
        WidgetryTreeView::selection(app.world(), view)
            .unwrap()
            .selected_ids(),
        &HashSet::from([a, c, deep])
    );
    assert_eq!(
        WidgetryTreeView::selection(app.world(), other)
            .unwrap()
            .selected_ids(),
        &HashSet::from([leaf])
    );
    assert_eq!(
        WidgetryTreeView::state(app.world(), view)
            .unwrap()
            .expanded_entities(),
        &expanded
    );
    assert!(
        WidgetryTreeView::state(app.world(), other)
            .unwrap()
            .expanded_entities()
            .is_empty()
    );
    WidgetryTreeView::expand(app.world_mut(), view, b).unwrap();
    let items = WidgetryTreeView::state(app.world(), view)
        .unwrap()
        .visible_items();
    assert_eq!(
        items
            .iter()
            .map(|i| (i.entity, i.depth))
            .collect::<Vec<_>>(),
        vec![(b, 0), (a, 1), (leaf, 2), (deep, 3), (c, 0)]
    );
    assert_eq!(
        WidgetryTreeView::state(app.world(), other)
            .unwrap()
            .visible_items()
            .iter()
            .map(|i| i.entity)
            .collect::<Vec<_>>(),
        vec![b, c]
    );
}

#[test]
fn invalid_moves_are_atomic_and_report_error_severity() {
    use WidgetryTreeDropPlacement::*;
    for reason in [
        "self",
        "cycle_inside",
        "cycle_before",
        "cycle_after",
        "deleted",
        "foreign",
        "moving_disabled",
        "target_disabled",
        "parent_disabled",
        "unknown",
        "loading",
    ] {
        let (mut app, root, [view, _], [a, b, c, leaf, deep]) = fixture();
        let intent = match reason {
            "self" => movement(a, a, Inside),
            "cycle_inside" => movement(a, deep, Inside),
            "cycle_before" => movement(a, deep, Before),
            "cycle_after" => movement(a, deep, After),
            "deleted" => {
                app.world_mut().despawn(c);
                movement(c, b, Before)
            }
            "foreign" => {
                app.world_mut().entity_mut(c).remove::<ChildOf>();
                movement(c, b, Inside)
            }
            "moving_disabled" => {
                app.world_mut().entity_mut(a).insert(InteractionDisabled);
                movement(a, b, After)
            }
            "target_disabled" => {
                app.world_mut().entity_mut(b).insert(InteractionDisabled);
                movement(c, b, Inside)
            }
            "parent_disabled" => {
                app.world_mut().entity_mut(a).insert(InteractionDisabled);
                movement(c, leaf, Before)
            }
            "unknown" | "loading" => {
                app.world_mut()
                    .entity_mut(b)
                    .insert(if reason == "unknown" {
                        WidgetryTreeChildrenState::Unknown
                    } else {
                        WidgetryTreeChildrenState::Loading
                    });
                movement(c, b, Inside)
            }
            _ => panic!("unexpected case {reason}"),
        };
        let before = [root, a, b, leaf].map(|p| children(app.world(), p));
        let error = WidgetryTreeView::move_node(app.world_mut(), view, intent).unwrap_err();
        assert_eq!(
            error.severity(),
            bevy::ecs::error::Severity::Error,
            "{reason}"
        );
        assert_eq!(
            [root, a, b, leaf].map(|p| children(app.world(), p)),
            before,
            "{reason}"
        );
    }
}

#[test]
fn lazy_boundaries_apply_only_to_inside_and_queue_reads_latest_state() {
    use WidgetryTreeDropPlacement::*;
    for state in [
        WidgetryTreeChildrenState::Unknown,
        WidgetryTreeChildrenState::Loading,
        WidgetryTreeChildrenState::Loaded,
    ] {
        let (mut app, root, [view, _], [a, b, c, _, _]) = fixture();
        app.world_mut().entity_mut(b).insert(state);
        assert!(
            WidgetryTreeView::move_node(app.world_mut(), view, movement(c, b, Before)).unwrap()
        );
        assert_eq!(children(app.world(), root), vec![a, c, b]);
        assert!(WidgetryTreeView::move_node(app.world_mut(), view, movement(c, b, After)).unwrap());
        let result = WidgetryTreeView::move_node(app.world_mut(), view, movement(c, b, Inside));
        assert_eq!(result.is_ok(), state == WidgetryTreeChildrenState::Loaded);
        assert_eq!(
            app.world().get::<WidgetryTreeChildrenState>(b),
            Some(&state)
        );
    }
    let (mut app, root, [view, _], [a, b, c, _, _]) = fixture();
    let capture = bevy_widgetry_test_utils::ErrorCapture::default();
    app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
    app.world_mut()
        .entity_mut(b)
        .insert(WidgetryTreeChildrenState::Loaded);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryTreeView::queue_move_node(
        &mut Commands::new(&mut queue, app.world()),
        view,
        movement(c, b, Inside),
    );
    app.world_mut()
        .entity_mut(b)
        .insert(WidgetryTreeChildrenState::Loading);
    capture.run(|| queue.apply(app.world_mut()));
    assert_eq!(capture.take().len(), 1);
    assert_eq!(children(app.world(), root), vec![a, b, c]);
}

#[test]
fn moves_notify_only_after_hierarchy_commit_and_same_position_is_a_noop() {
    use WidgetryTreeDropPlacement::*;
    #[derive(Resource, Default)]
    struct Moves(usize);
    let (mut app, root, [view, _], [a, b, c, _, _]) = fixture();
    app.init_resource::<Moves>();
    app.add_observer(
        |event: On<WidgetryTreeEvent>, parents: Query<&ChildOf>, mut moves: ResMut<Moves>| {
            if let WidgetryTreeEventKind::NodeMoved {
                movement,
                new_parent,
                ..
            } = event.kind
            {
                assert_eq!(
                    parents.get(movement.moving_entity).unwrap().parent(),
                    new_parent
                );
                moves.0 += 1;
            }
        },
    );
    assert!(!WidgetryTreeView::move_node(app.world_mut(), view, movement(a, b, Before)).unwrap());
    assert_eq!(app.world().resource::<Moves>().0, 0);
    assert!(WidgetryTreeView::move_node(app.world_mut(), view, movement(c, a, Before)).unwrap());
    assert_eq!(app.world().resource::<Moves>().0, 1);
    assert!(!WidgetryTreeView::move_node(app.world_mut(), view, movement(c, a, Before)).unwrap());
    assert!(WidgetryTreeView::move_node(app.world_mut(), view, movement(a, a, Inside)).is_err());
    assert_eq!(app.world().resource::<Moves>().0, 1);
    assert_eq!(children(app.world(), root), vec![c, a, b]);
}

#[test]
fn every_sibling_before_after_pair_matches_original_entity_order() {
    for from in 0..3 {
        for target in 0..3 {
            for placement in [
                WidgetryTreeDropPlacement::Before,
                WidgetryTreeDropPlacement::After,
            ] {
                let (mut app, root, [view, _], [a, b, c, _, _]) = fixture();
                let nodes = [a, b, c];
                if from == target {
                    continue;
                }
                let mut expected = nodes.to_vec();
                let moving = expected.remove(from);
                let insertion = expected.iter().position(|e| *e == nodes[target]).unwrap()
                    + usize::from(placement == WidgetryTreeDropPlacement::After);
                expected.insert(insertion, moving);
                let changed = WidgetryTreeView::move_node(
                    app.world_mut(),
                    view,
                    movement(nodes[from], nodes[target], placement),
                )
                .unwrap();
                assert_eq!(changed, expected != nodes);
                assert_eq!(children(app.world(), root), expected);
            }
        }
    }
}
