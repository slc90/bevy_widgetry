use super::{
    bar::{TitleBar, TitleBarContent, TitleBarDragArea, WindowControls},
    close::CloseButton,
    maximize::MaximizeButton,
    minimize::MinimizeButton,
    resize::{WindowResizeArea, WindowResizeHandle},
};
use crate::window_root::{WindowContent, WindowRoot};
use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Button};
use bevy_widgetry_core::icon::Icon;
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 仅成功绑定的窗口接受运行时诊断；可选按钮按构造结果保留预期，不缓存部件实体。
#[derive(Component)]
struct WindowDiagnostics {
    /// 创建时实际启用的可选最小化按钮。
    minimize: bool,
    /// 创建时实际启用的可选最大化按钮。
    maximize: bool,
    /// 最近已报告的结构异常，重复帧不重复记录。
    reported: Option<WindowFault>,
}

/// 记录失效的内部部件及职责，以根实体聚合持续故障。
#[derive(Clone, Copy, PartialEq, Eq)]
struct WindowFault {
    /// 结构缺失或组件不全的所属节点。
    part: Entity,
    /// 中文诊断原因，不包含动态实体信息。
    reason: &'static str,
}

/// 在场景展开和公开参数校验后记住可选结构；修复 Root 时不清除此前的异常边沿。
pub(crate) fn register_window_diagnostics(world: &mut World, root: Entity) {
    if world.get::<WindowDiagnostics>(root).is_some() {
        return;
    }
    let controls = single_child::<TitleBar>(world, root)
        .and_then(|bar| single_child::<WindowControls>(world, bar));
    let minimize =
        controls.is_some_and(|controls| single_child::<MinimizeButton>(world, controls).is_some());
    let maximize =
        controls.is_some_and(|controls| single_child::<MaximizeButton>(world, controls).is_some());
    world.entity_mut(root).insert(WindowDiagnostics {
        minimize,
        maximize,
        reported: None,
    });
}

/// 在关闭清理之后检查，正常创建/关闭与调用方无效绑定不会生成错误或恢复日志。
pub(super) fn diagnose_windows(world: &mut World) {
    let roots: Vec<_> = world
        .query::<(Entity, &WindowDiagnostics)>()
        .iter(world)
        .map(|(entity, state)| (entity, state.minimize, state.maximize))
        .collect();
    for (entity, minimize, maximize) in roots {
        let fault = inspect_window(world, entity, minimize, maximize).err();
        if let Some(mut state) = world.get_mut::<WindowDiagnostics>(entity)
            && state.reported != fault
        {
            match fault {
                Some(error) => {
                    widgetry_error!(?entity, part = ?error.part, error = error.reason, "Window 内部结构异常")
                }
                None => widgetry_info!(?entity, "Window 内部结构恢复正常"),
            }
            state.reported = fault;
        }
    }
}

/// 按库定义的直接父子关系检查，不会把嵌套窗口或调用方插槽内容算作本窗口部件。
fn inspect_window(
    world: &World,
    root: Entity,
    minimize: bool,
    maximize: bool,
) -> Result<(), WindowFault> {
    let fault = |part, reason| WindowFault { part, reason };
    if world.get::<WindowRoot>(root).is_none()
        || world.get::<Node>(root).is_none()
        || world.get::<BackgroundColor>(root).is_none()
        || world.get::<BorderColor>(root).is_none()
    {
        return Err(fault(root, "根节点必需组件缺失"));
    }
    let bar = single_child::<TitleBar>(world, root).ok_or(fault(root, "标题栏缺失或重复"))?;
    let content =
        single_child::<WindowContent>(world, root).ok_or(fault(root, "内容容器缺失或重复"))?;
    let resize =
        single_child::<WindowResizeArea>(world, root).ok_or(fault(root, "缩放区域缺失或重复"))?;
    let drag =
        single_child::<TitleBarDragArea>(world, bar).ok_or(fault(bar, "拖动区域缺失或重复"))?;
    let title =
        single_child::<TitleBarContent>(world, bar).ok_or(fault(bar, "标题内容容器缺失或重复"))?;
    let controls =
        single_child::<WindowControls>(world, bar).ok_or(fault(bar, "按钮容器缺失或重复"))?;
    for part in [bar, content, resize, drag, title, controls] {
        if world.get::<Node>(part).is_none() {
            return Err(fault(part, "内部布局组件缺失"));
        }
    }
    if world.get::<BorderColor>(bar).is_none() {
        return Err(fault(bar, "标题栏边框组件缺失"));
    }
    let close = single_child::<CloseButton>(world, controls)
        .ok_or(fault(controls, "关闭按钮缺失或重复"))?;
    inspect_button(world, close)?;
    if minimize {
        let button = single_child::<MinimizeButton>(world, controls)
            .ok_or(fault(controls, "最小化按钮缺失或脱离所属窗口"))?;
        inspect_button(world, button)?;
    }
    if maximize {
        let button = single_child::<MaximizeButton>(world, controls)
            .ok_or(fault(controls, "最大化按钮缺失或脱离所属窗口"))?;
        inspect_button(world, button)?;
    }
    let handles = world
        .get::<Children>(resize)
        .ok_or(fault(resize, "缩放手柄缺失"))?;
    if handles.len() != 8 {
        return Err(fault(resize, "缩放手柄数量异常"));
    }
    for handle in handles.iter() {
        if world.get::<WindowResizeHandle>(handle).is_none() || world.get::<Node>(handle).is_none()
        {
            return Err(fault(handle, "缩放手柄必需组件缺失"));
        }
    }
    Ok(())
}

/// 已确认的系统按钮必须有交互状态、样式和唯一图标；图标加载等待由 Icon 自身处理。
fn inspect_button(world: &World, button: Entity) -> Result<(), WindowFault> {
    if world.get::<Button>(button).is_none()
        || world.get::<Hovered>(button).is_none()
        || world.get::<Node>(button).is_none()
        || world.get::<BackgroundColor>(button).is_none()
    {
        return Err(WindowFault {
            part: button,
            reason: "系统按钮必需组件缺失",
        });
    }
    if single_child::<Icon>(world, button).is_none() {
        return Err(WindowFault {
            part: button,
            reason: "系统按钮图标缺失或重复",
        });
    }
    Ok(())
}

/// 按直接子节点身份确认唯一部件，等价替换实体也能够被判定为恢复。
fn single_child<T: Component>(world: &World, parent: Entity) -> Option<Entity> {
    let mut matches = world
        .get::<Children>(parent)?
        .iter()
        .filter(|child| world.get::<T>(*child).is_some());
    let child = matches.next()?;
    matches.next().is_none().then_some(child)
}
