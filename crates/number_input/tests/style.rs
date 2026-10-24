#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input_focus::{FocusCause, InputFocus},
    picking::pointer::{Location, PointerAction, PointerButton},
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::ValueChange,
};
use bevy_widgetry_core::{input_target::WidgetryInputTarget, scene::spawn_scene};
use bevy_widgetry_number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin};
use bevy_widgetry_text_field::WidgetryTextField;
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Resource, Default)]
struct Changes(usize);

fn fixture(read_only: bool) -> (App, Entity, Entity, Entity, Entity, Entity) {
    let mut app = bevy_widgetry_test_utils::text_edit_app();
    app.add_plugins(WidgetryNumberInputPlugin)
        .init_resource::<Changes>();
    app.add_observer(|_: On<ValueChange<f64>>, mut changes: ResMut<Changes>| changes.0 += 1);
    let root = spawn_scene(app.world_mut(), bsn! {@WidgetryNumberInput<f64> {@value:{Some(10.0)}, @min:{Some(0.0)}, @max:{Some(10.0)}, @read_only:{read_only}}}).unwrap();
    let children = app.world().get::<Children>(root).unwrap();
    let (minus, field, plus) = (children[0], children[1], children[2]);
    let editor = app
        .world()
        .get::<WidgetryInputTarget>(field)
        .unwrap()
        .target;
    app.update();
    app.update();
    (app, root, minus, field, plus, editor)
}

#[test]
fn themes_refresh_the_shared_shell_and_owned_children_without_touching_drafts() {
    let (mut app, root, minus, field, plus, editor) = fixture(false);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    WidgetryTextField::set_text_in_world(app.world_mut(), field, "7.").unwrap();
    for mode in [
        WidgetryThemeMode::Light,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
        WidgetryThemeMode::Dark,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        for _ in 0..3 {
            app.update();
        }
        let colors = mode.colors();
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(colors.text_field.editable.focused.border)
        );
        assert_eq!(
            app.world().get::<TextColor>(editor).unwrap().0,
            colors.text_field.editable.focused.foreground
        );
        for button in [minus, plus] {
            assert_eq!(
                app.world().get::<BackgroundColor>(button).unwrap().0,
                colors.button.normal.background
            );
            let label = app.world().get::<Children>(button).unwrap()[0];
            assert_eq!(
                app.world().get::<TextColor>(label).unwrap().0,
                colors.button.normal.foreground
            );
        }
        assert_eq!(WidgetryTextField::text(app.world(), field).unwrap(), "7.");
        assert_eq!(
            WidgetryNumberInput::<f64>::value(app.world(), root).unwrap(),
            10.0
        );
        assert_eq!(app.world().resource::<Changes>().0, 0);
    }
}

#[test]
fn button_feedback_is_local_and_endpoints_remain_visually_enabled() {
    let (mut app, root, minus, _, plus, _) = fixture(false);
    let theme = app.world().resource::<WidgetryThemeMode>().colors();
    let geometry = app.world().get::<Node>(root).unwrap().clone();
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let camera =
        bevy_widgetry_test_utils::spawn_picking_camera(&mut app, window, UVec2::splat(600), 1.0);
    app.world_mut()
        .entity_mut(root)
        .insert(UiTargetCamera(camera));
    for _ in 0..3 {
        app.update();
    }
    let pointer = bevy_widgetry_test_utils::pointer_ids()[1];
    app.world_mut().spawn(pointer);
    let mut at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: app
            .world()
            .get::<UiGlobalTransform>(plus)
            .unwrap()
            .translation,
    };
    bevy_widgetry_test_utils::queue_pointer(
        &mut app,
        pointer,
        at.clone(),
        PointerAction::Move { delta: Vec2::ZERO },
    );
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(plus).unwrap().0,
        theme.button.hovered.background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(minus).unwrap().0,
        theme.button.normal.background
    );
    bevy_widgetry_test_utils::queue_pointer(
        &mut app,
        pointer,
        at.clone(),
        PointerAction::Press(PointerButton::Primary),
    );
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(plus).unwrap().0,
        theme.button.pressed.background
    );
    assert!(app.world().get::<InteractionDisabled>(plus).is_none());
    bevy_widgetry_test_utils::queue_pointer(
        &mut app,
        pointer,
        at.clone(),
        PointerAction::Release(PointerButton::Primary),
    );
    app.update();
    at.position = Vec2::splat(500.0);
    bevy_widgetry_test_utils::queue_pointer(
        &mut app,
        pointer,
        at,
        PointerAction::Move {
            delta: Vec2::splat(500.0),
        },
    );
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(plus).unwrap().0,
        theme.button.normal.background
    );
    assert_eq!(*app.world().get::<Node>(root).unwrap(), geometry);
    assert_eq!(app.world().resource::<Changes>().0, 0);
}

#[test]
fn read_only_and_inherited_disabled_use_matching_palettes_and_recover() {
    for read_only in [false, true] {
        let (mut app, root, minus, _, plus, editor) = fixture(read_only);
        let theme = app.world().resource::<WidgetryThemeMode>().colors();
        let palette = if read_only {
            theme.text_field.read_only
        } else {
            theme.text_field.editable
        };
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(palette.normal.border)
        );
        let parent = spawn_scene(app.world_mut(), bsn! { Node InteractionDisabled }).unwrap();
        app.world_mut().entity_mut(parent).add_child(root);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(editor, FocusCause::Navigated);
        app.update();
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(palette.disabled.border)
        );
        assert_eq!(
            app.world().get::<TextColor>(editor).unwrap().0,
            palette.disabled.foreground
        );
        for button in [minus, plus] {
            assert_eq!(
                app.world().get::<BackgroundColor>(button).unwrap().0,
                theme.button.disabled.background
            );
        }
        app.world_mut()
            .entity_mut(parent)
            .remove::<InteractionDisabled>();
        app.update();
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(palette.focused.border)
        );
        assert_eq!(app.world().resource::<Changes>().0, 0);
    }
}
