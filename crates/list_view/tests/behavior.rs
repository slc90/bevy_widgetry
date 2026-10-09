//! State：selected/active、focus、root/viewport/content/item disabled。
//! stimuli 为 pointer/keyboard、程序选择与 model mutation。
//! Guard：用户确认受 disabled 限制，程序允许。
//! invariant 为提交后通知、有效 identity、独立 repair 与 root 所属 projection。
//! Coupling：model 删除按旧 active 位置修复。
//! 共享 source 不共享 selection 或用户通知。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::a11y::AccessibilityNode;
use bevy::ecs::world::CommandQueue;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input_focus::FocusedInput;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::events::{Pointer, Scroll};
use bevy::picking::pointer::{PointerAction, PointerInput};
use bevy::prelude::*;
use bevy::ui::Selectable;
use bevy::ui::{InteractionDisabled, Pressed, ScrollPosition, Selected};
use bevy::ui_widgets::{ActiveDescendant, ListBox, ListItem, ScrollArea, ValueChange};
use bevy::window::PrimaryWindow;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewAppExt,
    WidgetryListViewItem, WidgetryListViewPlugin, WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_keyboard_dispatch, press_key, primary_cancel, primary_click,
    primary_press, primary_release, queue_key, scene_app,
};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, Option<WidgetryListItemId>, bool)>);

#[derive(Resource, Default)]
struct OuterKeys(usize);

fn fixture() -> (App, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<bevy::ui::UiScale>();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap()
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<Option<WidgetryListItemId>>>,
         states: Query<&WidgetryListViewState>,
         mut changes: ResMut<Changes>| {
            assert_eq!(states.get(event.source).unwrap().selected, event.value);
            changes.0.push((event.source, event.value, event.is_final));
        },
    );
    let mut model = WidgetryListModel::default();
    for index in 0..10 {
        model.push(index.to_string()).unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source, @item_height: 10.0,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}) bevy_widgetry_core::text::WidgetryText)])},
        }
    }).expect("合法 fixture 应展开").id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .expect("只有一个 viewport");
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100.0, 30.0),
        content_size: Vec2::new(100.0, 100.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.world_mut().spawn((
        bevy::picking::pointer::PointerId::Mouse,
        bevy::picking::pointer::PointerLocation::new(
            primary_press(Entity::PLACEHOLDER).pointer_location,
        ),
    ));
    app.update();
    (app, source, root, viewport)
}

#[test]
fn programmatic_selection_notifies_after_commit() {
    let (mut app, source, root, _) = fixture();
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1);
    app.add_observer(
        move |event: On<ValueChange<Option<WidgetryListItemId>>>,
              states: Query<&WidgetryListViewState>,
              mut count: ResMut<OuterKeys>| {
            assert_eq!(event.value, id);
            let state = states.get(event.source).unwrap();
            assert_eq!(state.selected, event.value);
            assert_eq!(state.active, event.value);
            count.0 += 1;
        },
    );
    app.init_resource::<OuterKeys>();
    select(&mut app, root, 1);
    select(&mut app, root, 1);
    assert_eq!(app.world().resource::<OuterKeys>().0, 1);
}

#[test]
fn explicit_clear_preserves_active_and_notifies_once() {
    let (mut app, source, root, viewport) = fixture();
    select(&mut app, root, 1);
    let active = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(8);
    WidgetryListView::<String>::set_active(&mut app.world_mut().commands(), root, Some(8));
    app.world_mut().flush();
    let scroll = app.world().get::<ScrollPosition>(viewport).unwrap().0;
    app.add_observer(
        |event: On<ValueChange<Option<WidgetryListItemId>>>,
         states: Query<&WidgetryListViewState>,
         mut count: ResMut<OuterKeys>| {
            assert!(event.value.is_none());
            assert_eq!(states.get(event.source).unwrap().selected, None);
            count.0 += 1;
        },
    );
    app.init_resource::<OuterKeys>();
    for _ in 0..2 {
        WidgetryListView::<String>::clear_selection(&mut app.world_mut().commands(), root);
    }
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        active
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0,
        scroll
    );
    assert_eq!(app.world().resource::<OuterKeys>().0, 1);
}

