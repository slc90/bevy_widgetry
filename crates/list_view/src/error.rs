use crate::{WidgetryListItemId, WidgetryListKey};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{
    any::{Any, TypeId},
    error::Error,
    fmt,
};

pub type ListFailure = WidgetryFailure<ListError>;

trait CapturedKey: fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}
impl<K: WidgetryListKey> CapturedKey for K {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct ListKeySnapshot {
    pub type_name: &'static str,
    value: Box<dyn CapturedKey>,
}
impl ListKeySnapshot {
    pub(crate) fn new<K: WidgetryListKey>(key: K) -> Self {
        Self {
            type_name: std::any::type_name::<K>(),
            value: Box::new(key),
        }
    }
    pub fn downcast_ref<K: 'static>(&self) -> Option<&K> {
        self.value.as_any().downcast_ref()
    }
}
impl fmt::Debug for ListKeySnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListKeySnapshot")
            .field("type_name", &self.type_name)
            .field("value", &self.value)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKeyRejection {
    MissingInSource,
    MissingInProjection,
    MissingInOrder,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListIndexSpace {
    Model,
    Display,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListSourceRejection {
    Unregistered,
    Unavailable,
    CannotEnumerate,
    MissingModel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListConfigurationRejection {
    MissingSource,
    MissingRenderer,
    InvalidItemHeight,
}

#[derive(Debug)]
pub enum ListError {
    RevisionExhausted {
        index: usize,
        len: usize,
        item_id: WidgetryListItemId,
        revision: u64,
        next_id: u64,
    },
    IdExhausted {
        index: usize,
        len: usize,
        item_id: WidgetryListItemId,
        next_id: u64,
    },
    InsertionIndex {
        index: usize,
        len: usize,
        next_id: u64,
    },
    InvalidView {
        view: Entity,
        item_type: &'static str,
        key_type: &'static str,
    },
    InvalidSource {
        view: Entity,
        source: Entity,
        item_type: &'static str,
        key_type: &'static str,
        reason: ListSourceRejection,
    },
    MissingPart {
        view: Entity,
        component: &'static str,
    },
    InvalidKey {
        view: Entity,
        source: Entity,
        key: ListKeySnapshot,
        reason: ListKeyRejection,
    },
    InvalidIndex {
        view: Entity,
        source: Entity,
        index: usize,
        len: usize,
        space: ListIndexSpace,
    },
    SelectionCardinality {
        view: Entity,
        source: Entity,
        selected: Vec<ListKeySnapshot>,
    },
    MissingPlugin {
        item_type: &'static str,
        key_type: &'static str,
        adapter_type: &'static str,
    },
    AdapterConflict {
        item_type: &'static str,
        key_type: &'static str,
        requested: TypeId,
        existing: TypeId,
        requested_type: &'static str,
    },
    Configuration {
        view: Entity,
        source: Entity,
        item_height: f32,
        renderer_missing: bool,
        reason: ListConfigurationRejection,
    },
    MissingRenderer {
        item_type: &'static str,
    },
}
impl fmt::Display for ListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RevisionExhausted { .. } => "ListModel 内容 revision 已耗尽",
            Self::IdExhausted { .. } => "ListModel item id 已耗尽",
            Self::InsertionIndex { .. } => "ListModel 插入 index 越界",
            Self::InvalidView { .. } => "ListView 不存在或类型不匹配",
            Self::InvalidSource { .. } => "ListView 数据源不可用或类型不匹配",
            Self::MissingPart { .. } => "ListView 缺失必需状态",
            Self::InvalidKey { .. } => "ListView 请求的 Key 已失效或不可用",
            Self::InvalidIndex { .. } => "ListView 请求的 index 越界",
            Self::SelectionCardinality { .. } => "ListView 单选模式不能选择多个 ID",
            Self::MissingPlugin { .. } => "注册 ListView 数据源前必须安装 WidgetryListViewPlugin",
            Self::AdapterConflict { .. } => "ListView 数据源适配器重复且不匹配",
            Self::Configuration { .. } => "ListView 构造配置无效",
            Self::MissingRenderer { .. } => "ListView renderer 缺失",
        })
    }
}
impl Error for ListError {}

#[track_caller]
pub(crate) fn failure(kind: ListError, operation: &'static str) -> ListFailure {
    let frame = ErrorFrame::new(operation).field("failure", format_args!("{kind:?}"));
    ListFailure::new(kind, frame)
}
#[track_caller]
pub(crate) fn invalid_view<T, K>(view: Entity, operation: &'static str) -> ListFailure {
    failure(
        ListError::InvalidView {
            view,
            item_type: std::any::type_name::<T>(),
            key_type: std::any::type_name::<K>(),
        },
        operation,
    )
}
#[track_caller]
pub(crate) fn invalid_source<T, K>(
    view: Entity,
    source: Entity,
    reason: ListSourceRejection,
    operation: &'static str,
) -> ListFailure {
    failure(
        ListError::InvalidSource {
            view,
            source,
            item_type: std::any::type_name::<T>(),
            key_type: std::any::type_name::<K>(),
            reason,
        },
        operation,
    )
}
#[track_caller]
pub(crate) fn missing(
    view: Entity,
    component: &'static str,
    operation: &'static str,
) -> ListFailure {
    failure(ListError::MissingPart { view, component }, operation)
}
#[track_caller]
pub(crate) fn invalid_key<K: WidgetryListKey>(
    view: Entity,
    source: Entity,
    key: K,
    reason: ListKeyRejection,
    operation: &'static str,
) -> ListFailure {
    failure(
        ListError::InvalidKey {
            view,
            source,
            key: ListKeySnapshot::new(key),
            reason,
        },
        operation,
    )
}
