use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{FailureState, WidgetryEcsError},
};
use bevy_widgetry_list_view::{ListError, ListFailure, WidgetryListItemId};
use std::{error::Error, fmt, sync::Arc};

pub type ComboFailure = WidgetryFailure<ComboError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComboPart {
    Children,
    Field,
    FieldContent,
    FieldProjection,
    ButtonContent,
    Popup,
    ListView,
    Selection,
    Item,
    Revision,
    DropdownIcon,
    Style,
    Colors,
    Background,
    Border,
    SearchField,
    SearchRuntime,
    SearchMatcher,
    InputTarget,
    EditableText,
    Clipboard,
    ListProjection,
}

#[derive(Debug)]
pub enum ComboError {
    InvalidRoot {
        root: Entity,
        item_type: &'static str,
    },
    InvalidSource {
        root: Entity,
        source: Entity,
        item_type: &'static str,
    },
    MissingPart {
        root: Entity,
        entity: Entity,
        part: ComboPart,
    },
    MissingList {
        root: Entity,
        source: Entity,
        popup: Option<Entity>,
    },
    ListMapping {
        root: Entity,
        source: Entity,
        list: Entity,
        actual_source: Option<Entity>,
    },
    MissingItem {
        root: Entity,
        source: Entity,
        list: Entity,
        item_id: WidgetryListItemId,
        index: Option<usize>,
        part: ComboPart,
    },
    Configuration {
        root: Entity,
        source: Entity,
        item_height: f32,
        max_visible_items: usize,
        viewport_height: f32,
    },
    InvalidColorTarget {
        root: Entity,
    },
    ManagedColors {
        root: Entity,
        owner: Entity,
    },
    List {
        source: Box<ListError>,
    },
    TextField {
        source: Box<bevy_widgetry_text_field::TextFieldError>,
    },
    Icon {
        source: Box<bevy_widgetry_core::icon::IconError>,
    },
    Button {
        source: Box<bevy_widgetry_button::ButtonError>,
    },
    Core {
        source: CoreError,
    },
}
impl fmt::Display for ComboError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidRoot { .. } => "ComboBox 不存在或类型不匹配",
            Self::InvalidSource { .. } => "ComboBox 数据源不存在或 Model 类型不匹配",
            Self::MissingPart { .. } => "ComboBox 缺失运行部件",
            Self::MissingList { .. } => "ComboBox 内部 ListView 缺失或不可用",
            Self::ListMapping { .. } => "ComboBox 内部 ListView 映射无效",
            Self::MissingItem { .. } => "ComboBox 选中项或渲染数据不存在",
            Self::Configuration { .. } => "ComboBox viewport 高度必须有限且行数非零",
            Self::InvalidColorTarget { .. } => "ComboBox 颜色目标不存在或类型不匹配",
            Self::ManagedColors { .. } => "ComboBox 托管部件颜色不可直接修改",
            Self::List { .. } => "ComboBox 列表操作失败",
            Self::TextField { .. } => "ComboBox 搜索文本操作失败",
            Self::Icon { .. } => "ComboBox 图标操作失败",
            Self::Button { .. } => "ComboBox Field 配色失败",
            Self::Core { .. } => "ComboBox 底层操作失败",
        })
    }
}
impl Error for ComboError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::List { source } => Some(source.as_ref()),
            Self::TextField { source } => Some(source.as_ref()),
            Self::Icon { source } => Some(source.as_ref()),
            Self::Button { source } => Some(source.as_ref()),
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}
#[track_caller]
pub(crate) fn failure(kind: ComboError, operation: &'static str) -> ComboFailure {
    let frame = ErrorFrame::new(operation).field("failure", format_args!("{kind:?}"));
    ComboFailure::new(kind, frame)
}
pub(crate) fn frame(root: Entity, operation: &'static str) -> ErrorFrame {
    ErrorFrame::new(operation).field("combo", root)
}
pub(crate) fn list(error: ListFailure, root: Entity, operation: &'static str) -> ComboFailure {
    error
        .map_kind(|source| ComboError::List {
            source: Box::new(source),
        })
        .with_context(frame(root, operation))
}
pub(crate) fn text(
    error: bevy_widgetry_text_field::TextFieldFailure,
    root: Entity,
    operation: &'static str,
) -> ComboFailure {
    error
        .map_kind(|source| ComboError::TextField {
            source: Box::new(source),
        })
        .with_context(frame(root, operation))
}
pub(crate) fn core(error: CoreFailure, root: Entity, operation: &'static str) -> ComboFailure {
    error
        .map_kind(|source| ComboError::Core { source })
        .with_context(frame(root, operation))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComboFault {
    Part {
        entity: Entity,
        part: ComboPart,
    },
    Source {
        source: Entity,
    },
    List {
        source: Entity,
        popup: Entity,
    },
    Mapping {
        source: Entity,
        list: Entity,
        actual_source: Option<Entity>,
    },
    Item {
        source: Entity,
        list: Entity,
        item_id: WidgetryListItemId,
        index: Option<usize>,
        part: ComboPart,
    },
}
impl ComboFault {
    pub(crate) fn into_error<T>(self, root: Entity) -> ComboError {
        match self {
            Self::Part { entity, part } => ComboError::MissingPart { root, entity, part },
            Self::Source { source } => ComboError::InvalidSource {
                root,
                source,
                item_type: std::any::type_name::<T>(),
            },
            Self::List { source, popup } => ComboError::MissingList {
                root,
                source,
                popup: Some(popup),
            },
            Self::Mapping {
                source,
                list,
                actual_source,
            } => ComboError::ListMapping {
                root,
                source,
                list,
                actual_source,
            },
            Self::Item {
                source,
                list,
                item_id,
                index,
                part,
            } => ComboError::MissingItem {
                root,
                source,
                list,
                item_id,
                index,
                part,
            },
        }
    }
}
pub(crate) fn part<T>(value: Option<T>, entity: Entity, part: ComboPart) -> Result<T, ComboFault> {
    value.ok_or(ComboFault::Part { entity, part })
}

pub(crate) enum RuntimeFailure {
    Fault(ComboFault),
    Domain(ComboFailure),
    External(BevyError),
}
impl From<ComboFault> for RuntimeFailure {
    fn from(value: ComboFault) -> Self {
        Self::Fault(value)
    }
}
impl From<ComboFailure> for RuntimeFailure {
    fn from(value: ComboFailure) -> Self {
        Self::Domain(value)
    }
}
impl RuntimeFailure {
    pub(crate) fn into_bevy_error<T>(self, root: Entity, operation: &'static str) -> BevyError {
        match self {
            Self::Fault(fault) => failure(fault.into_error::<T>(root), operation).into_bevy_error(),
            Self::Domain(error) => error.into_bevy_error(),
            Self::External(error) => error,
        }
    }
}
pub(crate) fn persistent<T>(
    state: &mut FailureState<ComboFault>,
    fault: ComboFault,
    root: Entity,
    operation: &'static str,
) -> WidgetryEcsError {
    state.failure(fault, || {
        Arc::new(
            failure(fault.into_error::<T>(root), operation).with_context(frame(root, operation)),
        )
    })
}
