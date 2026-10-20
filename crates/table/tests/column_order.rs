#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod common;
use bevy::camera::NormalizedRenderTarget;
use bevy::picking::{
    backend::HitData,
    events::{Pointer, PointerDrag, PointerDragEnd, PointerDragStart},
    pointer::{Location, PointerButton, PointerId},
};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy_widgetry_table::*;

fn header(app: &mut App, root: Entity, column: WidgetryTableColumnId) -> Entity {
    let world = app.world_mut();
    world
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(world)
        .find(|(e, h)| {
            h.column == column
                && world
                    .get::<ChildOf>(*e)
                    .and_then(|p| world.get::<ChildOf>(p.parent()))
                    .and_then(|p| world.get::<ChildOf>(p.parent()))
                    .is_some_and(|p| p.parent() == root)
        })
        .unwrap()
        .0
}

fn pointer(position: Vec2) -> Pointer {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 600,
                height: 400,
            },
            position,
        },
    )
}

fn pointer_at(app: &App, camera: Entity, physical: Vec2) -> Pointer {
    let camera = app.world().get::<Camera>(camera).unwrap();
    let origin = camera.physical_viewport_rect().unwrap().min.as_vec2();
    pointer((physical + origin) / camera.target_scaling_factor().unwrap())
}

fn drag(app: &mut App, header: Entity, end: Vec2) {
    begin_drag(app, header, end);
    end_drag(app, header, end);
}

fn begin_drag(app: &mut App, header: Entity, end: Vec2) {
    let start = app
        .world()
        .get::<UiGlobalTransform>(header)
        .unwrap()
        .translation;
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera>>()
        .single(app.world())
        .unwrap();
    let start_pointer = pointer_at(app, camera, start);
    let end_pointer = pointer_at(app, camera, end);
    let distance = end_pointer.location().position - start_pointer.location().position;
    app.world_mut().trigger(PointerDragStart {
        entity: header,
        pointer: start_pointer,
        button: PointerButton::Primary,
        hit: HitData::new(camera, 0.0, None, None),
    });
    app.world_mut().flush();
    app.world_mut().trigger(PointerDrag {
        entity: header,
        pointer: end_pointer,
        button: PointerButton::Primary,
        distance,
        delta: distance,
    });
    app.world_mut().flush();
}

fn end_drag(app: &mut App, header: Entity, end: Vec2) {
    let start = app
        .world()
        .get::<UiGlobalTransform>(header)
        .unwrap()
        .translation;
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera>>()
        .single(app.world())
        .unwrap();
    let pointer = pointer_at(app, camera, end);
    let distance = pointer.location().position - pointer_at(app, camera, start).location().position;
    app.world_mut().trigger(PointerDragEnd {
        entity: header,
        pointer,
        button: PointerButton::Primary,
        distance,
    });
    app.world_mut().flush();
}

fn enabled_fixture(rows: u32, columns: u32) -> (App, Entity, Entity, Entity) {
    let (mut app, source, root, body) = common::fixture(rows, columns);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(590);
    app.world_mut()
        .get_mut::<WidgetryTableColumnReordering>(root)
        .unwrap()
        .enabled = true;
    app.update();
    app.update();
    (app, source, root, body)
}

fn ids(app: &App, source: Entity) -> Vec<WidgetryTableColumnId> {
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    (0..model.column_count())
        .map(|i| model.column_id(i).unwrap())
        .collect()
}

fn order(app: &App, root: Entity) -> Vec<WidgetryTableColumnId> {
    app.world()
        .get::<WidgetryTableViewState>(root)
        .unwrap()
        .ordered_columns()
        .to_vec()
}

fn swap_first(app: &mut App, root: Entity, columns: &[WidgetryTableColumnId]) {
    let first = header(app, root, columns[0]);
    let second = header(app, root, columns[1]);
    let end = app
        .world()
        .get::<UiGlobalTransform>(first)
        .unwrap()
        .translation
        - Vec2::new(40.0, 0.0);
    drag(app, second, end);
}

#[derive(Resource, Default)]
struct Orders(Vec<(Entity, Vec<WidgetryTableColumnId>)>);

