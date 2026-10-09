use crate::headless::WidgetryScrollAreaViewport;
use crate::layout::{HorizontalScrollbar, VerticalScrollbar};
use crate::style::ScrollAreaThumb;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::ui::prelude::BorderRect;
use bevy::ui::{ComputedUiRenderTargetInfo, InteractionDisabled, UiGlobalTransform, UiTransform};
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};
use bevy_widgetry_core::disabled::WidgetryEffectiveDisabled;

#[derive(Component)]
pub(crate) struct SuspendedScrollbar(pub(crate) Scrollbar);

pub(crate) fn install(app: &mut App) {
    app.add_observer(on_effective)
        .add_observer(on_scrollbar)
        .add_observer(on_thumb_added)
        .add_observer(on_thumb_parent_inserted)
        .add_observer(on_thumb_parent_removed)
        .add_systems(
            PostUpdate,
            update_suspended_thumb
                .in_set(bevy::ui::UiSystems::Layout)
                .after(bevy::ui::ui_layout_system),
        );
}

type InputParts<'w, 's> = Query<
    'w,
    's,
    (),
    Or<(
        With<HorizontalScrollbar>,
        With<VerticalScrollbar>,
        With<ScrollAreaThumb>,
        With<WidgetryScrollAreaViewport>,
    )>,
>;

fn on_effective(
    event: On<Insert, WidgetryEffectiveDisabled>,
    parts: InputParts,
    mut commands: Commands,
) {
    if !parts.contains(event.entity) {
        return;
    }
    let entity = event.entity;
    commands.queue(move |world: &mut World| sync(world, entity));
}

fn on_scrollbar(event: On<Insert, Scrollbar>, parts: InputParts, mut commands: Commands) {
    if !parts.contains(event.entity) {
        return;
    }
    let entity = event.entity;
    commands.queue(move |world: &mut World| sync(world, entity));
}

fn on_thumb_added(event: On<Add, ScrollAreaThumb>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| sync(world, entity));
}

fn on_thumb_parent_inserted(
    event: On<Insert, ChildOf>,
    thumbs: Query<(), With<ScrollAreaThumb>>,
    mut commands: Commands,
) {
    if thumbs.contains(event.entity) {
        let entity = event.entity;
        commands.queue(move |world: &mut World| sync(world, entity));
    }
}

fn on_thumb_parent_removed(
    event: On<Remove, (ChildOf, ScrollAreaThumb)>,
    thumbs: Query<&ChildOf, With<ScrollAreaThumb>>,
    mut commands: Commands,
) {
    if let Ok(parent) = thumbs.get(event.entity) {
        let bar = parent.parent();
        commands.queue(move |world: &mut World| sync(world, bar));
    }
}

fn is_disabled(world: &World, entity: Entity) -> bool {
    world.get::<WidgetryEffectiveDisabled>(entity).map_or_else(
        || world.get::<InteractionDisabled>(entity).is_some(),
        |state| state.is_disabled(),
    )
}

