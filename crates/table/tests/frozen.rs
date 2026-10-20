#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod common;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy_widgetry_table::*;

fn part<C: Component>(world: &World, root: Entity) -> Entity {
    world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|&e| world.get::<C>(e).is_some())
        .unwrap()
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

#[test]
fn frozen_flexible_width_updates_compare_against_the_complete_viewport() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(400);
    {
        let mut layout = app
            .world_mut()
            .get_mut::<WidgetryTableLayout>(root)
            .unwrap();
        layout.default_column_width = WidgetryTableColumnWidth::Flexible(1.0);
        layout.frozen_columns = 1;
    }
    settle(&mut app);
    let first = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(0)
        .unwrap();
    let frozen = part::<WidgetryTableFrozenBody>(app.world(), root);
    let computed = app.world().get::<ComputedNode>(frozen).unwrap();
    let width = computed.size().x * computed.inverse_scale_factor();
    assert!(
        WidgetryTable::<u32>::set_column_width(app.world_mut(), root, first, width / 2.0).unwrap()
    );
    settle(&mut app);
    let computed = app.world().get::<ComputedNode>(frozen).unwrap();
    assert_eq!(
        computed.unrounded_size().x * computed.inverse_scale_factor(),
        width / 2.0
    );
}

#[test]
fn resized_frozen_text_is_laid_out_and_clipped_in_the_first_update() {
    use bevy::text::TextLayoutInfo;
    use bevy::ui::CalculatedClip;
    use bevy_widgetry_core::WidgetryAppExt;
    let (mut app, source, root, body) = common::fixture(100, 20);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(bevy_widgetry_asset::BuiltinFont::Default.path());
    bevy_widgetry_test_utils::advance_until(
        &mut app,
        std::time::Duration::from_secs(10),
        "Frozen Table font",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .frozen_columns = 2;
    settle(&mut app);
    common::scroll(&mut app, body, Vec2::new(500.0, 600.0));
    settle(&mut app);
    let first = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(0)
        .unwrap();
    for width in [180.0, 200.0, 240.0] {
        WidgetryTable::<u32>::set_column_width(app.world_mut(), root, first, width).unwrap();
        app.update();
        let cells = owned_cells(&mut app, root);
        for (shell, _) in cells.iter().filter(|(_, c)| c.column == first) {
            let text = app.world().get::<Children>(*shell).unwrap()[0];
            let center = app
                .world()
                .get::<UiGlobalTransform>(text)
                .unwrap()
                .translation;
            let shell_center = app
                .world()
                .get::<UiGlobalTransform>(*shell)
                .unwrap()
                .translation;
            if app
                .world()
                .get::<CalculatedClip>(*shell)
                .unwrap()
                .contains_point(shell_center)
            {
                assert!(
                    !app.world()
                        .get::<TextLayoutInfo>(text)
                        .unwrap()
                        .glyphs
                        .is_empty()
                );
                assert!(
                    app.world()
                        .get::<CalculatedClip>(text)
                        .unwrap()
                        .contains_point(center)
                );
            }
        }
    }
}

fn owned_cells(app: &mut App, root: Entity) -> Vec<(Entity, WidgetryTableCell)> {
    let world = app.world_mut();
    world
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(world)
        .filter(|(entity, _)| {
            let canvas = world.get::<ChildOf>(*entity).unwrap().parent();
            let viewport = world.get::<ChildOf>(canvas).unwrap().parent();
            world.get::<ChildOf>(viewport).unwrap().parent() == root
        })
        .map(|(e, c)| (e, *c))
        .collect()
}

#[test]
fn rectangle_drag_crosses_frozen_boundary_with_horizontal_scroll_and_releases_capture() {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerDrag, PointerDragEnd, PointerDragStart},
        hover::PointerCaptureMap,
        pointer::{Location, PointerButton, PointerId},
    };
    for scale in [1.0, 1.5] {
        let (mut app, source, root, body) = common::fixture(100, 20);
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(400);
        app.world_mut()
            .get_mut::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns = 1;
        settle(&mut app);
        common::scroll(&mut app, body, Vec2::new(600.0, 0.0));
        settle(&mut app);
        let first = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_id(0)
            .unwrap();
        let cells = owned_cells(&mut app, root);
        let (anchor, cell) = cells
            .iter()
            .filter(|(_, c)| c.column == first)
            .min_by_key(|(_, c)| format!("{:?}", c.row))
            .unwrap();
        let end = cells
            .iter()
            .filter(|(_, c)| c.row == cell.row && c.column != first)
            .max_by(|(a, _), (b, _)| {
                app.world()
                    .get::<UiGlobalTransform>(*a)
                    .unwrap()
                    .translation
                    .x
                    .total_cmp(
                        &app.world()
                            .get::<UiGlobalTransform>(*b)
                            .unwrap()
                            .translation
                            .x,
                    )
            })
            .unwrap();
        let start = app
            .world()
            .get::<UiGlobalTransform>(*anchor)
            .unwrap()
            .translation;
        let endpoint = app
            .world()
            .get::<UiGlobalTransform>(end.0)
            .unwrap()
            .translation;
        let camera = app
            .world_mut()
            .query_filtered::<Entity, With<Camera>>()
            .single(app.world())
            .unwrap();
        let pointer = |position| {
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
        };
        app.world_mut().trigger(PointerDragStart {
            entity: *anchor,
            pointer: pointer(start),
            button: PointerButton::Primary,
            hit: HitData::new(camera, 0.0, None, None),
        });
        app.world_mut().flush();
        app.world_mut().trigger(PointerDrag {
            entity: *anchor,
            pointer: pointer(endpoint),
            button: PointerButton::Primary,
            distance: endpoint - start,
            delta: endpoint - start,
        });
        app.world_mut().flush();
        assert_eq!(
            app.world()
                .get::<WidgetryTableState>(root)
                .unwrap()
                .selection(),
            WidgetryTableSelection::Cells {
                anchor: *cell,
                end: end.1
            }
        );
        assert!(
            app.world()
                .resource::<PointerCaptureMap>()
                .is_captured(&PointerId::Mouse)
        );
        app.world_mut().trigger(PointerDragEnd {
            entity: *anchor,
            pointer: pointer(endpoint),
            button: PointerButton::Primary,
            distance: endpoint - start,
        });
        app.world_mut().flush();
        assert!(
            !app.world()
                .resource::<PointerCaptureMap>()
                .is_captured(&PointerId::Mouse)
        );
    }
}