fn observe_order(
    event: On<WidgetryTableEvent>,
    states: Query<&WidgetryTableViewState>,
    mut orders: ResMut<Orders>,
) {
    if event.kind == WidgetryTableEventKind::ColumnOrderChanged {
        orders.0.push((
            event.entity,
            states.get(event.entity).unwrap().ordered_columns().to_vec(),
        ));
    }
}

#[test]
fn shared_views_preserve_schema_preferences_and_reset_commits_before_notifications() {
    let (mut app, source, root, _) = enabled_fixture(5, 4);
    let columns = ids(&app, source);
    let other=app.world_mut().spawn_scene(bsn!{@WidgetryTable::<u32>{@source:source} Node{position_type:PositionType::Absolute,top:px(180),width:px(590),height:px(144)}}).unwrap().id();
    app.init_resource::<Orders>().add_observer(observe_order);
    app.update();
    app.update();
    assert!(!WidgetryTable::<u32>::reset_column_order(app.world_mut(), root).unwrap());
    swap_first(&mut app, root, &columns);
    let manual = vec![columns[1], columns[0], columns[2], columns[3]];
    assert_eq!(order(&app, root), manual);
    assert_eq!(
        app.world().resource::<Orders>().0,
        vec![(root, manual.clone())]
    );
    assert_eq!(order(&app, other), columns);
    let rows = app
        .world()
        .get::<WidgetryTableViewState>(root)
        .unwrap()
        .ordered_rows()
        .iter()
        .rev()
        .copied()
        .collect();
    WidgetryTable::<u32>::update_view(
        app.world_mut(),
        root,
        WidgetryTableViewUpdate { rows, ..default() },
    )
    .unwrap();
    let added = {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryTableModel<u32>>(source)
            .unwrap();
        *model.row_mut(0).unwrap().unwrap() = 99;
        assert!(model.move_column(0, 3));
        model
            .insert_column(
                0,
                WidgetryTableColumn::new(
                    WidgetryTableHeaderValue::new("new".to_string()),
                    9,
                    |r: &u32, c: &u32| WidgetryTableCellValue::new(format!("{r}/{c}")),
                ),
            )
            .unwrap()
    };
    app.update();
    app.update();
    let mut expected = manual;
    expected.push(added);
    assert_eq!(order(&app, root), expected);
    assert_eq!(order(&app, other), ids(&app, source));
    let remove = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_index(columns[2])
        .unwrap();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_column(remove);
    app.update();
    app.update();
    expected.retain(|c| *c != columns[2]);
    assert_eq!(order(&app, root), expected);
    WidgetryTable::<u32>::queue_reset_column_order(&mut app.world_mut().commands(), root);
    assert!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .has_manual_column_order()
    );
    app.world_mut().flush();
    assert_eq!(order(&app, root), ids(&app, source));
    assert!(!WidgetryTable::<u32>::reset_column_order(app.world_mut(), root).unwrap());
    assert_eq!(app.world().resource::<Orders>().0.len(), 2);
    app.update();
    app.update();
    let current = order(&app, root);
    swap_first(&mut app, root, &current);
    assert!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .has_manual_column_order()
    );
    assert_eq!(order(&app, other), ids(&app, source));
}

#[test]
fn model_instance_replacement_clears_manual_ids_and_renders_new_source_values() {
    let (mut app, source, root, _) = enabled_fixture(5, 4);
    let columns = ids(&app, source);
    swap_first(&mut app, root, &columns);
    app.update();
    let mut model = WidgetryTableModel::<u32>::default();
    model.push_row(77).unwrap();
    for c in 0..4 {
        model
            .push_column(WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new(format!("N{c}")),
                c,
                |r: &u32, c: &u32| WidgetryTableCellValue::new(format!("new:{r}/{c}")),
            ))
            .unwrap();
    }
    app.world_mut().entity_mut(source).insert(model);
    app.update();
    app.update();
    assert_eq!(order(&app, root), ids(&app, source));
    assert!(
        !app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .has_manual_column_order()
    );
    for ((_, column), (_, text)) in common::projection(&mut app, root) {
        let index = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_index(column)
            .unwrap();
        assert_eq!(text, format!("new:77/{index}"));
    }
}

