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
    picking::events::{Click, Pointer, Press, Release},
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
    }
}

// 按下Press时的处理
fn handle_mini_button_on_press(
    event: On<Pointer<Press>>,
    mut q_state: Query<(Entity, Has<Pressed>), With<MiniButton>>,
    mut commands: Commands,
) {
    if let Ok((entity, has_pressed)) = q_state.get_mut(event.entity) {
        if !has_pressed {
            commands.entity(entity).insert(Pressed);
        }
    }
}

// 触发Click时的处理
fn handle_mini_button_on_click(
    event: On<Pointer<Click>>,
    mut q_state: Query<(Entity, Has<Pressed>), With<MiniButton>>,
    mut commands: Commands,
) {
    if let Ok((entity, has_pressed)) = q_state.get_mut(event.entity) {
        if has_pressed {
            commands.trigger(MiniActivate { entity });
        }
    }
}

// Release时的处理
fn handle_mini_button_on_release(
    event: On<Pointer<Release>>,
    mut q_state: Query<(Entity, Has<Pressed>), With<MiniButton>>,
    mut commands: Commands,
) {
    if let Ok((entity, has_pressed)) = q_state.get_mut(event.entity) {
        if has_pressed {
            commands.entity(entity).remove::<Pressed>();
        }
    }
}
