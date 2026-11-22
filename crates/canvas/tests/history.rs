#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;
use bevy_widgetry_job::JobFailure;

fn kind(error: JobFailure) -> CanvasErrorKind {
    error
        .diagnostics()
        .next()
        .unwrap()
        .source()
        .unwrap()
        .downcast_ref::<CanvasError>()
        .unwrap()
        .kind
}

fn create(world: &mut World, target: impl Into<CanvasEditTarget>) -> Entity {
    CanvasDocument::create_rectangle(
        world,
        target,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::splat(20.0),
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap()
}

#[test]
fn undo_and_redo_restore_the_same_entity_in_a_headless_document() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = create(&mut world, document);
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert!(
        CanvasDocument::draw_order(&world, document)
            .unwrap()
            .is_empty()
    );
    assert!(world.get_entity(shape).is_ok() && world.get::<ChildOf>(shape).is_none());
    assert!(world.get::<ChildOf>(shape).is_none());
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [shape]
    );
    CanvasDocument::remove_shape(&mut world, document, shape).unwrap();
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [shape]
    );
    assert!(world.get::<ChildOf>(shape).is_some());
}

#[test]
fn undo_does_not_orphan_a_shape_when_a_child_removal_observer_closes_the_document() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = create(&mut world, document);
    world.add_observer(move |event: On<Remove<ChildOf>>, mut commands: Commands| {
        if event.entity == shape {
            commands.queue(move |world: &mut World| {
                world.despawn(document);
            });
        }
    });

    assert!(CanvasDocument::undo(&mut world, document).is_err());
    assert!(world.get_entity(document).is_err());
    assert!(world.get_entity(shape).is_err());
}

#[test]
fn closing_a_document_from_a_replayed_transform_cleans_up_the_retained_shape() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = CanvasDocument::edit(&mut world, document, |world, token| {
        let shape = create(world, token);
        CanvasDocument::update_transform(world, token, shape, Transform::from_xyz(3.0, 4.0, 0.0))?;
        Ok(shape)
    })
    .unwrap();
    world.add_observer(
        move |event: On<Insert<Transform>>, mut commands: Commands| {
            if event.entity == shape {
                commands.queue(move |world: &mut World| {
                    world.despawn(document);
                });
            }
        },
    );

    assert!(CanvasDocument::undo(&mut world, document).is_err());
    assert!(world.get_entity(document).is_err());
    assert!(world.get_entity(shape).is_err());
}

#[test]
fn undo_uses_the_surviving_order_after_a_stale_item_is_repaired() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let stale = create(&mut world, document);
    let current = create(&mut world, document);
    world.despawn(stale);
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [current]
    );

    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert!(
        CanvasDocument::draw_order(&world, document)
            .unwrap()
            .is_empty()
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [current]
    );
}

#[test]
fn order_repair_inside_an_edit_keeps_created_items_in_the_replay_patch() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let stale = create(&mut world, document);
    world.despawn(stale);
    let created = CanvasDocument::edit(&mut world, document, |world, token| {
        let created = create(world, token);
        assert_eq!(CanvasDocument::shape_order(world, document)?, [created]);
        Ok(created)
    })
    .unwrap();
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert!(
        CanvasDocument::shape_order(&mut world, document)
            .unwrap()
            .is_empty()
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [created]
    );
}

#[test]
fn reordering_with_stale_entries_does_not_erase_an_earlier_deletion_from_the_edit() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let stale = create(&mut world, document);
    let second = create(&mut world, document);
    let third = create(&mut world, document);
    world.despawn(stale);
    CanvasDocument::edit(&mut world, document, |world, token| {
        CanvasDocument::remove_shape(world, token, first)?;
        assert!(CanvasDocument::reorder_shape(
            world,
            token,
            third,
            CanvasOrder::Back
        )?);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [third, second]
    );
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [first, second, third]
    );
}

