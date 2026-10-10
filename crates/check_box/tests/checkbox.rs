//! State：binary Checked 与 tri-state Unchecked/Checked/Indeterminate 分别由 Bevy adapter 和自有行为维护。
//! Stimuli：pointer、keyboard、公开 set/cycle queue、disabled、theme 和 asset materialization。
//! Guards：首次 Space/Enter、非 repeat Press。
//! disabled 拒绝用户操作，自有程序 API 仍允许。
//! 无效 root 报错。
//! Transitions：tri-state 按 Unchecked → Checked → Indeterminate → Unchecked 循环。
//! 同值 setter 保持 state。
//! Invariants：自有 tri-state 先提交 authority 再通知。
//! binary Checked 由 Bevy deferred self-update 写入。
//! Couplings：state 驱动 a11y/mark，theme 更新保留 selection 和 mark identity。
//! Coverage Map：本文件负责公开输入、queue 和 projection/style。
//! tri_state.rs 负责 next-state。
//! style.rs 负责完整优先级、私有 hierarchy 诊断与 mark cache，通用 SVG 行为归 Icon。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
use bevy::ecs::world::CommandQueue;
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput, NativeKey},
};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled, Pressed};
use bevy::ui_widgets::{ActivateOnPress, Checkbox, ValueChange};
use bevy::window::PrimaryWindow;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_check_box::{
    WidgetryCheckBox, WidgetryCheckBoxPlugin, WidgetryCheckState, WidgetryTriStateCheckbox,
};
use bevy_widgetry_theme::WidgetryThemeMode;

use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_keyboard_dispatch, add_ui_plugins, advance_until, cancel,
    drag_end, press, primary_click, primary_press, primary_release, queue_key, release, scene_app,
    spawn_ui_camera, switch_theme,
};
use std::time::Duration;

#[test]
fn internal_mark_rejects_public_icon_color_changes() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    for tri_state in [false, true] {
        let root = if tri_state {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
                .unwrap()
                .id()
        } else {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryCheckBox })
                .unwrap()
                .id()
        };
        app.world_mut().flush();
        let indicator = app.world().get::<Children>(root).unwrap()[0];
        let mark = app.world().get::<Children>(indicator).unwrap()[0];
        assert!(WidgetryIcon::set_color_in_world(app.world_mut(), mark, Color::BLACK).is_err());
        assert!(WidgetryIcon::clear_color_in_world(app.world_mut(), mark).is_err());
    }
}

#[test]
fn missing_indicator_reaches_system_error_handler() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTriStateCheckbox Children [Text("label") bevy_widgetry_core::text::WidgetryText] })
        .unwrap()
        .id();
    let healthy = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTriStateCheckbox Children [Text("healthy") bevy_widgetry_core::text::WidgetryText] })
        .unwrap()
        .id();
    app.update();
    let indicator = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut().despawn(indicator);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        root,
        WidgetryCheckState::Checked,
    );
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        healthy,
        WidgetryCheckState::Checked,
    );
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.update()));
    let errors = errors.take();
    assert!(!errors.is_empty());
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert!(
        logs.records()
            .iter()
            .any(|record| record.level == bevy::log::Level::ERROR)
    );
    assert_eq!(
        *app.world().get::<WidgetryCheckState>(root).unwrap(),
        WidgetryCheckState::Checked
    );
    assert_projection(&app, healthy, WidgetryCheckState::Checked);
}

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, WidgetryCheckState, bool)>);

fn record(
    event: On<ValueChange<WidgetryCheckState>>,
    state: Query<&WidgetryCheckState>,
    mut changes: ResMut<Changes>,
) {
    assert_eq!(state.get(event.source).unwrap(), &event.value);
    changes.0.push((event.source, event.value, event.is_final));
}

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

