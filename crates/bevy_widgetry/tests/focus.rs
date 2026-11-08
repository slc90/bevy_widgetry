#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]
#![cfg(all(
    feature = "button",
    feature = "combo_box",
    feature = "list_view",
    feature = "text_field"
))]

const ASYNC_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

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
    list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer},
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

fn input_entity(world: &World, root: Entity) -> Entity {
    world
        .get::<bevy_widgetry_core::input_target::WidgetryInputTarget>(root)
        .unwrap()
        .target
}

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
        assert!(!app.is_plugin_added::<TabNavigationPlugin>());
        app.add_observer(|event: On<FocusLost>, mut lost: ResMut<LostFocus>| {
            if event.entity == event.original_event_target() {
                lost.0.push(event.entity);
            }
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
        let input = input_entity(app.world(), text);
        press(&mut app, input);
        app.update();
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(input_entity(app.world(), text))
        );
        press(&mut app, button);
        assert!(app.world().get::<Pressed>(button).is_some());
        app.update();
        assert_eq!(app.world().get::<TabIndex>(button), Some(&TabIndex(-1)));
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
        assert_eq!(
            app.world().resource::<LostFocus>().0,
            [input_entity(app.world(), text)]
        );
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
    let input = input_entity(app.world(), text);
    press(&mut app, input);
    app.update();
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(input_entity(app.world(), text))
    );
    press(&mut app, node);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}

fn editing_app(combo_first: bool) -> App {
    let mut app = text_edit_app();
    #[cfg(feature = "window")]
    app.add_plugins(bevy_widgetry::window::WidgetryWindowPlugin);
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
    let emoji = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Emoji.path());
    advance_until(
        &mut app,
        ASYNC_WAIT_TIMEOUT,
        "facade 编辑前置字体",
        |world| {
            world.resource::<Assets<Font>>().contains(&font)
                && world.resource::<Assets<Font>>().contains(&emoji)
        },
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
                @renderer: {WidgetryListViewRenderer::new(|value: &String| bsn_list!{Text({value.clone()}) bevy_widgetry_core::text::WidgetryText})},
            }
            Node {width: px(200), height: px(32), left: px(20), top: px(80), position_type: PositionType::Absolute}
        }).unwrap().id();
        app.update();
        let input = input_entity(app.world(), text);
        press(&mut app, input);
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
                .get::<EditableText>(input_entity(app.world(), text))
                .unwrap()
                .value()
                .to_string(),
            "seed"
        );
        {
            let input = input_entity(app.world(), text);
            app.world_mut().get_mut::<EditableText>(input)
        }
        .unwrap()
        .queue_edit(TextEdit::SelectAll);
        app.update();
        assert_eq!(
            app.world()
                .get::<EditableText>(input_entity(app.world(), text))
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
                .get::<EditableText>(input_entity(app.world(), text))
                .unwrap()
                .value()
                .to_string(),
            "seed"
        );
        let state = list_snapshot(app.world(), list);
        let input = input_entity(app.world(), text);
        press(&mut app, input);
        app.world_mut().trigger(primary_click(input));
        app.world_mut().flush();
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(input_entity(app.world(), text))
        );
        // Remaining keyboard input can reach the hidden list after the popup closes in the same frame.
        // Consume the entire batch in one update so focus cleanup between frames cannot hide the fault.
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
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(input_entity(app.world(), text))
        );
        assert_eq!(list_snapshot(app.world(), list), state);
        assert!(app.world().resource::<ComboChanges>().0.is_empty());
        let content = app
            .world()
            .get::<EditableText>(input_entity(app.world(), text))
            .unwrap()
            .value()
            .to_string();
        assert_eq!(content, "seedx", "同帧 End/字符必须被 TextField 消费");
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ListSnapshot {
    selected: Option<bevy_widgetry_list_view::WidgetryListItemId>,
    active: Option<bevy_widgetry_list_view::WidgetryListItemId>,
}
fn list_snapshot(world: &World, root: Entity) -> ListSnapshot {
    ListSnapshot {
        selected: world
            .get::<bevy_widgetry_list_view::WidgetryListSelectionState>(root)
            .unwrap()
            .selected_id(),
        active: world
            .get::<bevy_widgetry_list_view::WidgetryListViewState>(root)
            .unwrap()
            .active,
    }
}

#[cfg(feature = "window")]
#[test]
fn window_composition_keeps_tab_on_the_editor_and_pointer_switches_focus_once() {
    use bevy::input_focus::FocusGained;
    use bevy::ui_widgets::Activate;
    use bevy_widgetry::window::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, owned_widgetry_window,
        widgetry_window_target,
    };
    #[derive(Resource, Default)]
    struct FocusEvents {
        transitions: Vec<(bool, Entity)>,
        activations: Vec<Entity>,
    }
    let mut app = editing_app(false);
    app.init_resource::<FocusEvents>();
    assert!(!app.is_plugin_added::<TabNavigationPlugin>());
    app.add_observer(|event: On<FocusGained>, mut seen: ResMut<FocusEvents>| {
        if event.entity == event.original_event_target() {
            seen.transitions.push((true, event.entity));
        }
    });
    app.add_observer(|event: On<FocusLost>, mut seen: ResMut<FocusEvents>| {
        if event.entity == event.original_event_target() {
            seen.transitions.push((false, event.entity));
        }
    });
    app.add_observer(|event: On<Activate>, mut seen: ResMut<FocusEvents>| {
        seen.activations.push(event.entity);
    });
    let root = app.world_mut().spawn_scene(bsn! {
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{
            @WidgetryTextField Name("CompositionText")--
            @WidgetryButton Name("CompositionButton")
        })
    }).unwrap().id();
    for _ in 0..3 {
        app.update();
    }
    let window = widgetry_window_target(app.world(), root).unwrap();
    app.world_mut().entity_mut(window).insert(PrimaryWindow);
    let named: std::collections::HashMap<_, _> = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .map(|(entity, name)| (name.as_str().to_owned(), entity))
        .collect();
    let field = named["CompositionText"];
    let button = named["CompositionButton"];
    let editor = input_entity(app.world(), field);
    assert!(app.world().get::<TabIndex>(field).is_none());
    assert_eq!(app.world().get::<TabIndex>(button), Some(&TabIndex(-1)));
    press(&mut app, editor);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    app.world_mut()
        .resource_mut::<FocusEvents>()
        .transitions
        .clear();
    for shift in [false, true] {
        if shift {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ShiftLeft);
        }
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: KeyCode::Tab,
                logical_key: Key::Tab,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            },
        );
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
        assert!(app.world().resource::<FocusEvents>().transitions.is_empty());
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ShiftLeft);
    queue_key(
        &mut app,
        KeyboardInput {
            key_code: KeyCode::KeyX,
            logical_key: Key::Character("x".into()),
            state: ButtonState::Pressed,
            text: Some("x".into()),
            repeat: false,
            window,
        },
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .value()
            .to_string(),
        "x"
    );
    press(&mut app, button);
    bevy_widgetry_test_utils::release(&mut app, button);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    assert_eq!(
        app.world().resource::<FocusEvents>().transitions,
        [(false, editor), (true, button)]
    );
    bevy_widgetry_test_utils::press_key(&mut app, window, KeyCode::Space);
    assert_eq!(app.world().resource::<FocusEvents>().activations, [button]);
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .value()
            .to_string(),
        "x"
    );
}
