#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput, NativeKey},
};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry_combo_box::*;
use bevy_widgetry_list_view::*;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, primary_click, primary_press, queue_key, text_edit_app,
};
use bevy_widgetry_text_field::WidgetryTextField;

#[derive(Resource, Default)]
struct Changes(Vec<Option<WidgetryListItemId>>);

struct Fixture {
    app: App,
    source: Entity,
    root: Entity,
    field: Entity,
    popup: Entity,
    input: Entity,
    list: Entity,
}

fn fixture(len: usize) -> Fixture {
    let mut app = text_edit_app();
    app.init_resource::<bevy::clipboard::Clipboard>();
    add_keyboard_dispatch(&mut app);
    app.register_widgetry_combo_box::<String>().unwrap();
    use bevy_widgetry_core::WidgetryAppExt;
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(bevy_widgetry_asset::BuiltinFont::Default.path());
    bevy_widgetry_test_utils::advance_until(
        &mut app,
        std::time::Duration::from_secs(10),
        "搜索编辑字体",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app.init_resource::<Changes>().add_observer(
        |event: On<ValueChange<Option<WidgetryListItemId>>>, mut changes: ResMut<Changes>| {
            changes.0.push(event.value);
        },
    );
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    let mut model = WidgetryListModel::default();
    for i in 0..len {
        model.push(format!("Item {i:05}")).unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source, @max_visible_items: 3,
            @search: {Some(WidgetryComboBoxMatcher::new(|q: &str, item: &String| item.contains(q)))},
            @no_results: {WidgetryComboBoxContent::new(|| bsn_list!{Text("Nothing matches")})},
            @renderer: {WidgetryListViewRenderer::new(|item: &String| bsn_list!{Text({item.clone()})})},
        }
        Node {width: px(260)}
    }).unwrap().id();
    let children = app.world().get::<Children>(root).unwrap();
    let (field, popup) = (children[0], children[1]);
    let children = app.world().get::<Children>(popup).unwrap();
    let (input, list) = (children[0], children[1]);
    app.update();
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().flush();
    app.update();
    app.update();
    Fixture {
        app,
        source,
        root,
        field,
        popup,
        input,
        list,
    }
}

