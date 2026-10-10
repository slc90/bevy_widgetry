//! State：Mouse / Custom 的普通按下、正常释放、取消、Pointer 与 target 失效。
//! Stimuli：PointerInput 经真实 UI backend；不直接 trigger Activate 或改业务 state。
//! Guards：cleanup 只归 session owner，程序 Pressed 保留，取消不成功 activation。
//! Couplings：共享适配与 Button / Checkbox / TriState / Radio 包装层。
//! 非 owner 的高层 Cancel/Release/DragEnd 在 Picking 派发后注入，分别保护 pointer/target/button ownership。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    picking::{hover::Hovered, pointer::*},
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Activate,
};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonPlugin},
    check_box::{WidgetryCheckBox, WidgetryCheckBoxPlugin, WidgetryTriStateCheckbox},
    radio_group::{WidgetryRadioGroup, WidgetryRadioGroupPlugin, WidgetryRadioOption},
};
use bevy_widgetry_test_utils::{picking_app, pointer_ids, queue_pointer, spawn_picking_camera};

#[derive(Resource, Default)]
struct Activations(usize);

fn fixture(id: PointerId, kind: &str) -> (App, Entity, Entity, Location) {
    let mut app = picking_app();
    app.add_plugins((
        WidgetryButtonPlugin,
        WidgetryCheckBoxPlugin,
        WidgetryRadioGroupPlugin,
    ))
    .init_resource::<Activations>();
    app.add_observer(|_: On<Activate>, mut seen: ResMut<Activations>| seen.0 += 1);
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
    let root=match kind {
        "button"=>app.world_mut().spawn_scene(bsn!{@WidgetryButton Node {width:px(100),height:px(60)} template(move |_|Ok(UiTargetCamera(camera)))}).unwrap().id(),
        "checkbox"=>app.world_mut().spawn_scene(bsn!{@WidgetryCheckBox Node {width:px(100),height:px(60)} template(move |_|Ok(UiTargetCamera(camera)))}).unwrap().id(),
        "tri"=>app.world_mut().spawn_scene(bsn!{@WidgetryTriStateCheckbox Node {width:px(100),height:px(60)} template(move |_|Ok(UiTargetCamera(camera)))}).unwrap().id(),
        _=> {
            let group=app.world_mut().spawn_scene(bsn!{@WidgetryRadioGroup Node {width:px(100)} template(move |_|Ok(UiTargetCamera(camera))) Children [@WidgetryRadioOption Node {width:px(100),height:px(60)}--@WidgetryRadioOption]}).unwrap().id();
            app.world().get::<Children>(group).unwrap()[0]
        }
    };
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    app.update();
    app.update();
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(30.0, 15.0),
    };
    (app, root, window, location)
}

fn input(app: &mut App, id: PointerId, location: &Location, action: PointerAction) {
    queue_pointer(app, id, location.clone(), action);
    app.update();
}