#[test]
fn programmatic_change_notifies_committed_state_once() {
    let (mut app, entity) = tri_app();
    for _ in 0..2 {
        WidgetryTriStateCheckbox::set_state(
            &mut app.world_mut().commands(),
            entity,
            WidgetryCheckState::Checked,
        );
    }
    assert!(app.world().resource::<Changes>().0.is_empty());
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(entity, WidgetryCheckState::Checked, true)]
    );
}

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
        app.update();
        assert_projection(&app, entity, expected);
    }
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        entity,
        WidgetryCheckState::Unchecked,
    );
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<WidgetryCheckState>(entity),
        Some(&WidgetryCheckState::Unchecked)
    );
    assert_eq!(app.world().resource::<Changes>().0.len(), 3);
}

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
    app.update();
    let indicator = app.world().get::<Children>(entity).unwrap()[0];
    let mark = app.world().get::<Children>(indicator).unwrap()[0];
    assert_eq!(
        app.world().get::<Visibility>(mark),
        Some(&Visibility::Inherited)
    );
    app.world_mut().entity_mut(entity).remove::<Checked>();
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(mark),
        Some(&Visibility::Hidden)
    );
    assert_eq!(app.world().get::<Children>(indicator).unwrap()[0], mark);
}

#[test]
fn caller_children_follow_indicator() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCheckBox Children [Text("Enable Shadows") bevy_widgetry_core::text::WidgetryText] })
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
        app.update();
        assert_projection(&app, entity, expected);
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
    assert_eq!(app.world().resource::<Changes>().0.len(), 4);
}

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
    app.world_mut().trigger(primary_press(entity));
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(entity, WidgetryCheckState::Checked, true)]
    );
    app.world_mut().trigger(primary_release(entity));
    app.world_mut().trigger(primary_click(entity));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(entity).is_none());
    assert_eq!(app.world().resource::<Changes>().0.len(), 1);
}

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

#[test]
fn theme_change_refreshes_checkboxes_immediately() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    let binary = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCheckBox })
        .unwrap()
        .id();
    let tri = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
        .unwrap()
        .id();
    app.update();

    switch_theme(&mut app, WidgetryThemeMode::Light);
    for root in [binary, tri] {
        let indicator = app.world().get::<Children>(root).unwrap()[0];
        assert_eq!(
            app.world().get::<BackgroundColor>(indicator).unwrap().0,
            WidgetryThemeMode::Light
                .colors()
                .check_box
                .unchecked
                .normal
                .background
        );
    }
}

#[test]
fn state_changes_refresh_checkbox_style() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCheckBox })
        .unwrap()
        .id();
    app.update();
    let indicator = app.world().get::<Children>(root).unwrap()[0];
    let colors = WidgetryThemeMode::Dark.colors();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.check_box.unchecked.normal.background
    );

    for (change, expected) in [
        (Some(Checked), colors.check_box.checked.normal.background),
        (None, colors.check_box.unchecked.normal.background),
    ] {
        if let Some(checked) = change {
            app.world_mut().entity_mut(root).insert(checked);
        } else {
            app.world_mut().entity_mut(root).remove::<Checked>();
        }
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(indicator).unwrap().0,
            expected
        );
    }

    app.world_mut().entity_mut(root).insert(Pressed);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.check_box.unchecked.pressed.background
    );
    app.world_mut().entity_mut(root).remove::<Pressed>();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.check_box.unchecked.normal.background
    );

    app.world_mut().entity_mut(root).insert(Hovered(true));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.check_box.unchecked.hovered.background
    );
    app.world_mut().entity_mut(root).insert(Hovered(false));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.check_box.unchecked.normal.background
    );

    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.check_box.unchecked.disabled.background
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.check_box.unchecked.normal.background
    );
}

fn assert_projection(app: &App, root: Entity, expected: WidgetryCheckState) {
    assert_eq!(app.world().get::<WidgetryCheckState>(root), Some(&expected));
    let toggled = match expected {
        WidgetryCheckState::Unchecked => Toggled::False,
        WidgetryCheckState::Checked => Toggled::True,
        WidgetryCheckState::Indeterminate => Toggled::Mixed,
    };
    assert_eq!(
        app.world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .0
            .toggled(),
        Some(toggled)
    );
    let indicator = app.world().get::<Children>(root).unwrap()[0];
    let mark = app.world().get::<Children>(indicator).unwrap()[0];
    assert_eq!(
        *app.world().get::<Visibility>(mark).unwrap(),
        if expected == WidgetryCheckState::Unchecked {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        }
    );
}

