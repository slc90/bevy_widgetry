use crate::assets::GalleryIcon;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry::icon::WidgetryIcon;
use bevy_widgetry::list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};

/// 示例 model 的业务内容，与 UI hierarchy 分开存活。
pub(crate) struct ComboBoxDemoItem {
    /// 可选文本，纯 icon 示例不提供文本。
    label: Option<&'static str>,
    /// Gallery 自有 icon，纯文本示例不提供 icon。
    icon: Option<GalleryIcon>,
}

/// 保存三个独立 model source；disabled 示例共享文本 model。
#[derive(Resource)]
pub(crate) struct ComboBoxDemoSources(pub(crate) [Entity; 3]);

/// 为现有展示完成新构造 contract 的必要接线。
pub(crate) struct ComboBoxDemoPlugin;

/// 区分 root 的示例名称，selection 通过 model-local id 表达。
#[derive(Component)]
struct ComboBoxDemo(&'static str);

/// 保留原四种示例；Field projection 待后续阶段接入。
pub(crate) fn scene(sources: [Entity; 3]) -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Row, column_gap: px(16), flex_wrap: FlexWrap::NoWrap }
        Children [
            (combo(sources[0], "Text")),
            (combo(sources[1], "Icon + Text")),
            (combo(sources[2], "Icon")),
            (combo(sources[0], "Disabled") InteractionDisabled),
        ]
    }
}

/// 使用同一个业务 renderer 构造列表，保留 Gallery 内容的自主字体与 icon 配置。
fn combo(source: Entity, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryComboBox::<ComboBoxDemoItem> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, item: &ComboBoxDemoItem| {
                let icon = item.icon.map(demo_icon);
                let text = item.label.map(|label| bsn! { Text(label) });
                bsn_list![(Node { align_items: AlignItems::Center, column_gap: px(6) } Children [{bsn_list![icon, text]}])]
            })},
        }
        template(move |_| Ok(ComboBoxDemo(name)))
        on(on_selection_changed)
        Node { width: px(200), flex_shrink: 0.0 }
    }
}

/// root 通知使用 stable id；初始化与程序化改选仍然静默。
fn on_selection_changed(
    event: On<ValueChange<WidgetryListItemId>>,
    demos: Query<&ComboBoxDemo, Without<InteractionDisabled>>,
) {
    if let Ok(demo) = demos.get(event.source) {
        info!(demo = demo.0, item_id = ?event.value, "选择 ComboBox 示例项");
    }
}

/// 使用 Gallery 自有 asset，并继承 wrapper 的 foreground color。
fn demo_icon(icon: GalleryIcon) -> impl Scene {
    bsn! {
        @WidgetryIcon { @path: {icon.path()}, @max_size: {Some(UVec2::new(16, 16))} }
        Node { width: px(16), height: px(16) }
    }
}

impl Plugin for ComboBoxDemoPlugin {
    fn build(&self, app: &mut App) {
        app.register_widgetry_combo_box::<ComboBoxDemoItem>();
        let text = ["Apple", "Banana", "Orange"].map(|label| ComboBoxDemoItem {
            label: Some(label),
            icon: None,
        });
        let icon_text = [
            (GalleryIcon::ButtonStar, "Star"),
            (GalleryIcon::Logo, "Logo"),
        ]
        .map(|(icon, label)| ComboBoxDemoItem {
            label: Some(label),
            icon: Some(icon),
        });
        let icons = [GalleryIcon::ButtonStar, GalleryIcon::Logo].map(|icon| ComboBoxDemoItem {
            label: None,
            icon: Some(icon),
        });
        let sources = [Vec::from(text), Vec::from(icon_text), Vec::from(icons)].map(|items| {
            let mut model = WidgetryListModel::default();
            for item in items {
                model.push(item);
            }
            app.world_mut().spawn(model).id()
        });
        app.insert_resource(ComboBoxDemoSources(sources));
    }
}