#[test]
fn resize_handles_in_both_regions_keep_column_identity_and_configuration() {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerDrag, PointerDragEnd, PointerDragStart},
        pointer::{Location, PointerButton, PointerId},
    };
    for column_index in [0, 1] {
        let (mut app, source, root, _) = common::fixture(100, 20);
        app.world_mut()
            .get_mut::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns = 1;
        settle(&mut app);
        let column = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_id(column_index)
            .unwrap();
        let header = app
            .world_mut()
            .query::<(Entity, &WidgetryTableColumnHeader)>()
            .iter(app.world())
            .find(|(_, h)| h.column == column)
            .unwrap()
            .0;
        let handle = app
            .world()
            .get::<Children>(header)
            .unwrap()
            .iter()
            .find(|&e| {
                app.world()
                    .get::<Node>(e)
                    .is_some_and(|n| n.width == px(6) && n.right == px(0))
            })
            .unwrap();
        let pointer = || {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 600,
                        height: 400,
                    },
                    position: Vec2::ZERO,
                },
            )
        };
        app.world_mut().trigger(PointerDragStart {
            entity: handle,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        });
        app.world_mut().flush();
        app.world_mut().trigger(PointerDrag {
            entity: handle,
            pointer: pointer(),
            button: PointerButton::Primary,
            distance: Vec2::new(30.0, 0.0),
            delta: Vec2::new(30.0, 0.0),
        });
        app.world_mut().flush();
        app.world_mut().trigger(PointerDragEnd {
            entity: handle,
            pointer: pointer(),
            button: PointerButton::Primary,
            distance: Vec2::new(30.0, 0.0),
        });
        app.world_mut().flush();
        settle(&mut app);
        let layout = app.world().get::<WidgetryTableLayout>(root).unwrap();
        assert_eq!(
            layout.column_widths()[&column],
            WidgetryTableColumnWidth::Fixed(150.0)
        );
        assert_eq!(layout.frozen_columns, 1);
        assert_eq!(
            app.world()
                .get::<WidgetryTableColumnHeader>(header)
                .unwrap()
                .column,
            column
        );
    }
}

