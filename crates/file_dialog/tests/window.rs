//! state 维度为embedded/owned、modal/nonmodal、open/resolved、owner与nested overwrite lifecycle。
//! stimuli为BSN、实际focused input、result、WindowCloseRequested与despawn。
//! guards为有效native parent、input window、IME与once-only decision。
//! invariants为业务root唯一、observer/deferred先消费result、owned window/camera完整回收。
//! 颜色覆盖由 FileDialog 根控制 Window 与 overwrite MessageBox，Theme/clear 不改变确认业务 state。

// 测试断言保护Window组合contract，不适用生产macro禁令。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::window::WindowRef;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_test_utils::{add_ui_plugins, scene_app};

fn fixture() -> App {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.insert_resource(WidgetryFileDialogRuntimeOptions {
        automatic: false,
        ..default()
    });
    app.add_plugins((
        WidgetryFileDialogPlugin,
        bevy::input_focus::InputDispatchPlugin,
        bevy::ui_widgets::TextInputPlugin,
    ));
    app
}

#[test]
fn independent_dialog_keeps_one_business_root_and_reclaims_its_window_and_camera() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @window: {Some(WidgetryFileDialogWindow::default())} } Name("OwnedFileDialog") }).unwrap().id();
    for _ in 0..4 {
        app.update();
    }
    let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
    let target = app.world_mut().query::<(Entity, &Window)>().iter(app.world()).find(|(entity, _)| matches!(app.world().get::<RenderTarget>(camera), Some(RenderTarget::Window(WindowRef::Entity(window))) if *window == *entity)).unwrap().0;
    assert!(app.world().get::<Window>(target).is_some());
    assert!(app.world().get::<ChildOf>(root).is_none());
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryFileDialog>()
            .iter(app.world())
            .count(),
        1
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap();
    app.world_mut().flush();
    app.update();
    for entity in [root, target, camera] {
        assert!(app.world().get_entity(entity).is_err());
    }
}

#[test]
fn modal_requires_a_valid_native_parent_before_owned_resources_are_reserved() {
    let mut app = fixture();
    let before = app.world().entities().count_spawned();
    let error = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! { @WidgetryFileDialog { @window: {Some(WidgetryFileDialogWindow {modality: WidgetryFileDialogModality::Modal, ..default()})} } }).err().unwrap();
    assert!(error.to_string().contains("parent"));
    assert_eq!(app.world().entities().count_spawned(), before);
}

use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::text::EditableText;
use bevy_widgetry_message_box::*;
use bevy_widgetry_test_utils::{press_key, queue_key};
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, owned_widgetry_window,
    widgetry_window_target,
};

fn parent(app: &mut App) -> (Entity, Entity, Entity) {
    let root = app.world_mut().spawn_scene(bsn! {
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{Name("ParentEditor") @bevy_widgetry_text_field::WidgetryTextField bevy::input_focus::tab_navigation::TabIndex::default()})
    }).unwrap().id();
    app.update();
    let native = widgetry_window_target(app.world(), root).unwrap();
    let editor = named(app, "ParentEditor", root);
    (root, native, editor)
}
fn descendant(world: &World, mut entity: Entity, root: Entity) -> bool {
    loop {
        if entity == root {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return false;
        };
        entity = parent.parent();
    }
}
fn named(app: &mut App, label: &str, root: Entity) -> Entity {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(entity, name)| name.as_str() == label && descendant(app.world(), *entity, root))
        .unwrap()
        .0
}
fn dialog(app: &mut App, parent: Option<Entity>, modality: WidgetryFileDialogModality) -> Entity {
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @window: {Some(WidgetryFileDialogWindow {parent,modality,..default()})} } }).unwrap().id();
    for _ in 0..3 {
        app.update();
    }
    root
}
fn text_key(app: &mut App, native: Entity, text: &str) {
    queue_key(
        app,
        KeyboardInput {
            window: native,
            key_code: KeyCode::KeyA,
            logical_key: Key::Character(text.into()),
            text: Some(text.into()),
            state: ButtonState::Pressed,
            repeat: false,
        },
    );
}

