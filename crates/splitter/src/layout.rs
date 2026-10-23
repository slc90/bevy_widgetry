use crate::{
    WidgetrySplitter, WidgetrySplitterOrientation,
    splitter::{Parts, State},
};
use bevy::{
    prelude::*,
    text::{EmSize, RemSize},
    ui::{ComputedUiRenderTargetInfo, UiSystems},
};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};

#[derive(Component, Default)]
pub(crate) struct Diagnostics(FailureState);

pub(crate) fn install(app: &mut App) {
    app.add_message::<bevy::window::RequestRedraw>()
        .add_systems(
            PostUpdate,
            update.after(UiSystems::Prepare).before(UiSystems::Layout),
        );
}

fn error(reason: &str) -> BevyError {
    BevyError::error(reason.to_owned())
}

#[derive(Clone, Copy)]
struct Length {
    fixed: f32,
    fraction: f32,
}

impl Length {
    fn resolve(self, base: f32) -> f32 {
        self.fixed + self.fraction * base
    }
}

fn insets(
    node: &Node,
    parent_width: f32,
    viewport: Vec2,
    em: EmSize,
    rem: RemSize,
    scale: f32,
) -> Result<Vec2, BevyError> {
    // ComputedNode rounds padding and borders; resolve the native lengths in logical space instead.
    let resolve = |v| -> Result<f32, BevyError> {
        Ok(length(v, viewport, em, rem)?.map_or(0.0, |v| v.resolve(parent_width)))
    };
    // Bevy snaps length-based borders before layout, while percentage borders remain proportional.
    let border = |v| -> Result<f32, BevyError> {
        let value = resolve(v)?;
        Ok(if value > 0.0 && !matches!(v, Val::Percent(_)) {
            (value * scale).floor().max(1.0) / scale
        } else {
            value
        })
    };
    Ok(Vec2::new(
        resolve(node.padding.left)?
            + resolve(node.padding.right)?
            + border(node.border.left)?
            + border(node.border.right)?,
        resolve(node.padding.top)?
            + resolve(node.padding.bottom)?
            + border(node.border.top)?
            + border(node.border.bottom)?,
    ))
}

type ReadNodes<'w, 's> = Query<
    'w,
    's,
    (
        &'static Node,
        &'static ComputedNode,
        &'static ComputedUiRenderTargetInfo,
        Option<&'static EmSize>,
        Option<&'static ChildOf>,
    ),
>;

fn content_width(entity: Entity, nodes: &ReadNodes, rem: RemSize) -> Result<f32, BevyError> {
    let (node, computed, target, em, parent) = nodes
        .get(entity)
        .map_err(|_| error("Splitter ancestor layout is missing"))?;
    let parent_width = if let Some(parent) = parent.filter(|p| nodes.contains(p.parent())) {
        content_width(parent.parent(), nodes, rem)?
    } else {
        target.logical_size().x
    };
    Ok((computed.unrounded_size.x * computed.inverse_scale_factor
        - insets(
            node,
            parent_width,
            target.logical_size(),
            em.copied().unwrap_or_default(),
            rem,
            target.scale_factor(),
        )?
        .x)
        .max(0.0))
}

fn main_inset(
    node: &Node,
    orientation: WidgetrySplitterOrientation,
    width: f32,
    viewport: Vec2,
    em: EmSize,
    rem: RemSize,
    scale: f32,
) -> Result<Length, BevyError> {
    let (padding, borders) = match orientation {
        WidgetrySplitterOrientation::Horizontal => (
            [node.padding.left, node.padding.right],
            [node.border.left, node.border.right],
        ),
        WidgetrySplitterOrientation::Vertical => (
            [node.padding.top, node.padding.bottom],
            [node.border.top, node.border.bottom],
        ),
    };
    let mut sum = Length {
        fixed: 0.0,
        fraction: 0.0,
    };
    for (value, border) in padding
        .into_iter()
        .map(|v| (v, false))
        .chain(borders.into_iter().map(|v| (v, true)))
    {
        let Some(mut part) = length(value, viewport, em, rem)? else {
            continue;
        };
        if border && part.fixed > 0.0 {
            part.fixed = (part.fixed * scale).floor().max(1.0) / scale;
        }
        if orientation == WidgetrySplitterOrientation::Vertical {
            part.fixed += part.fraction * width;
            part.fraction = 0.0;
        }
        sum.fixed += part.fixed;
        sum.fraction += part.fraction;
    }
    Ok(sum)
}

