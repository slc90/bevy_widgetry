use crate::common::{dark, kamuri_violet, light, pink_dream};
use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySliderStateColors {
    pub track: Color,
    pub fill: Color,
    pub thumb: Color,
    pub focus_ring: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetrySliderColors {
    pub normal: WidgetrySliderStateColors,
    pub hovered: WidgetrySliderStateColors,
    pub pressed: WidgetrySliderStateColors,
    pub disabled: WidgetrySliderStateColors,
}

pub const LIGHT: WidgetrySliderColors = WidgetrySliderColors {
    normal: WidgetrySliderStateColors {
        track: light::BORDER,
        fill: light::PRIMARY,
        thumb: light::PRIMARY,
        focus_ring: light::PRIMARY,
    },
    hovered: WidgetrySliderStateColors {
        track: light::BORDER,
        fill: light::PRIMARY_HOVER,
        thumb: light::PRIMARY_HOVER,
        focus_ring: light::PRIMARY_HOVER,
    },
    pressed: WidgetrySliderStateColors {
        track: light::BORDER,
        fill: light::PRIMARY_PRESSED,
        thumb: light::PRIMARY_PRESSED,
        focus_ring: light::PRIMARY_PRESSED,
    },
    disabled: WidgetrySliderStateColors {
        track: light::DISABLED_BORDER,
        fill: light::DISABLED_TEXT,
        thumb: light::DISABLED_TEXT,
        focus_ring: Color::NONE,
    },
};

pub const DARK: WidgetrySliderColors = WidgetrySliderColors {
    normal: WidgetrySliderStateColors {
        track: dark::BORDER,
        fill: dark::PRIMARY,
        thumb: dark::PRIMARY,
        focus_ring: dark::PRIMARY,
    },
    hovered: WidgetrySliderStateColors {
        track: dark::BORDER,
        fill: dark::PRIMARY_HOVER,
        thumb: dark::PRIMARY_HOVER,
        focus_ring: dark::PRIMARY_HOVER,
    },
    pressed: WidgetrySliderStateColors {
        track: dark::BORDER,
        fill: dark::PRIMARY_PRESSED,
        thumb: dark::PRIMARY_PRESSED,
        focus_ring: dark::PRIMARY_PRESSED,
    },
    disabled: WidgetrySliderStateColors {
        track: dark::DISABLED_BORDER,
        fill: dark::DISABLED_TEXT,
        thumb: dark::DISABLED_TEXT,
        focus_ring: Color::NONE,
    },
};

pub const PINK_DREAM: WidgetrySliderColors = WidgetrySliderColors {
    normal: WidgetrySliderStateColors {
        track: pink_dream::BORDER,
        fill: pink_dream::PRIMARY,
        thumb: pink_dream::PRIMARY,
        focus_ring: pink_dream::PRIMARY,
    },
    hovered: WidgetrySliderStateColors {
        track: pink_dream::BORDER,
        fill: pink_dream::PRIMARY_HOVER,
        thumb: pink_dream::PRIMARY_HOVER,
        focus_ring: pink_dream::PRIMARY_HOVER,
    },
    pressed: WidgetrySliderStateColors {
        track: pink_dream::BORDER,
        fill: pink_dream::PRIMARY_PRESSED,
        thumb: pink_dream::PRIMARY_PRESSED,
        focus_ring: pink_dream::PRIMARY_PRESSED,
    },
    disabled: WidgetrySliderStateColors {
        track: pink_dream::DISABLED_BORDER,
        fill: pink_dream::DISABLED_TEXT,
        thumb: pink_dream::DISABLED_TEXT,
        focus_ring: Color::NONE,
    },
};

pub const KAMURI_VIOLET: WidgetrySliderColors = WidgetrySliderColors {
    normal: WidgetrySliderStateColors {
        track: kamuri_violet::BORDER,
        fill: kamuri_violet::PRIMARY,
        thumb: kamuri_violet::PRIMARY,
        focus_ring: kamuri_violet::PRIMARY,
    },
    hovered: WidgetrySliderStateColors {
        track: kamuri_violet::BORDER,
        fill: kamuri_violet::PRIMARY_HOVER,
        thumb: kamuri_violet::PRIMARY_HOVER,
        focus_ring: kamuri_violet::PRIMARY_HOVER,
    },
    pressed: WidgetrySliderStateColors {
        track: kamuri_violet::BORDER,
        fill: kamuri_violet::PRIMARY_PRESSED,
        thumb: kamuri_violet::PRIMARY_PRESSED,
        focus_ring: kamuri_violet::PRIMARY_PRESSED,
    },
    disabled: WidgetrySliderStateColors {
        track: kamuri_violet::DISABLED_BORDER,
        fill: kamuri_violet::DISABLED_TEXT,
        thumb: kamuri_violet::DISABLED_TEXT,
        focus_ring: Color::NONE,
    },
};