#[test]
fn invalid_programmatic_requests_preserve_state_and_report_errors() {
    let (mut app, _, root, viewport) = fixture();
    select(&mut app, root, 1);
    let state = *app.world().get::<WidgetryListViewState>(root).unwrap();
    let scroll = app.world().get::<ScrollPosition>(viewport).unwrap().0;
    let changes = app.world().resource::<Changes>().0.clone();
    app.set_error_handler(ErrorCapture::handler());
    WidgetryListView::<String>::set_selected(&mut app.world_mut().commands(), root, usize::MAX);
    WidgetryListView::<String>::set_active(&mut app.world_mut().commands(), root, Some(usize::MAX));
    WidgetryListView::<String>::clear_selection(
        &mut app.world_mut().commands(),
        Entity::PLACEHOLDER,
    );
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), 3);
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert!(
        errors
            .iter()
            .all(|error| error.to_string().contains("ListView"))
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        3
    );
    assert_eq!(app.world().get::<WidgetryListViewState>(root), Some(&state));
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0,
        scroll
    );
    assert_eq!(app.world().resource::<Changes>().0, changes);
}

#[test]
fn queued_updates_validate_execution_time_source_and_state() {
    for missing_source in [true, false] {
        let (mut app, source, root, _) = fixture();
        app.set_error_handler(ErrorCapture::handler());
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, app.world());
            WidgetryListView::<String>::set_selected(&mut commands, root, 1);
            WidgetryListView::<String>::set_active(&mut commands, root, Some(1));
            WidgetryListView::<String>::clear_selection(&mut commands, root);
        }
        if missing_source {
            app.world_mut().despawn(source);
        } else {
            app.world_mut()
                .entity_mut(root)
                .remove::<WidgetryListViewState>();
        }
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        errors.run(|| logs.run(|| queue.apply(app.world_mut())));
        let errors = errors.take();
        assert_eq!(errors.len(), 3);
        assert!(
            errors
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::Level::ERROR)
                .count(),
            3
        );
        assert_eq!(
            app.world().get::<WidgetryListViewState>(root).is_some(),
            missing_source
        );
        assert!(
            app.world()
                .get::<WidgetryListViewState>(root)
                .is_none_or(|state| state.selected.is_none() && state.active.is_none())
        );
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}

fn row(app: &mut App, index: usize) -> Entity {
    app.world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find(|(_, item)| item.index == index)
        .expect("目标应已 rendered")
        .0
}

fn select(app: &mut App, root: Entity, index: usize) {
    let mut commands = app.world_mut().commands();
    WidgetryListView::<String>::set_selected(&mut commands, root, index);
    app.world_mut().flush();
}

fn keyboard(app: &mut App, root: Entity) -> Entity {
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<bevy::ui::UiScale>();
    add_keyboard_dispatch(app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    window
}

fn wheel(app: &mut App, target: Entity) {
    let click = primary_click(target);
    app.world_mut().trigger(Pointer::new(
        click.pointer_id,
        click.pointer_location.clone(),
        Scroll {
            x: 0.0,
            y: -10.0,
            unit: MouseScrollUnit::Pixel,
            hit: click.hit.clone(),
            phase: bevy::input::touch::TouchPhase::Moved,
        },
        target,
    ));
    app.world_mut().flush();
}

#[test]
fn descendant_click_selects_stable_id_once_and_projects_focus() {
    let (mut app, source, root, _) = fixture();
    let target = row(&mut app, 1);
    let child = app.world().get::<Children>(target).unwrap()[0];
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1)
        .unwrap();
    app.world_mut().trigger(primary_click(child));
    app.update();
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        WidgetryListViewState {
            selected: Some(id),
            active: Some(id)
        }
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    assert!(app.world().get::<Selected>(target).is_some());
    assert_eq!(
        app.world().get::<ActiveDescendant>(root).unwrap().0,
        Some(target)
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(id), true)]
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    WidgetryListView::<String>::set_active(&mut app.world_mut().commands(), root, None);
    app.world_mut().flush();
    app.world_mut().trigger(primary_click(target));
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        Some(id)
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
}

