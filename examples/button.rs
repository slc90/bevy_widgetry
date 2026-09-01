use bevy::color::palettes::basic::RED;
use bevy::picking::pointer::{PointerId, PointerInteraction, PointerLocation};
use bevy::prelude::*;
use bevy_widgetry::{MiniActivate, MiniButton, MiniButtonPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MiniButtonPlugin)
        .add_systems(Startup, start_up)
        .add_systems(Update, test_cancel)
        .run();
}

fn start_up(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            MiniButton,
            Node {
                left: Val::Px(100.0),
                top: Val::Px(100.0),
                width: Val::Px(200.0),
                height: Val::Px(100.0),
                ..Default::default()
            },
            BackgroundColor(RED.into()),
        ))
        .observe(|mini_activate: On<MiniActivate>| {
            info!("mini_activate: {:?}", mini_activate.entity);
        });
}

fn test_cancel(
    keys: Res<ButtonInput<KeyCode>>,
    pointers: Query<(&PointerId, &PointerLocation, &PointerInteraction)>,
    mini_button: Query<Entity, With<MiniButton>>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::KeyC) {
        return;
    }

    let Ok(button) = mini_button.single() else {
        return;
    };

    let Some((pointer_id, pointer_location, interaction)) =
        pointers.iter().find(|(id, _, _)| id.is_mouse())
    else {
        return;
    };

    let Some(location) = pointer_location.location().cloned() else {
        return;
    };

    let Some((_, hit)) = interaction.iter().find(|(entity, _)| *entity == button) else {
        info!("mouse is not over MiniButton");
        return;
    };

    commands.trigger(Pointer::new(
        *pointer_id,
        location,
        Cancel { hit: hit.clone() },
        button,
    ));
}