#[test]
fn window_tab_keeps_caller_group_order_and_nested_modal_scope() {
    use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(bsn! {
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{
            TabGroup::new(10) Children [Name("Later") TabIndex(0)]--
            TabGroup::new(-1) Children [Name("Earlier") TabIndex(8)]--
            TabGroup::modal() Children [Name("ModalFirst") TabIndex(0)-- Name("ModalLast") TabIndex(1)]
        })
    }).unwrap().id();
    app.update();
    let native = widgetry_window_target(app.world(), root).unwrap();
    let later = named(&mut app, "Later", root);
    let earlier = named(&mut app, "Earlier", root);
    let first = named(&mut app, "ModalFirst", root);
    let last = named(&mut app, "ModalLast", root);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(later, FocusCause::Pressed);
    press_key(&mut app, native, KeyCode::Tab);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(earlier));
    press_key(&mut app, native, KeyCode::Tab);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(later));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(first, FocusCause::Pressed);
    for expected in [last, first, last] {
        press_key(&mut app, native, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(expected));
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    press_key(&mut app, native, KeyCode::Tab);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(first));
}

#[test]
fn window_tab_restores_visible_focus_after_pointer_focus() {
    let mut app = fixture();
    let (root, native, editor) = parent(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Pressed);
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocusVisible>()
        .0 = false;
    press_key(&mut app, native, KeyCode::Tab);
    assert!(
        app.world()
            .resource::<bevy::input_focus::InputFocusVisible>()
            .0
    );
    assert!(descendant(
        app.world(),
        app.world().resource::<InputFocus>().get().unwrap(),
        root
    ));
}

#[test]
fn closed_sort_popup_is_not_restored_as_remembered_window_focus() {
    let mut app = fixture();
    let root = dialog(&mut app, None, WidgetryFileDialogModality::NonModal);
    let native = widgetry_window_target(app.world(), root).unwrap();
    let sort = named(&mut app, "FileDialogSort", root);
    let field = app.world().get::<Children>(sort).unwrap()[0];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
    press_key(&mut app, native, KeyCode::Enter);
    let list = app.world().resource::<InputFocus>().get().unwrap();
    let popup = app.world().get::<ChildOf>(list).unwrap().parent();
    assert_eq!(
        app.world().get::<Visibility>(popup),
        Some(&Visibility::Visible)
    );
    press_key(&mut app, native, KeyCode::ArrowDown);
    press_key(&mut app, native, KeyCode::Enter);
    assert_eq!(
        app.world().get::<Visibility>(popup),
        Some(&Visibility::Hidden)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .preferences()
            .sort,
        WidgetryFileDialogSort::NameDescending
    );
    let active = app
        .world()
        .get::<bevy_widgetry_list_view::WidgetryListViewState>(list)
        .unwrap()
        .active;
    press_key(&mut app, native, KeyCode::ArrowDown);
    assert_ne!(app.world().resource::<InputFocus>().get(), Some(list));
    assert_eq!(
        app.world()
            .get::<bevy_widgetry_list_view::WidgetryListViewState>(list)
            .unwrap()
            .active,
        active
    );
    press_key(&mut app, native, KeyCode::Enter);
    assert_eq!(
        app.world().get::<Visibility>(popup),
        Some(&Visibility::Hidden)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .preferences()
            .sort,
        WidgetryFileDialogSort::NameDescending
    );
}