fn outer_length(mut length: Length, node: &Node, inset: Length, base: f32) -> Length {
    if node.box_sizing == BoxSizing::ContentBox {
        length.fixed += inset.fixed;
        length.fraction += inset.fraction;
    } else if length.resolve(base) < inset.resolve(base) {
        length = inset;
    }
    length
}

fn length(
    value: Val,
    viewport: Vec2,
    em: EmSize,
    rem: RemSize,
) -> Result<Option<Length>, BevyError> {
    let result = match value {
        Val::Auto => return Ok(None),
        Val::Percent(value) => Length {
            fixed: 0.0,
            fraction: value / 100.0,
        },
        _ => Length {
            fixed: value
                .resolve(1.0, 0.0, viewport, em, rem)
                .map_err(|_| error("Splitter could not resolve Pane constraint"))?,
            fraction: 0.0,
        },
    };
    if !result.fixed.is_finite()
        || result.fixed < 0.0
        || !result.fraction.is_finite()
        || result.fraction < 0.0
    {
        return Err(error(
            "Splitter Pane constraints must be finite and nonnegative",
        ));
    }
    Ok(Some(result))
}

fn limits(node: &Node, orientation: WidgetrySplitterOrientation) -> (Val, Val) {
    match orientation {
        WidgetrySplitterOrientation::Horizontal => (node.min_width, node.max_width),
        WidgetrySplitterOrientation::Vertical => (node.min_height, node.max_height),
    }
}

fn root_bound(a: Length, b: Length, thickness: f32) -> Result<f32, BevyError> {
    let remaining = 1.0 - a.fraction - b.fraction;
    let bound = (a.fixed + b.fixed + thickness) / remaining;
    if remaining <= 0.0 || !bound.is_finite() {
        return Err(error(
            "Splitter Pane constraints leave no space for the Handle",
        ));
    }
    Ok(bound)
}

struct PaneLimits {
    min: Length,
    max: Option<Length>,
    raw_conflict: bool,
}

fn pane_limits(
    node: &Node,
    orientation: WidgetrySplitterOrientation,
    inset: Length,
    base: f32,
    viewport: Vec2,
    em: EmSize,
    rem: RemSize,
) -> Result<PaneLimits, BevyError> {
    let (min, max) = limits(node, orientation);
    let min = length(min, viewport, em, rem)?.unwrap_or(Length {
        fixed: 0.0,
        fraction: 0.0,
    });
    let max = length(max, viewport, em, rem)?;
    Ok(PaneLimits {
        raw_conflict: max.is_some_and(|max| max.resolve(base) < min.resolve(base)),
        min: outer_length(min, node, inset, base),
        max: max.map(|max| outer_length(max, node, inset, base)),
    })
}

fn reachable(
    a: &PaneLimits,
    b: &PaneLimits,
    base: f32,
    thickness: f32,
) -> Result<Option<(f32, f32)>, BevyError> {
    if a.raw_conflict || b.raw_conflict {
        return Err(error("Splitter Pane minimum exceeds maximum"));
    }
    let available = base - thickness;
    let lower = a
        .min
        .resolve(base)
        .max(available - b.max.map_or(f32::INFINITY, |max| max.resolve(base)));
    let upper = a
        .max
        .map_or(f32::INFINITY, |max| max.resolve(base))
        .min(available - b.min.resolve(base));
    let tolerance = 8.0 * f32::EPSILON * base.max(1.0);
    if lower > upper + tolerance {
        return Ok(None);
    }
    let lower = lower.clamp(0.0, available);
    let upper = upper.max(lower).clamp(lower, available);
    Ok(Some((lower, upper)))
}

pub(crate) fn constrain_ratio(
    world: &mut World,
    root: Entity,
    ratio: f32,
) -> Result<f32, BevyError> {
    let bounds = ratio_bounds(world, root)?
        .ok_or_else(|| error("Splitter requires an initialized valid main-axis layout"))?;
    Ok(bounds.clamp(ratio))
}

pub(crate) struct RatioBounds {
    pub(crate) available: f32,
    lower: f32,
    upper: f32,
}
impl RatioBounds {
    pub(crate) fn clamp(&self, ratio: f32) -> f32 {
        ratio.clamp(self.lower, self.upper)
    }
}

