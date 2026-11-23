use crate::error::{CanvasResult, PublicResult, invalid, missing};
use crate::{
    CanvasConnector, CanvasDocument, CanvasEditTarget, CanvasGroup, CanvasIssue, CanvasPart,
};
use bevy::{ecs::entity::EntityHashSet, math::DVec2, prelude::*};
use bevy_widgetry_diagnostics::ErrorFrame;

const BOUNDS_ABSOLUTE_TOLERANCE: f64 = 0.0001;
const BOUNDS_ROUNDING_TOLERANCE: f64 = 4.0 * f32::EPSILON as f64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasAlignment {
    Left,
    HorizontalCenter,
    Right,
    Top,
    VerticalCenter,
    Bottom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasAlignTarget {
    Selection,
    View(Entity),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasDistribution {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
struct Member {
    entity: Entity,
    bounds: Rect,
}

impl CanvasAlignment {
    fn axis(self) -> usize {
        match self {
            Self::Left | Self::HorizontalCenter | Self::Right => 0,
            Self::Top | Self::VerticalCenter | Self::Bottom => 1,
        }
    }

    fn coordinate(self, bounds: Rect, axis: usize) -> f64 {
        let min = f64::from(bounds.min[axis]);
        let max = f64::from(bounds.max[axis]);
        match self {
            Self::Left | Self::Top => min,
            Self::Right | Self::Bottom => max,
            Self::HorizontalCenter | Self::VerticalCenter => f64::midpoint(min, max),
        }
    }
}

impl CanvasDocument {
    /// Aligns the chosen edge or center of each item to the selection or a View.
    pub fn align_items(
        world: &mut World,
        edit: impl Into<CanvasEditTarget>,
        items: &[Entity],
        alignment: CanvasAlignment,
        target: CanvasAlignTarget,
    ) -> PublicResult<bool> {
        let edit = edit.into();
        let document = edit.document();
        crate::history::apply(world, edit, |world, document| {
            let members = members(world, document, items)?;
            let bounds = match target {
                CanvasAlignTarget::Selection => {
                    let Some(bounds) = members
                        .iter()
                        .map(|member| member.bounds)
                        .reduce(|left, right| left.union(right))
                    else {
                        return Ok(false);
                    };
                    bounds
                }
                CanvasAlignTarget::View(view) => visible_bounds(world, document, view)?,
            };
            let axis = alignment.axis();
            let coordinate = |bounds| alignment.coordinate(bounds, axis);
            let target = coordinate(bounds);
            let offsets = members
                .into_iter()
                .map(|member| (member, target - coordinate(member.bounds)));
            apply_offsets(world, document, axis, offsets, |bounds, distance| {
                [coordinate(bounds) + distance]
            })
        })
        .map_err(|error| {
            error
                .at("document", document)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.align_items"))
        })
    }

    /// Sets equal gaps between the items along the chosen axis, keeping the outer items fixed.
    pub fn distribute_items(
        world: &mut World,
        edit: impl Into<CanvasEditTarget>,
        items: &[Entity],
        distribution: CanvasDistribution,
    ) -> PublicResult<bool> {
        let edit = edit.into();
        let document = edit.document();
        crate::history::apply(world, edit, |world, document| {
            let mut members = members(world, document, items)?;
            if members.len() < 3 {
                return Ok(false);
            }
            let axis = match distribution {
                CanvasDistribution::Horizontal => 0,
                CanvasDistribution::Vertical => 1,
            };
            let compare = |left: (f32, Entity), right: (f32, Entity)| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.cmp(&right.1))
            };
            let left_edge = |member: Member| (member.bounds.min[axis], member.entity);
            members.sort_unstable_by(|left, right| compare(left_edge(*left), left_edge(*right)));
            let [first, middle @ .., last] = members.as_slice() else {
                return Ok(false);
            };
            let start = f64::from(first.bounds.max[axis]);
            let end = f64::from(last.bounds.min[axis]);
            let extent = |member: &Member| {
                f64::from(member.bounds.max[axis]) - f64::from(member.bounds.min[axis])
            };
            let gap =
                (end - start - middle.iter().map(extent).sum::<f64>()) / (members.len() - 1) as f64;
            let mut offsets = Vec::with_capacity(members.len());
            offsets.push((*first, 0.0));
            let mut cursor = start + gap;
            let mut previous = left_edge(*first);
            let end_order = left_edge(*last);
            for &member in middle {
                let current = (cursor as f32, member.entity);
                if compare(previous, current).is_gt() || compare(current, end_order).is_gt() {
                    return Ok(false);
                }
                offsets.push((member, cursor - f64::from(member.bounds.min[axis])));
                previous = current;
                cursor += extent(&member) + gap;
            }
            offsets.push((*last, 0.0));
            apply_offsets(world, document, axis, offsets, |bounds, distance| {
                let mut min = bounds.min.as_dvec2();
                let mut max = bounds.max.as_dvec2();
                min[axis] += distance;
                max[axis] += distance;
                [min.x, min.y, max.x, max.y]
            })
        })
        .map_err(|error| {
            error
                .at("document", document)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.distribute_items"))
        })
    }
}

fn members(world: &World, document: Entity, items: &[Entity]) -> CanvasResult<Vec<Member>> {
    crate::model::validate_document(world, document)?;
    let selected: EntityHashSet = items.iter().copied().collect();
    if selected.len() != items.len() {
        return Err(invalid(CanvasIssue::AlignmentItem));
    }
    items
        .iter()
        .map(|&item| {
            let (_, layer) = crate::order::item_placement(world, document, item)
                .ok_or_else(|| crate::model::invalid_ownership(world, item))?;
            if !layer.editable() || world.get::<CanvasConnector>(item).is_some() {
                return Err(invalid(CanvasIssue::AlignmentItem).at("item", item));
            }
            let mut child = item;
            while let Some(parent) = world
                .get::<ChildOf>(child)
                .map(ChildOf::parent)
                .filter(|&parent| parent != document)
            {
                if selected.contains(&parent) {
                    return Err(invalid(CanvasIssue::AlignmentItem).at("item", item));
                }
                child = parent;
            }
            let bounds = crate::groups::document_bounds(world, item)?
                .ok_or_else(|| invalid(CanvasIssue::AlignmentBounds).at("item", item))?;
            Ok(Member {
                entity: item,
                bounds,
            })
        })
        .collect()
}

fn visible_bounds(world: &World, document: Entity, view: Entity) -> CanvasResult<Rect> {
    let state = world
        .get::<crate::view::ViewState>(view)
        .ok_or_else(|| missing(CanvasPart::View).at("view", view))?;
    if state.document != Some(document) {
        return Err(invalid(CanvasIssue::AlignmentTarget).at("view", view));
    }
    let node = world
        .get::<ComputedNode>(view)
        .ok_or_else(|| missing(CanvasPart::Layout).at("view", view))?;
    let half_size = (node.size() * node.inverse_scale_factor()) / state.zoom * 0.5;
    let min = state.center - half_size;
    let max = state.center + half_size;
    if !min.is_finite() || !max.is_finite() || !min.cmplt(max).all() {
        return Err(invalid(CanvasIssue::AlignmentTarget).at("view", view));
    }
    Ok(Rect { min, max })
}

fn apply_offsets<const N: usize>(
    world: &mut World,
    document: Entity,
    axis: usize,
    offsets: impl IntoIterator<Item = (Member, f64)>,
    projected_bounds: impl Fn(Rect, f64) -> [f64; N],
) -> CanvasResult<bool> {
    let mut updates = Vec::new();
    let mut groups = Vec::new();
    for (Member { entity, bounds }, distance) in offsets {
        if world.get::<CanvasGroup>(entity).is_some() {
            groups.push((entity, projected_bounds(bounds, distance)));
        }
        if distance == 0.0 {
            continue;
        }
        let parent = crate::groups::parent_transform(world, entity)?;
        let mut delta = DVec2::ZERO;
        delta[axis] = distance;
        let mut transform = crate::groups::local_transform(world, entity)?;
        let position = transform.translation.truncate().as_dvec2();
        let next = (position + crate::scale::local_delta(parent, delta)).as_vec2();
        if next != position.as_vec2() {
            transform.translation = next.extend(transform.translation.z);
            updates.push((entity, transform));
        }
    }
    if updates.is_empty() {
        return Ok(false);
    }
    if !groups.is_empty() {
        crate::connector::validate_following_layout(world, document, &updates, |world| {
            for (group, expected) in groups {
                let invalid_bounds = || invalid(CanvasIssue::AlignmentBounds).at("item", group);
                let projected =
                    crate::groups::document_bounds(world, group)?.ok_or_else(invalid_bounds)?;
                for (actual, target) in projected_bounds(projected, 0.0).into_iter().zip(expected) {
                    let tolerance = BOUNDS_ABSOLUTE_TOLERANCE
                        + BOUNDS_ROUNDING_TOLERANCE * actual.abs().max(target.abs());
                    if (actual - target).abs() > tolerance {
                        return Err(invalid_bounds());
                    }
                }
            }
            Ok(())
        })?;
    }
    crate::batch::update_selected_transforms(world, document, &updates)?;
    Ok(true)
}