#[test]
fn ranges_navigation_and_tsv_use_display_columns_while_values_keep_model_identity() {
    use bevy_widgetry_test_utils::press_key;
    let (mut app, source, root, _) = enabled_fixture(4, 4);
    let columns = ids(&app, source);
    bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
    swap_first(&mut app, root, &columns);
    app.update();
    app.update();
    app.register_table_copy_formatter(WidgetryTableCopyFormatter::new(|s: &String| Ok(s.clone())));
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let row = model.row_id(0).unwrap();
    let last = model.row_id(3).unwrap();
    WidgetryTable::<u32>::set_selection(
        app.world_mut(),
        root,
        WidgetryTableSelection::Cells {
            anchor: WidgetryTableCell {
                row,
                column: columns[1],
            },
            end: WidgetryTableCell {
                row: last,
                column: columns[2],
            },
        },
    )
    .unwrap();
    assert_eq!(
        WidgetryTable::<u32>::selection_range(app.world(), root)
            .unwrap()
            .unwrap()
            .columns(),
        0..3
    );
    let tsv = WidgetryTable::<u32>::selection_tsv(app.world_mut(), root)
        .unwrap()
        .unwrap();
    assert_eq!(
        tsv,
        "0/1\t0/0\t0/2\n1/1\t1/0\t1/2\n2/1\t2/0\t2/2\n3/1\t3/0\t3/2"
    );
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(root, bevy::input_focus::FocusCause::Navigated);
    WidgetryTable::<u32>::set_focused_cell(
        app.world_mut(),
        root,
        Some(WidgetryTableCell {
            row,
            column: columns[1],
        }),
    )
    .unwrap();
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    press_key(&mut app, window, KeyCode::ArrowRight);
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(WidgetryTableCell {
            row,
            column: columns[0]
        })
    );
    WidgetryTable::<u32>::update_view(
        app.world_mut(),
        root,
        WidgetryTableViewUpdate {
            rows: vec![last, row],
            ..default()
        },
    )
    .unwrap();
    WidgetryTable::<u32>::set_selection(app.world_mut(), root, WidgetryTableSelection::All)
        .unwrap();
    assert_eq!(
        WidgetryTable::<u32>::selection_tsv(app.world_mut(), root)
            .unwrap()
            .unwrap(),
        "3/1\t3/0\t3/2\t3/3\n0/1\t0/0\t0/2\t0/3"
    );
}

#[test]
fn cancellation_disabled_invalid_drop_and_virtualization_release_capture_without_reordering() {
    use bevy::picking::{events::PointerCancel, hover::PointerCaptureMap};
    use bevy::ui::InteractionDisabled;
    for reason in 0..8 {
        let (mut app, source, root, body) = enabled_fixture(5, 20);
        let columns = ids(&app, source);
        let target = header(&mut app, root, columns[0]);
        let endpoint = app
            .world()
            .get::<UiGlobalTransform>(target)
            .unwrap()
            .translation
            + Vec2::new(150.0, 0.0);
        let selection = WidgetryTableSelection::Columns {
            anchor: columns[0],
            end: columns[2],
        };
        WidgetryTable::<u32>::set_selection(app.world_mut(), root, selection).unwrap();
        begin_drag(&mut app, target, endpoint);
        assert!(
            app.world()
                .resource::<PointerCaptureMap>()
                .is_captured(&PointerId::Mouse)
        );
        assert_eq!(app.world().get::<Children>(root).unwrap().len(), 7);
        assert_eq!(order(&app, root), columns);
        match reason {
            0 => {
                app.world_mut().trigger(PointerCancel {
                    entity: target,
                    pointer: pointer(endpoint),
                    hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                });
                app.world_mut().flush();
            }
            1 => {
                app.world_mut().entity_mut(root).insert(InteractionDisabled);
                app.update();
            }
            2 => {
                app.world_mut()
                    .get_mut::<WidgetryTableColumnReordering>(root)
                    .unwrap()
                    .enabled = false;
                app.update();
            }
            3 => {
                end_drag(&mut app, target, Vec2::new(200.0, 250.0));
            }
            4 => {
                app.world_mut()
                    .get_mut::<WidgetryTableModel<u32>>(source)
                    .unwrap()
                    .remove_column(0);
                app.update();
            }
            5 => {
                common::scroll(&mut app, body, Vec2::new(1500.0, 0.0));
                app.update();
                app.update();
            }
            6 => {
                assert!(!WidgetryTable::<u32>::reset_column_order(app.world_mut(), root).unwrap());
            }
            _ => {
                app.world_mut().despawn(root);
                app.world_mut().flush();
            }
        }
        assert!(
            !app.world()
                .resource::<PointerCaptureMap>()
                .is_captured(&PointerId::Mouse),
            "reason {reason}"
        );
        if app.world().entities().contains(root) {
            assert!(
                app.world()
                    .get::<bevy_widgetry_cursor::CursorLease>(root)
                    .is_none()
            );
            assert!(
                !app.world()
                    .get::<WidgetryTableViewState>(root)
                    .unwrap()
                    .has_manual_column_order()
            );
            assert_eq!(app.world().get::<Children>(root).unwrap().len(), 6);
            if reason != 4 {
                assert_eq!(
                    app.world()
                        .get::<WidgetryTableState>(root)
                        .unwrap()
                        .selection(),
                    selection
                );
            }
        }
    }
}

