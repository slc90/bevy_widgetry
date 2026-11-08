const ROW_PADDING_X: f32 = 8.0;
const ROW_BORDER_WIDTH: f32 = 1.0;
const ROW_CORNER_RADIUS: f32 = 3.0;

use crate::error::{RuntimeFault, core, failure};
use crate::source::DataSource;
use crate::view::{BottomSpacer, ListDiagnostics, TopSpacer, validate_source};
use crate::{WidgetryListKey, WidgetryListProjection};
use crate::{WidgetryListView, WidgetryListViewItem};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::ListItem;
use bevy::window::RequestRedraw;
use bevy_widgetry_core::disabled::set_intrinsic_disabled;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::scene::{SceneFailure, attempt};
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::WidgetryEcsError};
use std::sync::Arc;

use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
use std::ops::Range;

#[derive(Component, Clone)]
pub(crate) struct ListRuntime {
    pub(crate) viewport: Entity,
    pub(crate) content: Entity,
    top: Entity,
    bottom: Entity,
    pub(crate) range: Range<usize>,
    pub(crate) rows: Vec<Entity>,
}

#[derive(Component)]
struct RenderedRevision(Option<u64>);

pub(crate) fn visible_range(
    len: usize,
    height: f32,
    viewport: f32,
    offset: f32,
) -> (f32, Range<usize>) {
    if !viewport.is_finite() || viewport <= 0.0 {
        return (0.0, 0..0);
    }
    let maximum = (len as f32 * height - viewport).max(0.0);
    let offset = if offset.is_finite() {
        offset.clamp(0.0, maximum)
    } else {
        0.0
    };
    let start = ((offset / height).floor() as usize).min(len);
    let end = (((offset + viewport) / height).ceil() as usize).min(len);
    (offset, start..end)
}

enum RuntimeFailure<K: WidgetryListKey> {
    Fault(RuntimeFault<K>),
    External(BevyError),
}
impl<K: WidgetryListKey> From<RuntimeFault<K>> for RuntimeFailure<K> {
    fn from(fault: RuntimeFault<K>) -> Self {
        Self::Fault(fault)
    }
}
fn part<T, K: WidgetryListKey>(
    value: Option<T>,
    entity: Entity,
    component: &'static str,
) -> Result<T, RuntimeFailure<K>> {
    value.ok_or_else(|| RuntimeFault::Part { entity, component }.into())
}

fn runtime<K: WidgetryListKey>(
    world: &World,
    root: Entity,
) -> Result<ListRuntime, RuntimeFailure<K>> {
    let viewport = part(
        world.get::<Children>(root).and_then(|children| {
            children
                .iter()
                .find(|child| world.get::<WidgetryScrollAreaViewport>(*child).is_some())
        }),
        root,
        "WidgetryScrollAreaViewport",
    )?;
    let content = part(
        world.get::<Children>(viewport).and_then(|children| {
            children
                .iter()
                .find(|child| world.get::<WidgetryScrollAreaContent>(*child).is_some())
        }),
        viewport,
        "WidgetryScrollAreaContent",
    )?;
    let children = world.get::<Children>(content);
    Ok(ListRuntime {
        viewport,
        content,
        top: part(
            children.and_then(|children| {
                children
                    .iter()
                    .find(|child| world.get::<TopSpacer>(*child).is_some())
            }),
            content,
            "TopSpacer",
        )?,
        bottom: part(
            children.and_then(|children| {
                children
                    .iter()
                    .find(|child| world.get::<BottomSpacer>(*child).is_some())
            }),
            content,
            "BottomSpacer",
        )?,
        range: 0..0,
        rows: Vec::new(),
    })
}

