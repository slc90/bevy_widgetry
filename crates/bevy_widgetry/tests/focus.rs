// 测试及其 helper 使用断言和 expect 验证 contract；生产代码仍禁止主动 panic。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

//! State：TextField/Button/ComboBox focus、Popup closed/open、文本/selection 与列表 authority。
//! Stimuli：真实 pointer 和受控 keyboard batch，覆盖两种 plugin 顺序；invariant 为输入只归当前 focus。
//! Coupling：TextField 外部点击关闭 Popup 后，同帧 keyboard 交给文本，隐藏列表 state/通知保持。

#![cfg(test)]

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{
        FocusLost, InputFocus,
        tab_navigation::{TabIndex, TabNavigationPlugin},
    },
    prelude::*,
    text::{EditableText, TextEdit},
    ui::Pressed,
    window::PrimaryWindow,
};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonPlugin},
    combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt, WidgetryComboBoxPlugin},
    list_view::{
        WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer, WidgetryListViewState,
    },
    style::WidgetryAppExt,
    text_field::{WidgetryTextField, WidgetryTextFieldPlugin},
};
use bevy_widgetry_asset::BuiltinFont;
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, advance_until, press, primary_click, queue_key, text_edit_app,
    text_input_app,
};
use std::time::Duration;

/// 记录实际 dispatch 的 focus lost 目标，避免仅凭 resource 值判断集成成功。
#[derive(Resource, Default)]
struct LostFocus(Vec<Entity>);

/// 只收集 facade ComboBox root 用户通知，隐藏列表不能误发结果。
#[derive(Resource, Default)]
struct ComboChanges(Vec<(Entity, WidgetryListItemId)>);

// TextField 与 Button 的真实组合通过官方 pointer focus 将 focus 转到 TabIndex(-1) Button。
#[test]
fn button_press_focuses_button_even_when_propagation_stops() {
    for button_first in [false, true] {
        let mut app = text_input_app();
        if button_first {
            app.add_plugins(WidgetryButtonPlugin);
            app.add_plugins(WidgetryTextFieldPlugin);
        } else {
            app.add_plugins(WidgetryTextFieldPlugin);
            app.add_plugins(WidgetryButtonPlugin);
        }
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.init_resource::<LostFocus>();
        assert!(app.is_plugin_added::<TabNavigationPlugin>());
        app.add_observer(|event: On<FocusLost>, mut lost: ResMut<LostFocus>| {
            lost.0.push(event.entity)
        });
        let text = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTextField })
            .unwrap()
            .id();
        let button = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();
        press(&mut app, text);
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(text));
        press(&mut app, button);
        app.update();
        assert!(app.world().get::<Pressed>(button).is_some());
        assert_eq!(app.world().get::<TabIndex>(button), Some(&TabIndex(-1)));
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
        assert_eq!(app.world().resource::<LostFocus>().0, [text]);
    }
}

// 从 TextField 点击无 TabIndex ancestor 的普通 Node 时，官方 AcquireFocus 清除 focus。
#[test]
fn plain_node_press_clears_text_focus() {
    let mut app = text_input_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    let text = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    let node = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
    press(&mut app, text);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(text));
    press(&mut app, node);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}

/// 真实编辑 fixture 在构造 Widget 前预加载字体，输入消费不靠之后额外 update 等待。
fn editing_app(combo_first: bool) -> App {
    let mut app = text_edit_app();
    if combo_first {
        app.add_plugins(WidgetryComboBoxPlugin)
            .add_plugins(WidgetryTextFieldPlugin);
    } else {
        app.add_plugins(WidgetryTextFieldPlugin)
            .add_plugins(WidgetryComboBoxPlugin);
    }
    app.register_widgetry_combo_box::<String>()
        .unwrap()
        .init_resource::<ComboChanges>();
    app.add_observer(
        |event: On<bevy::ui_widgets::ValueChange<WidgetryListItemId>>,
         roots: Query<(), With<WidgetryComboBox<String>>>,
         mut changes: ResMut<ComboChanges>| {
            if roots.contains(event.source) {
                changes.0.push((event.source, event.value));
            }
        },
    );
    add_keyboard_dispatch(&mut app);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    advance_until(
        &mut app,
        Duration::from_secs(10),
        "facade 编辑前置字体",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app
}

/// TextField→Popup/ListView→TextField 完整消费者路径，两种 plugin 顺序均把同帧后续输入交还文本。
#[test]
fn text_field_reclaims_input_after_closing_combo_popup_in_the_same_frame() {
    for combo_first in [false, true] {
        let mut app = editing_app(combo_first);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let text = app.world_mut().spawn_scene(bsn! {
            @WidgetryTextField
            Node {width: px(200), height: px(32), left: px(20), top: px(20), position_type: PositionType::Absolute}
        }).unwrap().id();
        let mut model = WidgetryListModel::default();
        for value in ["one", "two", "three"] {
            model.push(value.to_string()).unwrap();
        }
        let source = app.world_mut().spawn(model).id();
        let combo = app.world_mut().spawn_scene(bsn! {
            @WidgetryComboBox::<String> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
            }
            Node {width: px(200), height: px(32), left: px(20), top: px(80), position_type: PositionType::Absolute}
        }).unwrap().id();
        app.update();
        press(&mut app, text);
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: KeyCode::KeyS,
                logical_key: Key::Character("seed".into()),
                state: ButtonState::Pressed,
                text: Some("seed".into()),
                repeat: false,
                window,
            },
        );
        app.update();
        assert_eq!(
            app.world()
                .get::<EditableText>(text)
                .unwrap()
                .value()
                .to_string(),
            "seed"
        );
        app.world_mut()
            .get_mut::<EditableText>(text)
            .unwrap()
            .queue_edit(TextEdit::SelectAll);
        app.update();
        assert_eq!(
            app.world()
                .get::<EditableText>(text)
                .unwrap()
                .editor()
                .raw_selection()
                .text_range(),
            0..4
        );
        let children = app.world().get::<Children>(combo).unwrap();
        let (field, popup) = (children[0], children[1]);
        let list = app.world().get::<Children>(popup).unwrap()[0];
        press(&mut app, field);
        app.world_mut().trigger(primary_click(field));
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
        assert_eq!(
            app.world()
                .get::<EditableText>(text)
                .unwrap()
                .value()
                .to_string(),
            "seed"
        );
        let state = *app.world().get::<WidgetryListViewState>(list).unwrap();
        press(&mut app, text);
        app.world_mut().trigger(primary_click(text));
        app.world_mut().flush();
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(text));
        // 先让 ArrowDown/End 有机会误改隐藏 active，再插入字符和 Enter；整批只推进一次。
        for (key_code, logical_key, value) in [
            (KeyCode::ArrowDown, Key::ArrowDown, None),
            (KeyCode::End, Key::End, None),
            (KeyCode::KeyX, Key::Character("x".into()), Some("x")),
            (KeyCode::Enter, Key::Enter, None),
        ] {
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code,
                    logical_key,
                    state: ButtonState::Pressed,
                    text: value.map(Into::into),
                    repeat: false,
                    window,
                },
            );
        }
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(text));
        assert_eq!(
            *app.world().get::<WidgetryListViewState>(list).unwrap(),
            state
        );
        assert!(app.world().resource::<ComboChanges>().0.is_empty());
        let content = app
            .world()
            .get::<EditableText>(text)
            .unwrap()
            .value()
            .to_string();
        assert_eq!(content, "seedx", "同帧 End/字符必须被 TextField 消费");
    }
}
