use std::time::Duration;

use bevy::{
    app::{App, Plugin, Update},
    ecs::query::Has,
    ecs::{
        component::Component,
        entity::Entity,
        event::EntityEvent,
        observer::On,
        query::With,
        system::{Commands, Query, Res},
    },
    log::info,
    picking::events::{Cancel, DragEnd, Pointer, Press, Release},
    time::{Time, Timer, TimerMode},
    ui::InteractionDisabled,
    ui_widgets::Button,
};

// 扩展官方headless Button
#[derive(Component, Debug)]
#[require(Button)]
pub struct LongPressButton {
    // 长按持续时间，单位毫秒
    pub press_duration: u64,
}

impl Default for LongPressButton {
    fn default() -> Self {
        Self {
            press_duration: 500,
        }
    }
}

// 按下Press后增加的临时状态，直到计时结束或者提前Release/Cancel/DragEnd
#[derive(Component)]
pub struct LongPressPending {
    pub timer: Timer,
}

// 对外暴露的长按到时间时触发的事件
#[derive(EntityEvent)]
pub struct LongPressEvent {
    pub entity: Entity,
}

// 用于外部注册
pub struct LongPressPlugin;

impl Plugin for LongPressPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_long_press_button_on_press)
            .add_observer(handle_long_press_button_on_release)
            .add_observer(handle_long_press_button_on_drag_end)
            .add_observer(handle_long_press_button_on_cancel)
            .add_systems(Update, update_long_press);
    }
}

