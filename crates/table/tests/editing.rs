#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod common;
use bevy::prelude::*;
use bevy_widgetry_table::*;

#[derive(Component)]
struct Draft(String);

#[derive(Resource, Default)]
struct Requests(Vec<WidgetryTableEditRequested>);

fn editor() -> WidgetryTableCellEditor<String> {
    WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::SingleLine,
        |v: &String| {
            let v = v.clone();
            bsn_list! { template(move |_| Ok(Draft(v.clone()))) }
        },
        |world, editor| {
            let child = world.get::<Children>(editor).unwrap()[0];
            Ok(world.get::<Draft>(child).unwrap().0.clone())
        },
    )
}

#[test]
fn readonly_cell_resolves_to_lock_cursor_and_restores_default_when_editing_is_off() {
    use bevy::picking::pointer::{Location, PointerAction, PointerId};
    use bevy::window::{CursorIcon, SystemCursorIcon};
    use bevy_widgetry_cursor::WidgetryCursorTheme;
    use bevy_widgetry_test_utils::queue_pointer;

    let (mut app, _, root, _) = common::fixture(1, 1);
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| false));
    let window = app
        .world_mut()
        .spawn(Window {
            resolution: (600, 400).into(),
            ..default()
        })
        .id();
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(camera)
        .insert(bevy::camera::RenderTarget::Window(
            bevy::window::WindowRef::Entity(window),
        ));
    app.world_mut()
        .entity_mut(root)
        .insert(UiTargetCamera(camera));
    app.world_mut()
        .resource_mut::<bevy::picking::input::PointerInputSettings>()
        .is_mouse_enabled = false;
    app.update();
    app.update();
    let cell = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryTableCell>>()
        .single(app.world())
        .unwrap();
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: app
            .world()
            .get::<UiGlobalTransform>(cell)
            .unwrap()
            .translation,
    };
    queue_pointer(
        &mut app,
        PointerId::Mouse,
        location,
        PointerAction::Move { delta: Vec2::ONE },
    );
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Cell))
    );

    app.register_asset_loader(bevy::image::ImageLoader::new(
        bevy::image::CompressedImageFormats::NONE,
    ));
    app.insert_resource(WidgetryCursorTheme::Kamuri);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app.update();
        if let Some(CursorIcon::Custom(bevy::window::CustomCursor::Image(image))) =
            app.world().get::<CursorIcon>(window)
        {
            assert_eq!(image.texture_atlas.as_ref().unwrap().index, 44);
            assert_eq!(image.hotspot, (14, 37));
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "ReadOnly Kamuri atlas did not load"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    app.insert_resource(WidgetryCursorTheme::System);
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::NotAllowed))
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<bevy::ui::InteractionDisabled>();
    app.world_mut()
        .get_mut::<WidgetryTableEditing>(root)
        .unwrap()
        .enabled = false;
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    assert!(
        app.world()
            .get::<bevy_widgetry_cursor::WidgetryCursorHint>(cell)
            .is_none()
    );
    app.world_mut()
        .get_mut::<WidgetryTableEditing>(root)
        .unwrap()
        .enabled = true;
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Cell))
    );
}

#[test]
fn editing_requires_opt_in_and_explicit_application_receipt() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    app.register_table_cell_editor(editor());
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    assert!(!WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap());
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    assert_eq!(common::projection(&mut app, root).len(), 4);
    assert!(WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap());
    assert!(app.world().get::<WidgetryTableEditState>(root).is_some());
}

#[test]
fn draft_and_pending_survive_revision_and_scroll_until_receipt() {
    let (mut app, source, root, body) = common::fixture(30, 2);
    app.register_table_cell_editor(editor());
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    assert!(WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap());
    let editor = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .editor();
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .row_mut(0)
        .unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .editor(),
        editor
    );
    assert!(WidgetryTable::<u32>::submit_edit(app.world_mut(), root).unwrap());
    let request = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .pending()
        .unwrap();
    assert!(!WidgetryTable::<u32>::submit_edit(app.world_mut(), root).unwrap());
    assert!(!WidgetryTable::<u32>::cancel_edit(app.world_mut(), root).unwrap());
    common::scroll(&mut app, body, Vec2::new(0., 700.));
    assert!(app.world().entities().contains(editor));
    WidgetryTable::<u32>::resolve_edit(
        app.world_mut(),
        root,
        request,
        WidgetryTableEditResult::Rejected("business error".into()),
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .error(),
        Some("business error")
    );
    app.update();
    let feedback = app
        .world()
        .get::<Children>(editor)
        .unwrap()
        .iter()
        .find(|e| app.world().get::<Text>(*e).is_some())
        .unwrap();
    assert!(
        app.world()
            .get::<BackgroundColor>(feedback)
            .unwrap()
            .0
            .to_srgba()
            .alpha
            > 0.99
    );
    assert!(WidgetryTable::<u32>::submit_edit(app.world_mut(), root).unwrap());
    let next = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .pending()
        .unwrap();
    assert_ne!(request, next);
    assert!(
        WidgetryTable::<u32>::resolve_edit(
            app.world_mut(),
            root,
            request,
            WidgetryTableEditResult::Accepted
        )
        .is_err()
    );
    WidgetryTable::<u32>::resolve_edit(
        app.world_mut(),
        root,
        next,
        WidgetryTableEditResult::Accepted,
    )
    .unwrap();
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
    app.update();
}

