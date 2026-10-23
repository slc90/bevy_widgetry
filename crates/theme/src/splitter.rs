use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySplitterColors {
    pub normal: Color,
    pub hovered: Color,
    pub dragging: Color,
    pub disabled: Color,
}

pub const LIGHT: WidgetrySplitterColors = WidgetrySplitterColors {
    normal: light::BORDER,
    hovered: light::PRIMARY_HOVER,
    dragging: light::PRIMARY,
    disabled: light::SUBTLE_BORDER,
};
pub const DARK: WidgetrySplitterColors = WidgetrySplitterColors {
    normal: dark::BORDER,
    hovered: dark::PRIMARY_HOVER,
    dragging: dark::PRIMARY,
    disabled: dark::SUBTLE_BORDER,
};
pub const PINK_DREAM: WidgetrySplitterColors = WidgetrySplitterColors {
    normal: pink_dream::BORDER,
    hovered: pink_dream::PRIMARY_HOVER,
    dragging: pink_dream::PRIMARY,
    disabled: pink_dream::SUBTLE_BORDER,
};
pub const KAMURI_VIOLET: WidgetrySplitterColors = WidgetrySplitterColors {
    normal: kamuri_violet::BORDER,
    hovered: kamuri_violet::PRIMARY_HOVER,
    dragging: kamuri_violet::PRIMARY,
    disabled: kamuri_violet::SUBTLE_BORDER,
};
