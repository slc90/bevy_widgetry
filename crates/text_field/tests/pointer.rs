#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#![cfg(test)]

mod support;

use bevy::{
    input::keyboard::Key, input_focus::InputFocus, picking::pointer::*, prelude::*,
    ui::InteractionDisabled, ui_widgets::TextInputPlugin,
};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_test_utils::{
    advance_until, picking_app, pointer_ids, queue_pointer, spawn_picking_camera,
};
use bevy_widgetry_text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};
use std::time::Duration;

#[test]
fn pointer_position_word_and_drag_selection_share_identity_independent_guards() {
    for id in pointer_ids() {
        for kind in ["normal", "readonly", "disabled"] {
            let mut app = picking_app();
            app.init_resource::<ButtonInput<Key>>()
                .add_message::<bevy::window::Ime>()
                .add_plugins((
                    WidgetryAssetPlugin,
                    TextInputPlugin,
                    WidgetryTextFieldPlugin,
                ));
            let font = app
                .world()
                .resource::<AssetServer>()
                .load::<Font>(BuiltinFont::Default.path());
            advance_until(
                &mut app,
                Duration::from_secs(10),
                "Pointer TextField 字体",
                |world| world.resource::<Assets<Font>>().contains(&font),
            )
            .unwrap();
            app.set_default_font(bevy::text::FontSource::Handle(font));
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
            let root = if kind == "readonly" {
                app.world_mut().spawn_scene(bsn!{@WidgetryReadOnlyTextField { @value: {("hello world").to_owned()} } Node {width:px(240),height:px(40)} template(move |_|Ok(UiTargetCamera(camera)))}).unwrap().id()
            } else {
                app.world_mut().spawn_scene(bsn!{@WidgetryTextField { @value: {("hello world").to_owned()} } Node {width:px(240),height:px(40)} template(move |_|Ok(UiTargetCamera(camera)))}).unwrap().id()
            };
            if kind == "disabled" {
                app.world_mut().entity_mut(root).insert(InteractionDisabled);
            }
            if id != PointerId::Mouse {
                app.world_mut().spawn(id);
            }
            for _ in 0..4 {
                app.update();
            }
            let original = support::editor(app.world(), root)
                .unwrap()
                .editor()
                .raw_selection()
                .text_range();
            let mut location = Location {
                target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                    .normalize(None)
                    .unwrap(),
                position: Vec2::new(15.0, 18.0),
            };
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Move {
                    delta: location.position,
                },
            );
            app.update();
            for click in 0..2 {
                queue_pointer(
                    &mut app,
                    id,
                    location.clone(),
                    PointerAction::Press(PointerButton::Primary),
                );
                app.update();
                queue_pointer(
                    &mut app,
                    id,
                    location.clone(),
                    PointerAction::Release(PointerButton::Primary),
                );
                app.update();
                if click == 0 && kind != "disabled" {
                    let caret = support::editor(app.world(), root)
                        .unwrap()
                        .editor()
                        .raw_selection()
                        .text_range();
                    assert!(caret.is_empty());
                    assert!(caret.start < 11);
                }
            }
            let edit = support::editor(app.world(), root).unwrap();
            if kind == "disabled" {
                assert_eq!(edit.editor().raw_selection().text_range(), original);
            } else {
                assert_eq!(
                    app.world().resource::<InputFocus>().get(),
                    Some(support::editor_entity(app.world(), root))
                );
                assert_eq!(edit.editor().raw_selection().text_range(), 0..5);
            }
            *app.world_mut()
                .resource_mut::<bevy::time::TimeUpdateStrategy>() =
                bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_millis(600));
            app.update();
            *app.world_mut()
                .resource_mut::<bevy::time::TimeUpdateStrategy>() =
                bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_millis(16));
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Press(PointerButton::Primary),
            );
            app.update();
            location.position.x = 110.0;
            queue_pointer(
                &mut app,
                id,
                location.clone(),
                PointerAction::Move {
                    delta: Vec2::new(95.0, 0.0),
                },
            );
            app.update();
            queue_pointer(&mut app, id, location, PointerAction::Cancel);
            app.update();
            let edit = support::editor(app.world(), root).unwrap();
            assert_eq!(edit.value().to_string(), "hello world");
            if kind == "disabled" {
                assert_eq!(edit.editor().raw_selection().text_range(), original);
            } else {
                assert!(edit.editor().raw_selection().text_range().len() > 5);
            }
        }
    }
}
