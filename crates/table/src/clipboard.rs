use crate::runtime_error::Site;
use crate::*;
use bevy::clipboard::{Clipboard, ClipboardPlugin};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::{FocusedInput, InputFocus};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
pub type TableCopyFailure = bevy_widgetry_diagnostics::bevy::WidgetryEcsError;

#[derive(Debug)]
pub enum WidgetryTableCopyFormatError {
    Diagnostic(TableCopyFailure),
    External(Box<dyn std::error::Error + Send + Sync>),
}
impl WidgetryTableCopyFormatError {
    pub fn diagnostic<E: std::error::Error + Send + Sync + 'static>(
        failure: bevy_widgetry_diagnostics::WidgetryFailure<E>,
    ) -> Self {
        Self::Diagnostic(TableCopyFailure::new(failure))
    }
    pub fn external(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::External(Box::new(error))
    }
}

#[derive(Debug)]
pub enum TableCopyCellFault {
    RowUnavailable,
    ColumnUnavailable,
    ValueUnavailable,
    MissingFormatter {
        actual: TypeId,
    },
    FormatterType {
        actual: TypeId,
        expected: TypeId,
    },
    Callback {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
#[derive(Debug)]
pub enum TableCopyFault {
    MissingPart {
        entity: Entity,
        component: &'static str,
    },
    ColumnIndex {
        index: usize,
        len: usize,
    },
    Cell {
        cell: WidgetryTableCell,
        reason: TableCopyCellFault,
    },
    LengthOverflow {
        field_len: usize,
        output_len: usize,
        quoted: bool,
        separator: Option<char>,
    },
    Reserve {
        field_len: usize,
        output_len: usize,
        additional: usize,
        source: std::collections::TryReserveError,
    },
    Clipboard {
        bytes: usize,
        source: bevy::clipboard::ClipboardError,
    },
}
impl std::fmt::Display for TableCopyFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MissingPart { .. } => "Table 复制所需部件失效",
            Self::ColumnIndex { .. } => "Table 复制列位置失效",
            Self::Cell { .. } => "Table 复制单元格取值或格式化失败",
            Self::LengthOverflow { .. } => "Table TSV 字段长度溢出",
            Self::Reserve { .. } => "Table TSV 分配容量失败",
            Self::Clipboard { .. } => "原生剪贴板写入失败",
        })
    }
}
impl std::error::Error for TableCopyFault {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cell {
                reason: TableCopyCellFault::Callback { source },
                ..
            } => Some(source.as_ref()),
            Self::Reserve { source, .. } => Some(source),
            Self::Clipboard { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub struct WidgetryTableCopyFormatter<V>(
    Arc<dyn Fn(&V) -> Result<String, WidgetryTableCopyFormatError> + Send + Sync>,
);

impl<V> WidgetryTableCopyFormatter<V> {
    pub fn new(
        format: impl Fn(&V) -> Result<String, WidgetryTableCopyFormatError> + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(format))
    }
}

struct Formatter {
    value_type: TypeId,
    format: Arc<
        dyn Fn(&WidgetryTableCellValue) -> Option<Result<String, WidgetryTableCopyFormatError>>
            + Send
            + Sync,
    >,
}

impl<V: Send + Sync + 'static> From<WidgetryTableCopyFormatter<V>> for Formatter {
    fn from(formatter: WidgetryTableCopyFormatter<V>) -> Self {
        Self {
            value_type: TypeId::of::<V>(),
            format: Arc::new(move |value| {
                value.downcast_ref::<V>().map(|value| (formatter.0)(value))
            }),
        }
    }
}

#[derive(Resource, Default)]
pub struct WidgetryTableCopyFormatterRegistry {
    defaults: HashMap<TypeId, Formatter>,
    columns: HashMap<(Entity, WidgetryTableColumnId), Formatter>,
}

impl WidgetryTableCopyFormatterRegistry {
    pub fn register_default<V: Send + Sync + 'static>(
        &mut self,
        formatter: WidgetryTableCopyFormatter<V>,
    ) {
        self.defaults.insert(TypeId::of::<V>(), formatter.into());
    }

    pub fn register_column<V: Send + Sync + 'static>(
        &mut self,
        source: Entity,
        column: WidgetryTableColumnId,
        formatter: WidgetryTableCopyFormatter<V>,
    ) {
        self.columns.insert((source, column), formatter.into());
    }

    pub fn remove_source(&mut self, source: Entity) {
        self.columns.retain(|(owner, _), _| *owner != source);
    }

    pub fn remove_default<V: 'static>(&mut self) -> bool {
        self.defaults.remove(&TypeId::of::<V>()).is_some()
    }
}