#[test]
fn disabled_click_and_offscreen_focus_keep_logical_state() {
    let (mut app, source, root, viewport) = fixture();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        None
    );
    assert_eq!(app.world().get::<ActiveDescendant>(root).unwrap().0, None);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(1, true);
    app.update();
    let target = row(&mut app, 1);
    app.world_mut().trigger(primary_click(target));
    app.update();
    let state = *app.world().get::<WidgetryListViewState>(root).unwrap();
    assert_eq!(state.selected, None);
    assert_eq!(
        state.active,
        app.world()
            .get::<WidgetryListModel<String>>(source)
            .unwrap()
            .id(1)
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 70.0;
    app.update();
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        state
    );
    assert_eq!(app.world().get::<ActiveDescendant>(root).unwrap().0, None);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 0.0;
    app.update();
    let restored = row(&mut app, 1);
    assert!(app.world().get::<InteractionDisabled>(restored).is_some());
    assert_eq!(
        app.world().get::<ActiveDescendant>(root).unwrap().0,
        Some(restored)
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(app.world().get::<ActiveDescendant>(root).unwrap().0, None);
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        state
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.update();
    assert_eq!(
        app.world().get::<ActiveDescendant>(root).unwrap().0,
        Some(restored)
    );
}

#[test]
fn structural_changes_repair_selection_and_active_independently() {
    let (mut app, source, root, _) = fixture();
    let target = row(&mut app, 1);
    app.world_mut().trigger(primary_click(target));
    app.update();
    let id = app
        .world()
        .get::<WidgetryListViewState>(root)
        .unwrap()
        .selected;
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .insert(0, "insert".into())
        .unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .selected,
        id
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .move_item(2, 10);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        id
    );
    let predecessor = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(9);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .remove(10);
    app.update();
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        WidgetryListViewState {
            selected: None,
            active: predecessor
        }
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .clear();
    app.update();
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        WidgetryListViewState::default()
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
}

#[test]
fn programmatic_selection_reveals_and_corrects_active_without_duplicate_notification() {
    let (mut app, source, root, viewport) = fixture();
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(8, true);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    select(&mut app, root, 8);
    app.update();
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(8);
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        WidgetryListViewState {
            selected: id,
            active: id
        }
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        70.0
    );
    let target = row(&mut app, 8);
    assert!(app.world().get::<Selected>(target).is_some());
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
    WidgetryListView::<String>::set_active(&mut app.world_mut().commands(), root, None);
    app.world_mut().flush();
    select(&mut app, root, 8);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        id
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .selected,
        id
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 1.0;
    select(&mut app, root, 0);
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
}

#[test]
fn keyboard_navigation_wraps_selects_and_pages_without_changing_active() {
    let (mut app, source, root, viewport) = fixture();
    let window = keyboard(&mut app, root);
    press_key(&mut app, window, KeyCode::ArrowUp);
    let last = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(9);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        last
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        70.0
    );
    press_key(&mut app, window, KeyCode::ArrowDown);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        app.world()
            .get::<WidgetryListModel<String>>(source)
            .unwrap()
            .id(0)
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(1, true);
    press_key(&mut app, window, KeyCode::ArrowDown);
    let disabled = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        disabled
    );
    press_key(&mut app, window, KeyCode::Space);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .selected,
        None
    );
    press_key(&mut app, window, KeyCode::End);
    press_key(&mut app, window, KeyCode::Enter);
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, last, true)]
    );
    press_key(&mut app, window, KeyCode::Space);
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
    press_key(&mut app, window, KeyCode::Home);
    let active = app
        .world()
        .get::<WidgetryListViewState>(root)
        .unwrap()
        .active;
    press_key(&mut app, window, KeyCode::PageDown);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        30.0
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        active
    );
    assert_eq!(app.world().get::<ActiveDescendant>(root).unwrap().0, None);
    press_key(&mut app, window, KeyCode::PageUp);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
}

