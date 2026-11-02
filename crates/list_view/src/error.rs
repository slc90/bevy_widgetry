use crate::{WidgetryListItemId, WidgetryListKey};
use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
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
    MissingRuntimePart {
        view: Entity,
        entity: Entity,
        component: &'static str,
    },
    MissingRowState {
        view: Entity,
        source: Entity,
        key: Box<ListKeySnapshot>,
        index: usize,
        row: Entity,
        revision: Option<u64>,
        component: &'static str,
    },
    UnavailableRow {
        view: Entity,
        source: Entity,
        key: ListKeySnapshot,
        index: usize,
    },
    DuplicateKey {
        view: Entity,
        source: Entity,
        key: ListKeySnapshot,
        index: usize,
        previous_index: usize,
    },
    TotalHeight {
        view: Entity,
        source: Entity,
        len: usize,
        item_height: f32,
    },
    InvalidColorTarget {
        entity: Entity,
    },
    ManagedColor {
        entity: Entity,
        owner: Entity,
    },
    Core {
        source: CoreError,
    },
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
            Self::MissingRuntimePart { .. } => "ListView 缺失运行部件或必需输出",
            Self::MissingRowState { .. } => "ListView 行缺失身份或渲染 revision",
            Self::UnavailableRow { .. } => "ListView 数据源行不可用",
            Self::DuplicateKey { .. } => "ListView 数据源投影身份重复",
            Self::TotalHeight { .. } => "ListView 总高度必须为有限值",
            Self::InvalidColorTarget { .. } => "ListView 颜色目标不存在或类型不匹配",
            Self::ManagedColor { .. } => "ListView 托管部件颜色不可直接修改",
            Self::Core { .. } => "ListView 底层操作失败",
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
impl Error for ListError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeFault<K: WidgetryListKey> {
    Source {
        source: Entity,
        reason: ListSourceRejection,
    },
    Part {
        entity: Entity,
        component: &'static str,
    },
    RowState {
        source: Entity,
        key: K,
        index: usize,
        row: Entity,
        revision: Option<u64>,
        component: &'static str,
    },
    Row {
        source: Entity,
        key: K,
        index: usize,
    },
    Index {
        source: Entity,
        index: usize,
        len: usize,
    },
    Duplicate {
        source: Entity,
        key: K,
        index: usize,
        previous_index: usize,
    },
    Height {
        source: Entity,
        len: usize,
        height_bits: u32,
    },
}
impl<K: WidgetryListKey> RuntimeFault<K> {
    pub(crate) fn into_error<T>(self, view: Entity) -> ListError {
        match self {
            Self::Source { source, reason } => ListError::InvalidSource {
                view,
                source,
                item_type: std::any::type_name::<T>(),
                key_type: std::any::type_name::<K>(),
                reason,
            },
            Self::Part { entity, component } => ListError::MissingRuntimePart {
                view,
                entity,
                component,
            },
            Self::RowState {
                source,
                key,
                index,
                row,
                revision,
                component,
            } => ListError::MissingRowState {
                view,
                source,
                key: Box::new(ListKeySnapshot::new(key)),
                index,
                row,
                revision,
                component,
            },
            Self::Row { source, key, index } => ListError::UnavailableRow {
                view,
                source,
                key: ListKeySnapshot::new(key),
                index,
            },
            Self::Index { source, index, len } => ListError::InvalidIndex {
                view,
                source,
                index,
                len,
                space: ListIndexSpace::Display,
            },
            Self::Duplicate {
                source,
                key,
                index,
                previous_index,
            } => ListError::DuplicateKey {
                view,
                source,
                key: ListKeySnapshot::new(key),
                index,
                previous_index,
            },
            Self::Height {
                source,
                len,
                height_bits,
            } => ListError::TotalHeight {
                view,
                source,
                len,
                item_height: f32::from_bits(height_bits),
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct StyleFault {
    pub(crate) entity: Entity,
    pub(crate) component: &'static str,
}

pub(crate) fn core(error: CoreFailure, view: Entity, operation: &'static str) -> ListFailure {
    error
        .map_kind(|source| ListError::Core { source })
        .with_context(ErrorFrame::new(operation).field("view", view))
}

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