pub trait WidgetryTableCopyAppExt {
    fn register_table_copy_formatter<V: Send + Sync + 'static>(
        &mut self,
        formatter: WidgetryTableCopyFormatter<V>,
    ) -> &mut Self;
    fn register_table_column_copy_formatter<V: Send + Sync + 'static>(
        &mut self,
        source: Entity,
        column: WidgetryTableColumnId,
        formatter: WidgetryTableCopyFormatter<V>,
    ) -> &mut Self;
}

impl WidgetryTableCopyAppExt for App {
    fn register_table_copy_formatter<V: Send + Sync + 'static>(
        &mut self,
        formatter: WidgetryTableCopyFormatter<V>,
    ) -> &mut Self {
        self.init_resource::<WidgetryTableCopyFormatterRegistry>()
            .world_mut()
            .resource_mut::<WidgetryTableCopyFormatterRegistry>()
            .register_default(formatter);
        self
    }

    fn register_table_column_copy_formatter<V: Send + Sync + 'static>(
        &mut self,
        source: Entity,
        column: WidgetryTableColumnId,
        formatter: WidgetryTableCopyFormatter<V>,
    ) -> &mut Self {
        self.init_resource::<WidgetryTableCopyFormatterRegistry>()
            .world_mut()
            .resource_mut::<WidgetryTableCopyFormatterRegistry>()
            .register_column(source, column, formatter);
        self
    }
}