fn runtime_fault_key<K: WidgetryListKey>(
    world: &World,
    root: Entity,
    fault: RuntimeFault<K>,
) -> RuntimeFault<K> {
    let RuntimeFault::Part { component, .. } = fault else {
        return fault;
    };
    let cached = world.get::<ListRuntime>(root);
    let viewport = || {
        cached.map(|runtime| runtime.viewport).or_else(|| {
            world
                .get::<Children>(root)?
                .iter()
                .find(|child| world.get::<WidgetryScrollAreaViewport>(*child).is_some())
        })
    };
    let content = || {
        cached.map(|runtime| runtime.content).or_else(|| {
            world
                .get::<Children>(viewport()?)?
                .iter()
                .find(|child| world.get::<WidgetryScrollAreaContent>(*child).is_some())
        })
    };
    // Cached validation names the child and its missing output; discovery only knows the parent and slot.
    // Keep that slot's identity across cleanup while retaining the first diagnostic's actual entity and cause.
    let (entity, component) = match component {
        "WidgetryScrollAreaViewport" => (root, component),
        "WidgetryScrollAreaContent" => (viewport().unwrap_or(root), component),
        "TopSpacer" | "TopSpacer Node" => (content().unwrap_or(root), "TopSpacer"),
        "BottomSpacer" | "BottomSpacer Node" => (content().unwrap_or(root), "BottomSpacer"),
        _ => return fault,
    };
    RuntimeFault::Part { entity, component }
}

pub(crate) fn reconcile<T: Send + Sync + 'static, K: WidgetryListKey>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T, K>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        if let Err(error) = validate_source::<T, K>(world, root) {
            clear_rows(world, root);
            world
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> {
                    Err(error.into_bevy_error())
                });
            continue;
        }
        let error = match reconcile_root::<T, K>(world, root) {
            Ok(valid_layout) => {
                if valid_layout
                    && let Some(mut diagnostics) = world.get_mut::<ListDiagnostics<K>>(root)
                {
                    diagnostics
                        .runtime
                        .recover(|| ErrorFrame::new("list_view.reconcile").field("view", root));
                }
                None
            }
            Err(RuntimeFailure::Fault(fault)) => {
                let key = runtime_fault_key(world, root, fault);
                let create = || -> Arc<dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
                    let mut frame = ErrorFrame::new("list_view.reconcile").field("view", root);
                    if let RuntimeFault::RowState {
                        source,
                        key,
                        index,
                        row,
                        revision,
                        ..
                    } = fault
                    {
                        frame = frame
                            .field("source", source)
                            .field("key", format_args!("{key:?}"))
                            .field("key_type", std::any::type_name::<K>())
                            .field("index", index)
                            .field("row", row)
                            .field("revision", format_args!("{revision:?}"));
                    }
                    Arc::new(
                        failure(fault.into_error::<T>(root), "list_view.reconcile")
                            .with_context(frame),
                    )
                };
                let error = match world.get_mut::<ListDiagnostics<K>>(root) {
                    Some(mut diagnostics) => diagnostics.runtime.failure(key, create),
                    None => WidgetryEcsError::from_shared(create()),
                };
                Some(error.into_bevy_error())
            }
            Err(RuntimeFailure::External(error)) => Some(error),
        };
        if let Some(error) = error {
            clear_rows(world, root);
            world
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(error) });
        }
    }
}