#[test]
fn root_disabled_blocks_user_input_and_restores_item_metadata() {
    let (mut app, source, root, viewport) = fixture();
    let window = keyboard(&mut app, root);
    let target = row(&mut app, 1);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(0, true);
    app.world_mut().trigger(primary_press(target));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_some());
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_none());
    wheel(&mut app, target);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
    app.world_mut().trigger(primary_click(target));
    press_key(&mut app, window, KeyCode::End);
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        WidgetryListViewState::default()
    );
    select(&mut app, root, 9);
    app.update();
    let state = *app.world().get::<WidgetryListViewState>(root).unwrap();
    assert!(state.selected.is_some());
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 0.0;
    app.update();
    let restored = row(&mut app, 1);
    assert!(app.world().get::<InteractionDisabled>(restored).is_some());
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert!(app.world().get::<InteractionDisabled>(restored).is_none());
    let item_disabled = row(&mut app, 0);
    assert!(
        app.world()
            .get::<InteractionDisabled>(item_disabled)
            .is_some()
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListModel<String>>(source)
            .unwrap()
            .is_disabled(1),
        Some(false)
    );
    wheel(&mut app, restored);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        10.0
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
}

#[test]
fn local_row_request_survives_model_capability_recovery() {
    let (mut app, source, _, _) = fixture();
    let target = row(&mut app, 1);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(1, true);
    app.update();
    app.world_mut()
        .entity_mut(target)
        .insert(InteractionDisabled);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(1, false);
    app.update();
    assert!(app.world().get::<InteractionDisabled>(target).is_some());
}

#[test]
fn pressed_lifecycle_cleans_up_without_leaking_to_replaced_entries() {
    let (mut app, source, _, viewport) = fixture();
    let target = row(&mut app, 1);
    let child = app.world().get::<Children>(target).unwrap()[0];
    app.world_mut().trigger(primary_press(child));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_some());
    app.world_mut().trigger(primary_release(child));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_none());
    app.world_mut().trigger(primary_press(child));
    app.world_mut().flush();
    app.world_mut().trigger(primary_cancel(child));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_none());
    app.world_mut().trigger(primary_press(child));
    app.world_mut().flush();
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .insert(1, "replacement".into())
        .unwrap();
    app.update();
    assert!(app.world().get::<Pressed>(target).is_none());
    app.world_mut().trigger(primary_press(target));
    app.world_mut().flush();
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(1, true);
    app.update();
    assert!(app.world().get::<Pressed>(target).is_none());
    app.world_mut().trigger(primary_press(target));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_none());
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 70.0;
    app.update();
    assert!(app.world().get_entity(target).is_err());
}

#[test]
fn accessibility_and_descendant_control_propagation_are_independent() {
    let (mut app, _, root, _) = fixture();
    assert!(app.world().get::<ListBox>(root).is_none());
    assert_eq!(
        app.world().get::<AccessibilityNode>(root).unwrap().role(),
        accesskit::Role::ListBox
    );
    assert_eq!(app.world().get::<TabIndex>(root).unwrap().0, 0);
    let target = row(&mut app, 0);
    assert!(app.world().get::<ListItem>(target).is_some());
    assert!(app.world().get::<Selectable>(target).is_some());
    assert!(app.world().get::<TabIndex>(target).is_none());
    assert_eq!(
        app.world().get::<AccessibilityNode>(target).unwrap().role(),
        accesskit::Role::ListItem
    );
    let child = app.world().get::<Children>(target).unwrap()[0];
    app.world_mut()
        .entity_mut(child)
        .observe(|mut event: On<Pointer<bevy::picking::events::Click>>| event.propagate(false));
    app.world_mut().trigger(primary_click(child));
    app.update();
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        WidgetryListViewState::default()
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn focused_descendant_and_disabled_wheel_do_not_activate_the_list_or_outer_scroll() {
    let (mut app, _, root, viewport) = fixture();
    let window = keyboard(&mut app, root);
    app.init_resource::<OuterKeys>();
    let parent=app.world_mut().spawn_scene(bsn! {
        ScrollArea
        Node {overflow: Overflow::scroll_y()}
        template(|_|Ok(ComputedNode {size:Vec2::splat(100.0),content_size:Vec2::splat(500.0),inverse_scale_factor:1.0,..default()}))
    }).unwrap().id();
    app.world_mut()
        .entity_mut(parent)
        .add_child(root)
        .observe(|_: On<FocusedInput<KeyboardInput>>, mut keys: ResMut<OuterKeys>| keys.0 += 1);
    let target = row(&mut app, 0);
    let child = app.world().get::<Children>(target).unwrap()[0];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(child, FocusCause::Navigated);
    press_key(&mut app, window, KeyCode::ArrowDown);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        None
    );
    assert_eq!(app.world().resource::<OuterKeys>().0, 1);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().flush();
    wheel(&mut app, target);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
    assert_eq!(app.world().get::<ScrollPosition>(parent).unwrap().0.y, 0.0);
}