fn key(f: &mut Fixture, code: KeyCode, text: Option<&str>) {
    queue_key(
        &mut f.app,
        KeyboardInput {
            key_code: code,
            logical_key: text.map_or_else(
                || match code {
                    KeyCode::Home => Key::Home,
                    KeyCode::End => Key::End,
                    KeyCode::Enter => Key::Enter,
                    KeyCode::Escape => Key::Escape,
                    KeyCode::ArrowUp => Key::ArrowUp,
                    KeyCode::ArrowDown => Key::ArrowDown,
                    KeyCode::ArrowLeft => Key::ArrowLeft,
                    KeyCode::Backspace => Key::Backspace,
                    _ => Key::Unidentified(NativeKey::Unidentified),
                },
                |s| Key::Character(s.into()),
            ),
            state: ButtonState::Pressed,
            text: text.map(Into::into),
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
    );
}

fn selected(f: &Fixture) -> Option<WidgetryListItemId> {
    WidgetryComboBox::<String>::selected_id(f.app.world(), f.root).unwrap()
}

#[test]
fn edits_keep_caret_semantics_and_ime_enter_does_not_confirm() {
    let mut f = fixture(20);
    key(&mut f, KeyCode::Digit1, Some("12"));
    f.app.update();
    let editor = f.app.world().resource::<InputFocus>().get().unwrap();
    let caret = f
        .app
        .world()
        .get::<bevy::text::EditableText>(editor)
        .unwrap()
        .editor()
        .raw_selection()
        .focus()
        .index();
    key(&mut f, KeyCode::Home, None);
    f.app.update();
    assert_eq!(
        f.app
            .world()
            .get::<bevy::text::EditableText>(editor)
            .unwrap()
            .editor()
            .raw_selection()
            .focus()
            .index(),
        caret
    );
    key(&mut f, KeyCode::ArrowLeft, None);
    key(&mut f, KeyCode::Backspace, None);
    f.app.update();
    assert_eq!(
        WidgetryTextField::text(f.app.world(), f.input).unwrap(),
        "2"
    );
    let window = f
        .app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(f.app.world())
        .unwrap();
    f.app.world_mut().write_message(bevy::window::Ime::Preedit {
        window,
        value: "中".into(),
        cursor: Some((0, 3)),
    });
    f.app.update();
    assert!(
        WidgetryTextField::editor(f.app.world(), f.input)
            .unwrap()
            .is_composing()
    );
    key(&mut f, KeyCode::Enter, None);
    f.app.update();
    assert_eq!(selected(&f), None);
    assert_eq!(
        f.app.world().get::<Visibility>(f.popup),
        Some(&Visibility::Visible)
    );
}

#[test]
fn editor_focus_projects_and_clears_list_navigation_style() {
    let mut f = fixture(4);
    let colors = f
        .app
        .world()
        .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
        .colors()
        .combo_box
        .list
        .container;
    assert_eq!(
        f.app.world().get::<BorderColor>(f.list),
        Some(&BorderColor::all(colors.focused.border))
    );
    f.app.world_mut().resource_mut::<InputFocus>().clear();
    f.app.update();
    assert_eq!(
        f.app.world().get::<BorderColor>(f.list),
        Some(&BorderColor::all(colors.normal.border))
    );
}

#[test]
fn confirm_precedes_later_text_in_the_same_batch() {
    let mut f = fixture(4);
    let first = f
        .app
        .world()
        .get::<WidgetryListModel<String>>(f.source)
        .unwrap()
        .id(0);
    key(&mut f, KeyCode::Enter, None);
    key(&mut f, KeyCode::KeyZ, Some("z"));
    f.app.update();
    assert_eq!(selected(&f), first);
    assert_eq!(WidgetryTextField::text(f.app.world(), f.input).unwrap(), "");
    f.app.world_mut().trigger(Activate { entity: f.field });
    f.app.world_mut().flush();
    f.app.update();
    let last = f
        .app
        .world()
        .get::<WidgetryListModel<String>>(f.source)
        .unwrap()
        .id(3);
    key(&mut f, KeyCode::Digit3, Some("00003"));
    key(&mut f, KeyCode::Enter, None);
    key(&mut f, KeyCode::KeyZ, Some("z"));
    f.app.update();
    assert_eq!(selected(&f), last);
    assert_eq!(
        WidgetryTextField::text(f.app.world(), f.input).unwrap(),
        "00003"
    );
    assert_eq!(
        f.app.world().get::<Visibility>(f.popup),
        Some(&Visibility::Hidden)
    );
}

#[derive(Resource, Default)]
struct OuterKeys(Vec<KeyCode>);

#[test]
fn search_confirmation_and_cancel_never_reach_outer_shortcuts() {
    use bevy::input_focus::FocusedInput;
    let mut f = fixture(4);
    let outer = f.app.world_mut().spawn_scene(bsn! {Node}).unwrap().id();
    f.app.world_mut().entity_mut(f.root).insert(ChildOf(outer));
    f.app.init_resource::<OuterKeys>().add_observer(
        move |event: On<FocusedInput<KeyboardInput>>, mut seen: ResMut<OuterKeys>| {
            if event.focused_entity == outer && event.input.state.is_pressed() {
                seen.0.push(event.input.key_code);
            }
        },
    );
    // Force the same editor-level propagation reset that the official observer may perform last.
    let editor = f.app.world().resource::<InputFocus>().get().unwrap();
    f.app.world_mut().entity_mut(editor).observe(
        move |mut event: On<FocusedInput<KeyboardInput>>| {
            if event.focused_entity == editor
                && matches!(event.input.key_code, KeyCode::Enter | KeyCode::Escape)
            {
                event.propagate(true);
            }
        },
    );
    key(&mut f, KeyCode::Escape, None);
    f.app.update();
    assert!(f.app.world().resource::<OuterKeys>().0.is_empty());
    f.app.world_mut().trigger(Activate { entity: f.field });
    f.app.world_mut().flush();
    f.app.update();
    key(&mut f, KeyCode::Enter, None);
    f.app.update();
    assert!(f.app.world().resource::<OuterKeys>().0.is_empty());
}

#[test]
fn query_and_model_changes_coalesce_and_stable_frames_do_not_rematch() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let mut app = scene_app();
    app.register_widgetry_combo_box::<String>().unwrap();
    let mut model = WidgetryListModel::default();
    model.push(String::from("Apple")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source,
            @search: {Some(WidgetryComboBoxMatcher::new(move |query: &str, item: &String| {counter.fetch_add(1, Ordering::Relaxed); item.contains(query)}))},
            @renderer: {WidgetryListViewRenderer::new(|item: &String| bsn_list!{Text({item.clone()})})},
        }
    }).unwrap().id();
    app.update();
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let input = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryTextField>>()
        .single(app.world())
        .unwrap();
    WidgetryTextField::set_text_in_world(app.world_mut(), input, "a").unwrap();
    WidgetryTextField::set_text_in_world(app.world_mut(), input, "Ban").unwrap();
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .push("Banana".into())
        .unwrap();
    app.update();
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert_eq!(
        WidgetryComboBox::<String>::search_state(app.world(), root)
            .unwrap()
            .matching_items,
        1
    );
    app.update();
    app.update();
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn typed_query_updates_before_rows_navigation_keeps_editor_focus_and_enter_stops_batch() {
    let mut f = fixture(20);
    let editor = f.app.world().resource::<InputFocus>().get().unwrap();
    assert!(
        f.app
            .world()
            .get::<bevy::text::EditableText>(editor)
            .is_some()
    );
    key(&mut f, KeyCode::Digit1, Some("1"));
    f.app.update();
    let state = WidgetryComboBox::<String>::search_state(f.app.world(), f.root).unwrap();
    assert_eq!((state.query.as_str(), state.matching_items), ("1", 11));
    assert_eq!(selected(&f), None);
    f.app
        .world_mut()
        .get_mut::<WidgetryListModel<String>>(f.source)
        .unwrap()
        .set_disabled(1, true);
    key(&mut f, KeyCode::Home, None);
    f.app.update();
    let state = WidgetryComboBox::<String>::search_state(f.app.world(), f.root).unwrap();
    assert_eq!(
        state.active,
        f.app
            .world()
            .get::<WidgetryListModel<String>>(f.source)
            .unwrap()
            .id(10)
    );
    assert_eq!(f.app.world().resource::<InputFocus>().get(), Some(editor));
    key(&mut f, KeyCode::End, None);
    key(&mut f, KeyCode::Enter, Some("\r"));
    key(&mut f, KeyCode::ArrowUp, None);
    key(&mut f, KeyCode::Enter, None);
    f.app.update();
    let last = f
        .app
        .world()
        .get::<WidgetryListModel<String>>(f.source)
        .unwrap()
        .id(19);
    assert_eq!(selected(&f), last);
    assert_eq!(f.app.world().resource::<Changes>().0, vec![last]);
    assert_eq!(
        f.app.world().get::<Visibility>(f.popup),
        Some(&Visibility::Hidden)
    );
    assert_ne!(f.app.world().resource::<InputFocus>().get(), Some(editor));
    f.app.world_mut().trigger(Activate { entity: f.field });
    f.app.world_mut().flush();
    f.app.update();
    let state = WidgetryComboBox::<String>::search_state(f.app.world(), f.root).unwrap();
    assert_eq!((state.query.as_str(), state.matching_items), ("", 20));
    assert_eq!(selected(&f), last);
}

