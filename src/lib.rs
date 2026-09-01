use bevy::{
    app::{App, Plugin, Update},
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
    ui_widgets::Button,
};

// 扩展官方headless Button
#[derive(Component, Debug)]
#[require(Button)]
pub struct LongPressButton {
    // 长按持续时间，单位毫秒
    pub press_duration: f32,
}

impl Default for LongPressButton {
    fn default() -> Self {
        Self {
            press_duration: 500.0,
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
    query: Query<(Entity, &LongPressButton)>,
    mut commands: Commands,
) {
    if let Ok((entity, long_press_button)) = query.get(event.entity) {
        commands.entity(entity).insert(LongPressPending {
            timer: Timer::from_seconds(long_press_button.press_duration / 1000.0, TimerMode::Once),
        });
        info!("press");
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
