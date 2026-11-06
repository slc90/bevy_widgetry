#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, ValueChange},
};
use bevy_widgetry_combo_box::{
    WidgetryComboBoxAppExt, WidgetryComboBoxHeadless as Combo, WidgetryComboBoxParts,
};
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListSelectionMode, WidgetryListView,
    WidgetryListViewRenderer,
};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, Option<WidgetryListItemId>)>);

fn fixture() -> (
    App,
    Entity,
    Entity,
    Entity,
    Entity,
    Entity,
    Vec<WidgetryListItemId>,
) {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.register_widgetry_combo_box::<u32>()
        .unwrap()
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<Option<WidgetryListItemId>>>, mut changes: ResMut<Changes>| {
            changes.0.push((event.source, event.value))
        },
    );
    let mut model = WidgetryListModel::default();
    let ids = vec![model.push(10u32).unwrap(), model.push(20u32).unwrap()];
    let source = app.world_mut().spawn(model).id();
    let root=spawn_scene(app.world_mut(),bsn! {
        @Combo::<u32> {@source:source}
        WidgetryComboBoxParts {trigger:#Trigger,popup:#Popup,list:#List}
        Children [Node Children [#Trigger @bevy_widgetry_button::WidgetryButtonHeadless]--
            Node Children [#Popup Node {height:px(111)} Visibility::Hidden Children [Node Children [
                #List @WidgetryListView::<u32> {@source:source,@selection_mode:WidgetryListSelectionMode::Single,@renderer:{WidgetryListViewRenderer::new(|_:&u32|bsn_list!{Node})}}
            ]]]]
    }).unwrap();
    let parts = *app.world().get::<WidgetryComboBoxParts>(root).unwrap();
    app.update();
    (
        app,
        root,
        source,
        parts.trigger,
        parts.popup,
        parts.list,
        ids,
    )
}

#[test]
fn nested_custom_parts_open_without_default_field_and_keep_selection_authority() {
    let (mut app, root, _, trigger, popup, list, ids) = fixture();
    assert_eq!(Combo::<u32>::selected_id(app.world(), root).unwrap(), None);
    app.world_mut().trigger(Activate { entity: trigger });
    app.world_mut().flush();
    assert!(Combo::<u32>::is_open(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    Combo::<u32>::select_id(&mut app.world_mut().commands(), root, ids[1]);
    app.world_mut().flush();
    assert_eq!(
        Combo::<u32>::selected_id(app.world(), root).unwrap(),
        Some(ids[1])
    );
    assert!(Combo::<u32>::is_open(app.world(), root).unwrap());
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(ids[1]))]
    );
    app.update();
    assert_eq!(app.world().get::<Node>(popup).unwrap().height, px(111));
    assert!(Combo::<u32>::close_in_world(app.world_mut(), root).unwrap());
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert!(!Combo::<u32>::close_in_world(app.world_mut(), root).unwrap());
}

#[test]
fn disabled_blocks_open_but_preserves_programmatic_selection_and_closes_immediately() {
    let (mut app, root, _, _, _, list, ids) = fixture();
    assert!(Combo::<u32>::open_in_world(app.world_mut(), root).unwrap());
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    assert!(!Combo::<u32>::is_open(app.world(), root).unwrap());
    assert!(!Combo::<u32>::open_in_world(app.world_mut(), root).unwrap());
    assert_ne!(app.world().resource::<InputFocus>().get(), Some(list));
    Combo::<u32>::select_id(&mut app.world_mut().commands(), root, ids[0]);
    app.world_mut().flush();
    assert_eq!(
        Combo::<u32>::selected_id(app.world(), root).unwrap(),
        Some(ids[0])
    );
}

#[test]
fn invalidated_parts_close_and_root_despawn_removes_the_nested_popup() {
    let (mut app, root, _, trigger, popup, list, _) = fixture();
    Combo::<u32>::open_in_world(app.world_mut(), root).unwrap();
    app.world_mut().despawn(trigger);
    app.update();
    assert!(!Combo::<u32>::is_open(app.world(), root).unwrap());
    assert!(Combo::<u32>::selected_id(app.world(), root).is_err());
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(list, FocusCause::Navigated);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut().despawn(root);
    assert!(app.world().get_entity(popup).is_err());
}

