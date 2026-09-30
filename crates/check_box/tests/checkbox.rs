//! State：binary Checked 与 tri-state Unchecked/Checked/Indeterminate 分别由官方 adapter 和自有行为维护。
//! Stimuli：pointer、keyboard、公开 set/cycle queue、disabled、theme 和 asset materialization。
//! Guards：首次 Space/Enter、非 repeat Press；disabled 拒绝用户操作，程序化仍允许；无效 root 静默。
//! Invariants：程序化无通知；用户 source/value/is_final 与实际 state 一致；state/a11y/mark 同步。
//! Coverage Map：本文件负责公开输入、队列和 projection/style；tri_state.rs 负责 next-state；
//! style.rs 负责完整优先级、私有 hierarchy 诊断与 mark cache，SVG 通用合同归 Icon。

#![cfg(test)]

use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
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
use bevy_widgetry_core::ThemeMode;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, add_ui_plugins, advance_until, cancel, drag_end, press, primary_click,
    primary_press, primary_release, queue_key, release, scene_app, spawn_ui_camera, switch_theme,
};
use std::time::Duration;

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
        app.update();
        assert_projection(&app, entity, expected);
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

/// ThemeChanged 在不推进 frame 时立即刷新二态与三态 CheckBox 的 indicator 配色。
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

    switch_theme(&mut app, ThemeMode::Light);
    for root in [binary, tri] {
        let indicator = app.world().get::<Children>(root).unwrap()[0];
        assert_eq!(
            app.world().get::<BackgroundColor>(indicator).unwrap().0,
            ThemeMode::Light.colors().control_background
        );
    }
}

/// Checked、Pressed、Hovered 和 disabled 的新增与移除都在下一次 Update 刷新配色。
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
    let colors = ThemeMode::Dark.colors();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.control_background
    );

    for (change, expected) in [
        (Some(Checked), colors.control_background_active),
        (None, colors.control_background),
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
        colors.control_background_pressed
    );
    app.world_mut().entity_mut(root).remove::<Pressed>();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.control_background
    );

    app.world_mut().entity_mut(root).insert(Hovered(true));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.control_background_hovered
    );
    app.world_mut().entity_mut(root).insert(Hovered(false));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.control_background
    );

    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.control_background_disabled
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(indicator).unwrap().0,
        colors.control_background
    );
}

/// 在 style/a11y systems 执行后检查业务投影，期望 state 由每个场景独立给出。
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

// 官方 dispatch 的 Space/Enter 首次 Press 改值；repeat、Release、无关键及 disabled 不产生通知，恢复后可用。
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

// 未提交的 Press 被 Cancel、DragEnd 或 Release 结束，各路径清除 Pressed 且保持三态与通知不变。
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

// 同一 flush 的连续 cycle 读取执行时 state；set 后 cycle、失效 root 和非三态 entity 均保持静默。
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
    let bare = app.world_mut().spawn(WidgetryCheckState::Checked).id();
    let deleted = app.world_mut().spawn_empty().id();
    for entity in [bare, deleted] {
        WidgetryTriStateCheckbox::set_state(
            &mut app.world_mut().commands(),
            entity,
            WidgetryCheckState::Indeterminate,
        );
        WidgetryTriStateCheckbox::cycle_state(&mut app.world_mut().commands(), entity);
    }
    app.world_mut().despawn(deleted);
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<WidgetryCheckState>(bare),
        Some(&WidgetryCheckState::Checked)
    );
    assert!(app.world().get_entity(deleted).is_err());
    assert!(app.world().resource::<Changes>().0.is_empty());
}

// 已预热两种 SVG 的公开三态控件同帧更新 image、visibility 和 a11y，隐藏后重新出现保留 mark identity。
#[test]
fn loaded_mark_projection_tracks_states_and_disabled() {
    let (mut app, root) = tri_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
    let warm = app.world_mut().spawn_scene(bsn! { Node Children [
        @WidgetryIcon { @path: {BuiltinIcon::CheckboxCheck.path()}, @max_size: {Some(UVec2::splat(12))} },
        @WidgetryIcon { @path: {BuiltinIcon::CheckboxIndeterminate.path()}, @max_size: {Some(UVec2::splat(12))} },
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
                    ThemeMode::Dark.colors().foreground_disabled
                } else {
                    ThemeMode::Dark.colors().control_border_active
                }
            );
            assert!(app.world().get::<InheritedVisibility>(child).unwrap().get());
        } else {
            assert!(!app.world().get::<InheritedVisibility>(child).unwrap().get());
        }
    }
    assert!(app.world().resource::<Changes>().0.is_empty());
}
