use bevy_widgetry::theme::{
    WIDGETRY_DARK_THEME, WIDGETRY_KAMURI_VIOLET_THEME, WIDGETRY_LIGHT_THEME,
    WIDGETRY_PINK_DREAM_THEME, WidgetryTheme,
};
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
            Self::KamuriViolet => WIDGETRY_KAMURI_VIOLET_THEME,
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