pub(crate) fn ratio_bounds(
    world: &mut World,
    root: Entity,
) -> Result<Option<RatioBounds>, BevyError> {
    let state = *world
        .get::<State>(root)
        .ok_or_else(|| error("Splitter runtime state is missing"))?;
    if !state.initialized {
        return Ok(None);
    }
    let parts = *world
        .get::<Parts>(root)
        .ok_or_else(|| error("Splitter internal parts are missing"))?;
    let rem = world.get_resource::<RemSize>().copied().unwrap_or_default();
    let mut query = world.query::<(
        &Node,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        Option<&EmSize>,
        Option<&ChildOf>,
    )>();
    let nodes = query.query(world);
    let [
        (node, computed, target, em, parent),
        (a, _, _, a_em, a_parent),
        (b, _, _, b_em, b_parent),
        (_, _, _, _, h_parent),
    ] = nodes
        .get_many([root, parts.a, parts.b, parts.handle])
        .map_err(|_| error("Splitter internal layout entities are missing"))?;
    if [a_parent, b_parent, h_parent]
        .iter()
        .any(|p| p.is_none_or(|p| p.parent() != root))
    {
        return Err(error("Splitter Pane or Handle parent is invalid"));
    }
    let parent_width = if let Some(parent) = parent.filter(|p| nodes.contains(p.parent())) {
        content_width(parent.parent(), &nodes, rem)?
    } else {
        target.logical_size().x
    };
    let viewport = target.logical_size();
    let inset = insets(
        node,
        parent_width,
        viewport,
        em.copied().unwrap_or_default(),
        rem,
        target.scale_factor(),
    )?;
    let content = computed.unrounded_size * computed.inverse_scale_factor - inset;
    let base = state.orientation.main(content);
    if !base.is_finite() || base <= state.thickness {
        return Ok(None);
    }
    let a_inset = main_inset(
        a,
        state.orientation,
        content.x,
        viewport,
        a_em.copied().unwrap_or_default(),
        rem,
        target.scale_factor(),
    )?;
    let b_inset = main_inset(
        b,
        state.orientation,
        content.x,
        viewport,
        b_em.copied().unwrap_or_default(),
        rem,
        target.scale_factor(),
    )?;
    let a = pane_limits(
        a,
        state.orientation,
        a_inset,
        base,
        viewport,
        a_em.copied().unwrap_or_default(),
        rem,
    )?;
    let b = pane_limits(
        b,
        state.orientation,
        b_inset,
        base,
        viewport,
        b_em.copied().unwrap_or_default(),
        rem,
    )?;
    let Some((lower, upper)) = reachable(&a, &b, base, state.thickness)? else {
        return Ok(None);
    };
    let available = base - state.thickness;
    Ok(Some(RatioBounds {
        available,
        lower: lower / available,
        upper: upper / available,
    }))
}

