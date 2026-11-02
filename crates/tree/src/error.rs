use crate::{WidgetryTreeChildrenState, WidgetryTreeMove};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::WidgetryFailure;
use std::{error::Error, fmt};

pub type TreeFailure = WidgetryFailure<TreeError>;

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
            Self::Move { rejection } => Some(rejection.as_ref()),
            Self::List { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}
