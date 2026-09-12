use super::{ComboBoxDropdownIcon, ComboBoxFieldText, ComboBoxOptions};
use crate::headless::{ComboBox, ComboBoxField, ComboBoxOption, ComboBoxPopup};
use bevy::{app::Propagate, picking::hover::Hovered, prelude::*};
use bevy_widgetry_core::ForegroundColor;
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 样式层独立保存异常边沿，不要求触发交互或主题变更才能发现结构缺失。
#[derive(Component, Default, Debug)]
pub(super) struct StyledComboBoxDiagnostics {
    /// 已报告的首个样式异常；基础层级不完整时保留，不误报恢复。
    reported: Option<StyleFault>,
}

/// 描述异常所属节点和失效职责，正文保持中文，实体通过结构化字段定位。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StyleFault {
    /// 实际缺失组件或子节点的实体。
    part: Entity,
    /// 失效的样式职责。
    reason: &'static str,
}

/// 区分基础层尚不可检查与样式层正常，防止基础层损坏时错误地报告样式恢复。
enum StyleHealth {
    Unavailable,
    Checked(Option<StyleFault>),
}

/// 在 Update 的层级构造命令提交后检查，状态随根实体销毁，不为销毁输出恢复。
pub(super) fn diagnose_styled_combo_boxes(world: &mut World) {
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, With<StyledComboBoxDiagnostics>>()
        .iter(world)
        .collect();
    for entity in roots {
        let StyleHealth::Checked(fault) = inspect_style(world, entity) else {
            continue;
        };
        if let Some(mut diagnostics) = world.get_mut::<StyledComboBoxDiagnostics>(entity)
            && diagnostics.reported != fault
        {
            match fault {
                Some(error) => {
                    widgetry_error!(?entity, part = ?error.part, error = error.reason, "StyledComboBox 内部样式结构异常")
                }
                None => widgetry_info!(?entity, "StyledComboBox 内部样式结构恢复正常"),
            }
            diagnostics.reported = fault;
        }
    }
}

/// 只检查本层拥有的文本和样式数据；Field/Popup/Option 的基础身份由 headless 诊断负责。
fn inspect_style(world: &World, root: Entity) -> StyleHealth {
    let fault = |part, reason| StyleHealth::Checked(Some(StyleFault { part, reason }));
    let Some(labels) = world.get::<ComboBoxOptions>(root) else {
        return fault(root, "选项文本数据缺失");
    };
    if world.get::<ComboBox>(root).is_none() || world.get::<Node>(root).is_none() {
        return fault(root, "根节点必需组件缺失");
    }
    if labels.0.is_empty() {
        return fault(root, "内部选项文本为空");
    }
    let Some(field) = child_with::<ComboBoxField>(world, root) else {
        return StyleHealth::Unavailable;
    };
    let Some(popup) = child_with::<ComboBoxPopup>(world, root) else {
        return StyleHealth::Unavailable;
    };
    if !styled_node_is_complete(world, field) || world.get::<BorderColor>(field).is_none() {
        return fault(field, "输入区域样式组件缺失");
    }
    if !unique_text_child::<ComboBoxFieldText>(world, field) {
        return fault(field, "当前选项文本节点缺失或重复");
    }
    if !unique_text_child::<ComboBoxDropdownIcon>(world, field) {
        return fault(field, "下拉提示图标缺失或重复");
    }
    if world.get::<Node>(popup).is_none()
        || world.get::<BackgroundColor>(popup).is_none()
        || world.get::<BorderColor>(popup).is_none()
        || world.get::<GlobalZIndex>(popup).is_none()
    {
        return fault(popup, "弹层样式组件缺失");
    }
    let Some(options) = world.get::<Children>(popup) else {
        return StyleHealth::Unavailable;
    };
    if options.len() != labels.0.len() {
        return fault(root, "选项与文本数量不一致");
    }
    for (index, option) in options.iter().enumerate() {
        if !world
            .get::<ComboBoxOption>(option)
            .is_some_and(|option| option.index == index)
        {
            return StyleHealth::Unavailable;
        }
        if !styled_node_is_complete(world, option) {
            return fault(option, "选项样式组件缺失");
        }
        if !unique_text_child::<Text>(world, option) {
            return fault(option, "选项文本节点缺失或重复");
        }
    }
    StyleHealth::Checked(None)
}

/// 按私有身份寻找直接子节点，不把调用方的任意文本当成内部部件。
fn child_with<T: Component>(world: &World, parent: Entity) -> Option<Entity> {
    world
        .get::<Children>(parent)?
        .iter()
        .find(|child| world.get::<T>(*child).is_some())
}

/// 必需文本必须唯一且具有 Text；只存在标记或只存在普通子节点均不算恢复。
fn unique_text_child<T: Component>(world: &World, parent: Entity) -> bool {
    let Some(children) = world.get::<Children>(parent) else {
        return false;
    };
    let mut matches = children
        .iter()
        .filter(|child| world.get::<T>(*child).is_some());
    matches
        .next()
        .is_some_and(|child| world.get::<Text>(child).is_some())
        && matches.next().is_none()
}

/// 输入区域和选项共享的可视状态是样式同步能够执行的前提。
fn styled_node_is_complete(world: &World, entity: Entity) -> bool {
    world.get::<Node>(entity).is_some()
        && world.get::<Hovered>(entity).is_some()
        && world.get::<BackgroundColor>(entity).is_some()
        && world.get::<Propagate<ForegroundColor>>(entity).is_some()
}