#[test]
fn column_override_is_source_local_and_missing_or_wrong_type_is_readonly() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let a = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    let b = WidgetryTableCell {
        column: model.column_id(1).unwrap(),
        ..a
    };
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    app.register_table_column_editor(source, a.column, editor());
    assert!(WidgetryTable::<u32>::begin_edit(app.world_mut(), root, a).unwrap());
    WidgetryTable::<u32>::cancel_edit(app.world_mut(), root).unwrap();
    assert!(!WidgetryTable::<u32>::begin_edit(app.world_mut(), root, b).unwrap());
    app.register_table_cell_editor(editor());
    app.register_table_column_editor(
        source,
        a.column,
        WidgetryTableCellEditor::new(
            WidgetryTableEditorMode::SingleLine,
            |_: &u32| bsn_list! { Text("wrong") },
            |_, _| Ok(3u32),
        ),
    );
    assert!(!WidgetryTable::<u32>::begin_edit(app.world_mut(), root, a).unwrap());
    assert!(WidgetryTable::<u32>::begin_edit(app.world_mut(), root, b).unwrap());
    WidgetryTable::<u32>::cancel_edit(app.world_mut(), root).unwrap();
    app.update();
    let readonly = app
        .world_mut()
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(app.world())
        .find(|(_, c)| **c == a)
        .unwrap()
        .0;
    assert_eq!(
        app.world()
            .get::<bevy_widgetry_cursor::WidgetryCursorHint>(readonly),
        Some(&bevy_widgetry_cursor::WidgetryCursorHint::ReadOnly)
    );
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(readonly)
            .is_none()
    );
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    assert!(!WidgetryTable::<u32>::begin_edit(app.world_mut(), root, b).unwrap());
}

#[test]
fn request_is_typed_state_is_pending_before_notification_and_invalid_receipts_are_rejected() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    app.register_table_cell_editor(editor())
        .init_resource::<Requests>()
        .add_observer(
            |event: On<WidgetryTableEditRequested>,
             states: Query<&WidgetryTableEditState>,
             mut requests: ResMut<Requests>| {
                assert_eq!(
                    states.get(event.entity).unwrap().pending(),
                    Some(event.request_id)
                );
                requests.0.push(event.clone());
            },
        );
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
    let editor = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .editor();
    let draft = app.world().get::<Children>(editor).unwrap()[0];
    app.world_mut().get_mut::<Draft>(draft).unwrap().0 = "typed draft".into();
    WidgetryTable::<u32>::submit_edit(app.world_mut(), root).unwrap();
    let request = app.world().resource::<Requests>().0[0].clone();
    assert_eq!(
        request.value.downcast_ref::<String>().unwrap(),
        "typed draft"
    );
    assert_eq!(
        app.world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .row_revision(0),
        Some(0)
    );
    assert!(
        WidgetryTable::<u32>::resolve_edit(
            app.world_mut(),
            Entity::PLACEHOLDER,
            request.request_id,
            WidgetryTableEditResult::Accepted
        )
        .is_err()
    );
    app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .remove_row(0);
    assert!(
        WidgetryTable::<u32>::resolve_edit(
            app.world_mut(),
            root,
            request.request_id,
            WidgetryTableEditResult::Accepted
        )
        .is_err()
    );
    app.update();
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
}