#[test]
fn keyboard_guards_and_reenable_preserve_projection() {
    let (mut app, root) = tri_app();
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Pressed);
    let mut expected = WidgetryCheckState::Unchecked;
    let mut events = Vec::new();
    for (key, state, repeat, disabled, next) in [
        (
            KeyCode::Space,
            ButtonState::Pressed,
            false,
            false,
            Some(WidgetryCheckState::Checked),
        ),
        (KeyCode::Enter, ButtonState::Pressed, true, false, None),
        (KeyCode::Space, ButtonState::Released, false, false, None),
        (KeyCode::KeyX, ButtonState::Pressed, false, false, None),
        (KeyCode::Space, ButtonState::Pressed, false, true, None),
        (KeyCode::Enter, ButtonState::Pressed, false, true, None),
        (
            KeyCode::Enter,
            ButtonState::Pressed,
            false,
            false,
            Some(WidgetryCheckState::Indeterminate),
        ),
        (
            KeyCode::Space,
            ButtonState::Pressed,
            false,
            false,
            Some(WidgetryCheckState::Unchecked),
        ),
    ] {
        if disabled {
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
        } else {
            app.world_mut()
                .entity_mut(root)
                .remove::<InteractionDisabled>();
        }
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: key,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat,
                window,
            },
        );
        app.update();
        if let Some(next) = next {
            expected = next;
            events.push((root, next, true));
        }
        assert_projection(&app, root, expected);
        assert_eq!(app.world().resource::<Changes>().0, events);
    }
}

#[test]
fn interrupted_press_does_not_cycle() {
    let (mut app, root) = tri_app();
    for finish in [cancel, drag_end, release] {
        press(&mut app, root);
        assert!(app.world().get::<Pressed>(root).is_some());
        finish(&mut app, root);
        app.update();
        assert!(app.world().get::<Pressed>(root).is_none());
        assert_projection(&app, root, WidgetryCheckState::Unchecked);
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn queued_programmatic_actions_use_execution_state() {
    let (mut app, root) = tri_app();
    for expected in [
        WidgetryCheckState::Checked,
        WidgetryCheckState::Indeterminate,
        WidgetryCheckState::Unchecked,
    ] {
        WidgetryTriStateCheckbox::cycle_state(&mut app.world_mut().commands(), root);
        app.world_mut().commands().queue(move |world: &mut World| {
            assert_eq!(world.get::<WidgetryCheckState>(root), Some(&expected));
        });
    }
    app.world_mut().flush();
    app.update();
    assert_projection(&app, root, WidgetryCheckState::Unchecked);
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        root,
        WidgetryCheckState::Checked,
    );
    WidgetryTriStateCheckbox::cycle_state(&mut app.world_mut().commands(), root);
    app.world_mut().flush();
    app.update();
    assert_projection(&app, root, WidgetryCheckState::Indeterminate);
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![
            (root, WidgetryCheckState::Checked, true),
            (root, WidgetryCheckState::Indeterminate, true),
            (root, WidgetryCheckState::Unchecked, true),
            (root, WidgetryCheckState::Checked, true),
            (root, WidgetryCheckState::Indeterminate, true),
        ]
    );
}

#[test]
fn invalid_queued_targets_report_errors_without_changes() {
    let (mut app, healthy) = tri_app();
    app.set_error_handler(ErrorCapture::handler());
    let bare = app.world_mut().spawn(WidgetryCheckState::Checked).id();
    let missing = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(missing)
        .remove::<WidgetryCheckState>();
    let deleted = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
        .unwrap()
        .id();
    // 独立 queue 避免 World::despawn 自己 flush World 的 commands，保证删除发生在 setter 执行前。
    let mut queue = CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, app.world());
        for entity in [bare, missing, deleted, Entity::PLACEHOLDER] {
            WidgetryTriStateCheckbox::set_state(
                &mut commands,
                entity,
                WidgetryCheckState::Indeterminate,
            );
            WidgetryTriStateCheckbox::cycle_state(&mut commands, entity);
        }
    }
    app.world_mut().despawn(deleted);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| queue.apply(app.world_mut())));
    let errors = errors.take();
    assert_eq!(errors.len(), 8);
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert!(
        errors
            .iter()
            .all(|error| error.to_string().contains("三态 CheckBox"))
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        8
    );
    assert_eq!(
        app.world().get::<WidgetryCheckState>(bare),
        Some(&WidgetryCheckState::Checked)
    );
    assert!(app.world().get::<WidgetryCheckState>(missing).is_none());
    assert!(app.world().get_entity(deleted).is_err());
    assert_eq!(
        app.world().get::<WidgetryCheckState>(healthy),
        Some(&WidgetryCheckState::Unchecked)
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn triggering_notification_does_not_update_authority() {
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
        .unwrap()
        .id();
    app.world_mut().trigger(ValueChange {
        source: root,
        value: WidgetryCheckState::Checked,
        is_final: true,
    });
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<WidgetryCheckState>(root),
        Some(&WidgetryCheckState::Unchecked)
    );
}

