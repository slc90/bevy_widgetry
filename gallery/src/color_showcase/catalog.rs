mod fields;

use super::Toggle;
use crate::gallery::GalleryPage;
use bevy::prelude::*;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::text::WidgetryText;
use bevy_widgetry::theme::{WidgetryTheme, WidgetryThemeMode};
use std::collections::BTreeSet;

struct Catalog {
    fields: Vec<(String, Color)>,
    groups: Vec<(&'static str, Catalog)>,
}

trait ColorFields {
    fn catalog(self) -> Catalog;
}

pub(super) fn rows(
    page: GalleryPage,
    mode: WidgetryThemeMode,
    expanded: &BTreeSet<String>,
) -> Box<dyn SceneList> {
    let WidgetryTheme {
        text,
        icon,
        button,
        check_box,
        radio_group,
        text_field,
        combo_box,
        scroll_area,
        list_view,
        tree,
        table,
        tooltip,
        window,
        message_box,
        file_dialog,
        waveform,
    } = *mode.colors();
    let groups = match page {
        GalleryPage::Initializing => Vec::new(),
        GalleryPage::Button => vec![
            ("button", button.catalog()),
            ("radio_group", radio_group.catalog()),
            ("text", text.catalog()),
            ("icon", icon.catalog()),
        ],
        GalleryPage::CheckBox => vec![("check_box", check_box.catalog())],
        GalleryPage::ComboBox => vec![("combo_box", combo_box.catalog())],
        GalleryPage::ScrollArea => vec![("scroll_area", scroll_area.catalog())],
        GalleryPage::ListView => vec![("list_view", list_view.catalog())],
        GalleryPage::Tree => vec![("tree", tree.catalog())],
        GalleryPage::Table => vec![("table", table.catalog())],
        GalleryPage::TextField => vec![("text_field", text_field.catalog())],
        GalleryPage::Tooltip => vec![("tooltip", tooltip.catalog())],
        GalleryPage::Window => vec![
            ("window", window.catalog()),
            ("message_box", message_box.catalog()),
            ("file_dialog", file_dialog.catalog()),
        ],
        GalleryPage::Waveform => vec![("waveform", waveform.catalog())],
    };
    entries(
        Catalog {
            fields: Vec::new(),
            groups,
        },
        "",
        expanded,
    )
}

fn entries(catalog: Catalog, prefix: &str, expanded: &BTreeSet<String>) -> Box<dyn SceneList> {
    let fields: Vec<_> = catalog
        .fields
        .into_iter()
        .map(|(field, color)| swatch(format!("{prefix}.{field}"), color))
        .collect();
    let groups: Vec<_> = catalog.groups.into_iter().map(|(name, group)| {
        let path = if prefix.is_empty() { name.into() } else { format!("{prefix}.{name}") };
        let opened = expanded.contains(&path);
        let children = opened.then(|| entries(group, &path, expanded));
        bsn! {
            Node { flex_direction: FlexDirection::Column, row_gap: px(4), padding: UiRect::left(px(12)) }
            Children [
                @WidgetryButton template(move |_| Ok(Toggle { path: path.clone() }))
                    Name({format!("ThemeGroup:{path}")})
                    Node { align_self: AlignSelf::Start }
                    on(super::toggle)
                    Children [Text({format!("{} {path}", if opened { "−" } else { "+" })}) WidgetryText]--
                {children}
            ]
        }
    }).collect();
    Box::new(bsn_list! {{ fields }-- { groups }})
}

fn swatch(label: String, color: Color) -> impl Scene {
    let [r, g, b, a] = color.to_srgba().to_u8_array();
    let hex = format!("#{r:02X}{g:02X}{b:02X}{a:02X}");
    let tiles: Vec<_> = (0..8).map(|index| bsn! {
        Node { width: px(8), height: px(8) }
        BackgroundColor({if (index % 4 + index / 4) % 2 == 0 { Color::srgb_u8(220, 220, 220) } else { Color::srgb_u8(140, 140, 140) }})
    }).collect();
    bsn! {
        Name({format!("ThemeColor:{label}")})
        Node { column_gap: px(10), align_items: AlignItems::Center, flex_shrink: 0.0 }
        Children [
            Node { width: px(32), height: px(16), flex_shrink: 0.0, flex_wrap: FlexWrap::Wrap }
                Children [{tiles}-- Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100) } BackgroundColor(color)]--
            Text({format!("{label}  {hex}")}) WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }
        ]
    }
}
