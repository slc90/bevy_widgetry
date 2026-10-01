use crate::layout::TableGeometry;
use crate::view::{TableCanvas, TableDiagnostics};
use crate::*;
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::window::RequestRedraw;
use bevy_widgetry_core::scene::{apply_scene, spawn_scene};
use bevy_widgetry_core::{ColorTheme, ForegroundColor, ThemeMode};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::collections::HashMap;

/// Table 只记录自身 shell 引用；业务 source 和 value 不进入 physical cache。
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

/// Content rebuild 的唯一版本 key；position/style 修改不会调用 renderer。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct ContentVersion {
    row: u64,
    column: u64,
    generation: u64,
}

/// disabled 期间保存原 Pickable，恢复后精确还原用户配置。
#[derive(Component)]
struct DisabledPickable(Option<Pickable>);

/// Option 缺失在已确认 Table runtime 中属于 invariant，交由 root 诊断边界记录。
pub(crate) fn required<V>(value: Option<V>) -> Result<V, BevyError> {
    value.ok_or_else(|| BevyError::error("Table required runtime state missing"))
}

/// 从公开 BSN 固定四区解析 runtime；不构造第二棵 tree。
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

/// 各 Table 独立处理失败，一个错误不阻塞其他 source；重复异常去重日志但始终保留错误通道。
pub(crate) fn reconcile<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<WidgetryTable<T>>>()
        .iter(world)
        .collect();
    let mut failure = None;
    for root in roots {
        let result = reconcile_root::<T>(world, root);
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

/// 失败后清理已产生的 Content，保留固定 shell 与调用方 source，恢复时完整重建。
fn discard_projection(world: &mut World, root: Entity) {
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
}

/// Model、两种 renderer 和 per-view layout 共用一次 ordered projection，不建立 Row-centric hierarchy。
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
    let geometry = layout.resolve(ids, rows, measured.x)?;
    let disabled = world.get::<InteractionDisabled>(root).is_some();
    let style = required(world.get::<WidgetryTableStyle>(root))?.clone();
    let colors = *required(world.get_resource::<ThemeMode>())?.colors();
    let row_ids: Vec<_> = (0..rows)
        .map(|index| {
            required(world.get::<WidgetryTableModel<T>>(source))
                .and_then(|model| required(model.row_id(index)))
        })
        .collect::<Result<_, _>>()?;
    retain_shells(world, &mut runtime.cells, |(row, column)| {
        row_ids.contains(row) && geometry.columns.iter().any(|item| item.id == *column)
    });
    retain_shells(world, &mut runtime.column_headers, |id| {
        geometry.columns.iter().any(|item| item.id == *id)
    });
    retain_shells(world, &mut runtime.row_headers, |id| row_ids.contains(id));
    for column in &geometry.columns {
        let model = required(world.get::<WidgetryTableModel<T>>(source))?;
        let header = required(model.header(column.id))?.clone();
        let revision = required(model.column_revision(column.index))?;
        let registry = required(world.get_resource::<WidgetryTableHeaderRendererRegistry>())?;
        let version = ContentVersion {
            row: 0,
            column: revision,
            generation: registry.0.generation(header.type_id())?,
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
        world
            .entity_mut(entity)
            .insert(WidgetryTableColumnHeader { column: column.id });
        style_shell(world, entity, &style.column_header, &colors, disabled, true)?;
        runtime.column_headers.insert(column.id, entity);
    }
    for (index, row) in row_ids.iter().copied().enumerate() {
        let old = runtime.row_headers.get(&row).copied();
        let version = ContentVersion {
            row: index as u64,
            column: 0,
            generation: 0,
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
        world
            .entity_mut(entity)
            .insert(WidgetryTableRowHeader { row, index });
        style_shell(world, entity, &style.row_header, &colors, disabled, true)?;
        runtime.row_headers.insert(row, entity);
        for column in &geometry.columns {
            let model = required(world.get::<WidgetryTableModel<T>>(source))?;
            let value = required(model.cell(row, column.id))?;
            let registry = required(world.get_resource::<WidgetryTableCellRendererRegistry>())?;
            let version = ContentVersion {
                row: required(model.row_revision(index))?,
                column: required(model.column_revision(column.index))?,
                generation: registry.0.generation(value.type_id())?,
            };
            let old = runtime.cells.get(&(row, column.id)).copied();
            let scene = needs_content(world, old, version)
                .then(|| registry.0.scene(value.as_any()).map(|(_, scene)| scene))
                .transpose()?;
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
            world.entity_mut(entity).insert(WidgetryTableCell {
                row,
                column: column.id,
            });
            style_shell(world, entity, &style.cell, &colors, disabled, false)?;
            runtime.cells.insert((row, column.id), entity);
        }
    }
    {
        let mut node = required(world.get_mut::<Node>(root))?;
        node.grid_template_columns = vec![
            RepeatedGridTrack::px(1, layout.row_header_width),
            RepeatedGridTrack::flex(1, 1.0),
        ];
        node.grid_template_rows = vec![
            RepeatedGridTrack::px(1, layout.column_header_height),
            RepeatedGridTrack::flex(1, 1.0),
        ];
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
    style_shell(world, root, &style.table, &colors, disabled, true)?;
    style_shell(
        world,
        runtime.corner,
        &style.corner,
        &colors,
        disabled,
        true,
    )?;
    let previous = required(world.get::<ScrollPosition>(runtime.body))?.0;
    let offset = Vec2::new(
        clamp_offset(previous.x, geometry.width, measured.x),
        clamp_offset(previous.y, geometry.height, measured.y),
    );
    required(world.get_mut::<ScrollPosition>(runtime.body))?.0 = offset;
    required(world.get_mut::<ScrollPosition>(runtime.columns))?.0 = Vec2::new(offset.x, 0.0);
    required(world.get_mut::<ScrollPosition>(runtime.rows))?.0 = Vec2::new(0.0, offset.y);
    runtime.measured = measured;
    world.entity_mut(root).insert((runtime, geometry));
    project_disabled(world, root);
    Ok(())
}

/// 当帧 Layout 才能发现真实 viewport 变化；Reactive 宿主必须在尺寸不一致时继续推进求解。
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

/// 普通非法 scroll 输入修复为零；按完整 canvas 与 viewport 统一 clamp。
fn clamp_offset(offset: f32, total: f32, viewport: f32) -> f32 {
    if offset.is_finite() {
        offset.clamp(0.0, (total - viewport).max(0.0))
    } else {
        0.0
    }
}

/// 移除不再对应当前 ID 集合的 shell，identity 永不回指旧数据。
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

/// 只由内容版本驱动 renderer，style 或 position 更新不销毁 Content。
fn needs_content(world: &World, entity: Option<Entity>, version: ContentVersion) -> bool {
    entity.is_none_or(|entity| world.get::<ContentVersion>(entity) != Some(&version))
}

/// BSN 建立 shell 或替换它的 direct children；只有成功展开才推进 ContentVersion。
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
        world.entity_mut(entity).insert(node);
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

/// 尺寸由完整 Model 控制，绝对定位的 children 不缩短 scroll range。
fn size_canvas(
    world: &mut World,
    canvas: Entity,
    width: f32,
    height: f32,
) -> Result<(), BevyError> {
    let mut node = required(world.get_mut::<Node>(canvas))?;
    node.width = px(width);
    node.height = px(height);
    Ok(())
}

/// 解析当前 theme 与 explicit overrides，只更新 shell，不重建 renderer subtree。
fn style_shell(
    world: &mut World,
    entity: Entity,
    style: &WidgetryTableRegionStyle,
    colors: &ColorTheme,
    disabled: bool,
    header: bool,
) -> Result<(), BevyError> {
    let background = if disabled {
        style
            .disabled_background
            .unwrap_or(colors.control_background_disabled)
    } else {
        style.background.unwrap_or(if header {
            colors.control_background
        } else {
            Color::NONE
        })
    };
    let border = if disabled {
        style
            .disabled_border_color
            .unwrap_or(colors.control_border_disabled)
    } else {
        style.border_color.unwrap_or(colors.control_border)
    };
    let foreground = if disabled {
        style
            .disabled_foreground
            .unwrap_or(colors.foreground_disabled)
    } else {
        style.foreground.unwrap_or(colors.foreground)
    };
    required(world.get_mut::<BackgroundColor>(entity))?.set_if_neq(BackgroundColor(background));
    required(world.get_mut::<BorderColor>(entity))?.set_if_neq(BorderColor::all(border));
    if let Some(mut color) = world.get_mut::<Propagate<ForegroundColor>>(entity)
        && color.0 != ForegroundColor(foreground)
    {
        color.0 = ForegroundColor(foreground);
    }
    let mut node = required(world.get_mut::<Node>(entity))?;
    node.padding = style.padding;
    node.border = style.border;
    Ok(())
}

/// disabled 禁止整个自有 subtree 的 picking；保存并还原业务 Content 已有的 Pickable。
pub(crate) fn project_disabled(world: &mut World, root: Entity) {
    let disabled = world.get::<InteractionDisabled>(root).is_some();
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
        if disabled {
            if world.get::<DisabledPickable>(entity).is_none() {
                let saved = world.get::<Pickable>(entity).copied();
                world
                    .entity_mut(entity)
                    .insert((DisabledPickable(saved), Pickable::IGNORE));
            }
            if world
                .get::<Hovered>(entity)
                .is_some_and(|hovered| hovered.0)
            {
                world.entity_mut(entity).insert(Hovered(false));
            }
        } else if let Some(saved) = world.entity_mut(entity).take::<DisabledPickable>() {
            match saved.0 {
                Some(pickable) => {
                    world.entity_mut(entity).insert(pickable);
                }
                None => {
                    world.entity_mut(entity).remove::<Pickable>();
                }
            }
        }
    }
}
