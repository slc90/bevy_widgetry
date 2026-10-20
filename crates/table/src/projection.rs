use crate::layout::TableGeometry;
use crate::style::{Region, bind_part};
use crate::view::{TableCanvas, TableDiagnostics};
use crate::viewport::VisibleCells;
use crate::*;
use bevy::picking::events::PointerScroll;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::window::RequestRedraw;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::scene::{apply_scene, spawn_scene};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use std::any::TypeId;
use std::collections::{HashMap, HashSet};

#[derive(Component, Clone)]
pub(crate) struct TableRuntime {
    pub(crate) body: Entity,
    pub(crate) frozen_body: Entity,
    frozen_body_canvas: Entity,
    frozen_column_canvas: Entity,
    columns: Entity,
    rows: Entity,
    pub(super) corner: Entity,
    pub(crate) body_canvas: Entity,
    column_canvas: Entity,
    row_canvas: Entity,
    pub(super) cells: HashMap<(WidgetryTableRowId, WidgetryTableColumnId), Entity>,
    pub(super) column_headers: HashMap<WidgetryTableColumnId, Entity>,
    pub(super) row_headers: HashMap<WidgetryTableRowId, Entity>,
    measured: Vec2,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct ContentVersion {
    row: u64,
    column: u64,
    generation: u64,
    value_type: TypeId,
}

pub(crate) fn required<V>(value: Option<V>) -> Result<V, BevyError> {
    value.ok_or_else(|| BevyError::error("Table required runtime state missing"))
}

pub(crate) fn viewport_width(
    world: &World,
    root: Entity,
    layout: &WidgetryTableLayout,
) -> Result<f32, BevyError> {
    let computed = required(world.get::<ComputedNode>(root))?;
    Ok(
        (computed.content_box().width() * computed.inverse_scale_factor()
            - layout.row_header_width)
            .max(0.0),
    )
}

fn reparent_shell(
    world: &mut World,
    entity: Option<Entity>,
    canvas: Entity,
    alternate: Entity,
) -> Result<(), BevyError> {
    let Some(entity) = entity else {
        return Ok(());
    };
    let parent = required(world.get::<ChildOf>(entity))?.parent();
    if parent == alternate {
        required(world.get_entity_mut(canvas).ok())?.add_child(entity);
    } else if parent != canvas {
        return Err(BevyError::error("Table shell ownership changed"));
    }
    Ok(())
}

pub(crate) fn on_frozen_scroll(
    mut event: On<PointerScroll>,
    frozen: Query<
        &ChildOf,
        (
            With<WidgetryTableFrozenBody>,
            Without<bevy::ui::InteractionDisabled>,
        ),
    >,
    tables: Query<&TableRuntime>,
    mut commands: Commands,
) {
    let Ok(parent) = frozen.get(event.entity) else {
        return;
    };
    let Ok(runtime) = tables.get(parent.parent()) else {
        return;
    };
    let mut forwarded: PointerScroll = (*event).clone();
    forwarded.entity = runtime.body;
    event.propagate(false);
    commands.queue(move |world: &mut World| {
        if world.get::<WidgetryTableBody>(forwarded.entity).is_some() {
            world.trigger(forwarded);
        }
    });
}

fn runtime(world: &World, root: Entity) -> Result<TableRuntime, BevyError> {
    let children = required(world.get::<Children>(root))?;
    let part = |matches: fn(&World, Entity) -> bool| {
        required(children.iter().find(|&entity| matches(world, entity)))
    };
    let body = part(|world, entity| world.get::<WidgetryTableBody>(entity).is_some())?;
    let columns = part(|world, entity| world.get::<WidgetryTableColumnHeaders>(entity).is_some())?;
    let rows = part(|world, entity| world.get::<WidgetryTableRowHeaders>(entity).is_some())?;
    let corner = part(|world, entity| world.get::<WidgetryTableCorner>(entity).is_some())?;
    let frozen_body = part(|world, entity| world.get::<WidgetryTableFrozenBody>(entity).is_some())?;
    let frozen_columns = part(|world, entity| {
        world
            .get::<WidgetryTableFrozenColumnHeaders>(entity)
            .is_some()
    })?;
    let canvas = |entity| {
        required(world.get::<Children>(entity).and_then(|children| {
            children
                .iter()
                .find(|&entity| world.get::<TableCanvas>(entity).is_some())
        }))
    };
    Ok(TableRuntime {
        frozen_body,
        frozen_body_canvas: canvas(frozen_body)?,
        frozen_column_canvas: canvas(frozen_columns)?,
        body,
        columns,
        rows,
        corner,
        body_canvas: canvas(body)?,
        column_canvas: canvas(columns)?,
        row_canvas: canvas(rows)?,
        cells: default(),
        column_headers: default(),
        row_headers: default(),
        measured: Vec2::ZERO,
    })
}

pub(crate) fn reconcile<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<WidgetryTable<T>>>()
        .iter(world)
        .collect();
    let mut failure = None;
    for root in roots {
        if world.get::<WidgetryTable<T>>(root).is_none() {
            continue;
        }
        let result = reconcile_root::<T>(world, root);
        // A semantic event callback may legitimately despawn the root, leaving no diagnostic state to inspect.
        if !world.entities().contains(root) {
            continue;
        }
        let result =
            if let Some(mut diagnostics) = world.get_mut::<TableDiagnostics>(root) {
                diagnostics.0.observe(result,
                |error| widgetry_error!(entity = ?root, %error, "Table View reconciliation 失败"),
                || widgetry_info!(entity = ?root, "Table View 恢复正常"))
            } else {
                widgetry_error!(entity = ?root, "Table 诊断 state 缺失");
                Err(BevyError::error("Table diagnostics missing"))
            };
        if let Err(error) = result {
            discard_projection(world, root);
            if failure.is_none() {
                failure = Some(error);
            }
        }
    }
    failure.map_or(Ok(()), Err)
}

