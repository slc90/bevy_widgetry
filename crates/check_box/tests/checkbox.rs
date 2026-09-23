use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled, Pressed};
use bevy::ui_widgets::{ActivateOnPress, Checkbox, ValueChange};
use bevy_widgetry_check_box::{
    WidgetryCheckBox, WidgetryCheckBoxPlugin, WidgetryCheckState, WidgetryTriStateCheckbox,
};
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_test_utils::{primary_click, primary_press, primary_release, scene_app};

/// 保存用户 ValueChange 的内容，验证程序化操作不会写入事件流。
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, WidgetryCheckState, bool)>);

/// 收集三态用户事件的来源、state 和终值标记。
fn record(event: On<ValueChange<WidgetryCheckState>>, mut changes: ResMut<Changes>) {
    changes.0.push((event.source, event.value, event.is_final));
}

/// 创建完整三态 Scene 与 observer，供用户输入路径测试复用。
fn tri_app() -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin)
        .init_resource::<Changes>()
        .add_observer(record);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
        .expect("test entity exists")
        .id();
    app.update();
    (app, entity)
}

/// 三态 Scene 默认未选中，程序化 API 按三态循环并允许 disabled 时更新。
#[test]
fn tri_state_programmatic_cycle() {
    let (mut app, entity) = tri_app();
    assert_eq!(
        app.world().get::<WidgetryCheckState>(entity),
        Some(&WidgetryCheckState::Unchecked)
    );
    for expected in [
        WidgetryCheckState::Checked,
        WidgetryCheckState::Indeterminate,
        WidgetryCheckState::Unchecked,
    ] {
        WidgetryTriStateCheckbox::cycle_state(&mut app.world_mut().commands(), entity);
        app.world_mut().flush();
        assert_eq!(
            app.world().get::<WidgetryCheckState>(entity),
            Some(&expected)
        );
    }
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        entity,
        WidgetryCheckState::Unchecked,
    );
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        Entity::PLACEHOLDER,
        WidgetryCheckState::Checked,
    );
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<WidgetryCheckState>(entity),
        Some(&WidgetryCheckState::Unchecked)
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

/// 普通 Scene 保留官方 Checkbox，Click 经官方 self-update 更新 Checked。
#[test]
fn binary_uses_official_checkbox() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCheckBox })
        .expect("test entity exists")
        .id();
    app.update();
    assert!(app.world().get::<Checkbox>(entity).is_some());
    assert!(app.world().get::<Checked>(entity).is_none());
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().flush();
    assert!(app.world().get::<Checked>(entity).is_some());
}

/// 调用方 Children 追加在内建 indicator 后，仍保留原始文本 child。
#[test]
fn caller_children_follow_indicator() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCheckBox Children [Text("Enable Shadows")] })
        .expect("test scene expands")
        .id();
    app.update();
    let children = app
        .world()
        .get::<Children>(entity)
        .expect("root has children");
    assert_eq!(children.len(), 2);
    assert!(app.world().get::<WidgetryIcon>(children[0]).is_none());
    assert_eq!(
        app.world()
            .get::<Text>(children[1])
            .map(|text| text.0.as_str()),
        Some("Enable Shadows")
    );
}

/// 三态 Click 顺序、终值事件及 disabled 时的无交互边界保持一致。
#[test]
fn tri_state_click_and_disabled() {
    let (mut app, entity) = tri_app();
    for expected in [
        WidgetryCheckState::Checked,
        WidgetryCheckState::Indeterminate,
        WidgetryCheckState::Unchecked,
    ] {
        app.world_mut().trigger(primary_click(entity));
        app.world_mut().flush();
        assert_eq!(
            app.world().get::<WidgetryCheckState>(entity),
            Some(&expected)
        );
    }
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![
            (entity, WidgetryCheckState::Checked, true),
            (entity, WidgetryCheckState::Indeterminate, true),
            (entity, WidgetryCheckState::Unchecked, true),
        ]
    );
    app.world_mut()
        .entity_mut(entity)
        .insert(InteractionDisabled);
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Changes>().0.len(), 3);
    assert_eq!(
        app.world().get::<WidgetryCheckState>(entity),
        Some(&WidgetryCheckState::Unchecked)
    );
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        entity,
        WidgetryCheckState::Indeterminate,
    );
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<WidgetryCheckState>(entity),
        Some(&WidgetryCheckState::Indeterminate)
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 3);
}

/// ActivateOnPress 在 Press 发出一次变化，随后 Click 不重复循环。
#[test]
fn activate_on_press_cycles_once() {
    let (mut app, entity) = tri_app();
    app.world_mut().entity_mut(entity).insert(ActivateOnPress);
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(entity).is_some());
    assert_eq!(
        app.world().get::<WidgetryCheckState>(entity),
        Some(&WidgetryCheckState::Checked)
    );
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(entity).is_none());
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
}

/// 三态 Accessibility 由真实 state 驱动，程序化变化后同步到 Mixed。
#[test]
fn tri_state_accessibility_tracks_programmatic_state() {
    let (mut app, entity) = tri_app();
    let node = &app
        .world()
        .get::<AccessibilityNode>(entity)
        .expect("test entity exists")
        .0;
    assert_eq!(node.role(), Role::CheckBox);
    assert_eq!(node.toggled(), Some(Toggled::False));
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        entity,
        WidgetryCheckState::Indeterminate,
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<AccessibilityNode>(entity)
            .expect("test entity exists")
            .0
            .toggled(),
        Some(Toggled::Mixed)
    );
}

/// 共用 indicator 的唯一 WidgetryIcon 始终存在，Unchecked 只隐藏 mark。
#[test]
fn mark_entity_is_stable_across_states() {
    let (mut app, entity) = tri_app();
    let indicator = app
        .world()
        .get::<Children>(entity)
        .expect("test entity exists")[0];
    let mark = app
        .world()
        .get::<Children>(indicator)
        .expect("test entity exists")[0];
    assert!(app.world().get::<WidgetryIcon>(mark).is_some());
    assert_eq!(
        app.world().get::<Visibility>(mark),
        Some(&Visibility::Hidden)
    );
    for state in [
        WidgetryCheckState::Checked,
        WidgetryCheckState::Indeterminate,
        WidgetryCheckState::Unchecked,
    ] {
        WidgetryTriStateCheckbox::set_state(&mut app.world_mut().commands(), entity, state);
        app.update();
        assert_eq!(
            app.world()
                .get::<Children>(indicator)
                .expect("test entity exists")[0],
            mark
        );
        assert_eq!(
            app.world().get::<Visibility>(mark),
            Some(&if state == WidgetryCheckState::Unchecked {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            })
        );
    }
}
