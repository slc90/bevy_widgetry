use crate::assets::GalleryIcon;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::combo_box::{WidgetryComboBox, WidgetryComboBoxOptionFactory};
use bevy_widgetry::icon::Icon;

/// 同排展示文本、图文、纯图标和root禁用四种内容组合，所有控件保持相同宽度。
pub(crate) fn scene() -> impl Scene {
    let text = ["Apple", "Banana", "Orange"]
        .into_iter()
        .map(|label| WidgetryComboBoxOptionFactory::new(move || bsn_list![Text(label)]))
        .collect::<Vec<_>>();
    let icon_text = [
        (GalleryIcon::ButtonStar, "Star"),
        (GalleryIcon::Logo, "Logo"),
    ]
    .into_iter()
    .map(|(icon, label)| {
        WidgetryComboBoxOptionFactory::new(move || {
            bsn_list![
                (Node { align_items: AlignItems::Center, column_gap: px(6) }
                    Children [demo_icon(icon), Text(label)]),
            ]
        })
    })
    .collect::<Vec<_>>();
    let icons = [GalleryIcon::ButtonStar, GalleryIcon::Logo]
        .into_iter()
        .map(|icon| WidgetryComboBoxOptionFactory::new(move || bsn_list![demo_icon(icon)]))
        .collect::<Vec<_>>();
    let disabled = text.clone();
    bsn! {
        Node { flex_direction: FlexDirection::Row, column_gap: px(16), flex_wrap: FlexWrap::NoWrap }
        Children [
            (@WidgetryComboBox { @options: {text} } Node { width: px(200), flex_shrink: 0.0 }),
            (@WidgetryComboBox { @options: {icon_text} } Node { width: px(200), flex_shrink: 0.0 }),
            (@WidgetryComboBox { @options: {icons} } Node { width: px(200), flex_shrink: 0.0 }),
            (@WidgetryComboBox { @options: {disabled} } InteractionDisabled Node { width: px(200), flex_shrink: 0.0 }),
        ]
    }
}

/// 复用 Gallery 自有资源，Field 与列表中的图标都继承对应 wrapper 前景色。
fn demo_icon(icon: GalleryIcon) -> impl Scene {
    bsn! {
        @Icon { @path: {icon.path()}, @max_size: {Some(UVec2::new(16, 16))} }
        Node { width: px(16), height: px(16) }
    }
}
