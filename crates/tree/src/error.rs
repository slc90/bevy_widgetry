use crate::{WidgetryTreeChildrenState, WidgetryTreeMove};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::WidgetryFailure;
use std::{error::Error, fmt};

pub type TreeFailure = WidgetryFailure<TreeError>;
pub type TreeViewFailure = bevy_widgetry_diagnostics::bevy::WidgetryEcsError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeRendererIdentity {
    pub type_id: std::any::TypeId,
    pub name: &'static str,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeViewPart {
    List,
    State,
    ResolvedPalette,
    ColorState,
    ColorCache,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeViewFault {
    Source {
        view: Entity,
        source: Option<Entity>,
    },
    Part {
        view: Option<Entity>,
        node: Option<Entity>,
        entity: Entity,
        part: TreeViewPart,
    },
    Target {
        view: Entity,
        source: Option<Entity>,
        node: Option<Entity>,
    },
    Depth {
        root: Entity,
        node: Entity,
        depth: usize,
    },
    RendererMissing {
        content: Entity,
        node: Entity,
    },
    RendererAmbiguous {
        content: Entity,
        node: Entity,
        first: TreeRendererIdentity,
        second: TreeRendererIdentity,
    },
    RendererComponent {
        node: Entity,
        component: TreeRendererIdentity,
    },
    ExpanderOwner {
        button: Entity,
        node: Entity,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeMoveReason {
    InvalidView,
    InvalidSource,
    MissingList,
    SelfMove,
    NotInTree {
        node: Entity,
        at: Entity,
    },
    Disabled {
        entity: Entity,
    },
    MissingParent {
        entity: Entity,
    },
    MissingIndex {
        parent: Entity,
        node: Entity,
    },
    UnloadedChildren {
        target: Entity,
        state: WidgetryTreeChildrenState,
    },
    ParentCycle {
        entity: Entity,
    },
    DestinationDetached {
        entity: Entity,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeMoveRejection {
    pub view: Entity,
    pub source: Option<Entity>,
    pub model_root: Option<Entity>,
    pub movement: WidgetryTreeMove,
    pub old_parent: Option<Entity>,
    pub new_parent: Option<Entity>,
    pub reason: TreeMoveReason,
}
impl fmt::Display for TreeMoveRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tree 节点移动被拒绝: {:?}", self.reason)
    }
}
impl Error for TreeMoveRejection {}

#[derive(Debug)]
pub enum TreeError {
    View {
        fault: TreeViewFault,
    },
    RegistryExhausted {
        component: TreeRendererIdentity,
        generation: u64,
    },
    Configuration {
        view: Entity,
        source: Entity,
        indent_width: f32,
    },
    Indent {
        node: Entity,
        depth: u16,
        indent_width: f32,
        padding: f32,
    },
    Loader {
        node: Entity,
        state: Option<WidgetryTreeChildrenState>,
    },
    InvalidColors {
        view: Entity,
    },
    ManagedColors {
        view: Entity,
        owner: Entity,
    },
    Core {
        source: Box<bevy_widgetry_core::diagnostics::CoreError>,
    },
    Button {
        source: Box<bevy_widgetry_button::ButtonError>,
    },
    Move {
        rejection: Box<TreeMoveRejection>,
    },
    InvalidView {
        view: Entity,
    },
    MissingList {
        view: Entity,
        source: Entity,
    },
    List {
        source: Box<bevy_widgetry_list_view::ListError>,
    },
}
impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::View { .. } => "TreeView 运行结构或投影失败",
            Self::RegistryExhausted { .. } => "Tree renderer registry generation 已耗尽",
            Self::Configuration { .. } => "TreeView 必须提供 source 与有限非负 indent_width",
            Self::Indent { .. } => "TreeView 缩进溢出",
            Self::Loader { .. } => "Tree loader 完成目标失效或尚未请求",
            Self::InvalidColors { .. } => "TreeView 颜色目标无效",
            Self::ManagedColors { .. } => "TreeView 托管配色不可直接修改",
            Self::Core { .. } => "TreeView 底层操作失败",
            Self::Button { .. } => "TreeView 展开器配色失败",
            Self::Move { .. } => "Tree 节点移动失败",
            Self::InvalidView { .. } => "TreeView 拖放配置目标无效",
            Self::MissingList { .. } => "TreeView 拖放配置缺少内部列表",
            Self::List { .. } => "TreeView 列表操作失败",
        })
    }
}
impl Error for TreeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source.as_ref()),
            Self::Button { source } => Some(source.as_ref()),
            Self::Move { rejection } => Some(rejection.as_ref()),
            Self::List { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: TreeError, operation: &'static str) -> TreeFailure {
    let frame = bevy_widgetry_diagnostics::ErrorFrame::new(operation)
        .field("failure", format_args!("{kind:?}"));
    TreeFailure::new(kind, frame)
}
#[track_caller]
pub(crate) fn view_failure(fault: TreeViewFault, operation: &'static str) -> TreeViewFailure {
    TreeViewFailure::new(failure(TreeError::View { fault }, operation))
}
pub(crate) fn persistent(
    state: &mut bevy_widgetry_diagnostics::bevy::FailureState<TreeViewFault>,
    fault: TreeViewFault,
    operation: &'static str,
) -> TreeViewFailure {
    persistent_frame(
        state,
        fault,
        bevy_widgetry_diagnostics::ErrorFrame::new(operation),
    )
}
pub(crate) fn persistent_frame(
    state: &mut bevy_widgetry_diagnostics::bevy::FailureState<TreeViewFault>,
    fault: TreeViewFault,
    frame: bevy_widgetry_diagnostics::ErrorFrame,
) -> TreeViewFailure {
    state.failure(fault, || {
        std::sync::Arc::new(TreeFailure::new(
            TreeError::View { fault },
            frame.field("failure", format_args!("{fault:?}")),
        ))
    })
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ProjectionRequest {
    Sync,
    Expand(Entity),
    Collapse(Entity),
    ExpandAll,
    CollapseAll,
}
#[track_caller]
pub(crate) fn projection_failure(
    world: &World,
    view: Entity,
    fault: TreeViewFault,
    request: ProjectionRequest,
) -> TreeViewFailure {
    let target = match request {
        ProjectionRequest::Expand(node) | ProjectionRequest::Collapse(node) => Some(node),
        _ => None,
    };
    let node = if let TreeViewFault::Depth { node, .. } = fault {
        Some(node)
    } else {
        target
    };
    TreeViewFailure::new(TreeFailure::new(
        TreeError::View { fault },
        view_context(world, view, node, view, "tree.project_request")
            .field("request", format_args!("{request:?}"))
            .field("target", format_args!("{target:?}"))
            .field("failure", format_args!("{fault:?}")),
    ))
}
pub(crate) fn list(
    error: bevy_widgetry_list_view::ListFailure,
    world: &World,
    view: Entity,
    node: Option<Entity>,
    list: Entity,
    operation: &'static str,
) -> TreeViewFailure {
    TreeViewFailure::new(
        error
            .map_kind(|source| TreeError::List {
                source: Box::new(source),
            })
            .with_context(view_context(world, view, node, list, operation)),
    )
}
pub(crate) fn view_context(
    world: &World,
    view: Entity,
    node: Option<Entity>,
    part: Entity,
    operation: &'static str,
) -> bevy_widgetry_diagnostics::ErrorFrame {
    let source = world
        .get::<crate::WidgetryTreeView>(view)
        .map(crate::WidgetryTreeView::source);
    let expanded = node.and_then(|node| {
        world
            .get::<crate::WidgetryTreeState>(view)
            .map(|state| state.is_expanded(node))
    });
    bevy_widgetry_diagnostics::ErrorFrame::new(operation)
        .field("view", view)
        .field("source", format_args!("{source:?}"))
        .field("node", format_args!("{node:?}"))
        .field("part", part)
        .field("expanded", format_args!("{expanded:?}"))
        .field(
            "children_state",
            format_args!(
                "{:?}",
                node.and_then(|node| world.get::<crate::WidgetryTreeChildrenState>(node))
            ),
        )
}

// Native errors must reach the handler separately to retain each Severity and payload.
pub(crate) fn dispatch(
    world: &mut bevy::prelude::World,
    mut errors: Vec<bevy::prelude::BevyError>,
) -> Result<(), bevy::prelude::BevyError> {
    if errors.is_empty() {
        return Ok(());
    }
    let first = errors.remove(0);
    for error in errors {
        world.commands().queue(
            move |_: &mut bevy::prelude::World| -> Result<(), bevy::prelude::BevyError> {
                Err(error)
            },
        );
    }
    Err(first)
}
