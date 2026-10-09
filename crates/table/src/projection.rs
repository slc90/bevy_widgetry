use crate::layout::TableGeometry;
use crate::view::{TableCanvas, TableDiagnostics};
use crate::viewport::VisibleCells;
use crate::*;
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition, Selected};
use bevy::window::RequestRedraw;
use bevy_widgetry_core::ForegroundColor;
use bevy_widgetry_core::scene::{apply_scene, spawn_scene};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use bevy_widgetry_theme::WidgetryThemeMode;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};

#[derive(Component, Clone)]
pub(crate) struct TableRuntime {
    pub(crate) body: Entity,
    columns: Entity,
    rows: Entity,
    corner: Entity,
    pub(crate) body_canvas: Entity,
    column_canvas: Entity,
    row_canvas: Entity,
    cells: HashMap<(WidgetryTableRowId, WidgetryTableColumnId), Entity>,
    column_headers: HashMap<WidgetryTableColumnId, Entity>,
    row_headers: HashMap<WidgetryTableRowId, Entity>,
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

fn runtime(world: &World, root: Entity) -> Result<TableRuntime, BevyError> {
    let children = required(world.get::<Children>(root))?;
    let part = |matches: fn(&World, Entity) -> bool| {
        required(children.iter().find(|&entity| matches(world, entity)))
    };
    let body = part(|world, entity| world.get::<WidgetryTableBody>(entity).is_some())?;
    let columns = part(|world, entity| world.get::<WidgetryTableColumnHeaders>(entity).is_some())?;
    let rows = part(|world, entity| world.get::<WidgetryTableRowHeaders>(entity).is_some())?;
    let corner = part(|world, entity| world.get::<WidgetryTableCorner>(entity).is_some())?;
    let canvas = |entity| {
        required(world.get::<Children>(entity).and_then(|children| {
            children
                .iter()
                .find(|&entity| world.get::<TableCanvas>(entity).is_some())
        }))
    };
    Ok(TableRuntime {
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
        // 语义 event 回调销毁 root 属于正常 lifecycle，不再检查它的诊断 state。
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
    // resize terminal observer 可以通过 Commands 修改 Axis 或销毁 root。
    // 先完成回调再读取 Model，避免使用回调前失效的 ID 或 count。
    let model = world.get::<WidgetryTableModel<T>>(source).ok_or_else(|| {
        BevyError::error("Table requires a live matching WidgetryTableModel source")
    })?;
    let ids: Vec<_> = (0..model.column_count())
        .map(|index| required(model.column_id(index)))
        .collect::<Result<_, _>>()?;
    let rows = model.row_count();
    let layout = required(world.get::<WidgetryTableLayout>(root))?.clone();
    let computed = required(world.get::<ComputedNode>(runtime.body))?;
    let measured = computed.size() * computed.inverse_scale_factor();
    let inverse = computed.inverse_scale_factor();
    let geometry = layout.resolve(ids, rows, measured.x)?;
    crate::interaction::sync::<T>(world, root, source, runtime.body, &geometry, measured)?;
    let previous = required(world.get::<ScrollPosition>(runtime.body))?.0;
    let offset = Vec2::new(
        clamp_offset(previous.x, geometry.width, measured.x),
        clamp_offset(previous.y, geometry.height, measured.y),
    );
    // 官方 Layout 将 physical scroll 向下取整。
    // canvas 几何不取整，仍须使用同一实际 offset。
    let physical_offset = (offset / inverse).floor() * inverse;
    let visible = VisibleCells::new(&geometry, rows, physical_offset, measured);
    let disabled = world.get::<InteractionDisabled>(root).is_some();
    let style = required(world.get::<WidgetryTableStyle>(root))?.clone();
    let colors = required(world.get_resource::<WidgetryThemeMode>())?.colors();
    let header_rows =
        VisibleCells::new(&geometry, rows, physical_offset, Vec2::new(1.0, measured.y)).rows;
    let row_ids: Vec<_> = header_rows
        .map(|index| {
            required(world.get::<WidgetryTableModel<T>>(source))
                .and_then(|model| required(model.row_id(index)))
                .map(|id| (index, id))
        })
        .collect::<Result<_, _>>()?;
    let current_rows: HashSet<_> = row_ids.iter().map(|(_, row)| *row).collect();
    let header_columns =
        VisibleCells::new(&geometry, rows, physical_offset, Vec2::new(measured.x, 1.0)).columns;
    let current_columns: HashSet<_> = geometry.columns[header_columns.clone()]
        .iter()
        .map(|column| column.id)
        .collect();
    retain_shells(world, &mut runtime.cells, |(row, column)| {
        !visible.rows.is_empty()
            && current_rows.contains(row)
            && geometry.columns[visible.columns.clone()]
                .iter()
                .any(|item| item.id == *column)
    });
    retain_shells(world, &mut runtime.column_headers, |id| {
        current_columns.contains(id)
    });
    retain_shells(world, &mut runtime.row_headers, |id| {
        current_rows.contains(id)
    });
    for column in &geometry.columns[header_columns] {
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
        let scene = needs_content(world, old, version)
            .then(|| registry.0.scene(header.as_any()).map(|(_, scene)| scene))
            .transpose()?;
        let entity = shell(
            world,
            runtime.column_canvas,
            old,
            version,
            scene,
            column.left,
            0.0,
            column.width,
            layout.column_header_height,
        )?;
        let identity = WidgetryTableColumnHeader { column: column.id };
        if world.get::<WidgetryTableColumnHeader>(entity) != Some(&identity) {
            world.entity_mut(entity).insert(identity);
        }
        style_shell(
            world,
            root,
            entity,
            &style.column_header,
            &colors.table.column_header,
            disabled,
        )?;
        runtime.column_headers.insert(column.id, entity);
        crate::resize::ensure_handle(world, entity, column.id)?;
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
            Box::new(bsn_list![(Text({ (index + 1).to_string() }))]) as Box<dyn SceneList>
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
        style_shell(
            world,
            root,
            entity,
            &style.row_header,
            &colors.table.row_header,
            disabled,
        )?;
        runtime.row_headers.insert(row, entity);
        if !visible.rows.contains(&index) {
            continue;
        }
        for column in &geometry.columns[visible.columns.clone()] {
            let model = required(world.get::<WidgetryTableModel<T>>(source))?;
            let registry = required(world.get_resource::<WidgetryTableCellRendererRegistry>())?;
            let old = runtime.cells.get(&(row, column.id)).copied();
            let row_revision = required(model.row_revision(index))?;
            let column_revision = required(model.column_revision(column.index))?;
            let cached = old
                .and_then(|entity| world.get::<ContentVersion>(entity))
                .copied();
            let unchanged = if let Some(cached) = cached {
                cached.row == row_revision
                    && cached.column == column_revision
                    && cached.generation == registry.0.generation(cached.value_type)?
            } else {
                false
            };
            let (version, scene) = if unchanged {
                (required(cached)?, None)
            } else {
                let value = required(model.cell_at(index, column.index))?;
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
            let entity = shell(
                world,
                runtime.body_canvas,
                old,
                version,
                scene,
                column.left,
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
            style_shell(
                world,
                root,
                entity,
                &style.cell,
                &colors.table.cell,
                disabled,
            )?;
            runtime.cells.insert((row, column.id), entity);
        }
    }
    {
        let mut node = required(world.get_mut::<Node>(root))?;
        let columns = vec![
            RepeatedGridTrack::px(1, layout.row_header_width),
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
    size_canvas(world, runtime.body_canvas, geometry.width, geometry.height)?;
    size_canvas(
        world,
        runtime.column_canvas,
        geometry.width,
        layout.column_header_height,
    )?;
    size_canvas(
        world,
        runtime.row_canvas,
        layout.row_header_width,
        geometry.height,
    )?;
    style_shell(
        world,
        root,
        root,
        &style.table,
        &colors.table.table,
        disabled,
    )?;
    style_shell(
        world,
        root,
        runtime.corner,
        &style.corner,
        &colors.table.corner,
        disabled,
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
    runtime.measured = measured;
    world.entity_mut(root).insert((runtime, geometry));
    crate::resize::sync::<T>(world, root, source);
    Ok(())
}

// Header replacement 或横轴回收会销毁当前 resize handle。
// 先结束 gesture 并 flush callback，再读取 Model/layout，避免按回调前的几何继续 projection。
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
    let rows = model.row_count();
    let computed = required(world.get::<ComputedNode>(runtime.body))?;
    let measured = computed.size() * computed.inverse_scale_factor();
    let inverse = computed.inverse_scale_factor();
    let geometry =
        required(world.get::<WidgetryTableLayout>(root))?.resolve(ids, rows, measured.x)?;
    // 同时消费 keyboard reveal，确保后续 projection 不会再将当前 handle 滚出 viewport。
    crate::interaction::sync::<T>(world, root, source, runtime.body, &geometry, measured)?;
    let previous = required(world.get::<ScrollPosition>(runtime.body))?.0;
    let offset = Vec2::new(
        clamp_offset(previous.x, geometry.width, measured.x),
        clamp_offset(previous.y, geometry.height, measured.y),
    );
    let physical_offset = (offset / inverse).floor() * inverse;
    let visible = VisibleCells::new(&geometry, rows, physical_offset, Vec2::new(measured.x, 1.0));
    if !visible.columns.contains(&index) {
        crate::resize::cancel(world, root);
    }
    Ok(())
}

// viewport 的真实尺寸到当帧 Layout 才可见。
// 求解尺寸与 measurement 不一致时请求下一帧，避免 Reactive App 停在旧 projection。
pub(crate) fn request_geometry_redraw<T: Send + Sync + 'static>(
    roots: Query<&TableRuntime, With<WidgetryTable<T>>>,
    bodies: Query<&ComputedNode, With<WidgetryTableBody>>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    for runtime in &roots {
        if let Ok(computed) = bodies.get(runtime.body)
            && computed.size() * computed.inverse_scale_factor() != runtime.measured
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
        overflow: Overflow::clip(),
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
        // shell geometry 与 style 共用 Node。
        // 保留上次 style，避免每帧先清零再写回。
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
                template(move |_| Ok(version))
                template(move |_| Ok(node.clone()))
                template(|_| Ok(Propagate(ForegroundColor::default())))
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

fn style_shell(
    world: &mut World,
    root: Entity,
    entity: Entity,
    style: &WidgetryTableRegionStyle,
    colors: &bevy_widgetry_theme::WidgetryTableRegionColors,
    disabled: bool,
) -> Result<(), BevyError> {
    let (selected, focused) = crate::interaction::appearance(world, root, entity);
    if selected {
        if world.get::<Selected>(entity).is_none() {
            world.entity_mut(entity).insert(Selected);
        }
    } else if world.get::<Selected>(entity).is_some() {
        world.entity_mut(entity).remove::<Selected>();
    }
    let hovered = world
        .get::<Hovered>(entity)
        .is_some_and(|hovered| hovered.0);
    let background = if disabled {
        style
            .disabled_background
            .unwrap_or(colors.disabled.background)
    } else if hovered {
        style
            .hovered_background
            .unwrap_or(colors.hovered.background)
    } else if selected {
        style
            .selected_background
            .unwrap_or(colors.selected.background)
    } else {
        style.background.unwrap_or(colors.normal.background)
    };
    let border = if disabled {
        style
            .disabled_border_color
            .unwrap_or(colors.disabled.border)
    } else if focused {
        style.focused_border_color.unwrap_or(colors.focused_border)
    } else {
        style.border_color.unwrap_or(colors.normal.border)
    };
    let foreground = if disabled {
        style
            .disabled_foreground
            .unwrap_or(colors.disabled.foreground)
    } else {
        style.foreground.unwrap_or(colors.normal.foreground)
    };
    required(world.get_mut::<BackgroundColor>(entity))?.set_if_neq(BackgroundColor(background));
    required(world.get_mut::<BorderColor>(entity))?.set_if_neq(BorderColor::all(border));
    if let Some(mut color) = world.get_mut::<Propagate<ForegroundColor>>(entity)
        && color.0 != ForegroundColor(foreground)
    {
        color.0 = ForegroundColor(foreground);
    }
    let mut node = required(world.get_mut::<Node>(entity))?;
    if node.padding != style.padding {
        node.padding = style.padding;
    }
    if node.border != style.border {
        node.border = style.border;
    }
    Ok(())
}