#[test]
fn keyboard_reveal_keeps_frozen_x_and_reveals_y_when_all_columns_are_frozen() {
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy::ui::ScrollPosition;
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key};
    let (mut app, source, root, body) = common::fixture(100, 20);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .frozen_columns = 1;
    settle(&mut app);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let row = model.row_id(90).unwrap();
    let first = model.column_id(0).unwrap();
    let second = model.column_id(1).unwrap();
    let last = model.column_id(19).unwrap();
    WidgetryTable::<u32>::set_focused_cell(
        app.world_mut(),
        root,
        Some(WidgetryTableCell { row, column: last }),
    )
    .unwrap();
    press_key(&mut app, window, KeyCode::ArrowLeft);
    settle(&mut app);
    let offset = app.world().get::<ScrollPosition>(body).unwrap().0;
    assert!(offset.x > 1000.0 && offset.y > 2000.0);
    WidgetryTable::<u32>::set_focused_cell(
        app.world_mut(),
        root,
        Some(WidgetryTableCell {
            row,
            column: second,
        }),
    )
    .unwrap();
    press_key(&mut app, window, KeyCode::ArrowLeft);
    settle(&mut app);
    assert_eq!(
        app.world()
            .get::<WidgetryTableState>(root)
            .unwrap()
            .focused_cell(),
        Some(WidgetryTableCell { row, column: first })
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(body).unwrap().0.x,
        offset.x
    );
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .frozen_columns = 99;
    settle(&mut app);
    let top = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .row_id(0)
        .unwrap();
    WidgetryTable::<u32>::set_focused_cell(
        app.world_mut(),
        root,
        Some(WidgetryTableCell {
            row: top,
            column: first,
        }),
    )
    .unwrap();
    press_key(&mut app, window, KeyCode::ArrowDown);
    settle(&mut app);
    assert!(app.world().get::<ScrollPosition>(body).unwrap().0.y < 100.0);
}

#[test]
fn scale_and_fractional_scroll_keep_header_cell_and_row_alignment() {
    for scale in [1.0, 1.25, 1.5] {
        let (mut app, source, root, body) = common::fixture(100, 20);
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        app.world_mut()
            .get_mut::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns = 1;
        settle(&mut app);
        common::scroll(&mut app, body, Vec2::new(500.3, 300.7));
        settle(&mut app);
        let first = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_id(0)
            .unwrap();
        let (fixed, identity) = owned_cells(&mut app, root)
            .into_iter()
            .find(|(_, c)| c.column == first)
            .unwrap();
        let header = app
            .world_mut()
            .query::<(Entity, &WidgetryTableColumnHeader)>()
            .iter(app.world())
            .find(|(_, h)| h.column == first)
            .unwrap()
            .0;
        let row_header = app
            .world_mut()
            .query::<(Entity, &WidgetryTableRowHeader)>()
            .iter(app.world())
            .find(|(_, h)| h.row == identity.row)
            .unwrap()
            .0;
        assert_eq!(
            app.world()
                .get::<UiGlobalTransform>(fixed)
                .unwrap()
                .translation
                .x,
            app.world()
                .get::<UiGlobalTransform>(header)
                .unwrap()
                .translation
                .x
        );
        assert_eq!(
            app.world()
                .get::<UiGlobalTransform>(fixed)
                .unwrap()
                .translation
                .y,
            app.world()
                .get::<UiGlobalTransform>(row_header)
                .unwrap()
                .translation
                .y
        );
    }
}