#[test]
fn creating_then_deleting_a_whole_subtree_preserves_the_redo_branch() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let original = create(&mut world, document);
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    let temporary = CanvasDocument::edit(&mut world, document, |world, token| {
        let first = create(world, token);
        let second = create(world, token);
        let group = CanvasDocument::group_items(world, token, &[first, second], "Temporary")?;
        let layer = CanvasDocument::create_layer(world, token, 1, "Temporary")?;
        CanvasDocument::move_item_to_layer(world, token, group, layer)?;
        CanvasDocument::remove_layer(world, token, layer, CanvasLayerRemoval::DeleteContents)?;
        Ok([first, second, group])
    })
    .unwrap();
    assert!(
        CanvasDocument::draw_order(&world, document)
            .unwrap()
            .is_empty()
    );
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
    for item in temporary {
        assert!(world.get_entity(item).is_err());
    }
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [original]
    );
}

#[test]
fn undo_reorder_ignores_a_following_item_that_left_the_container() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    let following = create(&mut world, document);
    assert!(
        CanvasDocument::reorder_shape(&mut world, document, second, CanvasOrder::Lower).unwrap()
    );
    world.entity_mut(following).insert(ChildOf(other));
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [second, first]
    );
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [first, second]
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [second, first]
    );
}

#[test]
fn replay_reorder_ignores_removed_neighbors_inside_the_changed_range() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let transferred = create(&mut world, document);
    let moved = create(&mut world, document);
    let last = create(&mut world, document);
    assert!(CanvasDocument::reorder_shape(&mut world, document, moved, CanvasOrder::Back).unwrap());
    world.entity_mut(transferred).insert(ChildOf(other));
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [moved, first, last]
    );
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [first, moved, last]
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [moved, first, last]
    );
    assert_eq!(world.get::<ChildOf>(transferred).unwrap().parent(), other);
}

#[test]
fn replay_rejects_a_changed_item_moved_to_another_document_without_partial_writes() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let transferred = create(&mut world, document);
    let moved = create(&mut world, document);
    let last = create(&mut world, document);
    let transform = Transform::from_xyz(15.0, 0.0, 0.0);
    CanvasDocument::edit(&mut world, document, |world, token| {
        assert!(CanvasDocument::reorder_shape(
            world,
            token,
            moved,
            CanvasOrder::Back
        )?);
        CanvasDocument::update_transform(world, token, transferred, transform)
    })
    .unwrap();
    world.entity_mut(transferred).insert(ChildOf(other));
    assert_eq!(
        CanvasDocument::shape_order(&mut world, document).unwrap(),
        [moved, first, last]
    );

    let error = CanvasDocument::undo(&mut world, document).unwrap_err();
    assert_eq!(
        kind(error),
        CanvasErrorKind::Invalid(CanvasIssue::Ownership)
    );
    assert_eq!(world.get::<Transform>(transferred), Some(&transform));
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [moved, first, last]
    );
    assert!(!CanvasDocument::can_redo(&world, document).unwrap());
}

#[test]
fn undo_reports_ownership_for_a_created_item_moved_to_another_document() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let transferred = create(&mut world, document);
    world.entity_mut(transferred).insert(ChildOf(other));
    assert!(
        CanvasDocument::shape_order(&mut world, document)
            .unwrap()
            .is_empty()
    );

    let error = CanvasDocument::undo(&mut world, document).unwrap_err();
    assert_eq!(
        kind(error),
        CanvasErrorKind::Invalid(CanvasIssue::Ownership)
    );
    assert_eq!(world.get::<ChildOf>(transferred).unwrap().parent(), other);
}