#[test]
fn keyboard_and_outside_click_own_the_draft_and_pending_intent() {
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key, primary_press};
    let (mut app, source, root, _) = common::fixture(2, 2);
    add_keyboard_dispatch(&mut app);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.register_table_cell_editor(editor());
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    WidgetryTable::<u32>::set_focused_cell(app.world_mut(), root, Some(cell)).unwrap();
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(root, bevy::input_focus::FocusCause::Navigated);
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Enter);
    let editor = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .editor();
    app.world_mut().trigger(primary_press(editor));
    app.world_mut().flush();
    assert!(app.world().get::<WidgetryTableEditState>(root).is_some());
    let popup = app
        .world_mut()
        .spawn(WidgetryTableEditorPopup { editor })
        .id();
    app.world_mut().trigger(primary_press(popup));
    app.world_mut().flush();
    assert!(app.world().get::<WidgetryTableEditState>(root).is_some());
    app.world_mut().trigger(primary_press(root));
    app.world_mut().flush();
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
    WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Escape);
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
    WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Enter);
    let request = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .pending()
        .unwrap();
    app.world_mut().trigger(primary_press(root));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .pending(),
        Some(request)
    );
}

#[test]
fn multiline_requires_control_enter_and_parser_errors_do_not_emit_requests() {
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key, primary_click};
    let (mut app, source, root, _) = common::fixture(2, 2);
    add_keyboard_dispatch(&mut app);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    let editor = WidgetryTableCellEditor::new(
        WidgetryTableEditorMode::MultiLine,
        |v: &String| {
            let v = v.clone();
            bsn_list! {template(move |_| Ok(Draft(v.clone())))}
        },
        |world, root| {
            let child = world.get::<Children>(root).unwrap()[0];
            let value = &world.get::<Draft>(child).unwrap().0;
            if value == "invalid" {
                Err("parse failure".into())
            } else {
                Ok(value.clone())
            }
        },
    );
    app.register_table_cell_editor(editor);
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    let shell = common::projection(&mut app, root)[&(cell.row, cell.column)].0;
    let mut click = primary_click(shell);
    click.count = 2;
    app.world_mut().trigger(click);
    app.world_mut().flush();
    let editor = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .editor();
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Enter);
    assert!(
        app.world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .pending()
            .is_none()
    );
    let draft = app.world().get::<Children>(editor).unwrap()[0];
    app.world_mut().get_mut::<Draft>(draft).unwrap().0 = "invalid".into();
    assert!(!WidgetryTable::<u32>::submit_edit(app.world_mut(), root).unwrap());
    assert_eq!(
        app.world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .error(),
        Some("parse failure")
    );
    app.world_mut().get_mut::<Draft>(draft).unwrap().0 = "valid\nmultiline".into();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Enter);
    assert!(
        app.world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .pending()
            .is_some()
    );
}

#[test]
fn application_write_updates_shared_views_and_reprojection_keeps_editor_identity() {
    let (mut app, source, root, _) = common::fixture(30, 2);
    app.register_table_cell_editor(editor());
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    let other = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryTable::<u32> { @source: source } Node {width:px(286),height:px(144)} },
        )
        .unwrap()
        .id();
    app.update();
    app.update();
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    let mut rows = (0..model.row_count())
        .filter_map(|i| model.row_id(i))
        .collect::<Vec<_>>();
    WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
    let editor = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .editor();
    rows.reverse();
    WidgetryTable::<u32>::update_view(
        app.world_mut(),
        root,
        WidgetryTableViewUpdate { rows, ..default() },
    )
    .unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .editor(),
        editor
    );
    let shell = app.world().get::<ChildOf>(editor).unwrap().parent();
    assert!(matches!(app.world().get::<Node>(shell).unwrap().top, Val::Px(y) if y > 500.));
    WidgetryTable::<u32>::submit_edit(app.world_mut(), root).unwrap();
    let request = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .pending()
        .unwrap();
    assert!(
        WidgetryTable::<u32>::resolve_edit(
            app.world_mut(),
            other,
            request,
            WidgetryTableEditResult::Accepted
        )
        .is_err()
    );
    *app.world_mut()
        .get_mut::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .row_mut(0)
        .unwrap()
        .unwrap() = 99;
    WidgetryTable::<u32>::resolve_edit(
        app.world_mut(),
        root,
        request,
        WidgetryTableEditResult::Accepted,
    )
    .unwrap();
    WidgetryTable::<u32>::reset_view(app.world_mut(), root).unwrap();
    app.update();
    let world = app.world_mut();
    let matching = world
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(world)
        .filter(|(_, c)| **c == cell)
        .map(|(e, _)| e)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 2);
    for shell in matching {
        let content = world.get::<Children>(shell).unwrap()[0];
        assert_eq!(world.get::<Text>(content).unwrap().0, "99/0");
    }
}