#[test]
fn changing_freeze_region_preserves_editor_and_empty_axes_keep_configuration() {
    #[derive(Component)]
    struct Draft;
    let (mut app, source, root, _) = common::fixture(2, 3);
    app.register_table_cell_editor(WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::SingleLine,
        |_: &String| bsn_list! {template(|_|Ok(Draft))},
        |_, _| Ok("draft".to_owned()),
    ));
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
    let editor = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .editor();
    for n in [1, 0, 99] {
        app.world_mut()
            .get_mut::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns = n;
        settle(&mut app);
        assert_eq!(
            app.world()
                .get::<WidgetryTableEditState>(root)
                .unwrap()
                .editor(),
            editor
        );
        let shell = app.world().get::<ChildOf>(editor).unwrap().parent();
        let canvas = app.world().get::<ChildOf>(shell).unwrap().parent();
        let viewport = app.world().get::<ChildOf>(canvas).unwrap().parent();
        assert_eq!(
            app.world()
                .get::<WidgetryTableFrozenBody>(viewport)
                .is_some(),
            n > 0
        );
    }
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .clear_rows();
    settle(&mut app);
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
    assert!(owned_cells(&mut app, root).is_empty());
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .clear_columns();
    settle(&mut app);
    let frozen = part::<WidgetryTableFrozenBody>(app.world(), root);
    assert_eq!(
        app.world().get::<ComputedNode>(frozen).unwrap().size().x,
        0.0
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns,
        99
    );
}

#[test]
fn strict_count_survives_narrowing_resize_and_schema_changes() {
    let (mut app, source, root, _) = common::fixture(100, 5);
    let frozen = part::<WidgetryTableFrozenBody>(app.world(), root);
    for n in [0, 1, 2, 99] {
        app.world_mut()
            .get_mut::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns = n;
        settle(&mut app);
        let computed = app.world().get::<ComputedNode>(frozen).unwrap();
        assert_eq!(
            computed.size().x * computed.inverse_scale_factor(),
            n.min(5) as f32 * 120.0
        );
        assert_eq!(
            app.world()
                .get::<WidgetryTableLayout>(root)
                .unwrap()
                .frozen_columns,
            n
        );
    }
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(90);
    settle(&mut app);
    let first = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(0)
        .unwrap();
    WidgetryTable::<u32>::set_column_width(app.world_mut(), root, first, 240.0).unwrap();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .push_column(WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new("new".to_owned()),
            (),
            |row: &u32, _: &()| WidgetryTableCellValue::new(row.to_string()),
        ))
        .unwrap();
    settle(&mut app);
    let computed = app.world().get::<ComputedNode>(frozen).unwrap();
    assert_eq!(computed.size().x * computed.inverse_scale_factor(), 840.0);
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns,
        99
    );
    assert!(
        owned_cells(&mut app, root)
            .iter()
            .all(|(_, c)| c.column == first)
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_column(0);
    settle(&mut app);
    let computed = app.world().get::<ComputedNode>(frozen).unwrap();
    assert_eq!(computed.size().x * computed.inverse_scale_factor(), 600.0);
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(root)
            .unwrap()
            .frozen_columns,
        99
    );
}