pub(crate) fn sync(world: &mut World, entity: Entity) {
    if world.get::<WidgetryScrollAreaViewport>(entity).is_some() {
        let bars = world
            .get::<ChildOf>(entity)
            .and_then(|parent| world.get::<Children>(parent.parent()))
            .map(|children| {
                children
                    .iter()
                    .filter(|&child| {
                        world.get::<HorizontalScrollbar>(child).is_some()
                            || world.get::<VerticalScrollbar>(child).is_some()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for bar in bars {
            sync_bar(world, bar);
        }
        return;
    }
    let entity = if world.get::<ScrollAreaThumb>(entity).is_some() {
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return;
        };
        parent.parent()
    } else {
        entity
    };
    sync_bar(world, entity);
}

fn sync_bar(world: &mut World, entity: Entity) {
    if !world.entities().contains(entity)
        || world.get::<HorizontalScrollbar>(entity).is_none()
            && world.get::<VerticalScrollbar>(entity).is_none()
    {
        return;
    }
    // 官方 thumb/track 输入通过 Scrollbar 改写 target，需同时检查输入部件与目标 viewport。
    // 程序化 ScrollPosition 独立，几何仍由下方 layout 同步。
    let target = world
        .get::<Scrollbar>(entity)
        .map(|bar| bar.target)
        .or_else(|| {
            world
                .get::<SuspendedScrollbar>(entity)
                .map(|bar| bar.0.target)
        });
    let disabled = is_disabled(world, entity)
        || target.is_some_and(|target| is_disabled(world, target))
        || world.get::<Children>(entity).is_some_and(|children| {
            children.iter().any(|child| {
                world.get::<ScrollAreaThumb>(child).is_some() && is_disabled(world, child)
            })
        });
    if disabled {
        if let Some(bar) = world.entity_mut(entity).take::<Scrollbar>() {
            let Ok(mut target) = world.get_entity_mut(entity) else {
                return;
            };
            target.insert(SuspendedScrollbar(bar));
        }
        crate::pointer::cancel_bar(world, entity);
    } else if let Some(bar) = world.entity_mut(entity).take::<SuspendedScrollbar>() {
        let Ok(mut target) = world.get_entity_mut(entity) else {
            return;
        };
        target.insert(bar.0);
    }
}

fn update_suspended_thumb(
    q_scroll_area: Query<(&ScrollPosition, &ComputedNode), Without<ScrollbarThumb>>,
    q_scrollbar: Query<
        (
            &SuspendedScrollbar,
            &ComputedNode,
            &UiGlobalTransform,
            &Children,
        ),
        Without<ScrollbarThumb>,
    >,
    mut q_thumb: Query<
        (
            &ScrollbarThumb,
            &UiTransform,
            &ComputedUiRenderTargetInfo,
            &mut ComputedNode,
            &mut UiGlobalTransform,
        ),
        With<ScrollbarThumb>,
    >,
) {
    for (scrollbar, scrollbar_node, scrollbar_transform, children) in q_scrollbar.iter() {
        let scrollbar = &scrollbar.0;
        let Ok(scroll_area) = q_scroll_area.get(scrollbar.target) else {
            continue;
        };

        let visible_size = (scroll_area.1.size() - scroll_area.1.scrollbar_size)
            * scroll_area.1.inverse_scale_factor;

        let content_size = scroll_area.1.content_size() * scroll_area.1.inverse_scale_factor;

        let track_length = scrollbar_node.size() * scrollbar_node.inverse_scale_factor;

        fn size_and_pos(
            content_size: f32,
            visible_size: f32,
            track_length: f32,
            min_size: f32,
            mut offset: f32,
        ) -> (f32, f32) {
            let thumb_size = if content_size > visible_size {
                (track_length * visible_size / content_size)
                    .max(min_size)
                    .min(track_length)
            } else {
                track_length
            };

            if content_size > visible_size {
                let max_offset = content_size - visible_size;

                offset = offset.clamp(0.0, max_offset);
            } else {
                offset = 0.0;
            }

            let thumb_pos = if content_size > visible_size {
                offset * (track_length - thumb_size) / (content_size - visible_size)
            } else {
                0.
            };

            (thumb_size, thumb_pos)
        }

        for child in children {
            let Ok((
                thumb,
                thumb_transform,
                target_info,
                mut thumb_node,
                mut thumb_global_transform,
            )) = q_thumb.get_mut(*child)
            else {
                continue;
            };

            let (thumb_logical_size, thumb_center) = match scrollbar.orientation {
                ControlOrientation::Horizontal => {
                    let (thumb_size, thumb_pos) = size_and_pos(
                        content_size.x,
                        visible_size.x,
                        track_length.x,
                        scrollbar.min_thumb_length,
                        scroll_area.0.x,
                    );
                    (
                        Vec2::new(thumb_size, track_length.y),
                        Vec2::new(thumb_pos + 0.5 * (thumb_size - track_length.x), 0.),
                    )
                }
                ControlOrientation::Vertical => {
                    let (thumb_size, thumb_pos) = size_and_pos(
                        content_size.y,
                        visible_size.y,
                        track_length.y,
                        scrollbar.min_thumb_length,
                        scroll_area.0.y,
                    );
                    (
                        Vec2::new(track_length.x, thumb_size),
                        Vec2::new(0., thumb_pos + 0.5 * (thumb_size - track_length.y)),
                    )
                }
            };

            let inverse_scale_factor = target_info.scale_factor().recip();
            let thumb_physical_size = thumb_logical_size * target_info.scale_factor();

            if thumb_node.size != thumb_physical_size
                || thumb_node.unrounded_size != thumb_physical_size
                || thumb_node.inverse_scale_factor != inverse_scale_factor
            {
                thumb_node.size = thumb_physical_size;
                thumb_node.unrounded_size = thumb_physical_size;
                thumb_node.inverse_scale_factor = inverse_scale_factor;
            }

            let border_radius = thumb.border_radius.resolve(
                target_info.scale_factor(),
                thumb_physical_size,
                target_info.physical_size().as_vec2(),
            );
            if thumb_node.border_radius != border_radius {
                thumb_node.border_radius = border_radius;
            }

            let resolve_border_val = |val: Val| {
                val.resolve(
                    target_info.scale_factor(),
                    thumb_physical_size.x,
                    target_info.physical_size().as_vec2(),
                )
                .unwrap_or(0.)
            };

            let mut resolved_border = BorderRect {
                min_inset: Vec2::new(
                    resolve_border_val(thumb.border.left),
                    resolve_border_val(thumb.border.top),
                ),
                max_inset: Vec2::new(
                    resolve_border_val(thumb.border.right),
                    resolve_border_val(thumb.border.bottom),
                ),
            };

            if thumb_node.size.x < resolved_border.min_inset.x + resolved_border.max_inset.x {
                let r =
                    thumb_node.size.x / (resolved_border.min_inset.x + resolved_border.max_inset.x);
                resolved_border.min_inset.x *= r;
                resolved_border.max_inset.x *= r;
            }

            if thumb_node.size.y < resolved_border.min_inset.y + resolved_border.max_inset.y {
                let r =
                    thumb_node.size.y / (resolved_border.min_inset.y + resolved_border.max_inset.y);
                resolved_border.min_inset.y *= r;
                resolved_border.max_inset.y *= r;
            }

            thumb_node.bypass_change_detection().border = resolved_border;

            let new_transform = scrollbar_transform.affine()
                * thumb_transform.compute_affine(
                    target_info.scale_factor(),
                    thumb_physical_size,
                    target_info.physical_size().as_vec2(),
                )
                * Affine2::from_translation(thumb_center * target_info.scale_factor());

            if thumb_global_transform.affine() != new_transform {
                *thumb_global_transform = new_transform.into();
            }
        }
    }
}
