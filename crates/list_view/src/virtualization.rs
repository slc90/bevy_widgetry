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
use bevy_widgetry_core::scene::{apply_scene, spawn_scene};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
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

fn invariant<T>(value: Option<T>) -> Result<T, BevyError> {
    value.ok_or_else(|| BevyError::error("ListView runtime invariant failed"))
}

fn runtime(world: &World, root: Entity) -> Result<ListRuntime, BevyError> {
    let viewport = invariant(world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|child| world.get::<WidgetryScrollAreaViewport>(*child).is_some())
    }))?;
    let content = invariant(world.get::<Children>(viewport).and_then(|children| {
        children
            .iter()
            .find(|child| world.get::<WidgetryScrollAreaContent>(*child).is_some())
    }))?;
    let children = invariant(world.get::<Children>(content))?;
    Ok(ListRuntime {
        viewport,
        content,
        top: invariant(
            children
                .iter()
                .find(|child| world.get::<TopSpacer>(*child).is_some()),
        )?,
        bottom: invariant(
            children
                .iter()
                .find(|child| world.get::<BottomSpacer>(*child).is_some()),
        )?,
        range: 0..0,
        rows: Vec::new(),
    })
}

pub(crate) fn reconcile<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T, K>>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if let Err(error) = validate_source::<T, K>(world, root) {
            clear_rows(world, root);
            if failure.is_none() {
                failure = Some(error);
            }
            continue;
        }
        let result = reconcile_root::<T, K>(world, root);
        let result = if let Some(mut diagnostics) = world.get_mut::<ListDiagnostics>(root) {
            diagnostics.runtime.observe(
                result,
                |error| widgetry_error!(?root, %error, "ListView runtime reconciliation 失败"),
                || widgetry_info!(?root, "ListView runtime 恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result {
            clear_rows(world, root);
            if failure.is_none() {
                failure = Some(error);
            }
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn reconcile_root<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
) -> Result<(), BevyError> {
    let bootstrap = world.get::<ListRuntime>(root).is_none();
    let mut runtime = world
        .get::<ListRuntime>(root)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| runtime(world, root))?;
    let view = invariant(world.get::<WidgetryListView<T, K>>(root))?;
    let source = view.source();
    let height = view.item_height();
    let renderer = view.renderer().clone();
    let projection =
        invariant(invariant(world.get_entity(root).ok())?.get_ref::<WidgetryListProjection<K>>())?;
    let len = projection.len();
    let changed = projection.is_changed();
    let adapter = *world.resource::<DataSource<T, K>>();
    if !(len as f32 * height).is_finite() {
        return Err(BevyError::error("ListView total height must be finite"));
    }
    let computed = invariant(world.get::<ComputedNode>(runtime.viewport))?;
    let viewport = computed.size().y * computed.inverse_scale_factor();
    let previous_offset = invariant(world.get::<ScrollPosition>(runtime.viewport))?
        .0
        .y;
    let (offset, range) = visible_range(len, height, viewport, previous_offset);
    if viewport.is_finite() && viewport > 0.0 && offset != previous_offset {
        invariant(world.get_mut::<ScrollPosition>(runtime.viewport))?
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
        let id = invariant(
            world
                .get::<WidgetryListProjection<K>>(root)
                .and_then(|projection| projection.id(index)),
        )?;
        let row = invariant((adapter.row)(world, source, id))?;
        let revision = row.revision;
        let disabled = row.disabled;
        let old = if runtime.range.contains(&index) {
            Some(runtime.rows[index - runtime.range.start])
        } else {
            None
        };
        let replace = old.is_none_or(|row| {
            (changed || range_changed)
                && (world
                    .get::<WidgetryListViewItem<K>>(row)
                    .is_none_or(|item| item.id != id)
                    || world
                        .get::<RenderedRevision>(row)
                        .is_none_or(|old| old.0 != revision || revision.is_none()))
        });
        let children = if replace {
            Some(renderer.render(row.value.as_ref())?)
        } else {
            None
        };
        let entity = match old {
            Some(entity) => {
                if let Some(children) = children {
                    let old_children = world
                        .get::<Children>(entity)
                        .map(|children| children.iter().collect::<Vec<_>>())
                        .unwrap_or_default();
                    for child in old_children {
                        world.despawn(child);
                    }
                    if let Err(error) = apply_scene(
                        &mut world.entity_mut(entity),
                        bsn! { Children [{children}] },
                    ) {
                        return Err(BevyError::error(error));
                    }
                    world.entity_mut(entity).insert((
                        WidgetryListViewItem { id, index },
                        RenderedRevision(revision),
                    ));
                }
                entity
            }
            None => {
                let children = invariant(children)?;
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
                        padding: UiRect::horizontal(px(8)), border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(3)),
                    }
                    Children [{children}]
                };
                spawn_scene(world, scene).map_err(BevyError::error)?
            }
        };
        if world.get::<ChildOf>(entity).is_none() {
            world.entity_mut(runtime.content).add_child(entity);
        }
        if replace {
            crate::style::invalidate_row(world, root, entity);
        }
        set_intrinsic_disabled(world, entity, disabled);
        rows.push(entity);
    }
    for (entity, extent) in [
        (runtime.top, range.start as f32 * height),
        (runtime.bottom, (len - range.end) as f32 * height),
    ] {
        let mut node = invariant(world.get_mut::<Node>(entity))?;
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

    Ok(())
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
                        world.get::<TopSpacer>(child).is_none()
                            && world.get::<BottomSpacer>(child).is_none()
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

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn finite_ranges_stay_within_content(
            len in 0usize..10_001, height in 1u16..129, viewport in 1u16..1025, offset in -10_000i32..2_000_000
        ) {
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
        for offset in [-1.0, f32::NEG_INFINITY, f32::INFINITY, f32::NAN] {
            assert_eq!(visible_range(10, 10.0, 21.0, offset), (0.0, 0..3));
        }
    }

    #[test]
    fn range_handles_boundaries_and_shrink() {
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
