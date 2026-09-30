use bevy::a11y::AccessibilityNode;
use bevy::input::keyboard::KeyboardInput;
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
    add_keyboard_dispatch, press_key, primary_cancel, primary_click, primary_press,
    primary_release, scene_app,
};

/// 用户通知携带稳定 id，programmatic 与结构修复不追加记录。
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, WidgetryListItemId, bool)>);

/// 只记录抵达外层 UI 的 focused input。
#[derive(Resource, Default)]
struct OuterKeys(usize);

/// 创建可见三行、业务 Text 作为 direct child 的 headless ListView。
fn fixture() -> (App, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<bevy::ui::UiScale>();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .init_resource::<Changes>();
    app.add_observer(
        |event: On<ValueChange<WidgetryListItemId>>, mut changes: ResMut<Changes>| {
            changes.0.push((event.source, event.value, event.is_final));
        },
    );
    let mut model = WidgetryListModel::default();
    for index in 0..10 {
        model.push(index.to_string());
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source, @item_height: 10.0,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
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
    app.update();
    (app, source, root, viewport)
}

/// public identity 可定位 row；测试不读取 private runtime。
fn row(app: &mut App, index: usize) -> Entity {
    app.world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find(|(_, item)| item.index == index)
        .expect("目标应已 rendered")
        .0
}

/// 通过 Commands 调用 public API，flush 后观察 authority state。
fn select(app: &mut App, root: Entity, index: usize) {
    let mut commands = app.world_mut().commands();
    WidgetryListView::<String>::set_selected(&mut commands, root, index);
    app.world_mut().flush();
}

/// 用官方 focused-input dispatch 发送真实 keyboard message。
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

/// 从 picking Pointer event 的相同 location 派发 wheel，实际执行官方 ScrollArea observer。
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

/// descendant click 建立 focus/active/selection；重复 click 只修正 focus/active，不重复通知。
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
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, id, true)]);
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.world_mut()
        .get_mut::<WidgetryListViewState>(root)
        .unwrap()
        .active = None;
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

/// disabled item 可 active 且取得 root focus，但用户 click 不改 selection；focus 和滚动只影响 physical projection。
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

/// selected 删除后清空，active 使用缓存旧位置选 successor；末尾退到 predecessor，insert/move 保留 id。
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

/// programmatic 设置静默、忽略 disabled、修正 active，并对部分可见和 offscreen 目标 top-align/clamp。
#[test]
fn programmatic_selection_reveals_and_corrects_active_silently() {
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
    assert!(app.world().resource::<Changes>().0.is_empty());
    app.world_mut()
        .get_mut::<WidgetryListViewState>(root)
        .unwrap()
        .active = None;
    select(&mut app, root, 8);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        id
    );
    select(&mut app, root, usize::MAX);
    select(&mut app, Entity::PLACEHOLDER, 0);
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

/// keyboard active 不自动提交 selection，disabled item 不跳过；Space/Enter 只通知真正改变，Page 只滚动。
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
        vec![(root, last.unwrap(), true)]
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

/// root disabled 即时关闭 wheel 和 pressed，保留 programmatic/model 更新；解除后恢复 entry 自身 disabled。
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
    assert!(app.world().resource::<Changes>().0.is_empty());
}

/// primary Pressed 由 release/cancel 清理，entry disabled、结构替换和虚拟销毁不保留旧 pressed。
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

/// root 提供 accessibility 与唯一 Tab stop，row 复用官方 ListItem/Selectable；业务控件可以停止 click。
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

/// descendant 的 keyboard focus 不交给 ListView 消费；disabled wheel 不穿透滚动外层容器。
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

/// before-layout 的程序请求在首次有效 viewport 上执行，不丢失 offscreen target。
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

/// Bevy Cancel 可能发给列表之外的当前 hovered entity，仍须清理原列表中该 pointer 的 press。
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

/// 没有任何 hovered entity 时只能收到原始 pointer cancel，不能留下 Pressed。
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