#[test]
fn undo_checks_ownership_before_replaying_a_layer_move() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let shape = create(&mut world, document);
    let layer = CanvasDocument::create_layer(&mut world, document, 1, "Target").unwrap();
    assert!(CanvasDocument::move_item_to_layer(&mut world, document, shape, layer).unwrap());
    world.entity_mut(shape).insert(ChildOf(other));

    let error = CanvasDocument::undo(&mut world, document).unwrap_err();
    assert_eq!(
        kind(error),
        CanvasErrorKind::Invalid(CanvasIssue::Ownership)
    );
    assert_eq!(world.get::<ChildOf>(shape).unwrap().parent(), other);
    assert!(!CanvasDocument::can_redo(&world, document).unwrap());
}

#[test]
fn grouping_and_ungrouping_publish_parent_and_local_transform_together() {
    #[derive(Resource, Default)]
    struct ObservedPositions(Vec<Vec3>);

    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    let initial = Vec3::new(32.0, 0.0, 0.0);
    CanvasDocument::update_transform(
        &mut world,
        document,
        first,
        Transform::from_translation(initial),
    )
    .unwrap();
    CanvasDocument::update_transform(
        &mut world,
        document,
        second,
        Transform::from_xyz(80.0, 0.0, 0.0),
    )
    .unwrap();
    world.init_resource::<ObservedPositions>();
    world.add_observer(
        move |event: On<Insert<Transform>>,
              transforms: Query<&Transform>,
              parents: Query<&ChildOf>,
              mut positions: ResMut<ObservedPositions>| {
            if event.entity == first {
                let local = transforms.get(first).unwrap();
                let parent = transforms
                    .get(parents.get(first).unwrap().parent())
                    .unwrap();
                positions.0.push(parent.transform_point(local.translation));
            }
        },
    );

    let group =
        CanvasDocument::group_items(&mut world, document, &[first, second], "Group").unwrap();
    CanvasDocument::ungroup(&mut world, document, group).unwrap();
    assert_eq!(world.resource::<ObservedPositions>().0, [initial, initial]);
}

#[test]
fn grouping_and_ungrouping_in_one_edit_keeps_members_alive() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let group = CanvasDocument::edit(&mut world, document, |world, token| {
        let group = CanvasDocument::group_items(world, token, &[first, second], "Group")?;
        CanvasDocument::ungroup(world, token, group)?;
        Ok(group)
    })
    .unwrap();
    assert!(world.get_entity(group).is_err());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [first, second]
    );
    assert!(world.get::<Children>(document).unwrap().contains(&first));
    assert!(world.get::<Children>(document).unwrap().contains(&second));
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
}

#[test]
fn replay_publishes_the_restored_local_transform_with_the_parent() {
    #[derive(Resource, Default)]
    struct ObservedPositions(Vec<Vec3>);

    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    let initial = Vec3::new(32.0, 0.0, 0.0);
    CanvasDocument::update_transform(
        &mut world,
        document,
        first,
        Transform::from_translation(initial),
    )
    .unwrap();
    CanvasDocument::update_transform(
        &mut world,
        document,
        second,
        Transform::from_xyz(80.0, 0.0, 0.0),
    )
    .unwrap();
    let group =
        CanvasDocument::group_items(&mut world, document, &[first, second], "Group").unwrap();
    let grouped = world.get::<Transform>(first).unwrap().translation;
    assert_ne!(grouped, initial);

    world.init_resource::<ObservedPositions>();
    world.add_observer(
        move |event: On<Insert<ChildOf>>, world: &World, mut commands: Commands| {
            if event.entity == first {
                let parent = world.get::<ChildOf>(first).unwrap().parent();
                let container = CanvasDocument::container_of(world, document, first).unwrap();
                if parent == document {
                    assert!(matches!(container, CanvasContainer::Layer(_)));
                } else {
                    assert_eq!(container, CanvasContainer::Group(group));
                }
                let translation = world.get::<Transform>(first).unwrap().translation;
                commands.queue(move |world: &mut World| {
                    world
                        .resource_mut::<ObservedPositions>()
                        .0
                        .push(translation);
                });
            }
        },
    );

    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(world.get::<ChildOf>(first).unwrap().parent(), document);
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(world.get::<ChildOf>(first).unwrap().parent(), group);
    assert_eq!(world.resource::<ObservedPositions>().0, [initial, grouped]);
}

