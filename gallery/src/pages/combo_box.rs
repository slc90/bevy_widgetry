use crate::assets::GalleryIcon;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::combo_box::{WidgetryComboBox, WidgetryComboBoxOptionFactory};
use bevy_widgetry::icon::WidgetryIcon;

/// 标识示例内容组合；所选 option 通过 Widget 公开的 index 区分。
#[derive(Component)]
struct ComboBoxDemo(&'static str);

/// 同排展示文本、图文、纯 icon 和 root disabled 四种内容组合，所有 Widget 保持相同宽度。
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
            (@WidgetryComboBox { @options: {text} } template(|_| Ok(ComboBoxDemo("Text"))) on(on_selection_changed) Node { width: px(200), flex_shrink: 0.0 }),
            (@WidgetryComboBox { @options: {icon_text} } template(|_| Ok(ComboBoxDemo("Icon + Text"))) on(on_selection_changed) Node { width: px(200), flex_shrink: 0.0 }),
            (@WidgetryComboBox { @options: {icons} } template(|_| Ok(ComboBoxDemo("Icon"))) on(on_selection_changed) Node { width: px(200), flex_shrink: 0.0 }),
            (@WidgetryComboBox { @options: {disabled} } template(|_| Ok(ComboBoxDemo("Disabled"))) on(on_selection_changed) InteractionDisabled Node { width: px(200), flex_shrink: 0.0 }),
        ]
    }
}

/// root 的 ValueChange 仅来自实际用户改值；初始化静默，程序化改选应由 Gallery 调用处记录。
fn on_selection_changed(
    event: On<ValueChange<usize>>,
    demos: Query<&ComboBoxDemo, Without<InteractionDisabled>>,
) {
    let Ok(demo) = demos.get(event.source) else {
        return;
    };
    info!(demo = demo.0, index = event.value, "选择 ComboBox 示例项");
}

/// 复用 Gallery 自有 asset，Field 与 list 中的 icon 都继承对应 wrapper 的 foreground color。
fn demo_icon(icon: GalleryIcon) -> impl Scene {
    bsn! {
        @WidgetryIcon { @path: {icon.path()}, @max_size: {Some(UVec2::new(16, 16))} }
        Node { width: px(16), height: px(16) }
    }
}