#[test]
fn programmatic_selection_waits_for_bootstrap_layout() {
    let (mut app, _, root, viewport) = fixture();
    app.world_mut()
        .get_mut::<ComputedNode>(viewport)
        .unwrap()
        .size = Vec2::ZERO;
    select(&mut app, root, 8);
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
    app.world_mut()
        .get_mut::<ComputedNode>(viewport)
        .unwrap()
        .size = Vec2::new(100.0, 30.0);
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        70.0
    );
    let target = row(&mut app, 8);
    assert!(app.world().get::<Selected>(target).is_some());
}

#[test]
fn cancel_outside_the_list_cleans_up_original_pressed_row() {
    let (mut app, _, _, _) = fixture();
    let target = row(&mut app, 1);
    let outside = app.world_mut().spawn_scene(bsn! {Node}).unwrap().id();
    app.world_mut().trigger(primary_press(target));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_some());
    app.world_mut().trigger(primary_cancel(outside));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_none());
}

#[test]
fn raw_cancel_without_hover_cleans_up_original_pressed_row() {
    let (mut app, _, _, _) = fixture();
    let target = row(&mut app, 1);
    let press = primary_press(target);
    let pointer = press.pointer_id;
    let location = press.pointer_location.clone();
    app.world_mut().trigger(press);
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(target).is_some());
    app.world_mut()
        .write_message(PointerInput::new(pointer, location, PointerAction::Cancel));
    app.update();
    assert!(app.world().get::<Pressed>(target).is_none());
}

fn scoped_row(app: &App, root: Entity, index: usize) -> Entity {
    let world = app.world();
    let viewport = world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|child| world.get::<WidgetryScrollAreaViewport>(*child).is_some())
        .unwrap();
    let content = world.get::<Children>(viewport).unwrap()[0];
    world
        .get::<Children>(content)
        .unwrap()
        .iter()
        .find(|child| {
            world
                .get::<WidgetryListViewItem>(*child)
                .is_some_and(|item| item.index == index)
        })
        .unwrap()
}

fn assert_projection(
    app: &App,
    root: Entity,
    selected: Option<WidgetryListItemId>,
    active: Option<WidgetryListItemId>,
) {
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(root).unwrap(),
        WidgetryListViewState { selected, active }
    );
    let source = app
        .world()
        .get::<WidgetryListView<String>>(root)
        .unwrap()
        .source();
    let model = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap();
    for id in [selected, active].into_iter().flatten() {
        assert!(model.index_of(id).is_some());
    }
    for index in 0..model.len() {
        let row = scoped_row(app, root, index);
        assert_eq!(
            app.world().get::<Selected>(row).is_some(),
            model.id(index) == selected
        );
    }
    let projected = active
        .filter(|_| app.world().resource::<InputFocus>().get() == Some(root))
        .map(|id| scoped_row(app, root, model.index_of(id).unwrap()));
    assert_eq!(
        app.world().get::<ActiveDescendant>(root).unwrap().0,
        projected
    );
}