#[test]
fn replay_publishes_the_group_transform_after_reattaching_its_members() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    let translation = Vec3::new(32.0, 0.0, 0.0);
    let group = CanvasDocument::edit(&mut world, document, |world, edit| {
        let group = CanvasDocument::group_items(world, edit, &[first, second], "Group")?;
        CanvasDocument::update_group_transform(
            world,
            edit,
            group,
            Transform::from_translation(translation),
        )?;
        Ok(group)
    })
    .unwrap();
    assert!(CanvasDocument::undo(&mut world, document).unwrap());

    world.add_observer(move |event: On<Insert<ChildOf>>, world: &World| {
        if event.entity == first {
            assert_eq!(
                CanvasDocument::document_transform(world, document, first)
                    .unwrap()
                    .transform_point3(Vec3::ZERO),
                translation,
            );
        }
    });
    world.add_observer(move |event: On<Insert<Transform>>, world: &World| {
        if event.entity == group {
            assert_eq!(
                CanvasDocument::document_transform(world, document, first)
                    .unwrap()
                    .transform_point3(Vec3::ZERO),
                translation,
            );
        }
    });
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
}

#[test]
fn scoped_edit_releases_the_writer_when_the_callback_returns_an_error() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let original = create(&mut world, document);
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let error = CanvasDocument::edit(&mut world, document, |world, edit| {
        create(world, edit);
        CanvasDocument::remove_shape(world, edit, Entity::PLACEHOLDER)?;
        Ok(())
    })
    .unwrap_err();
    assert_eq!(
        kind(error),
        CanvasErrorKind::Invalid(CanvasIssue::Ownership)
    );
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [original]
    );
}

#[test]
fn group_observers_repair_order_only_after_all_members_are_attached() {
    #[derive(Resource, Default)]
    struct Orders(Vec<Vec<Entity>>);

    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    world.init_resource::<Orders>();
    world.add_observer(move |_: On<Add<CanvasGroup>>, mut commands: Commands| {
        commands.queue(move |world: &mut World| {
            let order = CanvasDocument::shape_order(world, document).unwrap();
            world.resource_mut::<Orders>().0.push(order);
        });
    });

    let group =
        CanvasDocument::group_items(&mut world, document, &[first, second], "Group").unwrap();
    assert_eq!(world.resource::<Orders>().0, [vec![first, second]]);
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [group, first, second]
    );
    world.add_observer(move |event: On<Remove<ChildOf>>, mut commands: Commands| {
        if event.entity == group {
            commands.queue(move |world: &mut World| {
                let order = CanvasDocument::shape_order(world, document).unwrap();
                world.resource_mut::<Orders>().0.push(order);
            });
        }
    });
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        world.resource::<Orders>().0,
        [vec![first, second], vec![first, second]]
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [group, first, second]
    );
}

#[test]
fn shape_observers_see_matching_geometry_paint_and_transform_during_replay() {
    #[derive(Resource, Default)]
    struct Observed(Vec<(CanvasGeometry, Transform, CanvasPaint)>);

    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = create(&mut world, document);
    let original = (
        world.get::<CanvasGeometry>(shape).unwrap().clone(),
        Transform::default(),
        CanvasPaint::default(),
    );
    world.init_resource::<Observed>();
    world.add_observer(
        move |event: On<Insert<CanvasGeometry>>,
              shapes: Query<(&CanvasGeometry, &Transform, &CanvasPaint)>,
              mut observed: ResMut<Observed>| {
            if event.entity == shape {
                let (geometry, transform, paint) = shapes.get(shape).unwrap();
                observed.0.push((geometry.clone(), *transform, *paint));
            }
        },
    );
    let rectangle = CanvasRectangle {
        size: Vec2::splat(40.0),
        corner_radius: 0.0,
    };
    let transform = Transform::from_xyz(4.0, 3.0, 0.0);
    let paint = CanvasPaint {
        fill: Some(Color::BLACK),
        ..default()
    };
    CanvasDocument::update_rectangle(&mut world, document, shape, rectangle, transform, paint)
        .unwrap();
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    let modified = (CanvasGeometry::Rectangle(rectangle), transform, paint);
    assert_eq!(
        world.resource::<Observed>().0,
        [modified.clone(), original, modified]
    );
}