#[test]
fn ordinary_pressed_cleanup_is_owned_and_preserves_normal_clicks() {
    for id in pointer_ids() {
        for kind in ["button", "checkbox", "tri", "radio"] {
            for failure in ["cancel", "location", "pointer", "window", "ancestor"] {
                let (mut app, root, window, mut location) = fixture(id, kind);
                let manual=app.world_mut().spawn_scene(bsn!{@WidgetryButton Pressed Node {position_type:PositionType::Absolute,left:px(200),width:px(50),height:px(50)}}).unwrap().id();
                input(
                    &mut app,
                    id,
                    &location,
                    PointerAction::Move {
                        delta: location.position,
                    },
                );
                assert!(app.world().get::<Hovered>(root).unwrap().0, "{kind}");
                input(
                    &mut app,
                    id,
                    &location,
                    PointerAction::Press(PointerButton::Primary),
                );
                assert!(app.world().get::<Pressed>(root).is_some(), "{kind}");
                let pointer = app
                    .world_mut()
                    .query::<(Entity, &PointerId)>()
                    .iter(app.world())
                    .find(|(_, pointer)| **pointer == id)
                    .unwrap()
                    .0;
                location.position = Vec2::splat(350.0);
                input(
                    &mut app,
                    id,
                    &location,
                    PointerAction::Move {
                        delta: Vec2::splat(320.0),
                    },
                );
                match failure {
                    "ancestor" => {
                        let ancestor = app.world_mut().spawn(Node::default()).id();
                        app.world_mut()
                            .entity_mut(ancestor)
                            .add_children(&[root, manual]);
                        app.world_mut()
                            .entity_mut(ancestor)
                            .insert(InteractionDisabled);
                        app.world_mut().flush();
                        assert!(app.world().get::<Pressed>(root).is_none());
                        assert!(app.world().get::<Pressed>(manual).is_some());
                    }
                    "cancel" => {
                        queue_pointer(&mut app, id, location.clone(), PointerAction::Cancel)
                    }
                    "location" => {
                        app.world_mut()
                            .get_mut::<PointerLocation>(pointer)
                            .unwrap()
                            .location = None
                    }
                    "pointer" => {
                        app.world_mut().despawn(pointer);
                    }
                    _ => {
                        app.world_mut().despawn(window);
                    }
                }
                app.update();
                app.update();
                assert!(
                    app.world().get::<Pressed>(root).is_none(),
                    "{kind}/{failure}"
                );
                assert!(app.world().get::<Pressed>(manual).is_some());
                assert_eq!(app.world().resource::<Activations>().0, 0);
            }
            let (mut app, root, _, location) = fixture(id, kind);
            input(
                &mut app,
                id,
                &location,
                PointerAction::Move {
                    delta: location.position,
                },
            );
            input(
                &mut app,
                id,
                &location,
                PointerAction::Press(PointerButton::Primary),
            );
            input(
                &mut app,
                id,
                &location,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(app.world().get::<Pressed>(root).is_none());
            if kind == "button" {
                assert_eq!(app.world().resource::<Activations>().0, 1);
            }
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
            input(
                &mut app,
                id,
                &location,
                PointerAction::Press(PointerButton::Primary),
            );
            input(
                &mut app,
                id,
                &location,
                PointerAction::Release(PointerButton::Primary),
            );
            assert!(app.world().get::<Pressed>(root).is_none());
        }
    }
}

#[test]
fn unrelated_cancel_and_old_target_do_not_end_a_new_press() {
    for id in pointer_ids() {
        let (mut app, root, _, location) = fixture(id, "button");
        input(
            &mut app,
            id,
            &location,
            PointerAction::Move {
                delta: location.position,
            },
        );
        input(
            &mut app,
            id,
            &location,
            PointerAction::Press(PointerButton::Primary),
        );
        let unrelated = PointerId::Touch(7);
        app.world_mut().spawn(unrelated);
        input(
            &mut app,
            unrelated,
            &location,
            PointerAction::Move {
                delta: location.position,
            },
        );
        queue_pointer(&mut app, unrelated, location.clone(), PointerAction::Cancel);
        app.update();
        assert!(app.world().get::<Pressed>(root).is_some());
        let mut stale = location.clone();
        stale.target = bevy::camera::NormalizedRenderTarget::None {
            width: 1,
            height: 1,
        };
        queue_pointer(&mut app, id, stale, PointerAction::Cancel);
        app.update();
        assert!(app.world().get::<Pressed>(root).is_some());
        input(
            &mut app,
            id,
            &location,
            PointerAction::Release(PointerButton::Primary),
        );
        assert!(app.world().get::<Pressed>(root).is_none());
        assert_eq!(app.world().resource::<Activations>().0, 0);
        input(
            &mut app,
            id,
            &location,
            PointerAction::Press(PointerButton::Primary),
        );
        queue_pointer(&mut app, id, location.clone(), PointerAction::Cancel);
        queue_pointer(
            &mut app,
            id,
            location.clone(),
            PointerAction::Press(PointerButton::Primary),
        );
        app.update();
        assert!(app.world().get::<Pressed>(root).is_none());
        input(
            &mut app,
            id,
            &location,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            id,
            &location,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(app.world().resource::<Activations>().0, 1);
    }
}

#[derive(Resource, Default)]
struct StaleTerminal(Option<Terminal>);

enum Terminal {
    Cancel(bevy::picking::events::PointerCancel),
    Release(bevy::picking::events::PointerRelease),
    DragEnd(bevy::picking::events::PointerDragEnd),
}

#[test]
fn stale_high_level_terminals_do_not_clear_owned_pressed() {
    for id in pointer_ids() {
        for kind in ["button", "checkbox", "tri", "radio"] {
            for mismatch in ["pointer", "target", "button"] {
                for terminal in ["cancel", "release", "drag-end"] {
                    if mismatch == "button" && terminal == "cancel" {
                        continue;
                    }
                    let (mut app, root, _, location) = fixture(id, kind);
                    app.init_resource::<StaleTerminal>().add_systems(
                        PreUpdate,
                        (|mut event: ResMut<StaleTerminal>, mut commands: Commands| {
                            if let Some(event) = event.0.take() {
                                match event {
                                    Terminal::Cancel(event) => {
                                        commands.trigger(event);
                                    }
                                    Terminal::Release(event) => {
                                        commands.trigger(event);
                                    }
                                    Terminal::DragEnd(event) => {
                                        commands.trigger(event);
                                    }
                                }
                            }
                        })
                        .after(bevy::picking::PickingSystems::Hover)
                        .before(bevy::picking::PickingSystems::PostHover),
                    );
                    input(
                        &mut app,
                        id,
                        &location,
                        PointerAction::Move {
                            delta: location.position,
                        },
                    );
                    input(
                        &mut app,
                        id,
                        &location,
                        PointerAction::Press(PointerButton::Primary),
                    );
                    let stale_id = if mismatch == "pointer" {
                        PointerId::Touch(7)
                    } else {
                        id
                    };
                    let mut stale_location = location.clone();
                    if mismatch == "target" {
                        stale_location.target = bevy::camera::NormalizedRenderTarget::None {
                            width: 1,
                            height: 1,
                        };
                    }
                    let pointer = bevy::picking::events::Pointer::new(stale_id, stale_location);
                    let button = if mismatch == "button" {
                        PointerButton::Secondary
                    } else {
                        PointerButton::Primary
                    };
                    let stale = match terminal {
                        "cancel" => {
                            let mut event = bevy_widgetry_test_utils::primary_cancel(root);
                            event.pointer = pointer;
                            Terminal::Cancel(event)
                        }
                        "release" => {
                            let mut event = bevy_widgetry_test_utils::primary_release(root);
                            event.pointer = pointer;
                            event.button = button;
                            Terminal::Release(event)
                        }
                        _ => {
                            let mut event = bevy_widgetry_test_utils::primary_drag_end(root);
                            event.pointer = pointer;
                            event.button = button;
                            Terminal::DragEnd(event)
                        }
                    };
                    app.world_mut().resource_mut::<StaleTerminal>().0 = Some(stale);
                    app.update();
                    assert!(app.world().get::<Pressed>(root).is_some(), "{id:?}/{kind}");
                    input(
                        &mut app,
                        id,
                        &location,
                        PointerAction::Release(PointerButton::Primary),
                    );
                    assert!(app.world().get::<Pressed>(root).is_none());
                    if kind == "button" {
                        assert_eq!(app.world().resource::<Activations>().0, 1);
                    }
                }
            }
        }
    }
}