#[test]
fn deleted_active_uses_successor_then_predecessor_without_clearing_other_selection() {
    let (mut app, source, root, viewport) = fixture();
    app.world_mut()
        .get_mut::<ComputedNode>(viewport)
        .unwrap()
        .size
        .y = 100.0;
    let window = keyboard(&mut app, root);
    select(&mut app, root, 0);
    app.world_mut().resource_mut::<Changes>().0.clear();
    app.update();
    let selected = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(0);
    press_key(&mut app, window, KeyCode::ArrowDown);
    press_key(&mut app, window, KeyCode::ArrowDown);
    let successor = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(3);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .remove(2);
    app.update();
    assert_projection(&app, root, selected, successor);
    press_key(&mut app, window, KeyCode::End);
    let predecessor = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(7);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .remove(8);
    app.update();
    assert_projection(&app, root, selected, predecessor);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .remove(0);
    app.update();
    assert_projection(&app, root, None, predecessor);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn shared_source_views_isolate_user_selection_and_reconcile_their_own_rows() {
    let (mut app, source, first, first_viewport) = fixture();
    app.world_mut()
        .get_mut::<ComputedNode>(first_viewport)
        .unwrap()
        .size
        .y = 100.0;
    let second = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source, @item_height: 10.0,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}) bevy_widgetry_core::text::WidgetryText)])},
        }
    }).unwrap().id();
    let second_viewport = app
        .world()
        .get::<Children>(second)
        .unwrap()
        .iter()
        .find(|child| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(*child)
                .is_some()
        })
        .unwrap();
    app.world_mut()
        .entity_mut(second_viewport)
        .insert(ComputedNode {
            size: Vec2::splat(100.0),
            content_size: Vec2::splat(100.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
    app.update();
    select(&mut app, second, 0);
    app.world_mut().resource_mut::<Changes>().0.clear();
    app.update();
    let target = scoped_row(&app, first, 1);
    app.world_mut().trigger(primary_click(target));
    app.update();
    let model = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap();
    let zero = model.id(0);
    let one = model.id(1).unwrap();
    let successor = model.id(2);
    assert_projection(&app, first, Some(one), Some(one));
    assert_projection(&app, second, zero, zero);
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(first, Some(one), true)]
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .remove(1);
    app.update();
    assert_projection(&app, first, None, successor);
    assert_projection(&app, second, zero, zero);
    let before: Vec<_> = [first, second]
        .into_iter()
        .map(|root| {
            let row = scoped_row(&app, root, 0);
            (root, row, app.world().get::<Children>(row).unwrap()[0])
        })
        .collect();
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(0)
        .unwrap()
        .unwrap() = "shared revision".into();
    app.update();
    for (root, row, old_text) in before {
        assert_eq!(scoped_row(&app, root, 0), row);
        let text = app.world().get::<Children>(row).unwrap()[0];
        assert_ne!(text, old_text);
        assert!(app.world().get_entity(old_text).is_err());
        assert_eq!(app.world().get::<Text>(text).unwrap().0, "shared revision");
    }
    assert_projection(&app, first, None, successor);
    assert_projection(&app, second, zero, zero);
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(first, Some(one), true)]
    );
}

#[test]
fn invalid_pointer_cleanup_preserves_unowned_programmatic_pressed() {
    use bevy::picking::pointer::{PointerId, PointerLocation};
    for id in bevy_widgetry_test_utils::pointer_ids() {
        for failure in ["cancel", "location", "pointer"] {
            let (mut app, _, _, _) = fixture();
            let target = row(&mut app, 0);
            let manual = row(&mut app, 1);
            let mut press = primary_press(target);
            press.pointer_id = id;
            let location = press.pointer_location.clone();
            let pointer = if id == PointerId::Mouse {
                app.world_mut()
                    .query_filtered::<Entity, With<PointerId>>()
                    .single(app.world())
                    .unwrap()
            } else {
                app.world_mut()
                    .spawn((id, PointerLocation::new(location.clone())))
                    .id()
            };
            app.world_mut().entity_mut(manual).insert(Pressed);
            app.world_mut().trigger(press);
            app.world_mut().flush();
            app.update();
            assert!(app.world().get::<Pressed>(target).is_some());
            match failure {
                "cancel" => {
                    app.world_mut().write_message(PointerInput::new(
                        id,
                        location,
                        PointerAction::Cancel,
                    ));
                }
                "location" => {
                    app.world_mut()
                        .get_mut::<PointerLocation>(pointer)
                        .unwrap()
                        .location = None
                }
                _ => {
                    app.world_mut().despawn(pointer);
                }
            }
            app.update();
            app.update();
            assert!(
                app.world().get::<Pressed>(target).is_none(),
                "{id:?}/{failure}"
            );
            assert!(app.world().get::<Pressed>(manual).is_some());
            assert!(app.world().resource::<Changes>().0.is_empty());
        }
    }
}