#[test]
fn removal_observers_see_the_order_after_empty_groups_are_pruned() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let last = create(&mut world, document);
    CanvasDocument::group_items(&mut world, document, &[first, last], "Group").unwrap();
    CanvasDocument::remove_shape(&mut world, document, first).unwrap();

    world.add_observer(move |event: On<Remove<ChildOf>>, mut commands: Commands| {
        if event.entity == last {
            commands.queue(move |world: &mut World| {
                assert!(
                    CanvasDocument::draw_order(world, document)
                        .unwrap()
                        .is_empty()
                );
            });
        }
    });
    CanvasDocument::remove_shape(&mut world, document, last).unwrap();
    create(&mut world, document);
    let order = CanvasDocument::draw_order(&world, document).unwrap();
    assert_eq!(order.len(), 1);
    assert!(world.get::<CanvasShape>(order[0]).is_some());
}

#[test]
fn redoing_batch_deletion_detaches_all_items_without_losing_identity() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    let removed = [first, second];
    CanvasDocument::remove_shapes(&mut world, document, &removed).unwrap();
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        removed
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    for item in removed {
        assert!(world.get_entity(item).is_ok());
        assert!(world.get::<ChildOf>(item).is_none());
    }
    assert!(
        CanvasDocument::draw_order(&world, document)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn explicit_edits_coalesce_and_enforce_exclusive_writes() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let other = world.spawn(CanvasDocument).id();
    let committed_position = Transform::from_xyz(80.0, 0.0, 0.0);
    let (first, second, other_shape, edit) =
        CanvasDocument::edit(&mut world, document, |world, edit| {
            let first = create(world, edit);
            assert_eq!(
                kind(
                    CanvasDocument::update_transform(world, document, first, committed_position)
                        .unwrap_err()
                ),
                CanvasErrorKind::Busy
            );
            assert_eq!(
                kind(CanvasDocument::undo(world, document).unwrap_err()),
                CanvasErrorKind::Busy
            );
            assert_eq!(
                kind(CanvasDocument::redo(world, document).unwrap_err()),
                CanvasErrorKind::Busy
            );
            assert_eq!(
                kind(CanvasDocument::edit(world, document, |_, _| Ok(())).unwrap_err()),
                CanvasErrorKind::Busy
            );
            let second = create(world, edit);
            let other_shape = create(world, other);
            CanvasDocument::update_transforms(
                world,
                edit,
                &[(first, committed_position), (second, committed_position)],
            )
            .unwrap();
            Ok((first, second, other_shape, edit))
        })
        .unwrap();
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert!(!CanvasDocument::can_undo(&world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, other).unwrap(),
        [other_shape]
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [first, second]
    );
    assert_eq!(world.get::<Transform>(first), Some(&committed_position));
    assert_eq!(
        kind(CanvasDocument::remove_shape(&mut world, edit, second).unwrap_err()),
        CanvasErrorKind::Invalid(CanvasIssue::EditToken),
    );
}

