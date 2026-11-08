use bevy::prelude::*;

const SAMPLE_FONT_SIZE: f32 = 24.0;
const SAMPLE_GAP: f32 = 8.0;
const SECTION_MARGIN: f32 = 16.0;
const MIXED_TEXT: &str = "中文 English 12345\n你好，世界！😄 🎉 🚀\n❤️ ☀️ ⭐ ✔️\n👍🏽 🇨🇳 🏳️‍🌈";

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("DefaultFontExamples")
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(SAMPLE_GAP),
            margin: UiRect::bottom(px(SECTION_MARGIN)),
            flex_shrink: 0.0,
        }
        Children [
            Text("Default font · Emoji fallback") bevy_widgetry::text::WidgetryText--
            Name("DefaultFontMixedText") Text(MIXED_TEXT)
            bevy_widgetry::text::WidgetryText
            TextFont { font_size: FontSize::Px(SAMPLE_FONT_SIZE) }--
            Name("DefaultFontTintedText") Text(MIXED_TEXT)
            TextFont { font_size: FontSize::Px(SAMPLE_FONT_SIZE) }
            TextColor({Color::srgb_u8(220, 80, 120)})
        ]
    }
}