fn discard_projection(world: &mut World, root: Entity) {
    crate::editing::close(world, root);
    crate::selection_drag::finish(world, root);
    crate::interaction::clear(world, root);
    let canvases: Vec<_> = world
        .get::<Children>(root)
        .into_iter()
        .flat_map(|children| children.iter())
        .flat_map(|part| {
            world
                .get::<Children>(part)
                .into_iter()
                .flat_map(|children| children.iter())
        })
        .filter(|&entity| world.get::<TableCanvas>(entity).is_some())
        .collect();
    for canvas in canvases {
        let children = world
            .get::<Children>(canvas)
            .map(|children| children.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for child in children {
            world.despawn(child);
        }
        if let Some(mut node) = world.get_mut::<Node>(canvas) {
            node.width = px(0);
            node.height = px(0);
        }
    }
    world
        .entity_mut(root)
        .remove::<(TableRuntime, TableGeometry)>();
    crate::resize::cancel(world, root);
}

fn reconcile_root<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), BevyError> {
    let mut runtime = world
        .get::<TableRuntime>(root)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| runtime(world, root))?;
    let source = required(world.get::<WidgetryTable<T>>(root))?.source();
    crate::resize::sync::<T>(world, root, source);
    finish_stale_resize::<T>(world, root, source, &runtime)?;
    if world.get::<WidgetryTable<T>>(root).is_none() {
        return Ok(());
    }
    crate::view_state::refresh::<T>(world, root)?;
    crate::editing::repair_session::<T>(world, root);
    // Resize termination observers may change axes or despawn the root through Commands.
    // Finish callbacks before reading the Model to avoid using IDs or counts they invalidated.
    let model = world.get::<WidgetryTableModel<T>>(source).ok_or_else(|| {
        BevyError::error("Table requires a live matching WidgetryTableModel source")
    })?;
    let ids: Vec<_> = (0..model.column_count())
        .map(|index| required(model.column_id(index)))
        .collect::<Result<_, _>>()?;
    let rows = required(world.get::<WidgetryTableViewState>(root))?.len();
    let layout = required(world.get::<WidgetryTableLayout>(root))?.clone();
    let computed = required(world.get::<ComputedNode>(runtime.body))?;
    let measured = computed.size() * computed.inverse_scale_factor();
    let inverse = computed.inverse_scale_factor();
    let width = viewport_width(world, root, &layout)?;
    let geometry = layout.resolve(ids, rows, width)?;
    let scrolling_size = Vec2::new((width - geometry.frozen_width).max(0.0), measured.y);
    crate::interaction::sync::<T>(world, root, source, runtime.body, &geometry, scrolling_size)?;
    let previous = required(world.get::<ScrollPosition>(runtime.body))?.0;
    let offset = Vec2::new(
        clamp_offset(
            previous.x,
            geometry.width - geometry.frozen_width,
            scrolling_size.x,
        ),
        clamp_offset(previous.y, geometry.height, measured.y),
    );
    // Upstream Layout rounds physical scroll down.
    // Canvas geometry must use that same effective offset even though its coordinates are not rounded.
    let physical_offset = (offset / inverse).floor() * inverse;
    let full_size = Vec2::new(width, measured.y);
    let visible = VisibleCells::new(&geometry, rows, physical_offset, full_size);
    required(world.get::<WidgetryTableStyle>(root))?;
    bind_part(world, root, root, Region::Table)?;
    bind_part(world, root, runtime.corner, Region::Corner)?;
    let header_rows =
        VisibleCells::new(&geometry, rows, physical_offset, Vec2::new(1.0, measured.y)).rows;
    let row_ids: Vec<_> = header_rows
        .map(|index| {
            required(world.get::<WidgetryTableViewState>(root))
                .and_then(|projection| required(projection.row_id(index)))
                .map(|id| (index, id))
        })
        .collect::<Result<_, _>>()?;
    let current_rows: HashSet<_> = row_ids.iter().map(|(_, row)| *row).collect();
    let header_columns = VisibleCells::new(&geometry, rows, physical_offset, Vec2::new(width, 1.0));
    let current_columns: HashSet<_> = header_columns
        .column_indices()
        .map(|index| geometry.columns[index].id)
        .collect();
    let edited = world.get::<WidgetryTableEditState>(root).map(|s| s.cell());
    if let Some(cell) = edited
        && let Some(&entity) = runtime.cells.get(&(cell.row, cell.column))
        && let Some(row) = world
            .get::<WidgetryTableViewState>(root)
            .and_then(|s| s.display_index(cell.row))
        && let Some(column) = geometry.columns.iter().find(|c| c.id == cell.column)
    {
        let canvas = if column.frozen {
            runtime.frozen_body_canvas
        } else {
            runtime.body_canvas
        };
        let alternate = if column.frozen {
            runtime.body_canvas
        } else {
            runtime.frozen_body_canvas
        };
        reparent_shell(world, Some(entity), canvas, alternate)?;
        let mut node = required(world.get_mut::<Node>(entity))?;
        node.top = px(row as f32 * geometry.row_height);
        node.left = px(column.left
            - if column.frozen {
                0.0
            } else {
                geometry.frozen_width
            });
        node.width = px(column.width);
        node.height = px(geometry.row_height);
        node.overflow = Overflow::visible();
    }
    retain_shells(world, &mut runtime.cells, |(row, column)| {
        edited.is_some_and(|c| c.row == *row && c.column == *column)
            || !visible.rows.is_empty()
                && current_rows.contains(row)
                && visible
                    .column_indices()
                    .any(|index| geometry.columns[index].id == *column)
    });
    retain_shells(world, &mut runtime.column_headers, |id| {
        current_columns.contains(id)
    });
    retain_shells(world, &mut runtime.row_headers, |id| {
        current_rows.contains(id)
    });
    for column in header_columns
        .column_indices()
        .map(|index| &geometry.columns[index])
    {
        let model = required(world.get::<WidgetryTableModel<T>>(source))?;
        let header = required(model.column(column.index))?.header();
        let revision = required(model.column_revision(column.index))?;
        let registry = required(world.get_resource::<WidgetryTableHeaderRendererRegistry>())?;
        let version = ContentVersion {
            row: 0,
            column: revision,
            generation: registry.0.generation(header.type_id())?,
            value_type: header.type_id(),
        };
        let old = runtime.column_headers.get(&column.id).copied();
        let canvas = if column.frozen {
            runtime.frozen_column_canvas
        } else {
            runtime.column_canvas
        };
        let alternate = if column.frozen {
            runtime.column_canvas
        } else {
            runtime.frozen_column_canvas
        };
        let scene = needs_content(world, old, version)
            .then(|| registry.0.scene(header.as_any()).map(|(_, scene)| -> Box<dyn SceneList> {
                Box::new(bsn_list! {
                    Node { flex_grow: 1.0, min_width: px(0), height: percent(100), overflow: Overflow::clip(), align_items: AlignItems::Center }
                    Children [{scene}]
                })
            }))
            .transpose()?;
        reparent_shell(world, old, canvas, alternate)?;
        let entity = shell(
            world,
            canvas,
            old,
            version,
            scene,
            column.left
                - if column.frozen {
                    0.0
                } else {
                    geometry.frozen_width
                },
            0.0,
            column.width,
            layout.column_header_height,
        )?;
        let identity = WidgetryTableColumnHeader { column: column.id };
        if world.get::<WidgetryTableColumnHeader>(entity) != Some(&identity) {
            world.entity_mut(entity).insert(identity);
        }
        bind_part(world, root, entity, Region::ColumnHeader)?;
        runtime.column_headers.insert(column.id, entity);
        crate::resize::ensure_handle(world, entity, column.id)?;
        crate::header::ensure(world, root, entity, column.id)?;
    }
    for (index, row) in row_ids.iter().copied() {
        let old = runtime.row_headers.get(&row).copied();
        let version = ContentVersion {
            row: index as u64,
            column: 0,
            generation: 0,
            value_type: TypeId::of::<String>(),
        };
        let scene = needs_content(world, old, version).then(|| {
            Box::new(bsn_list!{Text({ (index + 1).to_string() }) bevy_widgetry_core::text::WidgetryText}) as Box<dyn SceneList>
        });
        let entity = shell(
            world,
            runtime.row_canvas,
            old,
            version,
            scene,
            0.0,
            index as f32 * geometry.row_height,
            layout.row_header_width,
            geometry.row_height,
        )?;
        let identity = WidgetryTableRowHeader { row, index };
        if world.get::<WidgetryTableRowHeader>(entity) != Some(&identity) {
            world.entity_mut(entity).insert(identity);
        }
        bind_part(world, root, entity, Region::RowHeader)?;
        runtime.row_headers.insert(row, entity);
        if !visible.rows.contains(&index) {
            continue;
        }
        for column in visible
            .column_indices()
            .map(|index| &geometry.columns[index])
        {
            let model = required(world.get::<WidgetryTableModel<T>>(source))?;
            let registry = required(world.get_resource::<WidgetryTableCellRendererRegistry>())?;
            let old = runtime.cells.get(&(row, column.id)).copied();
            let canvas = if column.frozen {
                runtime.frozen_body_canvas
            } else {
                runtime.body_canvas
            };
            let alternate = if column.frozen {
                runtime.body_canvas
            } else {
                runtime.frozen_body_canvas
            };
            let model_index = required(model.row_index(row))?;
            let row_revision = required(model.row_revision(model_index))?;
            let column_revision = required(model.column_revision(column.index))?;
            let cached = old
                .and_then(|entity| world.get::<ContentVersion>(entity))
                .copied();
            let unchanged = if let Some(cached) = cached {
                old.is_some_and(|entity| {
                    world
                        .get::<crate::editing::EditingContent>(entity)
                        .is_some()
                }) || cached.row == row_revision
                    && cached.column == column_revision
                    && cached.generation == registry.0.generation(cached.value_type)?
            } else {
                false
            };
            let (version, scene) = if unchanged {
                (required(cached)?, None)
            } else {
                let value = required(model.cell_at(model_index, column.index))?;
                let (generation, scene) = registry.0.scene(value.as_any())?;
                (
                    ContentVersion {
                        row: row_revision,
                        column: column_revision,
                        generation,
                        value_type: value.type_id(),
                    },
                    Some(scene),
                )
            };
            reparent_shell(world, old, canvas, alternate)?;
            let entity = shell(
                world,
                canvas,
                old,
                version,
                scene,
                column.left
                    - if column.frozen {
                        0.0
                    } else {
                        geometry.frozen_width
                    },
                index as f32 * geometry.row_height,
                column.width,
                geometry.row_height,
            )?;
            let identity = WidgetryTableCell {
                row,
                column: column.id,
            };
            if world.get::<WidgetryTableCell>(entity) != Some(&identity) {
                world.entity_mut(entity).insert(identity);
            }
            bind_part(world, root, entity, Region::Cell)?;
            runtime.cells.insert((row, column.id), entity);
        }
    }
    {
        let mut node = required(world.get_mut::<Node>(root))?;
        let columns = vec![
            RepeatedGridTrack::px(1, layout.row_header_width),
            RepeatedGridTrack::px(1, geometry.frozen_width),
            RepeatedGridTrack::flex(1, 1.0),
        ];
        let rows = vec![
            RepeatedGridTrack::px(1, layout.column_header_height),
            RepeatedGridTrack::flex(1, 1.0),
        ];
        if node.grid_template_columns != columns {
            node.grid_template_columns = columns;
        }
        if node.grid_template_rows != rows {
            node.grid_template_rows = rows;
        }
    }
    size_canvas(
        world,
        runtime.body_canvas,
        geometry.width - geometry.frozen_width,
        geometry.height,
    )?;
    size_canvas(
        world,
        runtime.frozen_body_canvas,
        geometry.frozen_width,
        geometry.height,
    )?;
    size_canvas(
        world,
        runtime.frozen_column_canvas,
        geometry.frozen_width,
        layout.column_header_height,
    )?;
    size_canvas(
        world,
        runtime.column_canvas,
        geometry.width - geometry.frozen_width,
        layout.column_header_height,
    )?;
    size_canvas(
        world,
        runtime.row_canvas,
        layout.row_header_width,
        geometry.height,
    )?;
    required(world.get_mut::<ScrollPosition>(runtime.body))?
        .map_unchanged(|value| &mut value.0)
        .set_if_neq(offset);
    required(world.get_mut::<ScrollPosition>(runtime.columns))?
        .map_unchanged(|value| &mut value.0)
        .set_if_neq(Vec2::new(offset.x, 0.0));
    required(world.get_mut::<ScrollPosition>(runtime.rows))?
        .map_unchanged(|value| &mut value.0)
        .set_if_neq(Vec2::new(0.0, offset.y));
    required(world.get_mut::<ScrollPosition>(runtime.frozen_body))?
        .map_unchanged(|value| &mut value.0)
        .set_if_neq(Vec2::new(0.0, offset.y));
    runtime.measured = measured;
    world.entity_mut(root).insert((runtime, geometry));
    crate::editing::sync::<T>(world, root)?;
    crate::resize::sync::<T>(world, root, source);
    crate::selection_drag::refresh::<T>(world, root)?;
    Ok(())
}