#[test]
fn another_world_cannot_use_an_expired_token_with_matching_local_ids() {
    let mut first = World::new();
    let first_document = first.spawn(CanvasDocument).id();
    let foreign = CanvasDocument::edit(&mut first, first_document, |_, token| Ok(token)).unwrap();

    let mut second = World::new();
    let second_document = second.spawn(CanvasDocument).id();
    assert_eq!(first_document, second_document);
    CanvasDocument::edit(&mut second, second_document, |world, token| {
        let shape = create(world, token);
        let error = CanvasDocument::remove_shape(world, foreign, shape).unwrap_err();
        assert_eq!(
            kind(error),
            CanvasErrorKind::Invalid(CanvasIssue::EditToken)
        );
        assert_eq!(CanvasDocument::draw_order(world, second_document)?, [shape]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn history_eviction_and_branch_changes_release_only_unreferenced_tombstones() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    CanvasDocument::set_history_capacity(&mut world, document, 2).unwrap();
    let deleted = create(&mut world, document);
    CanvasDocument::remove_shape(&mut world, document, deleted).unwrap();
    let branch = create(&mut world, document);
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert!(world.get_entity(deleted).is_ok() && world.get::<ChildOf>(deleted).is_none());
    assert!(world.get_entity(branch).is_ok() && world.get::<ChildOf>(branch).is_none());

    let transient = CanvasDocument::edit(&mut world, document, |world, edit| {
        let transient = create(world, edit);
        CanvasDocument::update_transform(
            world,
            edit,
            transient,
            Transform::from_xyz(1.0, 0.0, 0.0),
        )
        .unwrap();
        CanvasDocument::remove_shape(world, edit, transient).unwrap();
        Ok(transient)
    })
    .unwrap();
    assert!(world.get_entity(transient).is_err());
    assert!(CanvasDocument::can_redo(&world, document).unwrap());

    let first = create(&mut world, document);
    assert!(world.get_entity(branch).is_err());
    assert!(world.get_entity(deleted).is_ok() && world.get::<ChildOf>(deleted).is_none());
    let second = create(&mut world, document);
    assert!(world.get_entity(deleted).is_err());
    CanvasDocument::remove_shapes(&mut world, document, &[first, second]).unwrap();
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [first, second]
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    world.despawn(document);
    assert!(world.get_entity(first).is_err());
    assert!(world.get_entity(second).is_err());
}

#[test]
fn replay_restores_nested_group_and_layer_membership_across_compound_edits() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let base = CanvasDocument::default_layer(&world, document).unwrap();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    let third = create(&mut world, document);
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let (layer, inner, outer) = CanvasDocument::edit(&mut world, document, |world, edit| {
        let layer = CanvasDocument::create_layer(world, edit, 1, "Ink").unwrap();
        let inner = CanvasDocument::group_items(world, edit, &[first, second], "Inner").unwrap();
        let outer = CanvasDocument::group_items(world, edit, &[inner, third], "Outer").unwrap();
        CanvasDocument::update_group_transform(
            world,
            edit,
            outer,
            Transform::from_xyz(20.0, 0.0, 0.0),
        )
        .unwrap();
        CanvasDocument::move_item_to_layer(world, edit, outer, layer).unwrap();
        CanvasDocument::rename_layer(world, edit, layer, "Drawing").unwrap();
        CanvasDocument::set_layer_locked(world, edit, layer, true).unwrap();
        CanvasDocument::set_layer_visible(world, edit, layer, false).unwrap();
        CanvasDocument::reorder_layer(world, edit, layer, 0).unwrap();
        Ok((layer, inner, outer))
    })
    .unwrap();

    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [first, second, third]
    );
    assert_eq!(
        CanvasDocument::layer_order(&world, document).unwrap(),
        [base]
    );
    assert!(world.get_entity(outer).is_ok() && world.get::<ChildOf>(outer).is_none());

    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    let layers = CanvasDocument::layers(&world, document).unwrap();
    assert_eq!(layers[0].id(), layer);
    assert_eq!(layers[0].name(), "Drawing");
    assert!(layers[0].locked());
    assert!(!layers[0].visible());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [outer, inner, first, second, third]
    );
    assert_eq!(
        CanvasDocument::container_of(&world, document, outer).unwrap(),
        CanvasContainer::Layer(layer)
    );
    let projection = CanvasDocument::document_transform(&world, document, second).unwrap();

    CanvasDocument::remove_layer(
        &mut world,
        document,
        layer,
        CanvasLayerRemoval::DeleteContents,
    )
    .unwrap();
    assert!(world.get_entity(outer).is_ok() && world.get::<ChildOf>(outer).is_none());
    assert_eq!(world.get::<ChildOf>(inner).unwrap().parent(), outer);
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::document_transform(&world, document, second).unwrap(),
        projection
    );
    CanvasDocument::ungroup(&mut world, document, outer).unwrap();
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [inner, first, second, third]
    );
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(world.get::<ChildOf>(inner).unwrap().parent(), outer);
}