impl<T: Send + Sync + 'static> WidgetryTable<T> {
    pub fn selection_tsv(
        world: &mut World,
        root: Entity,
    ) -> Result<Option<String>, TableCopyFailure> {
        prepare::<T>(world, root)
    }
    pub fn copy_selection(world: &mut World, root: Entity) -> Result<bool, TableCopyFailure> {
        let site = Site::view::<T>(world, root);
        commit_copy(prepare::<T>(world, root), |text| {
            let mut clipboard = copy_required(
                world.get_resource_mut::<Clipboard>(),
                site,
                root,
                "Clipboard",
            )?;
            write_clipboard(text, site, |text| clipboard.set_text(text))
        })
    }
    pub fn queue_copy_selection(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| {
            Self::copy_selection(world, root)
                .map(|_| ())
                .map_err(TableCopyFailure::into_bevy_error)
        });
    }
}
#[track_caller]
fn copy_failure(fault: TableCopyFault, site: Site) -> TableCopyFailure {
    let frame = site
        .frame("table.copy")
        .field("failure", format_args!("{fault:?}"));
    TableCopyFailure::new(TableFailure::new(
        TableError::Copy {
            fault: Box::new(fault),
        },
        frame,
    ))
}
#[track_caller]
fn copy_required<V>(
    value: Option<V>,
    site: Site,
    entity: Entity,
    component: &'static str,
) -> Result<V, TableCopyFailure> {
    value.ok_or_else(|| copy_failure(TableCopyFault::MissingPart { entity, component }, site))
}
fn prepare<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<Option<String>, TableCopyFailure> {
    let site = Site::view::<T>(world, root);
    let source = copy_required(
        world.get::<WidgetryTable<T>>(root),
        site,
        root,
        "WidgetryTable<T>",
    )?
    .source();
    if world.get::<InteractionDisabled>(root).is_some()
        || world.get::<WidgetryTableEditState>(root).is_some()
    {
        return Ok(None);
    }
    crate::view_state::refresh::<T>(world, root).map_err(|failure| {
        TableCopyFailure::new(failure).with_context(site.frame("table.copy.projection"))
    })?;
    let model = copy_required(
        world.get::<WidgetryTableModel<T>>(source),
        site,
        source,
        "WidgetryTableModel<T>",
    )?;
    let view = copy_required(
        world.get::<WidgetryTableViewState>(root),
        site,
        root,
        "WidgetryTableViewState",
    )?;
    let state = copy_required(
        world.get::<WidgetryTableState>(root),
        site,
        root,
        "WidgetryTableState",
    )?;
    if !crate::interaction::valid(model, view, state.selection()) {
        return Ok(None);
    }
    let Some(range) = state
        .selection()
        .resolve(view, model.column_count(), |c| view.column_display_index(c))
    else {
        return Ok(None);
    };
    let rows = view.ordered_rows()[range.rows()].to_vec();
    let columns = range
        .columns()
        .map(|index| {
            view.column_id(index).ok_or_else(|| {
                copy_failure(
                    TableCopyFault::ColumnIndex {
                        index,
                        len: view.ordered_columns().len(),
                    },
                    site,
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let registry = world.get_resource::<WidgetryTableCopyFormatterRegistry>();
    let mut output = String::new();
    for (r, row) in rows.into_iter().enumerate() {
        for (c, column) in columns.iter().copied().enumerate() {
            let cell = WidgetryTableCell { row, column };
            let location = site.at(TableRegion::Cell, Some(row), Some(column));
            let fail = |reason| copy_failure(TableCopyFault::Cell { cell, reason }, location);
            let ri = model
                .row_index(row)
                .ok_or_else(|| fail(TableCopyCellFault::RowUnavailable))?;
            let ci = model
                .column_index(column)
                .ok_or_else(|| fail(TableCopyCellFault::ColumnUnavailable))?;
            let value = model
                .cell_at(ri, ci)
                .ok_or_else(|| fail(TableCopyCellFault::ValueUnavailable))?;
            let actual = value.type_id();
            let formatter = registry
                .and_then(|registry| {
                    registry
                        .columns
                        .get(&(source, column))
                        .or_else(|| registry.defaults.get(&actual))
                })
                .ok_or_else(|| fail(TableCopyCellFault::MissingFormatter { actual }))?;
            let mismatch = || {
                fail(TableCopyCellFault::FormatterType {
                    actual,
                    expected: formatter.value_type,
                })
            };
            if formatter.value_type != actual {
                return Err(mismatch());
            }
            let text = match (formatter.format)(&value).ok_or_else(mismatch)? {
                Ok(text) => text,
                Err(WidgetryTableCopyFormatError::Diagnostic(error)) => {
                    return Err(error.with_context(
                        location
                            .frame("table.copy.formatter")
                            .field("value_type", format_args!("{actual:?}")),
                    ));
                }
                Err(WidgetryTableCopyFormatError::External(source)) => {
                    return Err(fail(TableCopyCellFault::Callback { source }));
                }
            };
            let separator = if c > 0 {
                Some('\t')
            } else if r > 0 {
                Some('\n')
            } else {
                None
            };
            append_field(&mut output, &text, separator)
                .map_err(|fault| copy_failure(fault, location))?;
        }
    }
    Ok(Some(output))
}

fn append_field(
    output: &mut String,
    text: &str,
    separator: Option<char>,
) -> Result<(), TableCopyFault> {
    let quoted = text.contains(['\t', '\r', '\n', '"']);
    let quote_count = quoted.then(|| text.bytes().filter(|&b| b == b'"').count());
    let additional = field_length(text.len(), quote_count, separator.is_some()).ok_or(
        TableCopyFault::LengthOverflow {
            field_len: text.len(),
            output_len: output.len(),
            quoted,
            separator,
        },
    )?;
    output
        .try_reserve(additional)
        .map_err(|source| TableCopyFault::Reserve {
            field_len: text.len(),
            output_len: output.len(),
            additional,
            source,
        })?;
    if let Some(separator) = separator {
        output.push(separator);
    }
    if !quoted {
        output.push_str(text);
        return Ok(());
    }
    if quoted {
        output.push('"');
    }
    for ch in text.chars() {
        output.push(ch);
        if ch == '"' {
            output.push('"');
        }
    }
    if quoted {
        output.push('"');
    }
    Ok(())
}

fn field_length(field_len: usize, quote_count: Option<usize>, separator: bool) -> Option<usize> {
    let extra = match quote_count {
        Some(count) => count.checked_add(2)?,
        None => 0,
    };
    field_len
        .checked_add(extra)?
        .checked_add(usize::from(separator))
}
fn write_clipboard(
    text: &str,
    site: Site,
    write: impl FnOnce(&str) -> Result<(), bevy::clipboard::ClipboardError>,
) -> Result<(), TableCopyFailure> {
    write(text).map_err(|source| {
        copy_failure(
            TableCopyFault::Clipboard {
                bytes: text.len(),
                source,
            },
            site,
        )
    })
}

fn commit_copy(
    text: Result<Option<String>, TableCopyFailure>,
    write: impl FnOnce(&str) -> Result<(), TableCopyFailure>,
) -> Result<bool, TableCopyFailure> {
    let Some(text) = text? else {
        return Ok(false);
    };
    write(&text)?;
    Ok(true)
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<WidgetryTableCopyFormatterRegistry>();
    if !app.is_plugin_added::<ClipboardPlugin>() {
        app.add_plugins(ClipboardPlugin);
    }
}

pub(crate) fn on_key<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<
        (),
        (
            With<WidgetryTable<T>>,
            Without<InteractionDisabled>,
            Without<WidgetryTableEditState>,
        ),
    >,
    focus: Option<Res<InputFocus>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    let root = event.focused_entity;
    if event.input.state != ButtonState::Pressed
        || event.input.key_code != KeyCode::KeyC
        || !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
        || !views.contains(root)
        || focus.is_none_or(|focus| focus.get() != Some(root))
    {
        return;
    }
    event.propagate(false);
    WidgetryTable::<T>::queue_copy_selection(&mut commands, root);
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn serialized_field_lengths_check_quotes_separator_and_capacity_boundaries() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(field_length(0, None, false), Some(0));
        assert_eq!(field_length(4, Some(2), true), Some(9));
        assert_eq!(field_length(usize::MAX, None, false), Some(usize::MAX));
        assert_eq!(field_length(usize::MAX, None, true), None);
        assert_eq!(
            field_length(usize::MAX - 2, Some(0), false),
            Some(usize::MAX)
        );
        assert_eq!(field_length(usize::MAX - 1, Some(0), false), None);
        assert_eq!(field_length(1, Some(usize::MAX), false), None);
    }

    #[test]
    fn native_write_failure_keeps_clipboard_source_and_output_size() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        let site = Site::view::<u32>(&World::new(), Entity::PLACEHOLDER);
        let text = String::from("private TSV 内容");
        let bytes = text.len();
        let error = logs
            .run(|| {
                commit_copy(Ok(Some(text)), |text| {
                    write_clipboard(text, site, |_| {
                        Err(bevy::clipboard::ClipboardError::ClipboardOccupied)
                    })
                })
            })
            .unwrap_err();
        let diagnostic = error.diagnostics().next().unwrap();
        let mut cause = diagnostic.source();
        let mut found = false;
        while let Some(source) = cause {
            if let Some(TableCopyFault::Clipboard {
                bytes: actual,
                source: bevy::clipboard::ClipboardError::ClipboardOccupied,
            }) = source.downcast_ref::<TableCopyFault>()
            {
                assert_eq!(*actual, bytes);
                found = true;
            }
            if let Some(native) = source.downcast_ref::<bevy::clipboard::ClipboardError>() {
                assert!(matches!(
                    native,
                    bevy::clipboard::ClipboardError::ClipboardOccupied
                ));
            }
            cause = source.source();
        }
        assert!(found);
        assert_eq!(
            logs.diagnostic_records(
                diagnostic.origin().id(),
                bevy_widgetry_test_utils::DiagnosticPhase::Source
            )
            .len(),
            1
        );
        assert!(!format!("{error}").contains("private TSV"));
    }

    #[test]
    fn clipboard_commit_is_called_once_only_after_complete_success() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut writes = Vec::new();
        assert!(
            commit_copy(
                Err(copy_failure(
                    TableCopyFault::MissingPart {
                        entity: Entity::PLACEHOLDER,
                        component: "last cell"
                    },
                    Site::view::<u32>(&World::new(), Entity::PLACEHOLDER)
                )),
                |text| {
                    writes.push(text.to_owned());
                    Ok(())
                }
            )
            .is_err()
        );
        assert!(
            !commit_copy(Ok(None), |text| {
                writes.push(text.to_owned());
                Ok(())
            })
            .unwrap()
        );
        assert!(writes.is_empty());
        assert!(
            commit_copy(Ok(Some("a\tb\nc\td".into())), |text| {
                writes.push(text.to_owned());
                Ok(())
            })
            .unwrap()
        );
        assert_eq!(writes, ["a\tb\nc\td"]);
    }
}