fn update(
    mut roots: Query<
        (Entity, Option<&mut State>, Option<&Parts>, &mut Diagnostics),
        With<WidgetrySplitter>,
    >,
    mut nodes: ParamSet<(
        Query<(
            &mut Node,
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            Option<&EmSize>,
            Option<&ChildOf>,
        )>,
        ReadNodes,
    )>,
    rem: Option<Res<RemSize>>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) -> Result {
    let mut failure = None;
    for (root, state, parts, mut diagnostics) in &mut roots {
        let result = (|| -> Result<bool, BevyError> {
            let mut state = state.ok_or_else(|| error("Splitter runtime state is missing"))?;
            let parts = parts.ok_or_else(|| error("Splitter internal parts are missing"))?;
            let rem = rem.as_deref().copied().unwrap_or_default();
            let parent_width = {
                let read = nodes.p1();
                let (_, _, target, _, parent) = read
                    .get(root)
                    .map_err(|_| error("Splitter root layout is missing"))?;
                if let Some(parent) = parent.filter(|p| read.contains(p.parent())) {
                    content_width(parent.parent(), &read, rem)?
                } else {
                    target.logical_size().x
                }
            };
            let mut nodes = nodes.p0();
            let [
                (mut root_node, computed, target, root_em, _),
                (mut a_node, _, _, a_em, a_parent),
                (mut handle_node, _, _, _, h_parent),
                (mut b_node, _, _, b_em, b_parent),
            ] = nodes
                .get_many_mut([root, parts.a, parts.handle, parts.b])
                .map_err(|_| error("Splitter internal layout entities are missing"))?;
            if [a_parent, h_parent, b_parent]
                .iter()
                .any(|parent| parent.is_none_or(|parent| parent.parent() != root))
            {
                return Err(error("Splitter Pane or Handle parent is invalid"));
            }
            let viewport = target.logical_size();
            let root_insets = insets(
                &root_node,
                parent_width,
                viewport,
                root_em.copied().unwrap_or_default(),
                rem,
                target.scale_factor(),
            )?;
            let inset = state.orientation.main(root_insets);
            let base = state
                .orientation
                .main(computed.unrounded_size * computed.inverse_scale_factor)
                - inset;
            let content_width = (computed.unrounded_size.x * computed.inverse_scale_factor
                - root_insets.x)
                .max(0.0);
            let a_inset_length = main_inset(
                &a_node,
                state.orientation,
                content_width,
                viewport,
                a_em.copied().unwrap_or_default(),
                rem,
                target.scale_factor(),
            )?;
            let b_inset_length = main_inset(
                &b_node,
                state.orientation,
                content_width,
                viewport,
                b_em.copied().unwrap_or_default(),
                rem,
                target.scale_factor(),
            )?;
            let a_inset = a_inset_length.resolve(base);
            let b_inset = b_inset_length.resolve(base);
            let a_limits = pane_limits(
                &a_node,
                state.orientation,
                a_inset_length,
                base,
                viewport,
                a_em.copied().unwrap_or_default(),
                rem,
            )?;
            let b_limits = pane_limits(
                &b_node,
                state.orientation,
                b_inset_length,
                base,
                viewport,
                b_em.copied().unwrap_or_default(),
                rem,
            )?;
            let min = root_bound(a_limits.min, b_limits.min, state.thickness)?;
            let max = match (a_limits.max, b_limits.max) {
                (Some(a), Some(b)) if a.fraction + b.fraction < 1.0 => {
                    Some(root_bound(a, b, state.thickness)?)
                }
                _ => None,
            };
            if max.is_some_and(|max| max < min) {
                return Err(error("Splitter Pane minimum exceeds maximum"));
            }
            let root_inset = if root_node.box_sizing == BoxSizing::ContentBox {
                0.0
            } else {
                inset
            };
            let next_min = px(min + root_inset);
            let next_max = max.map_or(Val::Auto, |max| px(max + root_inset));
            let bounds_changed = limits(&root_node, state.orientation) != (next_min, next_max);
            if bounds_changed {
                match state.orientation {
                    WidgetrySplitterOrientation::Horizontal => {
                        root_node.min_width = next_min;
                        root_node.max_width = next_max;
                    }
                    WidgetrySplitterOrientation::Vertical => {
                        root_node.min_height = next_min;
                        root_node.max_height = next_max;
                    }
                }
                redraw.write(bevy::window::RequestRedraw);
            }
            if base <= state.thickness || !base.is_finite() {
                return Ok(false);
            }
            let tolerance = 8.0 * f32::EPSILON * base.max(1.0);
            // Validate percentage limits only after the native root has adopted its new range.
            if bounds_changed
                || base < min - tolerance
                || max.is_some_and(|max| base > max + tolerance)
            {
                redraw.write(bevy::window::RequestRedraw);
                return Ok(false);
            }
            let available = base - state.thickness;
            // Root bounds need another native layout pass before this interval can be feasible.
            let Some((lower, upper)) = reachable(&a_limits, &b_limits, base, state.thickness)?
            else {
                return Ok(false);
            };
            let a_size = (available * state.ratio).clamp(lower, upper);
            if !state.initialized {
                state.ratio = a_size / available;
                state.initialized = true;
            }
            let b_size = available - a_size;
            for (node, size, inset) in [
                (&mut a_node, a_size, a_inset),
                (&mut handle_node, state.thickness, 0.0),
                (&mut b_node, b_size, b_inset),
            ] {
                let mut next = (**node).clone();
                let size = if next.box_sizing == BoxSizing::ContentBox {
                    (size - inset).max(0.0)
                } else {
                    size
                };
                match state.orientation {
                    WidgetrySplitterOrientation::Horizontal => next.width = px(size),
                    WidgetrySplitterOrientation::Vertical => next.height = px(size),
                }
                next.flex_basis = Val::Auto;
                next.flex_grow = 0.0;
                next.flex_shrink = 0.0;
                if **node != next {
                    **node = next;
                    redraw.write(bevy::window::RequestRedraw);
                }
            }
            Ok(true)
        })();
        let result = match result {
            Ok(false) => continue,
            Ok(true) => Ok(()),
            Err(error) => Err(error),
        };
        let result = diagnostics.0.observe(
            result,
            |error| widgetry_error!(?root, %error, "Splitter 布局失效"),
            || widgetry_info!(?root, "Splitter 布局恢复正常"),
        );
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}