// Header replacement or horizontal virtualization can destroy the active resize handle.
// End the gesture and flush callbacks before reading Model and layout so projection uses geometry after those callbacks.
fn finish_stale_resize<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    source: Entity,
    runtime: &TableRuntime,
) -> Result<(), BevyError> {
    let Some(column) = crate::resize::active_column(world, root) else {
        return Ok(());
    };
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let index = required(model.column_index(column))?;
    let header = required(model.header(column))?;
    let registry = required(world.get_resource::<WidgetryTableHeaderRendererRegistry>())?;
    let version = ContentVersion {
        row: 0,
        column: required(model.column_revision(index))?,
        generation: registry.0.generation(header.type_id())?,
        value_type: header.type_id(),
    };
    if needs_content(world, runtime.column_headers.get(&column).copied(), version) {
        crate::resize::cancel(world, root);
        return Ok(());
    }
    let ids = (0..model.column_count())
        .map(|index| required(model.column_id(index)))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = required(world.get::<WidgetryTableViewState>(root))?.len();
    let computed = required(world.get::<ComputedNode>(runtime.body))?;
    let measured = computed.size() * computed.inverse_scale_factor();
    let inverse = computed.inverse_scale_factor();
    let layout = required(world.get::<WidgetryTableLayout>(root))?;
    let width = viewport_width(world, root, layout)?;
    let geometry = layout.resolve(ids, rows, width)?;
    let scrolling_size = Vec2::new((width - geometry.frozen_width).max(0.0), measured.y);
    // Consume pending keyboard reveal too so later projection cannot scroll the active handle out of the viewport.
    crate::interaction::sync::<T>(world, root, source, runtime.body, &geometry, scrolling_size)?;
    let previous = required(world.get::<ScrollPosition>(runtime.body))?.0;
    let offset = Vec2::new(
        clamp_offset(
            previous.x,
            geometry.width - geometry.frozen_width,
            scrolling_size.x,
        ),
        clamp_offset(previous.y, geometry.height, measured.y),
    );
    let physical_offset = (offset / inverse).floor() * inverse;
    let visible = VisibleCells::new(&geometry, rows, physical_offset, Vec2::new(width, 1.0));
    if !visible.contains_column(index) {
        crate::resize::cancel(world, root);
    }
    Ok(())
}