#[test]
fn modal_blocks_parent_keyboard_ime_and_cycles_tab_then_restores_valid_focus() {
    let mut app = fixture();
    let (parent_root, native, editor) = parent(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Pressed);
    let root = dialog(&mut app, Some(native), WidgetryFileDialogModality::Modal);
    let child_native = widgetry_window_target(app.world(), root).unwrap();
    assert!(descendant(
        app.world(),
        app.world().resource::<InputFocus>().get().unwrap(),
        root
    ));
    text_key(&mut app, native, "p");
    app.world_mut().write_message(bevy::window::Ime::Commit {
        window: native,
        value: "父".into(),
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .value()
            .to_string(),
        ""
    );
    for _ in 0..40 {
        press_key(&mut app, child_native, KeyCode::Tab);
        assert!(descendant(
            app.world(),
            app.world().resource::<InputFocus>().get().unwrap(),
            root
        ));
    }
    press_key(&mut app, child_native, KeyCode::Escape);
    assert!(app.world().get_entity(root).is_err());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert!(app.world().get_entity(parent_root).is_ok());
}

#[test]
fn nonmodal_text_is_routed_by_native_window_even_for_two_inputs_in_one_frame() {
    let mut app = fixture();
    let (_, native, editor) = parent(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Pressed);
    app.update();
    let root = dialog(&mut app, Some(native), WidgetryFileDialogModality::NonModal);
    let child_native = widgetry_window_target(app.world(), root).unwrap();
    let search = named(&mut app, "FileDialogSearch", root);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(search, FocusCause::Pressed);
    app.update();
    text_key(&mut app, native, "p");
    text_key(&mut app, child_native, "c");
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .value()
            .to_string(),
        "p"
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(search)
            .unwrap()
            .value()
            .to_string(),
        "c"
    );
    assert!(
        app.world()
            .get::<bevy_widgetry_window::WidgetryModalWindow>(root)
            .is_none()
    );
    app.world_mut().write_message(bevy::window::Ime::Commit {
        window: native,
        value: "错".into(),
    });
    app.world_mut().write_message(bevy::window::Ime::Commit {
        window: child_native,
        value: "中".into(),
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .value()
            .to_string(),
        "p"
    );
    assert!(
        app.world()
            .get::<EditableText>(search)
            .unwrap()
            .value()
            .to_string()
            .contains('中')
    );
    assert!(!app.world().get::<Window>(native).unwrap().ime_enabled);
    assert!(app.world().get::<Window>(child_native).unwrap().ime_enabled);
}

#[derive(Resource, Default)]
struct Results(Vec<WidgetryFileDialogResult>, usize);
fn observe(app: &mut App) {
    app.init_resource::<Results>().add_observer(
        |event: On<WidgetryFileDialogResultEvent>,
         mut results: ResMut<Results>,
         mut commands: Commands| {
            results.0.push(event.result.clone());
            let root = event.entity;
            commands.queue(move |world: &mut World| {
                assert!(
                    world
                        .get::<WidgetryFileDialogState>(root)
                        .unwrap()
                        .result()
                        .is_some()
                );
                world.resource_mut::<Results>().1 += 1;
            });
        },
    );
}
#[test]
fn repeated_close_requests_commit_one_cancel_before_deferred_cleanup_and_parent_loss_is_silent() {
    let mut app = fixture();
    observe(&mut app);
    let (parent_root, native, _) = parent(&mut app);
    let root = dialog(&mut app, Some(native), WidgetryFileDialogModality::NonModal);
    let child = widgetry_window_target(app.world(), root).unwrap();
    for _ in 0..2 {
        app.world_mut()
            .write_message(bevy::window::WindowCloseRequested { window: child });
    }
    app.update();
    assert_eq!(
        app.world().resource::<Results>().0,
        [WidgetryFileDialogResult::Cancelled]
    );
    assert_eq!(app.world().resource::<Results>().1, 1);
    assert!(app.world().get_entity(root).is_err());
    assert!(app.world().get_entity(child).is_err());
    let orphan = dialog(&mut app, Some(native), WidgetryFileDialogModality::Modal);
    app.world_mut().entity_mut(parent_root).despawn();
    app.world_mut().flush();
    app.update();
    assert!(app.world().get_entity(orphan).is_err());
    assert_eq!(app.world().resource::<Results>().0.len(), 1);
}

#[test]
fn nested_message_box_escape_returns_to_dialog_and_does_not_cancel_parent_dialog() {
    let mut app = fixture();
    let (_, native, _) = parent(&mut app);
    let root = dialog(&mut app, Some(native), WidgetryFileDialogModality::Modal);
    let child_native = widgetry_window_target(app.world(), root).unwrap();
    let before = app.world().resource::<InputFocus>().get();
    let nested=app.world_mut().spawn_scene(bsn! {@widgetry_message_box(child_native,"Nested",WidgetryMessageBoxButtons::YesNoCancel, Default::default(), bsn_list!{Text("Question") bevy_widgetry_core::text::WidgetryText})}).unwrap().id();
    app.update();
    let nested_native = widgetry_window_target(app.world(), nested).unwrap();
    assert!(descendant(
        app.world(),
        app.world().resource::<InputFocus>().get().unwrap(),
        nested
    ));
    press_key(&mut app, nested_native, KeyCode::Escape);
    assert!(app.world().get_entity(nested).is_err());
    assert!(app.world().get_entity(root).is_ok());
    assert_eq!(app.world().resource::<InputFocus>().get(), before);
    press_key(&mut app, child_native, KeyCode::Escape);
    assert!(app.world().get_entity(root).is_err());
}

#[test]
fn window_props_initialize_once_and_scene_patch_keeps_the_live_shell() {
    let mut app = fixture();
    let root = dialog(&mut app, None, WidgetryFileDialogModality::NonModal);
    let native = widgetry_window_target(app.world(), root).unwrap();
    let host = named(&mut app, "FileDialogContentHost", root);
    let search = named(&mut app, "FileDialogSearch", root);
    app.world_mut().entity_mut(root).apply_scene(bsn! { @WidgetryFileDialog { @window: {Some(WidgetryFileDialogWindow {modality: WidgetryFileDialogModality::Modal,..default()})} } }).unwrap();
    app.update();
    assert_eq!(widgetry_window_target(app.world(), root), Some(native));
    assert_eq!(named(&mut app, "FileDialogContentHost", root), host);
    assert_eq!(named(&mut app, "FileDialogSearch", root), search);
    assert!(
        app.world()
            .get::<bevy_widgetry_window::WidgetryModalWindow>(root)
            .is_none()
    );
}

fn awaiting_overwrite(app: &mut App) -> Entity {
    awaiting_overwrite_with_colors(app, Default::default())
}
fn awaiting_overwrite_with_colors(
    app: &mut App,
    colors: WidgetryFileDialogColorOverrides,
) -> Entity {
    let root=app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @colors: colors, @mode:WidgetryFileDialogMode::SaveFile, @initial_directory:{Some("C:/fixture".into())}, @window:{Some(WidgetryFileDialogWindow::default())} } }).unwrap().id();
    app.update();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let token = state.token();
    let snapshot = std::sync::Arc::new(
        WidgetryFileDialogSnapshot::prepare(
            token,
            "C:/fixture".into(),
            vec![],
            &state.query(),
            None,
        )
        .unwrap(),
    );
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Started {
            token,
            path: "C:/fixture".into(),
        },
    )
    .unwrap();
    let selection = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .projection_selection_job(snapshot.clone())
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Snapshot {
            snapshot,
            state: WidgetryFileDialogDirectoryState::Ready,
            selection,
        },
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Filename("same.txt".into()),
    )
    .unwrap();
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Validated {
            candidate,
            exists: true,
        },
    )
    .unwrap();
    app.update();
    app.update();
    root
}
#[test]
fn independent_window_and_late_confirmation_keep_host_colors_across_theme_and_clear() {
    use bevy_widgetry_test_utils::LogCapture;
    use bevy_widgetry_theme::WidgetryThemeMode;
    use bevy_widgetry_window::WidgetryWindowColorOverrides;
    let mut app = fixture();
    let mut colors = WidgetryFileDialogColorOverrides::default();
    let border = Color::linear_rgb(0.2, 0.6, 0.3);
    let frame = Color::linear_rgb(0.6, 0.1, 0.3);
    let body = Color::linear_rgb(0.3, 0.5, 0.1);
    let text = Color::linear_rgb(0.7, 0.2, 0.4);
    let action = Color::linear_rgb(0.4, 0.1, 0.7);
    colors.window.frame.normal.border = Some(border);
    colors.confirmation.window.frame.normal.background = Some(frame);
    colors.confirmation.body.normal.background = Some(body);
    colors.confirmation.body.normal.foreground = Some(text);
    colors.confirmation.action_button.normal.background = Some(action);
    let root = awaiting_overwrite_with_colors(&mut app, colors.clone());
    let child = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryMessageBox>>()
        .single(app.world())
        .unwrap();
    let label = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find(|(entity, text)| {
            text.0 == "The file already exists. Replace it?"
                && descendant(app.world(), *entity, child)
        })
        .unwrap()
        .0;
    let content = app.world().get::<ChildOf>(label).unwrap().parent();
    let body_host = app.world().get::<ChildOf>(content).unwrap().parent();
    let buttons = app
        .world_mut()
        .query_filtered::<Entity, With<bevy_widgetry_button::WidgetryButton>>()
        .iter(app.world())
        .filter(|entity| descendant(app.world(), *entity, child))
        .collect::<Vec<_>>();
    assert_eq!(buttons.len(), 3);
    for mode in [WidgetryThemeMode::Dark, WidgetryThemeMode::Light] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        app.update();
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(border)
        );
        assert_eq!(app.world().get::<BackgroundColor>(child).unwrap().0, frame);
        assert_eq!(
            app.world().get::<BackgroundColor>(body_host).unwrap().0,
            body
        );
        assert_eq!(app.world().get::<TextColor>(label).unwrap().0, text);
        for button in &buttons {
            assert_eq!(
                app.world().get::<BackgroundColor>(*button).unwrap().0,
                action
            );
        }
    }
    let logs = LogCapture::default();
    let errors = logs.run(|| {
        [
            WidgetryWindowColorOverrides::clear_in_world(app.world_mut(), root).unwrap_err(),
            bevy_widgetry_message_box::WidgetryMessageBoxColorOverrides::clear_in_world(
                app.world_mut(),
                child,
            )
            .unwrap_err(),
            WidgetryWindowColorOverrides::clear_in_world(app.world_mut(), child).unwrap_err(),
        ]
    });
    for error in errors {
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("托管"));
    }
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.target == "bevy_widgetry" && r.level == bevy::log::Level::ERROR)
            .count(),
        3
    );
    assert_eq!(
        WidgetryFileDialogColorOverrides::get(app.world(), root).unwrap(),
        &colors
    );
    let confirmation = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .confirmation()
        .clone();
    WidgetryFileDialogColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .confirmation(),
        &confirmation
    );
    let theme = WidgetryThemeMode::Light.colors().file_dialog;
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(theme.window.frame.normal.border)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(child).unwrap().0,
        theme.confirmation.window.frame.normal.background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(body_host).unwrap().0,
        theme.confirmation.body.normal.background
    );
    assert_eq!(
        app.world().get::<TextColor>(label).unwrap().0,
        theme.confirmation.body.normal.foreground
    );
    for button in buttons {
        assert_eq!(
            app.world().get::<BackgroundColor>(button).unwrap().0,
            theme.confirmation.action_button.normal.background
        );
    }
}
#[test]
fn overwrite_message_box_uses_fixed_candidate_and_yes_no_cancel_have_distinct_results() {
    for decision in [
        WidgetryMessageBoxResult::Yes,
        WidgetryMessageBoxResult::No,
        WidgetryMessageBoxResult::Cancel,
    ] {
        let mut app = fixture();
        observe(&mut app);
        let root = awaiting_overwrite(&mut app);
        let child = app
            .world_mut()
            .query_filtered::<Entity, With<WidgetryMessageBox>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            app.world()
                .get::<bevy_widgetry_window::WidgetryModalWindow>(child)
                .unwrap()
                .parent,
            widgetry_window_target(app.world(), root).unwrap()
        );
        WidgetryMessageBox::resolve(app.world_mut(), child, decision).unwrap();
        app.update();
        if decision == WidgetryMessageBoxResult::Yes {
            assert!(app.world().get_entity(root).is_err());
            assert_eq!(
                app.world().resource::<Results>().0,
                [WidgetryFileDialogResult::SavePath(
                    "C:/fixture/same.txt".into()
                )]
            );
        } else {
            assert!(app.world().get_entity(root).is_ok());
            assert!(app.world().resource::<Results>().0.is_empty());
            assert_eq!(
                app.world()
                    .get::<WidgetryFileDialogState>(root)
                    .unwrap()
                    .confirmation(),
                &WidgetryFileDialogConfirmation::Idle
            );
        }
        assert!(app.world().get_entity(child).is_err());
    }
}
#[test]
fn local_ime_escape_does_not_resolve_dialog_and_stale_overwrite_does_not_save_new_filename() {
    let mut app = fixture();
    observe(&mut app);
    let root = dialog(&mut app, None, WidgetryFileDialogModality::NonModal);
    let native = widgetry_window_target(app.world(), root).unwrap();
    let search = named(&mut app, "FileDialogSearch", root);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(search, FocusCause::Pressed);
    app.world_mut().write_message(bevy::window::Ime::Preedit {
        window: native,
        value: "中".into(),
        cursor: Some((0, 3)),
    });
    app.update();
    press_key(&mut app, native, KeyCode::Escape);
    assert!(app.world().get_entity(root).is_ok());
    assert!(app.world().resource::<Results>().0.is_empty());
    let save = awaiting_overwrite(&mut app);
    let child = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryMessageBox>>()
        .single(app.world())
        .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        save,
        WidgetryFileDialogAction::Filename("new.txt".into()),
    )
    .unwrap();
    WidgetryMessageBox::resolve(app.world_mut(), child, WidgetryMessageBoxResult::Yes).unwrap();
    app.update();
    assert!(app.world().resource::<Results>().0.is_empty());
    assert!(app.world().get_entity(save).is_ok());
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(save)
            .unwrap()
            .filename(),
        std::ffi::OsStr::new("new.txt")
    );
}
#[test]
fn sibling_modal_order_and_invalid_saved_focus_do_not_overwrite_application_disabled_state() {
    let mut app = fixture();
    let (_, native, editor) = parent(&mut app);
    app.world_mut()
        .entity_mut(editor)
        .insert(bevy::ui::InteractionDisabled);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Pressed);
    let first = dialog(&mut app, Some(native), WidgetryFileDialogModality::Modal);
    let second = dialog(&mut app, Some(native), WidgetryFileDialogModality::Modal);
    assert!(descendant(
        app.world(),
        app.world().resource::<InputFocus>().get().unwrap(),
        second
    ));
    app.world_mut().entity_mut(second).despawn();
    app.world_mut().flush();
    assert!(descendant(
        app.world(),
        app.world().resource::<InputFocus>().get().unwrap(),
        first
    ));
    app.world_mut().entity_mut(editor).despawn();
    app.world_mut().entity_mut(first).despawn();
    app.world_mut().flush();
    assert_ne!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert!(
        app.world()
            .resource::<InputFocus>()
            .get()
            .is_none_or(|entity| app.world().get_entity(entity).is_ok())
    );
}

