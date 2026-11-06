mod kamuri_violet;

use bevy_widgetry::theme::{
    WIDGETRY_DARK_THEME, WIDGETRY_LIGHT_THEME, WIDGETRY_PINK_DREAM_THEME, WidgetryTheme,
};
use kamuri_violet::KAMURI_VIOLET_THEME;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum GalleryTheme {
    Dark,
    Light,
    PinkDream,
    KamuriViolet,
}

impl GalleryTheme {
    pub(crate) const fn colors(self) -> WidgetryTheme {
        match self {
            Self::Dark => WIDGETRY_DARK_THEME,
            Self::Light => WIDGETRY_LIGHT_THEME,
            Self::PinkDream => WIDGETRY_PINK_DREAM_THEME,
            Self::KamuriViolet => KAMURI_VIOLET_THEME,
        }
    }

    pub(crate) fn from_colors(theme: &WidgetryTheme) -> Option<Self> {
        [Self::Dark, Self::Light, Self::PinkDream, Self::KamuriViolet]
            .into_iter()
            .find(|preset| preset.colors() == *theme)
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::PinkDream,
            Self::PinkDream => Self::KamuriViolet,
            Self::KamuriViolet => Self::Dark,
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use bevy_widgetry::theme::WidgetryThemePlugin;

    #[test]
    fn gallery_owned_palette_uses_the_public_theme_path() {
        let mut app = App::new();
        app.add_plugins(WidgetryThemePlugin);
        let theme = GalleryTheme::KamuriViolet.colors();
        assert!(WidgetryTheme::set_in_world(app.world_mut(), theme).unwrap());
        assert_eq!(*app.world().resource::<WidgetryTheme>(), theme);
        assert_eq!(
            GalleryTheme::from_colors(&theme),
            Some(GalleryTheme::KamuriViolet)
        );
        assert!(!WidgetryTheme::set_in_world(app.world_mut(), theme).unwrap());
    }

    #[test]
    fn gallery_configuration_restores_the_application_owned_palette() {
        let config = crate::config::AppConfig {
            theme: GalleryTheme::KamuriViolet,
            ..default()
        };
        let serialized = ron::ser::to_string(&config).unwrap();
        let restored: crate::config::AppConfig = ron::from_str(&serialized).unwrap();
        assert_eq!(restored.theme, GalleryTheme::KamuriViolet);
        assert_eq!(restored.theme.colors(), kamuri_violet::KAMURI_VIOLET_THEME);
    }
}
