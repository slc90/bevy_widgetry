use crate::view::{BottomSpacer, ListDiagnostics, TopSpacer, validate_source};
use crate::{WidgetryListModel, WidgetryListView, WidgetryListViewItem};
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::ui_widgets::ListItem;
use bevy::window::RequestRedraw;
use bevy_widgetry_core::ForegroundColor;
use bevy_widgetry_core::scene::{apply_scene, spawn_scene};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
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
struct RenderedRevision(u64);

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

pub(crate) fn reconcile<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if let Err(error) = validate_source::<T>(world, root) {
            discard_failed_rows(world, root);
            if failure.is_none() {
                failure = Some(error);
            }
            continue;
        }
        let result = reconcile_root::<T>(world, root);
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
            discard_failed_rows(world, root);
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

fn reconcile_root<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), BevyError> {
    let bootstrap = world.get::<ListRuntime>(root).is_none();
    let mut runtime = world
        .get::<ListRuntime>(root)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| runtime(world, root))?;
    let view = invariant(world.get::<WidgetryListView<T>>(root))?;
    let source = view.source();
    let height = view.item_height();
    let renderer = view.renderer().clone();
    let source_entity = invariant(world.get_entity(source).ok())?;
    let model = invariant(source_entity.get_ref::<WidgetryListModel<T>>())?;
    let len = model.len();
    let changed = model.is_changed();
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
        let model = invariant(world.get::<WidgetryListModel<T>>(source))?;
        let id = invariant(model.id(index))?;
        let revision = invariant(model.revision(index))?;
        let disabled = invariant(model.is_disabled(index))?
            || world.get::<InteractionDisabled>(root).is_some();
        let old = if runtime.range.contains(&index) {
            Some(runtime.rows[index - runtime.range.start])
        } else {
            None
        };
        let replace = old.is_none_or(|row| {
            (changed || range_changed)
                && (world
                    .get::<WidgetryListViewItem>(row)
                    .is_none_or(|item| item.id != id)
                    || world
                        .get::<RenderedRevision>(row)
                        .is_none_or(|old| old.0 != revision))
        });
        let children = if replace {
            Some(renderer.render(index, invariant(model.get(index))?)?)
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
                    template(move |_| Ok(WidgetryListViewItem {id,index}))
                    template(move |_| Ok(RenderedRevision(revision)))
                    BackgroundColor::default() BorderColor::default()
                    template(|_| Ok(Propagate(ForegroundColor::default())))
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
        if disabled != world.get::<InteractionDisabled>(entity).is_some() {
            if disabled {
                world.entity_mut(entity).insert(InteractionDisabled);
            } else {
                world.entity_mut(entity).remove::<InteractionDisabled>();
            }
        }
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
    // bootstrap 后再请求一帧，让首次有效 layout 尺寸进入同一路径；无需猜测行数或双 layout。
    if bootstrap || range_changed || changed {
        world.write_message(RequestRedraw);
    }

    Ok(())
}

fn discard_failed_rows(world: &mut World, root: Entity) {
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

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
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
