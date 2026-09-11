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
use std::time::Duration;

/// 扩展 Bevy Button 的长按计时；需注册 LongPressPlugin。
#[derive(Component, Debug)]
#[require(Button)]
pub struct LongPressButton {
    /// 触发一次长按所需的毫秒数，默认为 500；零表示下次计时更新即到期。
    pub press_duration: u64,
}

// 按下Press后增加的临时状态，直到计时结束或者提前Release/Cancel/DragEnd
#[derive(Component)]
struct LongPressPending {
    /// 当前按压的一次性计时器，到期或中断后移除。
    timer: Timer,
}

/// 一次按压达到计时阈值时发出；释放、取消或结束拖动会提前终止计时。
#[derive(EntityEvent)]
pub struct LongPressEvent {
    /// 接收该实体事件的控件根实体。
    pub entity: Entity,
}

/// 注册指针观察者和长按计时系统；应用需提供 Time 资源。
pub struct LongPressPlugin;

/// 仅为未禁用按钮建立计时状态，新的按压重新开始计时。
fn handle_long_press_button_on_press(
    event: On<Pointer<Press>>,
    query: Query<(Entity, &LongPressButton, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((entity, long_press_button, disabled)) = query.get(event.entity)
        && !disabled
    {
        commands.entity(entity).insert(LongPressPending {
            timer: Timer::new(
                Duration::from_millis(long_press_button.press_duration),
                TimerMode::Once,
            ),
        });
        info!("press");
    }
}

/// 释放指针时移除待完成计时，避免随后产生长按事件。
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

/// 结束拖动时终止待完成长按，避免拖动操作被识别为长按。
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

/// 取消指针交互时清理待完成计时。
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

impl Default for LongPressButton {
    fn default() -> Self {
        Self {
            press_duration: 500,
        }
    }
}

impl Plugin for LongPressPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_long_press_button_on_press)
            .add_observer(handle_long_press_button_on_release)
            .add_observer(handle_long_press_button_on_drag_end)
            .add_observer(handle_long_press_button_on_cancel)
            .add_systems(Update, update_long_press);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::press;
    use std::time::Duration;

    fn setup_long_press_button() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(LongPressPlugin);
        let button = app.world_mut().spawn(LongPressButton::default()).id();
        (app, button)
    }

    // 直接触发按压观察者，验证内部计时状态已在命令刷新后建立。
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

    // 将长按时长配置为 750 毫秒，验证按下后创建的计时器采用该配置值。
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

        // 直接检查计时器配置，无需等待真实时钟推进。
        assert_eq!(pending.timer.duration(), Duration::from_millis(750));
    }
}