// The viewport's actual dimensions become available only during Layout.
// Request another frame when resolved dimensions differ from measurement so a reactive App cannot retain stale projection.
pub(crate) fn request_geometry_redraw<T: Send + Sync + 'static>(
    roots: Query<
        (
            &TableRuntime,
            &TableGeometry,
            &WidgetryTableLayout,
            &ComputedNode,
        ),
        With<WidgetryTable<T>>,
    >,
    bodies: Query<&ComputedNode, With<WidgetryTableBody>>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    for (runtime, geometry, layout, root) in &roots {
        if let Ok(computed) = bodies.get(runtime.body)
            && (computed.size() * computed.inverse_scale_factor() != runtime.measured
                || (root.content_box().width() * root.inverse_scale_factor()
                    - layout.row_header_width)
                    .max(0.0)
                    != geometry.viewport_width)
        {
            redraw.write(RequestRedraw);
        }
    }
}

fn clamp_offset(offset: f32, total: f32, viewport: f32) -> f32 {
    if offset.is_finite() {
        offset.clamp(0.0, (total - viewport).max(0.0))
    } else {
        0.0
    }
}

fn retain_shells<K: Eq + std::hash::Hash>(
    world: &mut World,
    shells: &mut HashMap<K, Entity>,
    keep: impl Fn(&K) -> bool,
) {
    shells.retain(|id, entity| {
        if keep(id) {
            true
        } else {
            world.despawn(*entity);
            false
        }
    });
}