#[test]
fn no_results_model_changes_escape_and_disabled_preserve_selection() {
    let mut f = fixture(4);
    WidgetryComboBox::<String>::select_index(&mut f.app.world_mut().commands(), f.root, 2);
    f.app.world_mut().flush();
    let before = selected(&f);
    key(&mut f, KeyCode::KeyZ, Some("zzz"));
    f.app.update();
    assert_eq!(selected(&f), before);
    assert_eq!(
        f.app
            .world()
            .get::<WidgetryListProjection>(f.list)
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        f.app.world().get::<Node>(f.list).unwrap().display,
        Display::None
    );
    let empty = f.app.world().get::<Children>(f.popup).unwrap()[2];
    assert_eq!(
        f.app.world().get::<Node>(empty).unwrap().display,
        Display::Flex
    );
    key(&mut f, KeyCode::Enter, None);
    f.app.update();
    assert_eq!(
        f.app.world().get::<Visibility>(f.popup),
        Some(&Visibility::Visible)
    );
    let added = f
        .app
        .world_mut()
        .get_mut::<WidgetryListModel<String>>(f.source)
        .unwrap()
        .push("zzz".into())
        .unwrap();
    f.app.update();
    assert_eq!(
        f.app
            .world()
            .get::<WidgetryListProjection>(f.list)
            .unwrap()
            .ordered_ids(),
        &[added]
    );
    key(&mut f, KeyCode::Escape, None);
    key(&mut f, KeyCode::Enter, None);
    f.app.update();
    assert_eq!(selected(&f), before);
    assert_eq!(
        f.app.world().get::<Visibility>(f.popup),
        Some(&Visibility::Hidden)
    );
    f.app.world_mut().trigger(Activate { entity: f.field });
    f.app.world_mut().flush();
    f.app.update();
    assert_eq!(
        WidgetryComboBox::<String>::search_state(f.app.world(), f.root)
            .unwrap()
            .matching_items,
        5
    );
    f.app
        .world_mut()
        .get_mut::<WidgetryListModel<String>>(f.source)
        .unwrap()
        .remove(2);
    f.app.update();
    assert_eq!(selected(&f), None);
    f.app
        .world_mut()
        .entity_mut(f.root)
        .insert(InteractionDisabled);
    key(&mut f, KeyCode::KeyA, Some("a"));
    key(&mut f, KeyCode::Enter, None);
    f.app.update();
    assert_eq!(selected(&f), None);
    assert_eq!(
        f.app.world().get::<Visibility>(f.popup),
        Some(&Visibility::Hidden)
    );
    assert_eq!(WidgetryTextField::text(f.app.world(), f.input).unwrap(), "");
}

