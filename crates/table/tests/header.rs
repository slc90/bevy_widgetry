#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

mod common;

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{primary_click, primary_press, primary_release};
use common::{fixture, scroll};

#[derive(Resource, Default)]
struct Requests(Vec<(Entity, WidgetryTableEventKind)>);

fn configure(app: &mut App, source: Entity, root: Entity) -> Vec<WidgetryTableColumnId> {
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let columns = (0..model.column_count())
        .map(|index| model.column_id(index).unwrap())
        .collect::<Vec<_>>();
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableHeaderActions {
            sortable: columns.iter().copied().collect(),
            filterable: columns.iter().copied().collect(),
        });
    app.init_resource::<Requests>().add_observer(
        |event: On<WidgetryTableEvent>, mut requests: ResMut<Requests>| {
            requests.0.push((event.entity, event.kind));
        },
    );
    app.update();
    columns
}

fn button(
    app: &mut App,
    column: WidgetryTableColumnId,
    kind: WidgetryTableHeaderActionKind,
) -> Entity {
    app.world_mut()
        .query::<(Entity, &WidgetryTableHeaderAction)>()
        .iter(app.world())
        .find(|(_, action)| action.column == column && action.kind == kind)
        .unwrap()
        .0
}

fn click(app: &mut App, entity: Entity) {
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().flush();
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().flush();
}

fn label(app: &App, button: Entity) -> bevy::asset::AssetPath<'static> {
    let child = app.world().get::<Children>(button).unwrap()[0];
    app.world()
        .get::<WidgetryIcon>(child)
        .unwrap()
        .path()
        .unwrap()
        .clone()
}

#[test]
fn requests_wait_for_application_commit_and_sort_cycle_uses_confirmed_state() {
    let (mut app, source, root, _) = fixture(20, 10);
    assert!(!common::projection(&mut app, root).is_empty());
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryTableHeaderAction>()
            .iter(app.world())
            .count(),
        0
    );
    let columns = configure(&mut app, source, root);
    let sort = button(&mut app, columns[0], WidgetryTableHeaderActionKind::Sort);
    let original = app
        .world()
        .get::<WidgetryTableViewState>(root)
        .unwrap()
        .ordered_rows()
        .to_vec();
    for expected in [
        Some(WidgetryTableSortDirection::Ascending),
        Some(WidgetryTableSortDirection::Descending),
        None,
        Some(WidgetryTableSortDirection::Ascending),
    ] {
        click(&mut app, sort);
        assert_eq!(
            app.world().resource::<Requests>().0.last(),
            Some(&(
                root,
                WidgetryTableEventKind::SortRequested {
                    column: columns[0],
                    direction: expected
                }
            ))
        );
        assert_eq!(
            app.world()
                .get::<WidgetryTableState>(root)
                .unwrap()
                .selection(),
            WidgetryTableSelection::None
        );
        let before = app
            .world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .sort();
        click(&mut app, sort);
        assert_eq!(
            app.world()
                .get::<WidgetryTableViewState>(root)
                .unwrap()
                .sort(),
            before
        );
        assert_eq!(
            app.world()
                .get::<WidgetryTableViewState>(root)
                .unwrap()
                .ordered_rows(),
            original
        );
        WidgetryTable::<u32>::update_view(
            app.world_mut(),
            root,
            WidgetryTableViewUpdate {
                rows: original.clone(),
                sort: expected.map(|direction| WidgetryTableSort {
                    column: columns[0],
                    direction,
                }),
                filters: default(),
            },
        )
        .unwrap();
        app.update();
        assert_eq!(
            label(&app, sort),
            match expected {
                None => BuiltinIcon::TableSort.path(),
                Some(WidgetryTableSortDirection::Ascending) => BuiltinIcon::ChevronUp.path(),
                Some(WidgetryTableSortDirection::Descending) => BuiltinIcon::ChevronDown.path(),
            }
        );
    }
    let second = button(&mut app, columns[1], WidgetryTableHeaderActionKind::Sort);
    click(&mut app, second);
    assert_eq!(
        app.world().resource::<Requests>().0.last(),
        Some(&(
            root,
            WidgetryTableEventKind::SortRequested {
                column: columns[1],
                direction: Some(WidgetryTableSortDirection::Ascending)
            }
        ))
    );
    WidgetryTable::<u32>::update_view(
        app.world_mut(),
        root,
        WidgetryTableViewUpdate {
            rows: original,
            sort: Some(WidgetryTableSort {
                column: columns[1],
                direction: WidgetryTableSortDirection::Ascending,
            }),
            filters: columns[..2].iter().copied().collect(),
        },
    )
    .unwrap();
    app.update();
    assert_eq!(label(&app, sort), BuiltinIcon::TableSort.path());
    let filter = button(&mut app, columns[0], WidgetryTableHeaderActionKind::Filter);
    assert_eq!(label(&app, filter), BuiltinIcon::TableFilterActive.path());
    let filter2 = button(&mut app, columns[1], WidgetryTableHeaderActionKind::Filter);
    assert_eq!(label(&app, filter2), BuiltinIcon::TableFilterActive.path());
    click(&mut app, filter);
    assert_eq!(
        app.world().resource::<Requests>().0.last(),
        Some(&(root, WidgetryTableEventKind::FilterRequested(columns[0])))
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .filters()
            .len(),
        2
    );
}