#[test]
fn reset_rejects_wrong_views_and_queued_validation_uses_the_current_source() {
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    let (mut app, source, root, _) = enabled_fixture(4, 4);
    let columns = ids(&app, source);
    swap_first(&mut app, root, &columns);
    let capture = LogCapture::default();
    let error = capture
        .run(|| WidgetryTable::<u64>::reset_column_order(app.world_mut(), root))
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert!(error.to_string().contains("live View"));
    assert_eq!(capture.records().len(), 1);
    assert_eq!(
        order(&app, root),
        vec![columns[1], columns[0], columns[2], columns[3]]
    );
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    {
        let mut commands = app.world_mut().commands();
        commands.entity(source).despawn();
        WidgetryTable::<u32>::queue_reset_column_order(&mut commands, root);
    }
    errors.run(|| capture.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(
        order(&app, root),
        vec![columns[1], columns[0], columns[2], columns[3]]
    );
}

#[test]
fn threshold_header_actions_and_resize_strip_keep_their_separate_meanings() {
    use bevy::picking::hover::PointerCaptureMap;
    use bevy_widgetry_test_utils::primary_click;
    let (mut app, source, root, _) = enabled_fixture(4, 4);
    let columns = ids(&app, source);
    let target = header(&mut app, root, columns[0]);
    let center = app
        .world()
        .get::<UiGlobalTransform>(target)
        .unwrap()
        .translation;
    drag(&mut app, target, center + Vec2::new(2.0, 0.0));
    assert_eq!(order(&app, root), columns);
    assert!(
        !app.world()
            .resource::<PointerCaptureMap>()
            .is_captured(&PointerId::Mouse)
    );
    app.world_mut().trigger(primary_click(target));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .selection(),
        WidgetryTableSelection::Column(columns[0])
    );
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableHeaderActions {
            sortable: [columns[0]].into_iter().collect(),
            filterable: [columns[0]].into_iter().collect(),
        });
    app.update();
    app.update();
    let actions = app
        .world_mut()
        .query::<(Entity, &WidgetryTableHeaderAction)>()
        .iter(app.world())
        .map(|(e, _)| e)
        .collect::<Vec<_>>();
    for action in actions {
        drag(&mut app, action, center + Vec2::new(140.0, 0.0));
        assert_eq!(order(&app, root), columns);
    }
    let handle = app
        .world()
        .get::<Children>(target)
        .unwrap()
        .iter()
        .find(|&e| {
            app.world()
                .get::<Node>(e)
                .is_some_and(|n| n.width == px(6) && n.right == px(0))
        })
        .unwrap();
    let end = app
        .world()
        .get::<UiGlobalTransform>(handle)
        .unwrap()
        .translation
        + Vec2::new(40.0, 0.0);
    drag(&mut app, handle, end);
    assert_eq!(order(&app, root), columns);
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .column_widths()
            .get(&columns[0]),
        Some(&WidgetryTableColumnWidth::Fixed(160.0))
    );
}