#[test]
fn source_local_column_override_and_per_cell_predicate_do_not_enable_other_views() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    let mut model = WidgetryTableModel::default();
    let row = model.push_row(0u32).unwrap();
    let column = model
        .push_column(WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new("other".to_owned()),
            (),
            |_: &u32, _| WidgetryTableCellValue::new("other".to_owned()),
        ))
        .unwrap();
    assert_eq!(column, cell.column);
    let other_source = app.world_mut().spawn(model).id();
    let other=app.world_mut().spawn_scene(bsn! { @WidgetryTable::<u32> { @source:other_source, @editing:{WidgetryTableEditing::new(|_,_,_|true)} } Node{width:px(286),height:px(144)} }).unwrap().id();
    app.update();
    app.update();
    app.register_table_column_editor(source, cell.column, editor());
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| false));
    assert!(!WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap());
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    assert!(WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap());
    assert!(
        !WidgetryTable::<u32>::begin_edit(
            app.world_mut(),
            other,
            WidgetryTableCell { row, column }
        )
        .unwrap()
    );
    app.world_mut()
        .entity_mut(root)
        .insert(bevy::ui::InteractionDisabled);
    app.update();
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
}

#[test]
fn replacing_model_instance_cannot_accept_an_old_request_with_matching_ids() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    app.register_table_cell_editor(editor());
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
    WidgetryTable::<u32>::submit_edit(app.world_mut(), root).unwrap();
    let request = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .pending()
        .unwrap();
    let mut replacement = WidgetryTableModel::default();
    assert_eq!(replacement.push_row(99u32).unwrap(), cell.row);
    assert_eq!(
        replacement
            .push_column(WidgetryTableColumn::new(
                WidgetryTableHeaderValue::new("replacement".to_owned()),
                (),
                |v: &u32, _| WidgetryTableCellValue::new(v.to_string())
            ))
            .unwrap(),
        cell.column
    );
    app.world_mut().entity_mut(source).insert(replacement);
    assert!(
        WidgetryTable::<u32>::resolve_edit(
            app.world_mut(),
            root,
            request,
            WidgetryTableEditResult::Accepted
        )
        .is_err()
    );
    app.update();
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
}

#[test]
fn real_textfield_propagates_native_enter_and_control_enter_to_the_edit_transaction() {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, queue_key};
    use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};
    for multiline in [false, true] {
        let (mut app, source, root, _) = common::fixture(2, 2);
        app.add_plugins(WidgetryTextFieldPlugin);
        app.init_resource::<Requests>().add_observer(
            |event: On<WidgetryTableEditRequested>, mut requests: ResMut<Requests>| {
                requests.0.push(event.clone())
            },
        );
        add_keyboard_dispatch(&mut app);
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app.register_table_cell_editor(WidgetryTableCellEditor::new(if multiline {WidgetryTableEditorMode::MultiLine} else {WidgetryTableEditorMode::SingleLine},
            move |v:&String|bsn_list!{@WidgetryTextField{@value:{v.clone()},@multiline:multiline}},
            |world,root| {
                let field=world.get::<Children>(root).unwrap()[0];
                WidgetryTextField::text(world,field).map_err(|e|e.to_string())
            }));
        app.world_mut()
            .entity_mut(root)
            .insert(WidgetryTableEditing::new(|_, _, _| true));
        let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
        let cell = WidgetryTableCell {
            row: model.row_id(0).unwrap(),
            column: model.column_id(0).unwrap(),
        };
        WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
        app.update();
        let input = KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: Key::Enter,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        };
        if multiline {
            queue_key(&mut app, input.clone());
            app.update();
            assert!(
                app.world()
                    .get::<WidgetryTableEditState>(root)
                    .unwrap()
                    .pending()
                    .is_none()
            );
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ControlLeft);
            app.world_mut()
                .resource_mut::<ButtonInput<Key>>()
                .press(Key::Control);
        }
        let edit_root = app
            .world()
            .get::<WidgetryTableEditState>(root)
            .unwrap()
            .editor();
        let field = app.world().get::<Children>(edit_root).unwrap()[0];
        let native = app
            .world()
            .resource::<bevy::input_focus::InputFocus>()
            .get()
            .unwrap();
        app.world_mut()
            .get_mut::<bevy::text::EditableText>(native)
            .unwrap()
            .queue_edit(bevy::text::TextEdit::Insert("X".into()));
        queue_key(&mut app, input);
        app.update();
        let expected = WidgetryTextField::text(app.world(), field).unwrap();
        assert!(expected.contains('X'));
        assert_eq!(
            app.world()
                .resource::<Requests>()
                .0
                .last()
                .unwrap()
                .value
                .downcast_ref::<String>(),
            Some(&expected)
        );
        assert!(
            app.world()
                .get::<WidgetryTableEditState>(root)
                .unwrap()
                .pending()
                .is_some()
        );
    }
}

