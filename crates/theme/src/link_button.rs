use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryLinkButtonColors {
    pub normal: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub disabled: Color,
}

pub const LIGHT: WidgetryLinkButtonColors = WidgetryLinkButtonColors {
    normal: light::PRIMARY,
    hovered: light::PRIMARY_HOVER,
    pressed: light::PRIMARY_PRESSED,
    disabled: light::DISABLED_TEXT,
};

pub const DARK: WidgetryLinkButtonColors = WidgetryLinkButtonColors {
    normal: dark::PRIMARY,
    hovered: dark::PRIMARY_HOVER,
    pressed: dark::PRIMARY_PRESSED,
    disabled: dark::DISABLED_TEXT,
};

pub const PINK_DREAM: WidgetryLinkButtonColors = WidgetryLinkButtonColors {
    normal: pink_dream::PRIMARY,
    hovered: pink_dream::PRIMARY_HOVER,
    pressed: pink_dream::PRIMARY_PRESSED,
    disabled: pink_dream::DISABLED_TEXT,
};

pub const KAMURI_VIOLET: WidgetryLinkButtonColors = WidgetryLinkButtonColors {
    normal: kamuri_violet::PRIMARY,
    hovered: kamuri_violet::PRIMARY_HOVER,
    pressed: kamuri_violet::PRIMARY_PRESSED,
    disabled: kamuri_violet::DISABLED_TEXT,
};