#[test]
fn buttons_follow_virtualized_identity_configuration_disabled_and_deleted_columns() {
    let (mut app, source, root, body) = fixture(20, 10);
    let columns = configure(&mut app, source, root);
    let old = button(&mut app, columns[0], WidgetryTableHeaderActionKind::Sort);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    click(&mut app, old);
    assert!(app.world().resource::<Requests>().0.is_empty());
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    scroll(&mut app, body, Vec2::new(900.0, 0.0));
    assert!(app.world().get_entity(old).is_err());
    let target = app
        .world_mut()
        .query::<(Entity, &WidgetryTableHeaderAction)>()
        .iter(app.world())
        .find(|(_, action)| action.kind == WidgetryTableHeaderActionKind::Filter)
        .map(|(entity, action)| (entity, action.column))
        .unwrap();
    click(&mut app, target.0);
    assert_eq!(
        app.world().resource::<Requests>().0.last(),
        Some(&(root, WidgetryTableEventKind::FilterRequested(target.1)))
    );
    scroll(&mut app, body, Vec2::new(-900.0, 0.0));
    let target = button(&mut app, columns[0], WidgetryTableHeaderActionKind::Sort);
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_column(0);
    let count = app.world().resource::<Requests>().0.len();
    click(&mut app, target);
    assert_eq!(app.world().resource::<Requests>().0.len(), count);
    app.update();
    assert!(app.world().get_entity(target).is_err());
    let target = button(&mut app, columns[1], WidgetryTableHeaderActionKind::Sort);
    app.world_mut()
        .get_mut::<WidgetryTableHeaderActions>(root)
        .unwrap()
        .sortable
        .clear();
    click(&mut app, target);
    assert_eq!(app.world().resource::<Requests>().0.len(), count);
    app.update();
    assert!(app.world().get_entity(target).is_err());
    let header = app
        .world_mut()
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(app.world())
        .find(|(_, header)| header.column == columns[1])
        .unwrap()
        .0;
    click(&mut app, header);
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Column(columns[1])
    );
}

#[test]
fn shared_views_route_requests_independently_and_header_replacement_restores_buttons() {
    let (mut app, source, a, _) = fixture(20, 10);
    let columns = configure(&mut app, source, a);
    let config = app
        .world()
        .get::<WidgetryTableHeaderActions>(a)
        .unwrap()
        .clone();
    let b = app.world_mut().spawn_scene(bsn! {
        @WidgetryTable::<u32> { @source: source, @header_actions: {config.clone()} }
        Node { position_type: PositionType::Absolute, left: px(300), width: px(286), height: px(144) }
    }).unwrap().id();
    app.update();
    app.update();
    fn owned_button(app: &mut App, root: Entity, column: WidgetryTableColumnId) -> Entity {
        app.world_mut()
            .query::<(Entity, &WidgetryTableHeaderAction)>()
            .iter(app.world())
            .find(|(entity, action)| {
                if action.column != column || action.kind != WidgetryTableHeaderActionKind::Sort {
                    return false;
                }
                let mut current = *entity;
                while let Some(parent) = app.world().get::<ChildOf>(current) {
                    current = parent.parent();
                    if current == root {
                        return true;
                    }
                }
                false
            })
            .unwrap()
            .0
    }
    let first_a = owned_button(&mut app, a, columns[0]);
    let first_b = owned_button(&mut app, b, columns[0]);
    click(&mut app, first_b);
    assert_eq!(app.world().resource::<Requests>().0.last().unwrap().0, b);
    let rows = app
        .world()
        .get::<WidgetryTableViewState>(a)
        .unwrap()
        .ordered_rows()
        .to_vec();
    WidgetryTable::<u32>::update_view(
        app.world_mut(),
        a,
        WidgetryTableViewUpdate {
            rows: rows.iter().rev().copied().collect(),
            sort: Some(WidgetryTableSort {
                column: columns[0],
                direction: WidgetryTableSortDirection::Descending,
            }),
            filters: [columns[0]].into_iter().collect(),
        },
    )
    .unwrap();
    app.update();
    assert_eq!(label(&app, first_a), BuiltinIcon::ChevronDown.path());
    assert_eq!(label(&app, first_b), BuiltinIcon::TableSort.path());
    let old_sort = app.world().get::<WidgetryTableViewState>(a).unwrap().sort();
    assert!(
        WidgetryTable::<u32>::update_view(
            app.world_mut(),
            a,
            WidgetryTableViewUpdate {
                rows: vec![rows[0], rows[0]],
                sort: None,
                filters: default(),
            }
        )
        .is_err()
    );
    app.update();
    assert_eq!(label(&app, first_a), BuiltinIcon::ChevronDown.path());
    assert_eq!(
        app.world().get::<WidgetryTableViewState>(a).unwrap().sort(),
        old_sort
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .set_header(0, WidgetryTableHeaderValue::new("replacement".to_owned()))
        .unwrap();
    app.update();
    assert!(app.world().get_entity(first_a).is_err());
    let first_a = owned_button(&mut app, a, columns[0]);
    assert_eq!(label(&app, first_a), BuiltinIcon::ChevronDown.path());
    click(&mut app, first_a);
    assert_eq!(
        app.world().resource::<Requests>().0.last(),
        Some(&(
            a,
            WidgetryTableEventKind::SortRequested {
                column: columns[0],
                direction: None
            }
        ))
    );
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    assert_eq!(
        (0..model.row_count())
            .map(|index| model.row_id(index).unwrap())
            .collect::<Vec<_>>(),
        rows
    );
}
