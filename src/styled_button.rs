use bevy::{
    app::{App, HierarchyPropagatePlugin, Plugin, PostUpdate, Propagate, PropagateSet, Update},
    color::Color,
    ecs::{
        component::Component,
        lifecycle::RemovedComponents,
        query::{Added, Changed, Has, Or, With},
        schedule::IntoScheduleConfigs,
        system::Query,
    },
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    text::TextColor,
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Node, Pressed, UiRect, UiSystems, px},
    ui_widgets::Button,
    utils::default,
};

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ForegroundColor(pub Color);

impl Default for ForegroundColor {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

#[derive(Component, Default)]
#[require(
    Button,
    Hovered,
    TabIndex(-1),
    Node = styled_button_node(),
    BackgroundColor = styled_button_background(),
    BorderColor = styled_button_border(),
    Propagate::<ForegroundColor> = Propagate(ForegroundColor::default()),
)]
pub struct StyledButton;

fn styled_button_node() -> Node {
    Node {
        padding: UiRect::axes(px(12), px(6)),
        border: UiRect::all(px(1)),
        ..default()
    }
}

fn styled_button_background() -> BackgroundColor {
    BackgroundColor(Color::srgb(0.30, 0.30, 0.30))
}

fn styled_button_border() -> BorderColor {
    BorderColor::all(Color::srgb(0.0, 0.8, 1.0))
}

fn update_styled_button_background_changed(
    mut query: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
        ),
        (
            With<StyledButton>,
            Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
        ),
    >,
) {
    for (hovered, pressed, disabled, mut background) in &mut query {
        background.0 = resolve_button_background(hovered.0, pressed, disabled);
    }
}

fn update_styled_button_background_removed(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
        ),
        With<StyledButton>,
    >,
) {
    for entity in removed_pressed.read().chain(removed_disabled.read()) {
        if let Ok((hovered, pressed, disabled, mut background)) = query.get_mut(entity) {
            background.0 = resolve_button_background(hovered.0, pressed, disabled);
        }
    }
}

fn resolve_button_background(hovered: bool, pressed: bool, disabled: bool) -> Color {
    if disabled {
        Color::srgb(0.15, 0.15, 0.15)
    } else if pressed {
        Color::srgb(0.85, 0.12, 0.12)
    } else if hovered {
        Color::srgb(0.20, 0.65, 0.95)
    } else {
        Color::srgb(0.30, 0.30, 0.30)
    }
}

fn apply_foreground_color_to_text(
    mut query: Query<(&ForegroundColor, &mut TextColor), Changed<ForegroundColor>>,
) {
    for (foreground, mut text_color) in &mut query {
        text_color.0 = foreground.0;
    }
}

pub struct StyledButtonPlugin;

impl Plugin for StyledButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HierarchyPropagatePlugin::<ForegroundColor>::new(PostUpdate));

        app.configure_sets(
            PostUpdate,
            PropagateSet::<ForegroundColor>::default().in_set(UiSystems::Propagate),
        );

        app.add_systems(
            Update,
            (
                update_styled_button_background_changed,
                update_styled_button_background_removed,
            ),
        );

        app.add_systems(
            PostUpdate,
            apply_foreground_color_to_text
                .in_set(UiSystems::Propagate)
                .after(PropagateSet::<ForegroundColor>::default()),
        );
    }
}
