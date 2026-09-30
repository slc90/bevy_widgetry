use crate::view::{BottomSpacer, TopSpacer};
use crate::{WidgetryListModel, WidgetryListView, WidgetryListViewItem};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::ui_widgets::ListItem;
use bevy::window::RequestRedraw;
use bevy_widgetry_log::widgetry_error;
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
use std::ops::Range;

/// 同一批 UI rows 的引用，顺序始终对应 range.start + offset。
#[derive(Component, Clone)]
pub(crate) struct ListRuntime {
    pub(crate) viewport: Entity,
    pub(crate) content: Entity,
    top: Entity,
    bottom: Entity,
    pub(crate) range: Range<usize>,
    pub(crate) rows: Vec<Entity>,
}

/// 只记录 renderer 上次使用的内容版本，业务内容仍由 model 持有。
#[derive(Component)]
struct RenderedRevision(u64);

/// 非有限或零 viewport 尚无可见行；offset 先按真实列表高度收敛。
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

/// 不可恢复的 hierarchy/config invariant 必须先留下 Widgetry ERROR。
fn invariant<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| {
        widgetry_error!("ListView runtime hierarchy 或 model invariant 失效");
        panic!("ListView runtime invariant failed");
    })
}

/// 从 BSN 的固定 shell 解析一次 runtime，不寻找或创建第二份 UI tree。
fn runtime(world: &World, root: Entity) -> ListRuntime {
    let viewport = invariant(world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|child| world.get::<WidgetryScrollAreaViewport>(*child).is_some())
    }));
    let content = invariant(world.get::<Children>(viewport).and_then(|children| {
        children
            .iter()
            .find(|child| world.get::<WidgetryScrollAreaContent>(*child).is_some())
    }));
    let children = invariant(world.get::<Children>(content));
    ListRuntime {
        viewport,
        content,
        top: invariant(
            children
                .iter()
                .find(|child| world.get::<TopSpacer>(*child).is_some()),
        ),
        bottom: invariant(
            children
                .iter()
                .find(|child| world.get::<BottomSpacer>(*child).is_some()),
        ),
        range: 0..0,
        rows: Vec::new(),
    }
}

/// 唯一同步路径：layout 前按 index overlap 复用 wrapper，按 id/revision 替换 direct children。
pub(crate) fn reconcile<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let bootstrap = world.get::<ListRuntime>(root).is_none();
        let mut runtime = world
            .get::<ListRuntime>(root)
            .cloned()
            .unwrap_or_else(|| runtime(world, root));
        let view = invariant(world.get::<WidgetryListView<T>>(root));
        let source = view.source();
        let height = view.item_height();
        let renderer = view.renderer().clone();
        let source_entity = invariant(world.get_entity(source).ok());
        let model = invariant(source_entity.get_ref::<WidgetryListModel<T>>());
        let len = model.len();
        let changed = model.is_changed();
        if !(len as f32 * height).is_finite() {
            widgetry_error!(
                ?root,
                len,
                height,
                "ListView 总高度超出有限 logical px 范围"
            );
            panic!("ListView total height must be finite");
        }
        let computed = invariant(world.get::<ComputedNode>(runtime.viewport));
        let viewport = computed.size().y * computed.inverse_scale_factor();
        let previous_offset = invariant(world.get::<ScrollPosition>(runtime.viewport)).0.y;
        let (offset, range) = visible_range(len, height, viewport, previous_offset);
        if viewport.is_finite() && viewport > 0.0 && offset != previous_offset {
            invariant(world.get_mut::<ScrollPosition>(runtime.viewport))
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
            let model = invariant(world.get::<WidgetryListModel<T>>(source));
            let id = invariant(model.id(index));
            let revision = invariant(model.revision(index));
            let disabled = invariant(model.is_disabled(index))
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
            let children = replace.then(|| renderer.render(index, invariant(model.get(index))));
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
                        if let Err(error) = world
                            .entity_mut(entity)
                            .apply_scene(bsn! { Children [{children}] })
                        {
                            widgetry_error!(?root, index, %error, "ListView renderer Scene 展开失败");
                            panic!("ListView renderer Scene failed");
                        }
                        world.entity_mut(entity).insert((
                            WidgetryListViewItem { id, index },
                            RenderedRevision(revision),
                        ));
                    }
                    entity
                }
                None => {
                    let children = invariant(children);
                    let scene = bsn! {
                        ListItem Hovered::default()
                        template(move |_| Ok(WidgetryListViewItem {id,index}))
                        template(move |_| Ok(RenderedRevision(revision)))
                        Node { height: px(height), min_height: px(height), max_height: px(height), flex_shrink: 0.0, margin: UiRect::ZERO }
                        Children [{children}]
                    };
                    world
                        .spawn_scene(scene)
                        .unwrap_or_else(|error| {
                            widgetry_error!(?root, index, %error, "ListView row Scene 展开失败");
                            panic!("ListView row Scene failed");
                        })
                        .id()
                }
            };
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
            let mut node = invariant(world.get_mut::<Node>(entity));
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 部分可见也计入 range，非法 viewport 不猜容量，shrink 先 clamp offset。
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