fn reconcile_root<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
) -> Result<bool, RuntimeFailure<K>> {
    let bootstrap = world.get::<ListRuntime>(root).is_none();
    let mut runtime = world
        .get::<ListRuntime>(root)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| runtime::<K>(world, root))?;
    let view = part(
        world.get::<WidgetryListView<T, K>>(root),
        root,
        "WidgetryListView",
    )?;
    let source = view.source();
    let height = view.item_height();
    let renderer = view.renderer().clone();
    let entity = part(world.get_entity(root).ok(), root, "Entity")?;
    let projection = entity
        .get_ref::<WidgetryListProjection<K>>()
        .ok_or(RuntimeFault::Source {
            source,
            reason: crate::ListSourceRejection::CannotEnumerate,
        })?;
    let len = projection.len();
    let changed = projection.is_changed();
    let adapter = *part(world.get_resource::<DataSource<T, K>>(), root, "DataSource")?;
    if let Some((key, index, previous_index)) = projection.duplicate {
        return Err(RuntimeFault::Duplicate {
            source,
            key,
            index,
            previous_index,
        }
        .into());
    }
    if !(len as f32 * height).is_finite() {
        return Err(RuntimeFault::Height {
            source,
            len,
            height_bits: height.to_bits(),
        }
        .into());
    }
    part(
        world.get::<WidgetryScrollAreaViewport>(runtime.viewport),
        runtime.viewport,
        "WidgetryScrollAreaViewport",
    )?;
    part(
        world.get::<WidgetryScrollAreaContent>(runtime.content),
        runtime.content,
        "WidgetryScrollAreaContent",
    )?;
    part(
        world.get::<Node>(runtime.top),
        runtime.top,
        "TopSpacer Node",
    )?;
    part(
        world.get::<Node>(runtime.bottom),
        runtime.bottom,
        "BottomSpacer Node",
    )?;
    let computed = part(
        world.get::<ComputedNode>(runtime.viewport),
        runtime.viewport,
        "ComputedNode",
    )?;
    let viewport = computed.size().y * computed.inverse_scale_factor();
    let previous_offset = part(
        world.get::<ScrollPosition>(runtime.viewport),
        runtime.viewport,
        "ScrollPosition",
    )?
    .0
    .y;
    let (offset, range) = visible_range(len, height, viewport, previous_offset);
    if viewport.is_finite() && viewport > 0.0 && offset != previous_offset {
        part(
            world.get_mut::<ScrollPosition>(runtime.viewport),
            runtime.viewport,
            "ScrollPosition",
        )?
        .0
        .y = offset;
    }
    let range_changed = runtime.range != range;
    for (index, entity) in runtime.range.clone().zip(runtime.rows.iter().copied()) {
        if !range.contains(&index) {
            world.despawn(entity);
        }
    }
    let mut rows = Vec::with_capacity(range.len());
    for index in range.clone() {
        let id = world
            .get::<WidgetryListProjection<K>>(root)
            .and_then(|projection| projection.id(index))
            .ok_or(RuntimeFault::Index { source, index, len })?;
        let row = (adapter.row)(world, source, id).ok_or(RuntimeFault::Row {
            source,
            key: id,
            index,
        })?;
        let revision = row.revision;
        let disabled = row.disabled;
        let old = if runtime.range.contains(&index) {
            Some(runtime.rows[index - runtime.range.start])
        } else {
            None
        };
        let replace = match old {
            None => true,
            Some(entity) => {
                let missing = |component| RuntimeFault::RowState {
                    source,
                    key: id,
                    index,
                    row: entity,
                    revision,
                    component,
                };
                let item = world
                    .get::<WidgetryListViewItem<K>>(entity)
                    .ok_or_else(|| missing("WidgetryListViewItem"))?;
                let rendered = world
                    .get::<RenderedRevision>(entity)
                    .ok_or_else(|| missing("RenderedRevision"))?;
                (changed || range_changed)
                    && (item.id != id || rendered.0 != revision || revision.is_none())
            }
        };
        let children = if replace {
            Some(renderer.render(row.value.as_ref()).map_err(|error| {
                RuntimeFailure::External(
                    error
                        .with_context(row_frame(
                            root,
                            source,
                            id,
                            index,
                            None,
                            &range,
                            revision,
                            "list_view.render",
                        ))
                        .into_bevy_error(),
                )
            })?)
        } else {
            None
        };
        let entity = match old {
            Some(entity) => {
                if !world.entities().contains(entity) {
                    return Err(RuntimeFault::Part {
                        entity,
                        component: "row Entity",
                    }
                    .into());
                }
                if let Some(children) = children {
                    let old_children = world
                        .get::<Children>(entity)
                        .map(|children| children.iter().collect::<Vec<_>>())
                        .unwrap_or_default();
                    for child in old_children {
                        world.despawn(child);
                    }
                    if let Err(error) = attempt::apply_scene(
                        &mut world.entity_mut(entity),
                        bsn! { Children [{children}] },
                    ) {
                        return Err(RuntimeFailure::External(renderer_failure(
                            root,
                            error,
                            row_frame(
                                root,
                                source,
                                id,
                                index,
                                Some(entity),
                                &range,
                                revision,
                                "list_view.row_update",
                            ),
                        )));
                    }
                    world.entity_mut(entity).insert((
                        WidgetryListViewItem { id, index },
                        RenderedRevision(revision),
                    ));
                }
                entity
            }
            None => {
                let children = part(children, root, "renderer children")?;
                let scene = bsn! {
                    ListItem Hovered::default()
                    template(|_| Ok(bevy::picking::cursor::EntityCursor::default()))
                    template(move |_| Ok(crate::style::RowOwner(root)))
                    template(move |_| Ok(WidgetryListViewItem {id,index}))
                    template(move |_| Ok(RenderedRevision(revision)))
                    BackgroundColor::default() BorderColor::default()
                    template(|_| Ok(ResolvedForeground::default()))
                    Node {
                        width: percent(100), height: px(height), min_height: px(height), max_height: px(height),
                        box_sizing: BoxSizing::BorderBox, flex_shrink: 0.0, margin: UiRect::ZERO,
                        padding: UiRect::horizontal(px(ROW_PADDING_X)), border: UiRect::all(px(ROW_BORDER_WIDTH)), border_radius: BorderRadius::all(px(ROW_CORNER_RADIUS)),
                    }
                    Children [{children}]
                };
                let mut row = world.spawn(crate::style::RowOwner(root));
                let entity = row.id();
                let result = attempt::apply_scene(&mut row, scene);
                if let Err(error) = result {
                    world.despawn(entity);
                    return Err(RuntimeFailure::External(renderer_failure(
                        root,
                        error,
                        row_frame(
                            root,
                            source,
                            id,
                            index,
                            Some(entity),
                            &range,
                            revision,
                            "list_view.row_spawn",
                        ),
                    )));
                }
                entity
            }
        };
        if world.get::<ChildOf>(entity).is_none() {
            world.entity_mut(runtime.content).add_child(entity);
        }
        if replace {
            crate::style::invalidate_row(world, root, entity);
        }
        set_intrinsic_disabled(world, entity, disabled).map_err(|error| {
            RuntimeFailure::External(
                core(error, root, "list_view.project_disabled")
                    .with_context(row_frame(
                        root,
                        source,
                        id,
                        index,
                        Some(entity),
                        &range,
                        revision,
                        "list_view.project_disabled",
                    ))
                    .into_bevy_error(),
            )
        })?;
        rows.push(entity);
    }
    for (entity, extent, component) in [
        (runtime.top, range.start as f32 * height, "TopSpacer Node"),
        (
            runtime.bottom,
            (len - range.end) as f32 * height,
            "BottomSpacer Node",
        ),
    ] {
        let mut node = part(world.get_mut::<Node>(entity), entity, component)?;
        if node.height != px(extent) {
            node.height = px(extent);
        }
    }
    if range_changed {
        let mut order = Vec::with_capacity(rows.len() + 2);
        order.push(runtime.top);
        order.extend(rows.iter().copied());
        order.push(runtime.bottom);
        world.entity_mut(runtime.content).replace_children(&order);
    }
    runtime.range = range;
    runtime.rows = rows;
    world.entity_mut(root).insert(runtime);
    // Request another frame after bootstrap so the first valid layout dimensions enter the same update path.
    if bootstrap || range_changed || changed {
        world.write_message(RequestRedraw);
    }

    Ok(viewport.is_finite() && viewport > 0.)
}