#[test]
fn disabling_the_body_or_edited_cell_blocks_submission_and_discards_draft() {
    for disable_body in [false, true] {
        let (mut app, source, root, body) = common::fixture(2, 2);
        app.register_table_cell_editor(editor())
            .init_resource::<Requests>()
            .add_observer(
                |event: On<WidgetryTableEditRequested>, mut requests: ResMut<Requests>| {
                    requests.0.push(event.clone())
                },
            );
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
        let shell = app.world().get::<ChildOf>(editor).unwrap().parent();
        app.world_mut()
            .entity_mut(if disable_body { body } else { shell })
            .insert(bevy::ui::InteractionDisabled);
        assert!(!WidgetryTable::<u32>::submit_edit(app.world_mut(), root).unwrap());
        app.update();
        assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
        assert!(app.world().resource::<Requests>().0.is_empty());
    }
}

#[test]
fn escape_from_a_real_open_combo_popup_cancels_the_table_draft() {
    use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
    use bevy_widgetry_list_view::{WidgetryListModel, WidgetryListView, WidgetryListViewRenderer};
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key};
    let (mut app, source, root, _) = common::fixture(2, 2);
    add_keyboard_dispatch(&mut app);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.register_widgetry_combo_box::<String>().unwrap();
    let mut options = WidgetryListModel::default();
    options.push("one".to_owned()).unwrap();
    options.push("two".to_owned()).unwrap();
    let options = app.world_mut().spawn(options).id();
    app.register_table_cell_editor(WidgetryTableCellEditor::new(WidgetryTableEditorMode::SingleLine,
        move |_:&String|bsn_list! { @WidgetryComboBox::<String> { @source:options, @renderer:{WidgetryListViewRenderer::new(|v:&String|bsn_list!{Text({v.clone()}) bevy_widgetry_core::text::WidgetryText})} } },
        |_,_|Ok("selected".to_owned())));
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryTableEditing::new(|_, _, _| true));
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    WidgetryTable::<u32>::begin_edit(app.world_mut(), root, cell).unwrap();
    app.update();
    app.update();
    let editor = app
        .world()
        .get::<WidgetryTableEditState>(root)
        .unwrap()
        .editor();
    let combo = app.world().get::<Children>(editor).unwrap()[0];
    let field = app.world().get::<Children>(combo).unwrap()[0];
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: field });
    app.world_mut().flush();
    app.update();
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListView<String>>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get(),
        Some(list)
    );
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Escape);
    assert!(app.world().get::<WidgetryTableEditState>(root).is_none());
    assert!(!app.world().entities().contains(editor));
}

#[test]
fn closing_a_screen_during_batched_submissions_skips_all_destroyed_views() {
    let (mut app, source, root, _) = common::fixture(2, 2);
    app.register_table_cell_editor(editor());
    let screen = app
        .world_mut()
        .spawn_scene(bsn! {Node{width:px(600),height:px(400)}})
        .unwrap()
        .id();
    app.world_mut().entity_mut(screen).add_child(root);
    let mut roots = vec![root];
    for _ in 0..2 {
        let view = app
            .world_mut()
            .spawn_scene(
                bsn! {@WidgetryTable::<u32>{@source:source} Node{width:px(286),height:px(144)}},
            )
            .unwrap()
            .id();
        app.world_mut().entity_mut(screen).add_child(view);
        roots.push(view);
    }
    app.update();
    app.update();
    let model = app.world().get::<WidgetryTableModel<u32>>(source).unwrap();
    let cell = WidgetryTableCell {
        row: model.row_id(0).unwrap(),
        column: model.column_id(0).unwrap(),
    };
    app.add_observer(
        move |_: On<WidgetryTableEditRequested>, mut commands: Commands| {
            commands.entity(screen).try_despawn();
        },
    );
    for root in &roots {
        app.world_mut()
            .entity_mut(*root)
            .insert(WidgetryTableEditing::new(|_, _, _| true));
        WidgetryTable::<u32>::begin_edit(app.world_mut(), *root, cell).unwrap();
        app.world_mut()
            .trigger(WidgetryTableSubmitEdit { entity: *root });
    }
    app.world_mut().flush();
    app.update();
    for root in roots {
        assert!(!app.world().entities().contains(root));
    }
    assert!(app.world().get::<WidgetryTableModel<u32>>(source).is_some());
}
