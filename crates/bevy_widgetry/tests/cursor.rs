#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#![cfg(all(
    feature = "button",
    feature = "check_box",
    feature = "radio_group",
    feature = "text_field",
    feature = "list_view"
))]

use bevy::{
    picking::{cursor::EntityCursor, pointer::*},
    prelude::*,
    ui::InteractionDisabled,
    window::{CursorIcon, SystemCursorIcon},
};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonPlugin},
    check_box::{WidgetryCheckBox, WidgetryCheckBoxPlugin, WidgetryTriStateCheckbox},
    cursor::WidgetryCursorPlugin,
    radio_group::{WidgetryRadioGroup, WidgetryRadioGroupPlugin, WidgetryRadioOption},
    text_field::{WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin},
};
use bevy_widgetry_test_utils::{picking_app, pointer_ids, queue_pointer, spawn_picking_camera};

#[test]
fn basic_widgets_use_mouse_cursor_and_inherited_disabled_in_the_same_update() {
    for kind in ["button", "checkbox", "tri", "radio", "text", "readonly"] {
        let mut app = picking_app();
        app.add_plugins((
            WidgetryButtonPlugin,
            WidgetryCheckBoxPlugin,
            WidgetryRadioGroupPlugin,
            WidgetryTextFieldPlugin,
        ));
        assert!(app.is_plugin_added::<WidgetryCursorPlugin>());
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: (400, 400).into(),
                    ..default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();
        let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
        let scene: Box<dyn Scene> = match kind {
            "button" => Box::new(bsn! { @WidgetryButton }),
            "checkbox" => Box::new(bsn! { @WidgetryCheckBox }),
            "tri" => Box::new(bsn! { @WidgetryTriStateCheckbox }),
            "radio" => Box::new(bsn! { @WidgetryRadioGroup Children [@WidgetryRadioOption] }),
            "text" => Box::new(bsn! { @WidgetryTextField }),
            _ => Box::new(bsn! { @WidgetryReadOnlyTextField }),
        };
        let root = app.world_mut().spawn_scene(bsn! { @{scene} Node { width: px(100), height: px(60) } template(move |_| Ok(UiTargetCamera(camera))) }).unwrap().id();
        let declaration = if kind == "radio" {
            app.world().get::<Children>(root).unwrap()[0]
        } else if matches!(kind, "text" | "readonly") {
            app.world()
                .get::<bevy_widgetry_core::input_target::WidgetryInputTarget>(root)
                .unwrap()
                .target
        } else {
            root
        };
        let icon = if matches!(kind, "text" | "readonly") {
            SystemCursorIcon::Text
        } else {
            SystemCursorIcon::Pointer
        };
        assert_eq!(
            app.world().get::<EntityCursor>(declaration),
            Some(&EntityCursor::System(icon)),
            "{kind}"
        );
        app.update();
        app.update();
        let mut location = Location {
            target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                .normalize(None)
                .unwrap(),
            position: Vec2::new(20.0, 15.0),
        };
        queue_pointer(
            &mut app,
            PointerId::Mouse,
            location.clone(),
            PointerAction::Move { delta: Vec2::ONE },
        );
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(icon)),
            "{kind}"
        );
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(SystemCursorIcon::NotAllowed)),
            "{kind}"
        );
        app.world_mut()
            .entity_mut(root)
            .remove::<InteractionDisabled>();
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(icon)),
            "{kind}"
        );
        location.position = Vec2::splat(350.0);
        queue_pointer(
            &mut app,
            PointerId::Mouse,
            location.clone(),
            PointerAction::Move {
                delta: Vec2::splat(330.0),
            },
        );
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(SystemCursorIcon::Default)),
            "{kind}"
        );
        let custom = pointer_ids()[1];
        app.world_mut().spawn(custom);
        location.position = Vec2::new(20.0, 15.0);
        queue_pointer(
            &mut app,
            custom,
            location,
            PointerAction::Move { delta: Vec2::ONE },
        );
        app.update();
        assert_eq!(
            app.world().get::<CursorIcon>(window),
            Some(&CursorIcon::System(SystemCursorIcon::Default)),
            "{kind}"
        );
    }
}

#[test]
fn materialized_list_row_stops_ancestor_cursor_and_allows_child_override() {
    use bevy_widgetry::list_view::{
        WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewItem,
        WidgetryListViewPlugin, WidgetryListViewRenderer,
    };
    let mut app = picking_app();
    app.add_plugins((WidgetryListViewPlugin, WidgetryButtonPlugin));
    app.register_widgetry_list_view::<i32>().unwrap();
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 400).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let mut model = WidgetryListModel::default();
    model.push(1).unwrap();
    let source = app.world_mut().spawn(model).id();
    app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<i32> {
            @source: source,
            @item_height: 50.0,
            @renderer: {WidgetryListViewRenderer::new(|_: &i32| bsn_list! { Node { width: px(100), height: px(50) } })},
        }
        Node { width: px(100), height: px(100) }
        template(move |_| Ok(UiTargetCamera(camera)))
        template(|_| Ok(EntityCursor::System(SystemCursorIcon::Pointer)))
    }).unwrap();
    for _ in 0..5 {
        app.update();
    }
    let row = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListViewItem>>()
        .single(app.world())
        .unwrap();
    let position = app
        .world()
        .get::<bevy::ui::UiGlobalTransform>(row)
        .unwrap()
        .translation;
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position,
    };
    queue_pointer(
        &mut app,
        PointerId::Mouse,
        location.clone(),
        PointerAction::Move { delta: Vec2::ONE },
    );
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Default))
    );
    let button = app.world_mut().spawn_scene(bsn! {
        @WidgetryButton
        Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: percent(100), height: percent(100) }
    }).unwrap().id();
    app.world_mut().entity_mut(row).add_child(button);
    app.update();
    queue_pointer(
        &mut app,
        PointerId::Mouse,
        location,
        PointerAction::Move { delta: Vec2::ONE },
    );
    app.update();
    assert_eq!(
        app.world().get::<CursorIcon>(window),
        Some(&CursorIcon::System(SystemCursorIcon::Pointer))
    );
}