#[test]
fn loaded_mark_projection_tracks_states_and_disabled() {
    let (mut app, root) = tri_app();
    add_ui_plugins(&mut app);
    // 把 visibility 消费放在允许的最早位置，避免偶然调度顺序掩盖同帧投影缺失。
    app.configure_sets(
        PostUpdate,
        bevy::camera::visibility::VisibilitySystems::VisibilityPropagate
            .before(bevy::ui::UiSystems::Propagate),
    );
    spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
    let label = app
        .world_mut()
        .spawn((
            Text::new("label"),
            bevy_widgetry_core::text::WidgetryText,
            ChildOf(root),
        ))
        .id();
    let mut overrides = bevy_widgetry_check_box::WidgetryCheckBoxColorOverrides::default();
    overrides.unchecked.normal.foreground = Some(Color::linear_rgb(0.8, 0.1, 0.2));
    overrides.checked.normal.foreground = Some(Color::linear_rgb(0.2, 0.3, 0.7));
    overrides.indeterminate.disabled.foreground = Some(Color::linear_rgb(0.1, 0.8, 0.3));
    bevy_widgetry_check_box::WidgetryCheckBoxColorOverrides::set_in_world(
        app.world_mut(),
        root,
        overrides.clone(),
    )
    .unwrap();
    let warm = app.world_mut().spawn_scene(bsn! { Node Children [
        @WidgetryIcon { @path: {BuiltinIcon::CheckboxCheck.path()}, @max_size: {Some(UVec2::splat(12))} }--
        @WidgetryIcon { @path: {BuiltinIcon::CheckboxIndeterminate.path()}, @max_size: {Some(UVec2::splat(12))} }
    ] }).unwrap().id();
    let icons = app.world().get::<Children>(warm).unwrap().to_vec();
    advance_until(
        &mut app,
        Duration::from_secs(2),
        "CheckBox 预加载两种 mark",
        |world| {
            icons
                .iter()
                .all(|icon| world.get::<Children>(*icon).is_some())
        },
    )
    .unwrap();
    let images: Vec<_> = icons
        .iter()
        .map(|icon| {
            let image = app.world().get::<Children>(*icon).unwrap()[0];
            app.world().get::<ImageNode>(image).unwrap().image.clone()
        })
        .collect();
    let indicator = app.world().get::<Children>(root).unwrap()[0];
    let mark = app.world().get::<Children>(indicator).unwrap()[0];
    for (state, disabled, image_index) in [
        (WidgetryCheckState::Checked, false, 0),
        (WidgetryCheckState::Indeterminate, true, 1),
        (WidgetryCheckState::Unchecked, false, 1),
        (WidgetryCheckState::Checked, false, 0),
    ] {
        if disabled {
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
        } else {
            app.world_mut()
                .entity_mut(root)
                .remove::<InteractionDisabled>();
        }
        WidgetryTriStateCheckbox::set_state(&mut app.world_mut().commands(), root, state);
        app.update();
        assert_projection(&app, root, state);
        let expected_label = match state {
            WidgetryCheckState::Unchecked => overrides.unchecked.normal.foreground.unwrap(),
            WidgetryCheckState::Checked => overrides.checked.normal.foreground.unwrap(),
            WidgetryCheckState::Indeterminate => {
                overrides.indeterminate.disabled.foreground.unwrap()
            }
        };
        assert_eq!(
            app.world().get::<TextColor>(label).unwrap().0,
            expected_label
        );
        assert_eq!(app.world().get::<Children>(indicator).unwrap()[0], mark);
        let child = app.world().get::<Children>(mark).unwrap()[0];
        let image = app.world().get::<ImageNode>(child).unwrap();
        assert_eq!(image.image, images[image_index]);
        assert!(
            app.world()
                .resource::<Assets<Image>>()
                .get(&image.image)
                .is_some()
        );
        if state != WidgetryCheckState::Unchecked {
            assert_eq!(
                image.color,
                if disabled {
                    WidgetryThemeMode::Dark
                        .colors()
                        .check_box
                        .unchecked
                        .disabled
                        .foreground
                } else {
                    WidgetryThemeMode::Dark
                        .colors()
                        .check_box
                        .checked
                        .normal
                        .mark
                }
            );
            assert!(app.world().get::<InheritedVisibility>(child).unwrap().get());
        } else {
            assert!(!app.world().get::<InheritedVisibility>(child).unwrap().get());
        }
    }
    assert_eq!(app.world().resource::<Changes>().0.len(), 4);
}

