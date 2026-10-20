use crate::*;
use bevy::clipboard::{Clipboard, ClipboardPlugin};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::{FocusedInput, InputFocus};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_internal_log::widgetry_error;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

pub struct WidgetryTableCopyFormatter<V>(
    Arc<dyn Fn(&V) -> Result<String, BevyError> + Send + Sync>,
);

impl<V> WidgetryTableCopyFormatter<V> {
    pub fn new(format: impl Fn(&V) -> Result<String, BevyError> + Send + Sync + 'static) -> Self {
        Self(Arc::new(format))
    }
}

struct Formatter {
    value_type: TypeId,
    format: Arc<dyn Fn(&WidgetryTableCellValue) -> Result<String, BevyError> + Send + Sync>,
}

impl<V: Send + Sync + 'static> From<WidgetryTableCopyFormatter<V>> for Formatter {
    fn from(formatter: WidgetryTableCopyFormatter<V>) -> Self {
        Self {
            value_type: TypeId::of::<V>(),
            format: Arc::new(move |value| {
                let value = value
                    .downcast_ref::<V>()
                    .ok_or_else(|| BevyError::error("Table copy formatter type mismatch"))?;
                (formatter.0)(value)
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
    pub fn selection_tsv(world: &mut World, root: Entity) -> Result<Option<String>, BevyError> {
        prepare::<T>(world, root)
            .inspect_err(|error| widgetry_error!(?root, %error, "Table TSV 生成失败"))
    }

    pub fn copy_selection(world: &mut World, root: Entity) -> Result<bool, BevyError> {
        commit_copy(prepare::<T>(world, root), |text| {
            world
                .get_resource_mut::<Clipboard>()
                .ok_or_else(|| BevyError::error("Table copy requires Clipboard"))?
                .set_text(text)
                .map_err(BevyError::error)
        })
        .inspect_err(|error| widgetry_error!(?root, %error, "Table 复制选区失败"))
    }

    pub fn queue_copy_selection(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| Self::copy_selection(world, root).map(|_| ()));
    }
}

fn prepare<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<Option<String>, BevyError> {
    let source = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table copy requires a live matching View"))?
        .source();
    if world.get::<InteractionDisabled>(root).is_some()
        || world.get::<WidgetryTableEditState>(root).is_some()
    {
        return Ok(None);
    }
    crate::view_state::refresh::<T>(world, root)?;
    let model = world
        .get::<WidgetryTableModel<T>>(source)
        .ok_or_else(|| BevyError::error("Table copy requires a live matching Model"))?;
    let view = world
        .get::<WidgetryTableViewState>(root)
        .ok_or_else(|| BevyError::error("Table copy requires a View projection"))?;
    let state = world
        .get::<WidgetryTableState>(root)
        .ok_or_else(|| BevyError::error("Table copy requires selection state"))?;
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
        .map(|i| {
            view.column_id(i)
                .ok_or_else(|| BevyError::error("Table copy column is unavailable"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let registry = world.get_resource::<WidgetryTableCopyFormatterRegistry>();
    let mut output = String::new();
    for (r, row) in rows.into_iter().enumerate() {
        for (c, column) in columns.iter().copied().enumerate() {
            let cell = WidgetryTableCell { row, column };
            let text = (|| {
                let ri = model
                    .row_index(row)
                    .ok_or_else(|| BevyError::error("Row is unavailable"))?;
                let ci = model
                    .column_index(column)
                    .ok_or_else(|| BevyError::error("Column is unavailable"))?;
                let value = model
                    .cell_at(ri, ci)
                    .ok_or_else(|| BevyError::error("Cell value is unavailable"))?;
                let formatter = registry
                    .and_then(|registry| {
                        registry
                            .columns
                            .get(&(source, column))
                            .or_else(|| registry.defaults.get(&value.type_id()))
                    })
                    .ok_or_else(|| BevyError::error("No registered Table copy formatter"))?;
                if formatter.value_type != value.type_id() {
                    return Err(BevyError::error("Table copy formatter type mismatch"));
                }
                (formatter.format)(&value)
            })()
            .map_err(|error: BevyError| {
                BevyError::error(format!("Table copy cell {cell:?}: {error}"))
            })?;
            let separator = if c > 0 {
                Some('\t')
            } else if r > 0 {
                Some('\n')
            } else {
                None
            };
            append_field(&mut output, &text, separator)
                .map_err(|error| BevyError::error(format!("Table copy cell {cell:?}: {error}")))?;
        }
    }
    Ok(Some(output))
}

fn append_field(output: &mut String, text: &str, separator: Option<char>) -> Result<(), BevyError> {
    let quoted = text.contains(['\t', '\r', '\n', '"']);
    let extra = if quoted {
        text.bytes().filter(|&b| b == b'"').count().checked_add(2)
    } else {
        Some(0)
    };
    let additional = extra
        .and_then(|extra| text.len().checked_add(extra))
        .and_then(|len| len.checked_add(usize::from(separator.is_some())))
        .ok_or_else(|| BevyError::error("Table TSV field length overflow"))?;
    output.try_reserve(additional).map_err(BevyError::error)?;
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

fn commit_copy(
    text: Result<Option<String>, BevyError>,
    write: impl FnOnce(&str) -> Result<(), BevyError>,
) -> Result<bool, BevyError> {
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
    fn clipboard_commit_is_called_once_only_after_complete_success() {
        let mut writes = Vec::new();
        assert!(
            commit_copy(Err(BevyError::error("last cell failed")), |text| {
                writes.push(text.to_owned());
                Ok(())
            })
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
