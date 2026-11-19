#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::color::palettes::tailwind;
use bevy::{
    camera::RenderTarget,
    picking::pointer::{Location, PointerAction, PointerButton, PointerId},
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button, ButtonPlugin},
    window::{WindowCloseRequested, WindowRef},
};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_test_utils::{picking_app, queue_pointer, scene_app, spawn_ui_camera};
use bevy_widgetry_theme::WidgetryTheme;
use bevy_widgetry_window::{
    WidgetryModalWindow, WidgetryWindowAction, WidgetryWindowBackground, WidgetryWindowBehavior,
    owned_widgetry_window_with_controls, prepare_native_window, widgetry_window_target,
    widgetry_window_with_controls,
};

fn controls(root: &mut EntityWorldMut) -> Entity {
    let title_bar = root.get::<Children>().unwrap()[0];
    root.world_scope(|world| world.get::<Children>(title_bar).unwrap()[2])
}

fn custom_controls() -> impl SceneList {
    bsn_list! {
        Button Node { width: px(116), column_gap: px(6) }
        BackgroundColor({Color::from(tailwind::PURPLE_700)})
        Children [@WidgetryIcon { @path: {BuiltinIcon::WindowMinimize.path()}, @max_size: {Some(UVec2::splat(24))} }]--
        Node { width: px(86) } template(|_| Ok(Pickable::IGNORE))
    }
}

#[test]
fn custom_controls_replace_the_entire_region_in_owned_and_borrowed_windows() {
    let mut app = picking_app();
    app.add_plugins((bevy_widgetry_window::WidgetryWindowPlugin, ButtonPlugin));
    let behavior = WidgetryWindowBehavior {
        double_click_maximize: false,
        ..default()
    };
    let world = app.world_mut();
    let native = world.spawn(prepare_native_window(Window::default())).id();
    let camera = world.spawn(Camera2d).id();
    let mut borrowed = world.spawn_scene(bsn! {
        @widgetry_window_with_controls(native, camera, behavior, WidgetryWindowBackground::Theme,
            default(), bsn_list!{}, custom_controls(), bsn_list! {})
    }).unwrap();
    let borrowed_controls = controls(&mut borrowed);
    let mut owned = world.spawn_scene(bsn! {
        @owned_widgetry_window_with_controls(Window {resolution: (600, 400).into(), ..default()}, behavior, WidgetryWindowBackground::Theme,
            default(), bsn_list!{}, custom_controls(), bsn_list! { Text("Content") })
        template(move |_| Ok(WidgetryModalWindow {parent: native}))
    }).unwrap();
    let owned_root = owned.id();
    let owned_controls = controls(&mut owned);
    let owned_native = widgetry_window_target(world, owned_root).unwrap();
    let owned_camera = world.get::<UiTargetCamera>(owned_root).unwrap().0;
    app.update();
    let temporary = spawn_ui_camera(&mut app, UVec2::new(600, 400), 1.0);
    let world = app.world_mut();
    let computed = world.get::<Camera>(temporary).unwrap().computed.clone();
    world.despawn(temporary);
    world.get_mut::<Camera>(owned_camera).unwrap().computed = computed;

    let world = app.world_mut();
    for (slot, root_native) in [(borrowed_controls, native), (owned_controls, owned_native)] {
        let children = world.get::<Children>(slot).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(world.get::<Node>(children[1]).unwrap().width, px(86));
        assert!(!world.get::<Pickable>(slot).unwrap().is_hoverable);
        world
            .get_mut::<Window>(root_native)
            .unwrap()
            .enabled_buttons
            .close = false;
    }
    let button = world.get::<Children>(owned_controls).unwrap()[0];
    let icon = world.get::<Children>(button).unwrap()[0];
    let mut theme = *world.resource::<WidgetryTheme>();
    theme.window.title_bar.normal.border = Color::WHITE;
    WidgetryTheme::set_in_world(world, theme).unwrap();
    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<Node>(button).unwrap().width, px(116));
    assert_eq!(
        world.get::<BackgroundColor>(button).unwrap().0,
        Color::from(tailwind::PURPLE_700)
    );
    assert!(world.get::<InteractionDisabled>(button).is_none());
    assert_eq!(
        world.get::<WidgetryIcon>(icon).unwrap().path(),
        Some(&BuiltinIcon::WindowMinimize.path())
    );

    world
        .entity_mut(button)
        .observe(move |_: On<Activate>, mut commands: Commands| {
            WidgetryWindowAction::RequestClose.queue(&mut commands, owned_root);
        });
    let spacer = world.get::<Children>(owned_controls).unwrap()[1];
    let center = |entity| world.get::<UiGlobalTransform>(entity).unwrap().translation;
    let (button_center, spacer_center) = (center(button), center(spacer));
    let target = RenderTarget::Window(WindowRef::Entity(owned_native))
        .normalize(None)
        .unwrap();
    let send = |app: &mut App, position, action| {
        queue_pointer(
            app,
            PointerId::Mouse,
            Location {
                target: target.clone(),
                position,
            },
            action,
        );
        app.update();
    };
    let primary = PointerButton::Primary;
    let take_move_request = |app: &mut App| {
        app.world_mut()
            .get_mut::<Window>(owned_native)
            .unwrap()
            .internal
            .take_move_request()
    };
    send(
        &mut app,
        spacer_center,
        PointerAction::Move { delta: Vec2::ONE },
    );
    send(&mut app, spacer_center, PointerAction::Press(primary));
    assert!(take_move_request(&mut app));
    send(&mut app, spacer_center, PointerAction::Release(primary));
    assert!(
        app.world()
            .resource::<Messages<WindowCloseRequested>>()
            .is_empty()
    );

    send(
        &mut app,
        button_center,
        PointerAction::Move {
            delta: button_center - spacer_center,
        },
    );
    for enabled in [false, true] {
        app.world_mut()
            .get_mut::<Window>(owned_native)
            .unwrap()
            .enabled_buttons
            .close = enabled;
        send(&mut app, button_center, PointerAction::Press(primary));
        assert!(!take_move_request(&mut app));
        send(&mut app, button_center, PointerAction::Release(primary));
        let closed = app
            .world_mut()
            .resource_mut::<Messages<WindowCloseRequested>>()
            .drain()
            .map(|event| event.window)
            .collect::<Vec<_>>();
        let expected = enabled
            .then_some(owned_native)
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(closed, expected);
    }
    assert!(app.world().get::<Window>(native).is_some());
}

#[test]
fn empty_custom_controls_omit_default_buttons_and_respect_shell_configuration() {
    let mut app = scene_app();
    app.add_plugins(bevy_widgetry_window::WidgetryWindowPlugin);
    let mut root = app
        .world_mut()
        .spawn_scene(bsn! {
            @owned_widgetry_window_with_controls(Window::default(), WidgetryWindowBehavior {
                resizable: false, double_click_maximize: false,
            }, WidgetryWindowBackground::Theme, default(), bsn_list! {}, bsn_list! {}, bsn_list! {})
        })
        .unwrap();
    assert_eq!(root.get::<Children>().unwrap().len(), 2);
    let slot = controls(&mut root);
    app.update();
    assert!(
        app.world()
            .get::<Children>(slot)
            .is_none_or(|children| children.is_empty())
    );
}