#[test]
fn materialized_indeterminate_checkbox_consumes_ready_glyph_and_colors_in_same_frame() {
    use bevy_widgetry_core::ui::WidgetryUiSystems;
    let mut app = scene_app();
    app.add_plugins(WidgetryCheckBoxPlugin);
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
    app.configure_sets(
        PostUpdate,
        bevy::camera::visibility::VisibilitySystems::VisibilityPropagate
            .before(bevy::ui::UiSystems::Propagate),
    );
    let warm = app.world_mut().spawn_scene(bsn! { Node Children [
        @WidgetryIcon { @path: {BuiltinIcon::CheckboxCheck.path()} }--
        @WidgetryIcon { @path: {BuiltinIcon::CheckboxIndeterminate.path()}, @max_size: {Some(UVec2::splat(12))} }
    ] }).unwrap().id();
    let icons = app.world().get::<Children>(warm).unwrap().to_vec();
    advance_until(
        &mut app,
        Duration::from_secs(2),
        "同帧构造预加载 glyph",
        |world| {
            icons
                .iter()
                .all(|icon| world.get::<Children>(*icon).is_some())
        },
    )
    .unwrap();
    let expected_image = app.world().get::<Children>(icons[1]).unwrap()[0];
    let expected_image = app
        .world()
        .get::<ImageNode>(expected_image)
        .unwrap()
        .image
        .clone();
    let color = Color::linear_rgb(0.4, 0.7, 0.1);
    let mut colors = bevy_widgetry_check_box::WidgetryCheckBoxColorOverrides::default();
    colors.indeterminate.normal.foreground = Some(color);
    colors.indeterminate.normal.mark = Some(color);
    app.add_systems(
        PostUpdate,
        (move |mut commands: Commands, mut spawned: Local<bool>| {
            if !*spawned {
                let root = commands
                    .spawn_scene(bsn! {
                        @WidgetryTriStateCheckbox { @colors: {colors.clone()} }
                        Children [Text("late label") bevy_widgetry_core::text::WidgetryText]
                    })
                    .id();
                WidgetryTriStateCheckbox::set_state(
                    &mut commands,
                    root,
                    WidgetryCheckState::Indeterminate,
                );
                *spawned = true;
            }
        })
        .in_set(WidgetryUiSystems::Materialize),
    );
    app.update();
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryTriStateCheckbox>>()
        .single(app.world())
        .unwrap();
    let children = app.world().get::<Children>(root).unwrap();
    let indicator = children[0];
    let label = children[1];
    let mark = app.world().get::<Children>(indicator).unwrap()[0];
    let image = app.world().get::<Children>(mark).unwrap()[0];
    assert_eq!(app.world().get::<TextColor>(label).unwrap().0, color);
    assert_eq!(
        app.world().get::<ImageNode>(image).unwrap().image,
        expected_image
    );
    assert_eq!(app.world().get::<ImageNode>(image).unwrap().color, color);
    assert!(app.world().get::<InheritedVisibility>(image).unwrap().get());
    assert!(
        app.world()
            .get::<bevy::ui::ComputedStackIndex>(image)
            .unwrap()
            .0
            > app
                .world()
                .get::<bevy::ui::ComputedStackIndex>(root)
                .unwrap()
                .0
    );
}