#[test]
fn large_search_virtualizes_results_and_pointer_confirmation_resets_on_reopen() {
    let mut f = fixture(10_000);
    for _ in 0..2 {
        f.app.update();
    }
    let rows = f
        .app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(f.app.world())
        .map(|(e, i)| (e, i.id))
        .collect::<Vec<_>>();
    assert!(rows.len() <= 6, "{} rows", rows.len());
    assert!(!rows.is_empty());
    let (row, id) = rows[0];
    f.app.world_mut().trigger(primary_press(row));
    f.app.world_mut().flush();
    f.app.world_mut().trigger(primary_click(row));
    f.app.world_mut().flush();
    f.app.update();
    assert_eq!(selected(&f), Some(id));
    assert_eq!(
        f.app.world().get::<Visibility>(f.popup),
        Some(&Visibility::Hidden)
    );
    f.app.world_mut().trigger(Activate { entity: f.field });
    f.app.world_mut().flush();
    f.app.update();
    assert_eq!(
        WidgetryComboBox::<String>::search_state(f.app.world(), f.root)
            .unwrap()
            .matching_items,
        10_000
    );
    key(&mut f, KeyCode::End, None);
    f.app.update();
    f.app.update();
    assert!(
        f.app
            .world_mut()
            .query::<&ScrollPosition>()
            .iter(f.app.world())
            .any(|pos| pos.0.y > 0.0)
    );
    assert!(
        f.app
            .world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(f.app.world())
            .count()
            <= 6
    );
}

#[test]
fn local_search_filters_one_projection_without_losing_hidden_selection() {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<String>().unwrap();
    let mut model = WidgetryListModel::default();
    let apple = model.push(String::from("Apple")).unwrap();
    model.push(String::from("Banana")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let first = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source,
            @search: {Some(WidgetryComboBoxMatcher::new(|query: &str, item: &String| item.to_lowercase().contains(&query.to_lowercase())))},
            @renderer: {WidgetryListViewRenderer::new(|item: &String| bsn_list!{Text({item.clone()})})},
        }
    }).unwrap().id();
    let second = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> { @source: source, @renderer: {WidgetryListViewRenderer::new(|item: &String| bsn_list!{Text({item.clone()})})} }
    }).unwrap().id();
    app.update();
    WidgetryComboBox::<String>::select_id(&mut app.world_mut().commands(), first, apple);
    app.world_mut().flush();
    let input = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryTextField>>()
        .single(app.world())
        .unwrap();
    WidgetryTextField::set_text_in_world(app.world_mut(), input, "ban").unwrap();
    app.update();
    let search = WidgetryComboBox::<String>::search_state(app.world(), first).unwrap();
    assert_eq!(search.query, "ban");
    assert_eq!(search.matching_items, 1);
    assert_eq!(
        WidgetryComboBox::<String>::selected_id(app.world(), first).unwrap(),
        Some(apple)
    );
    assert_eq!(
        WidgetryComboBox::<String>::selected_id(app.world(), second).unwrap(),
        None
    );
    let second_list = app
        .world_mut()
        .query::<(Entity, &WidgetryListView<String>)>()
        .iter(app.world())
        .find(|(root, _)| {
            app.world().get::<ChildOf>(*root).unwrap().parent()
                == app.world().get::<Children>(second).unwrap()[1]
        })
        .unwrap()
        .0;
    assert_eq!(
        app.world()
            .get::<WidgetryListProjection>(second_list)
            .unwrap()
            .len(),
        2
    );
}