#[test]
fn frozen_scroll_reaches_body_and_clips_separate_transparent_regions() {
    use bevy::ui::{CalculatedClip, ScrollPosition};
    let (mut app, source, root, body) = common::fixture(100, 20);
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .frozen_columns = 1;
    settle(&mut app);
    let frozen = part::<WidgetryTableFrozenBody>(app.world(), root);
    common::scroll(&mut app, frozen, Vec2::new(500.0, 600.0));
    settle(&mut app);
    let offset = app.world().get::<ScrollPosition>(body).unwrap().0;
    assert!(offset.x > 0.0 && offset.y > 0.0);
    assert_eq!(
        app.world().get::<ScrollPosition>(frozen).unwrap().0,
        Vec2::new(0.0, offset.y)
    );
    let row_headers = part::<WidgetryTableRowHeaders>(app.world(), root);
    assert_eq!(
        app.world().get::<ScrollPosition>(row_headers).unwrap().0,
        Vec2::new(0.0, offset.y)
    );
    let first = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(0)
        .unwrap();
    let cells = owned_cells(&mut app, root);
    let (fixed, identity) = cells.iter().find(|(_, c)| c.column == first).unwrap();
    let (scrolling, _) = cells
        .iter()
        .find(|(_, c)| c.row == identity.row && c.column != first)
        .unwrap();
    let fixed_center = app
        .world()
        .get::<UiGlobalTransform>(*fixed)
        .unwrap()
        .translation;
    assert!(
        app.world()
            .get::<CalculatedClip>(*fixed)
            .unwrap()
            .contains_point(fixed_center)
    );
    assert!(
        !app.world()
            .get::<CalculatedClip>(*scrolling)
            .unwrap()
            .contains_point(fixed_center)
    );
    let body_left = app
        .world()
        .get::<UiGlobalTransform>(body)
        .unwrap()
        .translation
        .x
        - app.world().get::<ComputedNode>(body).unwrap().size().x / 2.0;
    assert!(
        !app.world()
            .get::<CalculatedClip>(*fixed)
            .unwrap()
            .contains_point(Vec2::new(body_left + 1.0, fixed_center.y))
    );
}

#[test]
fn shared_views_keep_independent_freeze_width_and_offsets() {
    use bevy::ui::ScrollPosition;
    let (mut app, source, root, body) = common::fixture(100, 20);
    let other=app.world_mut().spawn_scene(bsn!{@WidgetryTable::<u32>{@source:source} Node{position_type:PositionType::Absolute,left:px(300),width:px(286),height:px(144)}}).unwrap().id();
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .frozen_columns = 2;
    settle(&mut app);
    common::scroll(&mut app, body, Vec2::new(500.0, 600.0));
    settle(&mut app);
    let other_body = part::<WidgetryTableBody>(app.world(), other);
    assert_eq!(
        app.world().get::<ScrollPosition>(other_body).unwrap().0,
        Vec2::ZERO
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableLayout>(other)
            .unwrap()
            .frozen_columns,
        0
    );
    for (view, frozen_expected) in [(root, true), (other, false)] {
        let first = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_id(0)
            .unwrap();
        let cells = owned_cells(&mut app, view);
        let (cell, _) = cells.iter().find(|(_, c)| c.column == first).unwrap();
        let canvas = app.world().get::<ChildOf>(*cell).unwrap().parent();
        let viewport = app.world().get::<ChildOf>(canvas).unwrap().parent();
        assert_eq!(
            app.world()
                .get::<WidgetryTableFrozenBody>(viewport)
                .is_some(),
            frozen_expected
        );
    }
}

#[test]
fn frozen_columns_keep_screen_x_while_both_regions_virtualize_y() {
    let (mut app, source, root, body) = common::fixture(100, 20);
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(root)
        .unwrap()
        .frozen_columns = 1;
    app.update();
    app.update();
    let first = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(0)
        .unwrap();
    let initial = common::projection(&mut app, root);
    let (&key, &(entity, _)) = initial
        .iter()
        .find(|((_, column), _)| *column == first)
        .unwrap();
    let x = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .unwrap()
        .translation
        .x;
    common::scroll(&mut app, body, Vec2::new(1200.0, 0.0));
    let after = common::projection(&mut app, root);
    let same = after.get(&key).unwrap();
    assert_eq!(same.0, entity);
    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(entity)
            .unwrap()
            .translation
            .x,
        x
    );
    common::scroll(&mut app, body, Vec2::new(0.0, 1500.0));
    let after = common::projection(&mut app, root);
    assert!(after.len() < 50);
    assert!(!after.contains_key(&key));
    assert!(after.keys().any(|(_, column)| *column == first));
}