#[test]
fn a_group_created_then_deleted_in_a_transaction_retains_preexisting_children() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let first = create(&mut world, document);
    let second = create(&mut world, document);
    CanvasDocument::clear_history(&mut world, document).unwrap();

    let group = CanvasDocument::edit(&mut world, document, |world, edit| {
        let layer = CanvasDocument::create_layer(world, edit, 1, "Temporary").unwrap();
        let group = CanvasDocument::group_items(world, edit, &[first, second], "Grouped").unwrap();
        CanvasDocument::move_item_to_layer(world, edit, group, layer).unwrap();
        CanvasDocument::remove_layer(world, edit, layer, CanvasLayerRemoval::DeleteContents)
            .unwrap();
        Ok(group)
    })
    .unwrap();

    assert!(world.get_entity(group).is_ok() && world.get::<ChildOf>(group).is_none());
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [first, second]
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert!(
        CanvasDocument::draw_order(&world, document)
            .unwrap()
            .is_empty()
    );
    assert_eq!(world.get::<ChildOf>(first).unwrap().parent(), group);
    CanvasDocument::clear_history(&mut world, document).unwrap();
    assert!(world.get_entity(group).is_err());
    assert!(world.get_entity(first).is_err());
    assert!(world.get_entity(second).is_err());
}

#[test]
fn replay_restores_shape_paint_path_topology_and_order() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let shape = create(&mut world, document);
    let path = CanvasDocument::create_path(
        &mut world,
        document,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::ZERO),
                CanvasPathCommand::LineTo(Vec2::X),
            ],
        },
        Transform::default(),
        CanvasPaint::default(),
    )
    .unwrap();
    let original = world.get::<CanvasGeometry>(path).unwrap().clone();
    CanvasDocument::clear_history(&mut world, document).unwrap();
    let paint = CanvasDocument::edit(&mut world, document, |world, edit| {
        let paint = CanvasPaint {
            fill: Some(Color::BLACK),
            ..default()
        };
        CanvasDocument::update_rectangle(
            world,
            edit,
            shape,
            CanvasRectangle {
                size: Vec2::splat(40.0),
                corner_radius: 0.0,
            },
            Transform::default(),
            paint,
        )
        .unwrap();
        CanvasDocument::append_line(world, edit, path, Vec2::Y, Vec2::ONE).unwrap();
        CanvasDocument::set_path_segment(
            world,
            edit,
            path,
            1,
            CanvasPathSegment::Quadratic { control: Vec2::ONE },
        )
        .unwrap();
        CanvasDocument::reorder_shape(world, edit, shape, CanvasOrder::Front).unwrap();
        Ok(paint)
    })
    .unwrap();
    let final_path = world.get::<CanvasGeometry>(path).unwrap().clone();
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(
        world.get::<CanvasPaint>(shape),
        Some(&CanvasPaint::default())
    );
    assert_eq!(world.get::<CanvasGeometry>(path), Some(&original));
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [shape, path]
    );
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(world.get::<CanvasPaint>(shape), Some(&paint));
    assert_eq!(world.get::<CanvasGeometry>(path), Some(&final_path));
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [path, shape]
    );
}