#[test]
fn cancellation_before_first_update_still_notifies_once_and_does_not_create_an_owned_window() {
    let mut app = fixture();
    observe(&mut app);
    let root = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryFileDialog { @window:{Some(WidgetryFileDialogWindow::default())} } },
        )
        .unwrap()
        .id();
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap();
    assert!(
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Reopen).is_err()
    );
    app.update();
    assert_eq!(
        app.world().resource::<Results>().0,
        [WidgetryFileDialogResult::Cancelled]
    );
    assert_eq!(app.world().resource::<Results>().1, 1);
    assert!(app.world().get_entity(root).is_err());
    assert_eq!(
        app.world_mut().query::<&Window>().iter(app.world()).count(),
        1
    );
}

#[test]
fn parent_with_a_missing_camera_is_rejected_before_owned_allocation() {
    let mut app = fixture();
    let (parent_root, native, _) = parent(&mut app);
    let camera = app.world().get::<UiTargetCamera>(parent_root).unwrap().0;
    app.world_mut().entity_mut(camera).despawn();
    let before = app.world().entities().count_spawned();
    let result = bevy_widgetry_core::scene::spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryFileDialog { @window:{Some(WidgetryFileDialogWindow {parent:Some(native),modality:WidgetryFileDialogModality::Modal,..default()})} } },
    );
    assert!(result.is_err());
    assert_eq!(app.world().entities().count_spawned(), before);
}