#[test]
fn real_pointer_release_commits_once_without_turning_drag_into_column_selection() {
    use bevy::picking::hover::PointerCaptureMap;
    use bevy::picking::pointer::PointerAction;
    use bevy_widgetry_test_utils::queue_pointer;
    for same_frame in [false, true] {
        let (mut app, source, root, _) = enabled_fixture(5, 4);
        let columns = ids(&app, source);
        app.init_resource::<Orders>().add_observer(observe_order);
        let selection = WidgetryTableSelection::Columns {
            anchor: columns[0],
            end: columns[2],
        };
        WidgetryTable::<u32>::set_selection(app.world_mut(), root, selection).unwrap();
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: (600, 400).into(),
                    ..default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();
        let camera = app
            .world_mut()
            .query_filtered::<Entity, With<Camera>>()
            .single(app.world())
            .unwrap();
        let target = bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window));
        app.world_mut().entity_mut(camera).insert(target.clone());
        app.world_mut()
            .entity_mut(root)
            .insert(UiTargetCamera(camera));
        app.world_mut()
            .resource_mut::<bevy::picking::input::PointerInputSettings>()
            .is_mouse_enabled = false;
        app.update();
        app.update();
        let first = header(&mut app, root, columns[0]);
        let second = header(&mut app, root, columns[1]);
        let start = Location {
            target: target.normalize(None).unwrap(),
            position: app
                .world()
                .get::<UiGlobalTransform>(second)
                .unwrap()
                .translation,
        };
        let end = Location {
            position: app
                .world()
                .get::<UiGlobalTransform>(first)
                .unwrap()
                .translation
                - Vec2::new(40.0, 0.0),
            ..start.clone()
        };
        queue_pointer(
            &mut app,
            PointerId::Mouse,
            start.clone(),
            PointerAction::Move {
                delta: start.position,
            },
        );
        app.update();
        queue_pointer(
            &mut app,
            PointerId::Mouse,
            start.clone(),
            PointerAction::Press(PointerButton::Primary),
        );
        app.update();
        queue_pointer(
            &mut app,
            PointerId::Mouse,
            end.clone(),
            PointerAction::Move {
                delta: end.position - start.position,
            },
        );
        if same_frame {
            queue_pointer(
                &mut app,
                PointerId::Mouse,
                end.clone(),
                PointerAction::Release(PointerButton::Primary),
            );
        }
        app.update();
        if !same_frame {
            assert!(
                app.world()
                    .resource::<PointerCaptureMap>()
                    .is_captured(&PointerId::Mouse)
            );
            assert!(
                app.world()
                    .get::<bevy_widgetry_cursor::CursorLease>(root)
                    .is_some()
            );
            queue_pointer(
                &mut app,
                PointerId::Mouse,
                end,
                PointerAction::Release(PointerButton::Primary),
            );
            app.update();
        }
        app.update();
        assert_eq!(
            order(&app, root),
            vec![columns[1], columns[0], columns[2], columns[3]]
        );
        assert_eq!(app.world().resource::<Orders>().0.len(), 1);
        assert_eq!(
            app.world()
                .get::<WidgetryTableState>(root)
                .unwrap()
                .selection(),
            selection
        );
        assert!(
            !app.world()
                .resource::<PointerCaptureMap>()
                .is_captured(&PointerId::Mouse)
        );
        assert!(
            app.world()
                .get::<bevy_widgetry_cursor::CursorLease>(root)
                .is_none()
        );
    }
}

