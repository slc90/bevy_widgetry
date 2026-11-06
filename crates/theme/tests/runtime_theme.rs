#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_theme::{
    ThemeError, WIDGETRY_DARK_THEME, WidgetryTheme, WidgetryThemeChanged, WidgetryThemePlugin,
};

#[derive(Resource, Default)]
struct Seen(Vec<WidgetryTheme>);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(WidgetryThemePlugin)
        .init_resource::<Seen>()
        .add_observer(
            |_: On<WidgetryThemeChanged>, theme: Res<WidgetryTheme>, mut seen: ResMut<Seen>| {
                seen.0.push(*theme);
            },
        );
    app
}

#[test]
fn external_palette_commits_before_notification_and_equal_values_are_silent() {
    let mut app = app();
    let mut custom = WIDGETRY_DARK_THEME;
    custom.button.normal.background = Color::srgb(0.1, 0.2, 0.3);
    custom.button.hovered.background = Color::srgb(0.3, 0.2, 0.1);
    assert!(WidgetryTheme::set_in_world(app.world_mut(), custom).unwrap());
    assert_eq!(*app.world().resource::<WidgetryTheme>(), custom);
    assert!(!WidgetryTheme::set_in_world(app.world_mut(), custom).unwrap());
    assert_eq!(app.world().resource::<Seen>().0, [custom]);

    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    WidgetryTheme::set(&mut commands, WIDGETRY_DARK_THEME);
    WidgetryTheme::set(&mut commands, WIDGETRY_DARK_THEME);
    WidgetryTheme::set(&mut commands, custom);
    assert_eq!(*app.world().resource::<WidgetryTheme>(), custom);
    queue.apply(app.world_mut());
    assert_eq!(
        app.world().resource::<Seen>().0,
        [custom, WIDGETRY_DARK_THEME, custom]
    );
}

#[test]
fn invalid_nested_colors_reject_the_entire_palette_without_notification() {
    let mut app = app();
    for color in [
        Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0),
        Color::srgba(0.0, 0.0, 0.0, 1.1),
    ] {
        let mut invalid = WIDGETRY_DARK_THEME;
        invalid.button.normal.background = Color::WHITE;
        invalid.file_dialog.status.disabled.foreground = color;
        let error = WidgetryTheme::set_in_world(app.world_mut(), invalid).unwrap_err();
        assert!(matches!(error.kind(), ThemeError::InvalidColor { .. }));
        assert_eq!(
            *app.world().resource::<WidgetryTheme>(),
            WIDGETRY_DARK_THEME
        );
        assert!(app.world().resource::<Seen>().0.is_empty());
    }
}

#[test]
fn invalid_waveform_palettes_cannot_bypass_color_override_constraints() {
    let mut app = app();
    for palette in [
        &[][..],
        &[Color::WHITE][..],
        &[Color::WHITE, Color::WHITE][..],
        &[Color::WHITE, Color::BLACK, Color::WHITE][..],
    ] {
        let mut invalid = WIDGETRY_DARK_THEME;
        invalid.waveform.disabled.palette = palette;
        let error = WidgetryTheme::set_in_world(app.world_mut(), invalid).unwrap_err();
        assert!(matches!(error.kind(), ThemeError::InvalidPalette { .. }));
        assert_eq!(
            *app.world().resource::<WidgetryTheme>(),
            WIDGETRY_DARK_THEME
        );
        assert!(app.world().resource::<Seen>().0.is_empty());
    }
}