#[test]
fn local_row_disabled_blocks_keyboard_confirmation_after_model_recovery() {
    let (mut app, source, root, _) = fixture();
    add_keyboard_dispatch(&mut app);
    let native = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let row = scoped_row(&app, root, 0);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(0, true);
    app.update();
    app.world_mut().entity_mut(row).insert(InteractionDisabled);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(0, false);
    app.update();
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    assert!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active
            .is_some()
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    for key in [KeyCode::Enter, KeyCode::Space] {
        press_key(&mut app, native, key);
    }
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .selected,
        None
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
    app.world_mut()
        .entity_mut(row)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    press_key(&mut app, native, KeyCode::Enter);
    assert!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .selected
            .is_some()
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
}

#[test]
fn local_container_disabled_blocks_same_frame_confirmation_of_unmaterialized_item() {
    for confirm in [KeyCode::Enter, KeyCode::Space] {
        for disable_content in [false, true] {
            let (mut app, source, root, viewport) = fixture();
            let disabled = if disable_content {
                app.world().get::<Children>(viewport).unwrap()[0]
            } else {
                viewport
            };
            let window = keyboard(&mut app, root);
            let last = app
                .world()
                .get::<WidgetryListModel<String>>(source)
                .unwrap()
                .id(9);
            assert!(
                !app.world_mut()
                    .query::<&WidgetryListViewItem>()
                    .iter(app.world())
                    .any(|item| Some(item.id) == last)
            );
            app.world_mut()
                .entity_mut(disabled)
                .insert(InteractionDisabled);
            app.world_mut().flush();
            for key_code in [KeyCode::End, confirm] {
                queue_key(
                    &mut app,
                    KeyboardInput {
                        key_code,
                        logical_key: Key::Unidentified(NativeKey::Unidentified),
                        state: ButtonState::Pressed,
                        text: None,
                        repeat: false,
                        window,
                    },
                );
            }
            app.update();
            let state = app.world().get::<WidgetryListViewState>(root).unwrap();
            assert_eq!(state.active, last);
            assert_eq!(state.selected, None);
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
            assert!(app.world().resource::<Changes>().0.is_empty());
            app.world_mut()
                .entity_mut(disabled)
                .remove::<InteractionDisabled>();
            app.world_mut().flush();
            press_key(&mut app, window, confirm);
            assert_eq!(
                app.world()
                    .get::<WidgetryListViewState>(root)
                    .unwrap()
                    .selected,
                last
            );
            assert_eq!(
                app.world().resource::<Changes>().0,
                vec![(root, last, true)]
            );
        }
    }
}

#[test]
fn local_disabled_cancels_owned_press_before_same_frame_recovery() {
    for id in bevy_widgetry_test_utils::pointer_ids() {
        for disable_viewport in [false, true] {
            let (mut app, _, _, viewport) = fixture();
            let target = row(&mut app, 0);
            let manual = row(&mut app, 1);
            app.world_mut().entity_mut(manual).insert(Pressed);
            let mut press = primary_press(target);
            press.pointer_id = id;
            app.world_mut().trigger(press);
            app.world_mut().flush();
            assert!(app.world().get::<Pressed>(target).is_some());
            let disabled = if disable_viewport { viewport } else { target };
            app.world_mut()
                .entity_mut(disabled)
                .insert(InteractionDisabled);
            app.world_mut().flush();
            assert!(
                app.world().get::<Pressed>(target).is_none(),
                "{id:?}/{disable_viewport}"
            );
            assert!(app.world().get::<Pressed>(manual).is_some());
            app.world_mut()
                .entity_mut(disabled)
                .remove::<InteractionDisabled>();
            app.world_mut().flush();
            assert!(app.world().get::<Pressed>(target).is_none());
            assert!(app.world().get::<Pressed>(manual).is_some());
            assert!(app.world().resource::<Changes>().0.is_empty());
        }
    }
}