#[test]
fn scaled_scrolled_headers_use_view_geometry_and_keep_offscreen_order_bounded() {
    use bevy::ui::ScrollPosition;
    for (dpi, scale) in [(1.0, 1.0), (1.0, 1.25), (2.0, 1.0), (2.0, 1.5)] {
        let (mut app, source, root, body) = enabled_fixture(2000, 40);
        let columns = ids(&app, source);
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        let camera = app
            .world_mut()
            .query_filtered::<Entity, With<Camera>>()
            .single(app.world())
            .unwrap();
        let mut camera = app.world_mut().get_mut::<Camera>(camera).unwrap();
        camera.computed.target_info.as_mut().unwrap().scale_factor = dpi;
        camera.viewport = Some(bevy::camera::Viewport {
            physical_position: UVec2::new(20, 10),
            physical_size: UVec2::new(580, 390),
            ..default()
        });
        app.world_mut()
            .get_mut::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns = 1;
        common::scroll(&mut app, body, Vec2::new(600.3, 500.7));
        app.update();
        app.update();
        app.update();
        let fixed = header(&mut app, root, columns[0]);
        let scrolling = header(&mut app, root, columns[6]);
        let end = app
            .world()
            .get::<UiGlobalTransform>(fixed)
            .unwrap()
            .translation
            - Vec2::new(40.0, 0.0);
        drag(&mut app, scrolling, end);
        app.update();
        app.update();
        assert_eq!(order(&app, root)[0], columns[6]);
        assert_eq!(
            app.world()
                .get::<WidgetryTableLayout>(root)
                .unwrap()
                .frozen_columns,
            1
        );
        assert!(app.world().get::<ScrollPosition>(body).unwrap().0.y > 490.0);
        assert!(common::projection(&mut app, root).len() < 200);
        let new = header(&mut app, root, columns[6]);
        let canvas = app.world().get::<ChildOf>(new).unwrap().parent();
        let region = app.world().get::<ChildOf>(canvas).unwrap().parent();
        assert!(
            app.world()
                .get::<WidgetryTableFrozenColumnHeaders>(region)
                .is_some()
        );
    }
}

#[test]
fn header_drag_is_opt_in_and_moves_only_the_view_across_the_frozen_boundary() {
    let (mut app, source, root, body) = common::fixture(5, 4);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(590);
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .frozen_columns = 1;
    app.update();
    app.update();
    let ids = (0..4)
        .map(|i| {
            app.world()
                .get::<WidgetryTableModel<u32>>(source)
                .unwrap()
                .column_id(i)
                .unwrap()
        })
        .collect::<Vec<_>>();
    let second = header(&mut app, root, ids[1]);
    let first = header(&mut app, root, ids[0]);
    let end = app
        .world()
        .get::<UiGlobalTransform>(first)
        .unwrap()
        .translation
        - Vec2::new(50.0, 0.0);
    drag(&mut app, second, end);
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .ordered_columns(),
        ids
    );
    app.world_mut()
        .get_mut::<WidgetryTableColumnReordering>(root)
        .unwrap()
        .enabled = true;
    drag(&mut app, second, end);
    assert_eq!(
        app.world()
            .get::<WidgetryTableViewState>(root)
            .unwrap()
            .ordered_columns(),
        &[ids[1], ids[0], ids[2], ids[3]]
    );
    app.update();
    app.update();
    let moved = header(&mut app, root, ids[1]);
    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(moved)
            .unwrap()
            .translation,
        app.world()
            .get::<UiGlobalTransform>(first)
            .unwrap()
            .translation
            - Vec2::new(120.0, 0.0)
    );
    let canvas = app.world().get::<ChildOf>(moved).unwrap().parent();
    let region = app.world().get::<ChildOf>(canvas).unwrap().parent();
    assert!(
        app.world()
            .get::<WidgetryTableFrozenColumnHeaders>(region)
            .is_some()
    );
    assert_eq!(
        (0..4)
            .map(|i| app
                .world()
                .get::<WidgetryTableModel<u32>>(source)
                .unwrap()
                .column_id(i)
                .unwrap())
            .collect::<Vec<_>>(),
        ids
    );
    common::scroll(&mut app, body, Vec2::new(-120.0, -28.0));
    app.update();
    app.update();
    let projected = common::projection(&mut app, root);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    for ((row, column), (_, text)) in projected {
        assert_eq!(
            &text,
            model
                .cell(row, column)
                .unwrap()
                .downcast_ref::<String>()
                .unwrap()
        );
    }
}
