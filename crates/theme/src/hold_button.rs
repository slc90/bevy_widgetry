use bevy::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryHoldButtonStateColors {
    pub background: Color,
    pub liquid: Color,
    pub wave: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetryHoldButtonColors {
    pub normal: WidgetryHoldButtonStateColors,
    pub hovered: WidgetryHoldButtonStateColors,
    pub pressed: WidgetryHoldButtonStateColors,
    pub disabled: WidgetryHoldButtonStateColors,
    pub success_border: Color,
}

#[cfg(feature = "light")]
pub const LIGHT: WidgetryHoldButtonColors = WidgetryHoldButtonColors {
    normal: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(235, 242, 249),
        liquid: Color::srgb_u8(55, 101, 145),
        wave: Color::srgb_u8(100, 145, 180),
        border: Color::srgb_u8(104, 132, 155),
        foreground: Color::srgb_u8(20, 38, 54),
    },
    hovered: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(242, 249, 255),
        liquid: Color::srgb_u8(55, 101, 145),
        wave: Color::srgb_u8(100, 145, 180),
        border: Color::srgb_u8(111, 139, 162),
        foreground: Color::srgb_u8(20, 38, 54),
    },
    pressed: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(227, 234, 241),
        liquid: Color::srgb_u8(55, 101, 145),
        wave: Color::srgb_u8(100, 145, 180),
        border: Color::srgb_u8(96, 124, 147),
        foreground: Color::srgb_u8(20, 38, 54),
    },
    disabled: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(235, 242, 249),
        liquid: Color::srgb_u8(145, 172, 197),
        wave: Color::srgb_u8(168, 194, 214),
        border: Color::srgb_u8(170, 187, 202),
        foreground: Color::srgb_u8(128, 140, 152),
    },
    success_border: Color::srgb_u8(45, 150, 225),
};

pub const DARK: WidgetryHoldButtonColors = WidgetryHoldButtonColors {
    normal: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(179, 194, 212),
        liquid: Color::srgb_u8(41, 62, 89),
        wave: Color::srgb_u8(87, 116, 150),
        border: Color::srgb_u8(108, 138, 174),
        foreground: Color::srgb_u8(236, 243, 255),
    },
    hovered: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(186, 201, 219),
        liquid: Color::srgb_u8(41, 62, 89),
        wave: Color::srgb_u8(87, 116, 150),
        border: Color::srgb_u8(115, 145, 181),
        foreground: Color::srgb_u8(236, 243, 255),
    },
    pressed: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(171, 186, 204),
        liquid: Color::srgb_u8(41, 62, 89),
        wave: Color::srgb_u8(87, 116, 150),
        border: Color::srgb_u8(100, 130, 166),
        foreground: Color::srgb_u8(236, 243, 255),
    },
    disabled: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(179, 194, 212),
        liquid: Color::srgb_u8(110, 128, 150),
        wave: Color::srgb_u8(133, 155, 181),
        border: Color::srgb_u8(144, 166, 193),
        foreground: Color::srgb_u8(208, 218, 234),
    },
    success_border: Color::srgb_u8(173, 219, 255),
};

#[cfg(feature = "pink_dream")]
pub const PINK_DREAM: WidgetryHoldButtonColors = WidgetryHoldButtonColors {
    normal: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(252, 234, 243),
        liquid: Color::srgb_u8(177, 94, 137),
        wave: Color::srgb_u8(217, 145, 181),
        border: Color::srgb_u8(211, 151, 184),
        foreground: Color::srgb_u8(61, 30, 49),
    },
    hovered: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(255, 241, 250),
        liquid: Color::srgb_u8(177, 94, 137),
        wave: Color::srgb_u8(217, 145, 181),
        border: Color::srgb_u8(218, 158, 191),
        foreground: Color::srgb_u8(61, 30, 49),
    },
    pressed: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(244, 226, 235),
        liquid: Color::srgb_u8(177, 94, 137),
        wave: Color::srgb_u8(217, 145, 181),
        border: Color::srgb_u8(203, 143, 176),
        foreground: Color::srgb_u8(61, 30, 49),
    },
    disabled: WidgetryHoldButtonStateColors {
        background: Color::srgb_u8(252, 234, 243),
        liquid: Color::srgb_u8(214, 164, 190),
        wave: Color::srgb_u8(234, 190, 212),
        border: Color::srgb_u8(232, 192, 214),
        foreground: Color::srgb_u8(156, 132, 146),
    },
    success_border: Color::srgb_u8(245, 171, 212),
};