fn renderer_failure(
    root: Entity,
    error: bevy::scene::SpawnSceneError,
    frame: ErrorFrame,
) -> BevyError {
    SceneFailure::from_native(error, root, "list_view.renderer")
        .with_context(frame)
        .into_bevy_error()
}

fn row_frame<K: WidgetryListKey>(
    root: Entity,
    source: Entity,
    key: K,
    index: usize,
    row: Option<Entity>,
    range: &Range<usize>,
    revision: Option<u64>,
    operation: &'static str,
) -> ErrorFrame {
    ErrorFrame::new(operation)
        .field("view", root)
        .field("source", source)
        .field("key", format_args!("{key:?}"))
        .field("key_type", std::any::type_name::<K>())
        .field("index", index)
        .field("row", format_args!("{row:?}"))
        .field("range", format_args!("{range:?}"))
        .field("revision", format_args!("{revision:?}"))
}

pub(crate) fn clear_rows(world: &mut World, root: Entity) {
    let content = world
        .get::<ListRuntime>(root)
        .map(|runtime| runtime.content)
        .or_else(|| {
            let viewport = world
                .get::<Children>(root)?
                .iter()
                .find(|&child| world.get::<WidgetryScrollAreaViewport>(child).is_some())?;
            world
                .get::<Children>(viewport)?
                .iter()
                .find(|&child| world.get::<WidgetryScrollAreaContent>(child).is_some())
        });
    if let Some(content) = content {
        let rows = world
            .get::<Children>(content)
            .map(|children| {
                children
                    .iter()
                    .filter(|&child| {
                        world
                            .get::<crate::style::RowOwner>(child)
                            .is_some_and(|owner| owner.0 == root)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for row in rows {
            world.despawn(row);
        }
    }
    if let Ok(mut root) = world.get_entity_mut(root) {
        root.remove::<ListRuntime>();
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn missing_cached_revision_is_reported_with_row_identity_before_retry() {
        let _guard = crate::LOG_CAPTURE_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(crate::WidgetryListViewPlugin)
            .set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        crate::WidgetryListViewAppExt::register_widgetry_list_view::<u32>(&mut app).unwrap();
        let mut model = crate::WidgetryListModel::default();
        let key = model.push(7u32).unwrap();
        let source = app.world_mut().spawn(model).id();
        let root = bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn!{
            @WidgetryListView::<u32> {@source:source,@renderer:{crate::WidgetryListViewRenderer::new(|_:&u32| bsn_list!{Node})}}
        }).unwrap();
        let viewport = app
            .world()
            .get::<Children>(root)
            .unwrap()
            .iter()
            .find(|child| {
                app.world()
                    .get::<WidgetryScrollAreaViewport>(*child)
                    .is_some()
            })
            .unwrap();
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::splat(100.),
            inverse_scale_factor: 1.,
            ..default()
        });
        app.update();
        let row = app.world().get::<ListRuntime>(root).unwrap().rows[0];
        app.world_mut().entity_mut(row).remove::<RenderedRevision>();
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        errors.run(|| app.update());
        let failures = errors.take();
        assert_eq!(failures.len(), 1);
        let diagnostic = failures[0]
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(diagnostic.contexts().iter().any(|frame| {
            frame
                .fields()
                .contains(&("source".to_owned(), source.to_string()))
                && frame
                    .fields()
                    .contains(&("key".to_owned(), format!("{key:?}")))
        }));
        assert!(app.world().get_entity(row).is_err());
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        assert_eq!(app.world().get::<ListRuntime>(root).unwrap().rows.len(), 1);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn finite_ranges_stay_within_content(
            len in 0usize..10_001, height in 1u16..129, viewport in 1u16..1025, offset in -10_000i32..2_000_000
        ) {
        let _guard = crate::LOG_CAPTURE_LOCK.lock().unwrap_or_else(|error| error.into_inner());
            let height = f32::from(height) / 4.0;
            let viewport = f32::from(viewport) / 4.0;
            let (offset, range) = visible_range(len, height, viewport, offset as f32 / 4.0);
            prop_assert!(offset.is_finite() && offset >= 0.0);
            prop_assert!(offset <= (len as f32 * height - viewport).max(0.0));
            prop_assert!(range.start <= range.end && range.end <= len);
        }
    }

    #[test]
    fn invalid_offsets_start_at_zero() {
        let _guard = crate::LOG_CAPTURE_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for offset in [-1.0, f32::NEG_INFINITY, f32::INFINITY, f32::NAN] {
            assert_eq!(visible_range(10, 10.0, 21.0, offset), (0.0, 0..3));
        }
    }

    #[test]
    fn range_handles_boundaries_and_shrink() {
        let _guard = crate::LOG_CAPTURE_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(visible_range(100, 10.0, 21.0, 0.0), (0.0, 0..3));
        assert_eq!(visible_range(100, 10.0, 20.0, 0.1), (0.1, 0..3));
        assert_eq!(visible_range(100, 10.0, 20.0, 990.0), (980.0, 98..100));
        assert_eq!(visible_range(2, 10.0, 20.0, 990.0), (0.0, 0..2));
        assert_eq!(visible_range(0, 10.0, 20.0, 990.0), (0.0, 0..0));
        for viewport in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(visible_range(100, 10.0, viewport, 5.0), (0.0, 0..0));
        }
    }
}
