use crate::combo_box::WidgetryComboBox;
use crate::field::ComboBoxField;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{
    Activate,
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
};
use bevy_widgetry_core::{ThemeChanged, ThemeMode, z_index};
use bevy_widgetry_list_view::{WidgetryListView, WidgetryListViewRenderer};
use bevy_widgetry_log::widgetry_error;

/// ComboBox 专属 Popup wrapper，内部列表交给 ListView。
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxPopup;

/// 使用官方 Popover 在下方或上方放置全宽 list，避免越过 window 边缘。
pub(crate) fn scene<T: Send + Sync + 'static>(
    source: Entity,
    item_height: f32,
    height: f32,
    renderer: WidgetryListViewRenderer<T>,
) -> impl Scene {
    // 将复合 ListView 的 Scene type erase，限制嵌套 Gallery Scene 展开时的 stack 占用。
    let list: Box<dyn Scene> = Box::new(bsn! {
        @WidgetryListView::<T> { @source: source, @item_height: item_height, @renderer: {renderer} }
        Node { width: percent(100), height: px(height) }
    });
    bsn! {
        ComboBoxPopup Visibility::Hidden GlobalZIndex({z_index::POPUP})
        Popover {
            positions: {vec![
                PopoverPlacement { side: PopoverSide::Bottom, align: PopoverAlign::Start, gap: 0.0 },
                PopoverPlacement { side: PopoverSide::Top, align: PopoverAlign::Start, gap: 0.0 },
            ]},
            window_margin: 8.0,
        }
        template(|context| Ok(BackgroundColor(context.resource::<ThemeMode>().colors().popup_background)))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().popup_border)))
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            flex_direction: FlexDirection::Column, align_items: AlignItems::Stretch,
            border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(4)),
        }
        Children [({list})]
    }
}

/// Field 直接 Activate 也必须检查 root 是否 disabled，避免只依赖 Button 镜像的同步时机。
pub(crate) fn handle_field_activate<T: Send + Sync + 'static>(
    event: On<Activate>,
    fields: Query<&ChildOf, With<ComboBoxField>>,
    roots: Query<(&Children, Has<InteractionDisabled>), With<WidgetryComboBox<T>>>,
    mut popups: Query<&mut Visibility, With<ComboBoxPopup>>,
) {
    let Ok(parent) = fields.get(event.entity) else {
        return;
    };
    let Ok((children, disabled)) = roots.get(parent.parent()) else {
        return;
    };
    if disabled {
        return;
    }
    let Some(popup) = children.iter().find(|&child| popups.contains(child)) else {
        widgetry_error!(root = ?parent.parent(), "ComboBox 缺少Popup");
        return;
    };
    if let Ok(mut visibility) = popups.get_mut(popup) {
        *visibility = if *visibility == Visibility::Hidden {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 用原始 pointer 目标判断外部 click，内部任意 children 同样属于 Widget。
pub(crate) fn handle_outside_click(
    event: On<Pointer<Click>>,
    parents: Query<&ChildOf>,
    mut popups: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
) {
    let target = event.original_event_target();
    for (parent, mut visibility) in &mut popups {
        if *visibility == Visibility::Visible
            && target != parent.parent()
            && !parents
                .iter_ancestors(target)
                .any(|ancestor| ancestor == parent.parent())
        {
            *visibility = Visibility::Hidden;
        }
    }
}

/// theme event 立即刷新 Popup 容器，不改变 visibility。
pub(crate) fn refresh_theme(
    event: On<ThemeChanged>,
    mut popups: Query<(&mut BackgroundColor, &mut BorderColor), With<ComboBoxPopup>>,
) {
    for (mut background, mut border) in &mut popups {
        background.0 = event.mode.colors().popup_background;
        *border = BorderColor::all(event.mode.colors().popup_border);
    }
}
