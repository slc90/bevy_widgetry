use bevy::{
    app::{App, Plugin},
    ecs::{
        component::Component,
        entity::Entity,
        event::EntityEvent,
        observer::On,
        query::{Has, With},
        system::{Commands, Query},
    },
    log::info,
    picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release},
    ui::Pressed,
};

// 自定义最小Button Marker
#[derive(Component)]
pub struct MiniButton;

// 自定义Activate-like observer事件
#[derive(Debug, EntityEvent)]
pub struct MiniActivate {
    pub entity: Entity,
}

// 用于加入自定义最小Button的observer
pub struct MiniButtonPlugin;

impl Plugin for MiniButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_mini_button_on_press);
        app.add_observer(handle_mini_button_on_click);
        app.add_observer(handle_mini_button_on_release);
        app.add_observer(handle_mini_button_on_drag_end);
        app.add_observer(handle_mini_button_on_cancel);
    }
}

// 按下Press时的处理
fn handle_mini_button_on_press(
    mut event: On<Pointer<Press>>,
    mut q_state: Query<(Entity, Has<Pressed>), With<MiniButton>>,
    mut commands: Commands,
) {
    if let Ok((entity, has_pressed)) = q_state.get_mut(event.entity) {
        event.propagate(false);
        if !has_pressed {
            commands.entity(entity).insert(Pressed);
        }
    }
}

// 触发Click时的处理
fn handle_mini_button_on_click(
    mut event: On<Pointer<Click>>,
    mut q_state: Query<(Entity, Has<Pressed>), With<MiniButton>>,
    mut commands: Commands,
) {
    if let Ok((entity, has_pressed)) = q_state.get_mut(event.entity) {
        event.propagate(false);
        if has_pressed {
            commands.trigger(MiniActivate { entity });
        }
    }
}

// Release时的处理
fn handle_mini_button_on_release(
    mut event: On<Pointer<Release>>,
    mut q_state: Query<(Entity, Has<Pressed>), With<MiniButton>>,
    mut commands: Commands,
) {
    if let Ok((entity, has_pressed)) = q_state.get_mut(event.entity) {
        event.propagate(false);
        if has_pressed {
            commands.entity(entity).remove::<Pressed>();
        }
    }
}

// 拖动结束时的处理
fn handle_mini_button_on_drag_end(
    mut event: On<Pointer<DragEnd>>,
    mut q_state: Query<(Entity, Has<Pressed>), With<MiniButton>>,
    mut commands: Commands,
) {
    info!("drag_end current: {}", event.entity);
    if let Ok((entity, has_pressed)) = q_state.get_mut(event.entity) {
        event.propagate(false);
        if has_pressed {
            commands.entity(entity).remove::<Pressed>();
        }
    }
}

// 取消结束时的处理
fn handle_mini_button_on_cancel(
    mut event: On<Pointer<Cancel>>,
    mut q_state: Query<(Entity, Has<Pressed>), With<MiniButton>>,
    mut commands: Commands,
) {
    info!("cancel current: {}", event.entity);
    if let Ok((entity, has_pressed)) = q_state.get_mut(event.entity) {
        event.propagate(false);
        if has_pressed {
            commands.entity(entity).remove::<Pressed>();
        }
    }
}