#[test]
fn custom_keyboard_confirmation_escape_and_outside_click_share_the_lifecycle() {
    use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key, primary_click, queue_key};
    let (mut app, root, _, trigger, _, list, ids) = fixture();
    app.init_resource::<ButtonInput<KeyCode>>();
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    Combo::<u32>::open_in_world(app.world_mut(), root).unwrap();
    press_key(&mut app, window, KeyCode::ArrowDown);
    assert!(Combo::<u32>::is_open(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    assert!(
        app.world()
            .get::<bevy_widgetry_list_view::WidgetryListViewState>(list)
            .unwrap()
            .active
            .is_some()
    );
    queue_key(
        &mut app,
        bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    queue_key(
        &mut app,
        bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::ArrowDown,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    queue_key(
        &mut app,
        bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    app.update();
    assert_eq!(
        Combo::<u32>::selected_id(app.world(), root).unwrap(),
        Some(ids[0])
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(ids[0]))]
    );
    assert!(!Combo::<u32>::is_open(app.world(), root).unwrap());
    Combo::<u32>::open_in_world(app.world_mut(), root).unwrap();
    press_key(&mut app, window, KeyCode::Escape);
    assert!(!Combo::<u32>::is_open(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(trigger));
    Combo::<u32>::open_in_world(app.world_mut(), root).unwrap();
    let outside = app.world_mut().spawn(Node::default()).id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(outside, FocusCause::Navigated);
    app.world_mut().trigger(primary_click(outside));
    app.world_mut().flush();
    assert!(!Combo::<u32>::is_open(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(outside));
    assert!(app.world().get::<InteractionDisabled>(list).is_some());
}

#[test]
fn empty_model_and_invalid_source_mapping_are_rejected_without_selection() {
    let (mut app, root, source, _, _, list, _) = fixture();
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .clear();
    assert!(!Combo::<u32>::open_in_world(app.world_mut(), root).unwrap());
    let other = app
        .world_mut()
        .spawn(WidgetryListModel::<u32>::default())
        .id();
    WidgetryListView::<u32>::set_source_in_world(app.world_mut(), list, other).unwrap();
    assert!(
        matches!(Combo::<u32>::selected_id(app.world(),root).unwrap_err().kind(),bevy_widgetry_combo_box::ComboError::ListMapping{actual_source:Some(actual),..} if *actual==other)
    );
    app.world_mut().despawn(source);
    assert!(matches!(
        Combo::<u32>::selected_id(app.world(), root)
            .unwrap_err()
            .kind(),
        bevy_widgetry_combo_box::ComboError::InvalidSource { .. }
    ));
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn parts_cannot_bind_a_foreign_root_or_a_multiple_selection_list() {
    use bevy_widgetry_combo_box::{ComboError, ComboPart};
    let (mut app, root, source, trigger, popup, list, _) = fixture();
    let multiple=spawn_scene(app.world_mut(),bsn! {
        @WidgetryListView::<u32> {@source:source,@renderer:{WidgetryListViewRenderer::new(|_:&u32|bsn_list!{Node})}}
        ChildOf(popup)
    }).unwrap();
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryComboBoxParts {
            trigger,
            popup,
            list: multiple,
        });
    assert!(
        matches!(Combo::<u32>::selected_id(app.world(),root).unwrap_err().kind(),ComboError::InvalidBinding{entity,part:ComboPart::Selection,..} if *entity==multiple)
    );
    let foreign = spawn_scene(
        app.world_mut(),
        bsn! {@Combo::<u32> {@source:source} Children [Node]},
    )
    .unwrap();
    let foreign_trigger = app.world().get::<Children>(foreign).unwrap()[0];
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryComboBoxParts {
            trigger: foreign_trigger,
            popup,
            list,
        });
    assert!(
        matches!(Combo::<u32>::selected_id(app.world(),root).unwrap_err().kind(),ComboError::InvalidBinding{entity,part:ComboPart::Field,..} if *entity==foreign_trigger)
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}