fn needs_content(world: &World, entity: Option<Entity>, version: ContentVersion) -> bool {
    entity.is_none_or(|entity| world.get::<ContentVersion>(entity) != Some(&version))
}

pub(crate) fn invalidate_content(world: &mut World, entity: Entity) {
    world.entity_mut(entity).remove::<ContentVersion>();
}

pub(crate) fn content_type(world: &World, entity: Entity, row: u64, column: u64) -> Option<TypeId> {
    world
        .get::<ContentVersion>(entity)
        .filter(|v| v.row == row && v.column == column)
        .map(|v| v.value_type)
}

fn shell(
    world: &mut World,
    canvas: Entity,
    old: Option<Entity>,
    version: ContentVersion,
    scene: Option<Box<dyn SceneList>>,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) -> Result<Entity, BevyError> {
    let node = Node {
        position_type: PositionType::Absolute,
        left: px(left),
        top: px(top),
        width: px(width),
        height: px(height),
        min_width: px(0),
        min_height: px(0),
        overflow: if old.is_some_and(|e| world.get::<crate::editing::EditingContent>(e).is_some()) {
            Overflow::visible()
        } else {
            Overflow::clip()
        },
        align_items: AlignItems::Center,
        box_sizing: BoxSizing::BorderBox,
        ..default()
    };
    let entity = if let Some(entity) = old {
        required(world.get::<Node>(entity))?;
        if required(world.get::<ChildOf>(entity))?.parent() != canvas {
            return Err(BevyError::error("Table shell ownership changed"));
        }
        if let Some(scene) = scene {
            crate::style::content_changed(world, entity);
            let children = world
                .get::<Children>(entity)
                .map(|children| children.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            for child in children {
                world.despawn(child);
            }
            apply_scene(&mut world.entity_mut(entity), bsn! { Children [{scene}] })
                .map_err(BevyError::error)?;
            world.entity_mut(entity).insert(version);
        }
        let mut current = required(world.get_mut::<Node>(entity))?;
        // Shell geometry and styling share the same Node.
        // Preserve the previous padding and border so each frame does not clear and then restore them.
        let mut node = node;
        node.padding = current.padding;
        node.border = current.border;
        current.set_if_neq(node);
        entity
    } else {
        let scene = required(scene)?;
        let entity = spawn_scene(
            world,
            bsn! {
                BackgroundColor::default() BorderColor::default() Hovered::default()
                template(|_| Ok(bevy::picking::cursor::EntityCursor::default()))
                template(move |_| Ok(version))
                template(move |_| Ok(node.clone()))
                template(|_| Ok(ResolvedForeground::default()))
                Children [{scene}]
            },
        )
        .map_err(BevyError::error)?;
        required(world.get_entity_mut(canvas).ok())?.add_child(entity);
        entity
    };
    Ok(entity)
}

fn size_canvas(
    world: &mut World,
    canvas: Entity,
    width: f32,
    height: f32,
) -> Result<(), BevyError> {
    let mut node = required(world.get_mut::<Node>(canvas))?;
    if node.width != px(width) {
        node.width = px(width);
    }
    if node.height != px(height) {
        node.height = px(height);
    }
    Ok(())
}
