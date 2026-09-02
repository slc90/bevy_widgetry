use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::Button;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update_button_style, update_button_style_remove))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn((
            // 行为
            Button,
            // 必须增加，不然query直接查不到entity，导致连pressed也不起作用了
            Hovered::default(),
            // 用于测试InteractionDisabled时的样式而增加
            // InteractionDisabled,
            // 固定的布局
            Node {
                width: px(160),
                height: px(48),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            // 当前最终视觉
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        ))
        .with_child((Text::new("Button"),));
}

fn resolve_button_color(hovered: bool, pressed: bool, disabled: bool) -> Color {
    if disabled {
        Color::srgb(0.12, 0.12, 0.12)
    } else if pressed {
        Color::srgb(0.15, 0.35, 0.65)
    } else if hovered {
        Color::srgb(0.25, 0.50, 0.85)
    } else {
        Color::srgb(0.25, 0.25, 0.25)
    }
}

fn update_button_style(
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
        ),
        (
            With<Button>,
            Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
        ),
    >,
) {
    for (hovered, pressed, disabled, mut background) in &mut buttons {
        background.0 = resolve_button_color(hovered.get(), pressed, disabled);
    }
}

fn update_button_style_remove(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
        ),
        With<Button>,
    >,
) {
    for entity in removed_pressed.read().chain(removed_disabled.read()) {
        let Ok((hovered, pressed, disabled, mut background)) = buttons.get_mut(entity) else {
            continue;
        };

        background.0 = resolve_button_color(hovered.get(), pressed, disabled);
    }
}