fn handle_long_press_button_on_press(
    event: On<Pointer<Press>>,
    query: Query<(Entity, &LongPressButton, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((entity, long_press_button, disabled)) = query.get(event.entity) {
        if !disabled {
            commands.entity(entity).insert(LongPressPending {
                timer: Timer::new(
                    Duration::from_millis(long_press_button.press_duration),
                    TimerMode::Once,
                ),
            });
            info!("press");
        }
    }
}

fn handle_long_press_button_on_release(
    event: On<Pointer<Release>>,
    query: Query<Entity, With<LongPressPending>>,
    mut commands: Commands,
) {
    if let Ok(entity) = query.get(event.entity) {
        commands.entity(entity).remove::<LongPressPending>();
        info!("release");
    }
}

fn handle_long_press_button_on_drag_end(
    event: On<Pointer<DragEnd>>,
    query: Query<Entity, With<LongPressPending>>,
    mut commands: Commands,
) {
    if let Ok(entity) = query.get(event.entity) {
        commands.entity(entity).remove::<LongPressPending>();
        info!("drag_end");
    }
}

fn handle_long_press_button_on_cancel(
    event: On<Pointer<Cancel>>,
    query: Query<Entity, With<LongPressPending>>,
    mut commands: Commands,
) {
    if let Ok(entity) = query.get(event.entity) {
        commands.entity(entity).remove::<LongPressPending>();
        info!("cancel");
    }
}

// 计时system，到时间后就触发LongPressEvent
fn update_long_press(
    time: Res<Time>,
    mut query: Query<(Entity, &mut LongPressPending)>,
    mut commands: Commands,
) {
    for (entity, mut pending) in query.iter_mut() {
        pending.timer.tick(time.delta());
        if pending.timer.just_finished() {
            commands.trigger(LongPressEvent { entity });
            info!("trigger long press event");
            commands.entity(entity).remove::<LongPressPending>();
            info!("remove LongPressPending");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    mod support {
        use super::*;

        use bevy::{
            camera::NormalizedRenderTarget,
            math::Vec2,
            picking::{
                backend::HitData,
                events::{Pointer, Press},
                pointer::{Location, PointerButton, PointerId},
            },
            ui_widgets::ButtonPlugin,
        };

        pub fn setup_button() -> (App, Entity) {
            let mut app = App::new();
            app.add_plugins(ButtonPlugin);

            let button = app.world_mut().spawn(Button).id();

            (app, button)
        }

        pub fn press(app: &mut App, entity: Entity) {
            app.world_mut().trigger(primary_press(entity));
            app.world_mut().flush();
        }

        fn primary_press(entity: Entity) -> Pointer<Press> {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 1,
                        height: 1,
                    },
                    position: Vec2::ZERO,
                },
                Press {
                    button: PointerButton::Primary,
                    hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    count: 1,
                },
                entity,
            )
        }

        pub fn release(app: &mut App, entity: Entity) {
            app.world_mut().trigger(primary_release(entity));
            app.world_mut().flush();
        }

        fn primary_release(entity: Entity) -> Pointer<Release> {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 1,
                        height: 1,
                    },
                    position: Vec2::ZERO,
                },
                Release {
                    button: PointerButton::Primary,
                    hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                },
                entity,
            )
        }

        pub fn setup_long_press_button() -> (App, Entity) {
            let mut app = App::new();
            app.add_plugins(LongPressPlugin);

            let long_press_button = app.world_mut().spawn(LongPressButton::default()).id();

            (app, long_press_button)
        }

        pub fn cancel(app: &mut App, entity: Entity) {
            app.world_mut().trigger(primary_cancel(entity));
            app.world_mut().flush();
        }

        fn primary_cancel(entity: Entity) -> Pointer<Cancel> {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 1,
                        height: 1,
                    },
                    position: Vec2::ZERO,
                },
                Cancel {
                    hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                },
                entity,
            )
        }

        pub fn drag_end(app: &mut App, entity: Entity) {
            app.world_mut().trigger(primary_drag_end(entity));
            app.world_mut().flush();
        }

        fn primary_drag_end(entity: Entity) -> Pointer<DragEnd> {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 1,
                        height: 1,
                    },
                    position: Vec2::ZERO,
                },
                DragEnd {
                    button: PointerButton::Primary,
                    distance: Vec2::ZERO,
                },
                entity,
            )
        }
    }

    use bevy::ui::{InteractionDisabled, Pressed};
    use support::*;

    #[test]
    fn pointer_press_adds_pressed() {
        let (mut app, button) = setup_button();

        assert!(!app.world().entity(button).contains::<Pressed>());

        press(&mut app, button);

        assert!(app.world().entity(button).contains::<Pressed>());
    }

    #[test]
    fn pointer_release_removes_pressed() {
        let (mut app, button) = setup_button();

        press(&mut app, button);
        assert!(app.world().entity(button).contains::<Pressed>());

        release(&mut app, button);
        assert!(!app.world().entity(button).contains::<Pressed>());
    }

    #[test]
    fn press_starts_long_press_pending() {
        let (mut app, long_press_button) = setup_long_press_button();

        press(&mut app, long_press_button);

        assert!(
            app.world()
                .entity(long_press_button)
                .contains::<LongPressPending>()
        );
    }

    #[test]
    fn release_cancels_long_press_pending() {
        let (mut app, button) = setup_long_press_button();

        press(&mut app, button);
        assert!(app.world().entity(button).contains::<LongPressPending>());

        release(&mut app, button);
        assert!(!app.world().entity(button).contains::<LongPressPending>());
    }

    #[test]
    fn cancel_cancels_long_press_pending() {
        let (mut app, button) = setup_long_press_button();

        press(&mut app, button);
        assert!(app.world().entity(button).contains::<LongPressPending>());

        cancel(&mut app, button);
        assert!(!app.world().entity(button).contains::<LongPressPending>());
    }

    #[test]
    fn drag_end_cancels_long_press_pending() {
        let (mut app, button) = setup_long_press_button();

        press(&mut app, button);
        assert!(app.world().entity(button).contains::<LongPressPending>());

        drag_end(&mut app, button);
        assert!(!app.world().entity(button).contains::<LongPressPending>());
    }

    #[test]
    fn disabled_button_does_not_start_long_press_pending() {
        let (mut app, button) = setup_long_press_button();

        app.world_mut()
            .entity_mut(button)
            .insert(InteractionDisabled);

        press(&mut app, button);

        assert!(!app.world().entity(button).contains::<LongPressPending>());
    }

    #[test]
    fn press_uses_configured_long_press_duration() {
        let mut app = App::new();
        app.add_plugins(LongPressPlugin);

        let button = app
            .world_mut()
            .spawn(LongPressButton {
                press_duration: 750,
            })
            .id();

        press(&mut app, button);

        let pending = app
            .world()
            .entity(button)
            .get::<LongPressPending>()
            .unwrap();

        assert_eq!(pending.timer.duration(), Duration::from_millis(750));
    }
}
