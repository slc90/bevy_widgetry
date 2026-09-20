use crate::combo_box::{WidgetryComboBox, WidgetryComboBoxOptionFactory};
use crate::field::ComboBoxField;
use crate::option::{self, ComboBoxOption};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Selected};
use bevy::ui_widgets::{Activate, ListBox};
use bevy_widgetry_core::{ThemeChanged, ThemeMode};
use bevy_widgetry_log::widgetry_error;

/// 内部 ListBox 容器，Visibility 同时作为 open state。
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxPopup;

/// 保持 Field 下方的 absolute positioning、全宽和层级，仅增加 border radius。
pub(crate) fn scene(options: &[WidgetryComboBoxOptionFactory]) -> impl Scene + use<> {
    let rows = options
        .iter()
        .enumerate()
        .map(|(index, factory)| option::scene(index, factory.build()))
        .collect::<Vec<_>>();
    bsn! {
        ComboBoxPopup ListBox Visibility::Hidden GlobalZIndex(100)
        template(|context| Ok(BackgroundColor(context.resource::<ThemeMode>().colors().popup_background)))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().popup_border)))
        Node {
            position_type: PositionType::Absolute,
            left: px(0), top: percent(100), width: percent(100),
            flex_direction: FlexDirection::Column, align_items: AlignItems::Stretch,
            border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(4)),
        }
        Children [{rows}]
    }
}

/// Field 直接 Activate 也必须检查 root 是否 disabled，避免只依赖 Button 镜像的同步时机。
pub(crate) fn handle_field_activate(
    event: On<Activate>,
    fields: Query<&ChildOf, With<ComboBoxField>>,
    roots: Query<(&Children, Has<InteractionDisabled>), With<WidgetryComboBox>>,
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

/// Bevy 0.19.1 没有重选 event，需在 row 上捕获已选 option 的 click；不改变 selection 或阻断 ListBox。
pub(crate) fn handle_reselect(
    event: On<Pointer<Click>>,
    rows: Query<&ChildOf, (With<ComboBoxOption>, With<Selected>)>,
    roots: Query<Has<InteractionDisabled>, With<WidgetryComboBox>>,
    mut popups: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
) {
    let Ok(parent) = rows.get(event.entity) else {
        return;
    };
    let Ok((root, mut visibility)) = popups.get_mut(parent.parent()) else {
        return;
    };
    if roots.get(root.parent()).is_ok_and(|disabled| !disabled) {
        *visibility = Visibility::Hidden;
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
