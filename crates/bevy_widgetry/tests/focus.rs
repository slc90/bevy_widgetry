//! State：TextField/Button/ComboBox focus、Popup closed/open、文本/selection 与列表 authority。
//! Stimuli：真实 pointer 和受控 keyboard batch，覆盖两种 plugin 顺序。
//! invariant 为输入只归当前 focus。
//! Coupling：TextField 外部点击关闭 Popup 后，同帧 keyboard 交给文本，隐藏列表 state/通知保持。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
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

#[derive(Resource, Default)]
struct LostFocus(Vec<Entity>);

#[derive(Resource, Default)]
struct ComboChanges(Vec<(Entity, Option<WidgetryListItemId>)>);

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
        |event: On<bevy::ui_widgets::ValueChange<Option<WidgetryListItemId>>>,
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
                @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list!{Text({value.clone()}) bevy_widgetry_core::text::WidgetryText})},
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
        // 同帧关闭 Popup 后的剩余 keyboard input 可能仍命中隐藏列表。
        // 一次 update 消费整批输入，避免逐帧 focus 清理掩盖错误。
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
