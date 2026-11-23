use crate::anchors::{AnchorSnap, Attachment};
use crate::error::{CanvasResult, PublicResult, invalid, missing};
use crate::{CanvasGeometry, CanvasInput, CanvasIssue, CanvasPart, CanvasView, view::ViewState};
use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    math::{Affine3A, DVec2},
    prelude::*,
};
use bevy_widgetry_diagnostics::ErrorFrame;
use std::cmp::Ordering;

const DEFAULT_TOLERANCE_PX: f32 = 8.0;
const RELEASE_MARGIN_PX: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CanvasObjectSnap {
    pub edges: bool,
    pub path_nodes: bool,
    pub connector_anchors: bool,
    pub tolerance_px: f32,
}

impl Default for CanvasObjectSnap {
    fn default() -> Self {
        Self {
            edges: false,
            path_nodes: false,
            connector_anchors: false,
            tolerance_px: DEFAULT_TOLERANCE_PX,
        }
    }
}

impl CanvasObjectSnap {
    pub(crate) fn validate(self) -> CanvasResult {
        if self.tolerance_px.is_finite() && self.tolerance_px > 0.0 {
            return Ok(());
        }
        Err(
            invalid(CanvasIssue::ObjectSnapTolerance).input(CanvasInput::Float {
                field: "tolerance_px",
                value: self.tolerance_px,
            }),
        )
    }
}

#[derive(EntityEvent)]
pub struct CanvasObjectSnapChanged {
    pub entity: Entity,
    pub settings: CanvasObjectSnap,
}

impl CanvasView {
    pub fn object_snap(world: &World, view: Entity) -> PublicResult<CanvasObjectSnap> {
        world
            .get::<ViewState>(view)
            .map(|state| state.object_snap)
            .ok_or_else(|| {
                missing(CanvasPart::View)
                    .at("view", view)
                    .into_failure(ErrorFrame::new("canvas.CanvasView.object_snap"))
            })
    }

    pub fn set_object_snap_in_world(
        world: &mut World,
        view: Entity,
        settings: CanvasObjectSnap,
    ) -> PublicResult<bool> {
        set_settings(world, view, settings).map_err(|error| {
            error.into_failure(ErrorFrame::new("canvas.CanvasView.set_object_snap"))
        })
    }

    pub fn set_object_snap(commands: &mut Commands, view: Entity, settings: CanvasObjectSnap) {
        commands.queue(move |world: &mut World| {
            set_settings(world, view, settings)
                .map(|_| ())
                .map_err(|error| {
                    error.into_bevy_error(ErrorFrame::new("canvas.CanvasView.set_object_snap"))
                })
        });
    }
}

fn set_settings(world: &mut World, view: Entity, settings: CanvasObjectSnap) -> CanvasResult<bool> {
    (|| {
        settings.validate()?;
        let mut state = world
            .get_mut::<ViewState>(view)
            .ok_or_else(|| missing(CanvasPart::View))?;
        if state.object_snap == settings {
            return Ok(false);
        }
        state.object_snap = settings;
        clear(world, view);
        world.trigger(CanvasObjectSnapChanged {
            entity: view,
            settings,
        });
        Ok(true)
    })()
    .map_err(|error: crate::error::CanvasFault| error.at("view", view))
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Edge { source: usize, target: usize },
    Node(usize),
    ConnectorAnchor(crate::CanvasAnchor),
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Target {
    entity: Entity,
    feature: Feature,
}

#[derive(Clone, Copy)]
struct Candidate {
    target: Target,
    draw_index: usize,
    distance: f64,
    held: bool,
    coordinate: f64,
    delta: DVec2,
}

impl Candidate {
    fn prefer(self, current: Self) -> bool {
        self.held
            .cmp(&current.held)
            .reverse()
            .then_with(|| self.distance.total_cmp(&current.distance))
            .then_with(|| current.draw_index.cmp(&self.draw_index))
            .then_with(|| self.target.entity.cmp(&current.target.entity))
            .then_with(|| self.target.feature.cmp(&current.target.feature))
            == Ordering::Less
    }
}

#[derive(Component, Default, Clone, Copy)]
pub(crate) struct DragSnap {
    axes: [Option<Target>; 2],
    node: Option<Target>,
    guides: [Option<f32>; 2],
    anchor: Option<Attachment>,
}

pub(crate) fn guides(world: &World, view: Entity) -> [Option<f32>; 2] {
    world
        .get::<DragSnap>(view)
        .map_or([None; 2], |drag| drag.guides)
}

pub(crate) fn clear(world: &mut World, view: Entity) {
    if let Some(mut drag) = world.get_mut::<DragSnap>(view) {
        *drag = DragSnap::default();
    }
}

#[derive(Default)]
struct Matches {
    axes: [Option<Candidate>; 2],
    node: Option<Candidate>,
}

impl Matches {
    fn correction(&self) -> Option<DVec2> {
        if let Some(node) = self.node {
            return Some(node.delta);
        }
        self.axes.iter().any(Option::is_some).then(|| {
            self.axes
                .iter()
                .flatten()
                .map(|candidate| candidate.delta)
                .sum()
        })
    }

    fn drag_state(&self) -> DragSnap {
        if let Some(node) = self.node {
            return DragSnap {
                node: Some(node.target),
                ..default()
            };
        }
        DragSnap {
            axes: self.axes.map(|axis| axis.map(|candidate| candidate.target)),
            guides: self
                .axes
                .map(|axis| axis.map(|candidate| candidate.coordinate as f32)),
            ..default()
        }
    }
}

#[derive(Clone, Copy)]
struct Tolerance {
    acquire: f64,
    release: f64,
}

impl Tolerance {
    fn for_view(state: &ViewState) -> Self {
        let zoom = f64::from(state.zoom);
        let tolerance = f64::from(state.object_snap.tolerance_px);
        Self {
            acquire: tolerance / zoom,
            release: (tolerance + RELEASE_MARGIN_PX) / zoom,
        }
    }

    fn consider(self, best: &mut Option<Candidate>, candidate: Candidate) {
        let limit = if candidate.held {
            self.release
        } else {
            self.acquire
        };
        if candidate.distance <= limit && best.is_none_or(|current| candidate.prefer(current)) {
            *best = Some(candidate);
        }
    }
}

fn visible_paint(entry: &EntityRef) -> bool {
    entry.get::<crate::CanvasPaint>().is_none_or(|paint| {
        paint.fill.is_some_and(crate::CanvasFill::visible)
            || paint
                .stroke
                .is_some_and(|stroke| stroke.color.alpha() > 0.0)
    })
}

pub(crate) fn connector_anchor(
    world: &World,
    view: Entity,
    point: Vec2,
    held: Option<Attachment>,
) -> CanvasResult<Option<AnchorSnap>> {
    let Some(state) = world
        .get::<ViewState>(view)
        .filter(|state| state.object_snap.connector_anchors)
    else {
        return Ok(None);
    };
    let Some(document) = state.document else {
        return Ok(None);
    };
    let point = point.as_dvec2();
    if !point.is_finite() || world.get::<crate::CanvasDocument>(document).is_none() {
        return Ok(None);
    }
    let tolerance = Tolerance::for_view(state);
    let mut candidate = None;
    let mut draw_index = 0;
    let order = crate::order::OrderComponents::new(world);
    let layers = crate::order::stack(world, document)?
        .layers()
        .iter()
        .filter(|layer| layer.visible());
    crate::order::visit_draw_order(
        world,
        layers,
        |entity, container, entry| -> CanvasResult<bool> {
            let Some(entry) =
                entry.filter(|entry| order.is_valid(entry, container.parent(document)))
            else {
                return Ok(false);
            };
            if entry.contains::<crate::CanvasGroup>() {
                return Ok(true);
            }
            let Some(geometry) = entry
                .get::<CanvasGeometry>()
                .filter(|_| visible_paint(entry))
            else {
                return Ok(false);
            };
            draw_index += 1;
            let projection = crate::groups::document_transform(world, entity)?;
            for (anchor, local) in crate::CanvasAnchor::ALL
                .into_iter()
                .zip(crate::anchors::local_points(geometry)?)
            {
                let Some(local) = local else {
                    continue;
                };
                let delta = project(projection, local) - point;
                tolerance.consider(
                    &mut candidate,
                    Candidate {
                        target: Target {
                            entity,
                            feature: Feature::ConnectorAnchor(anchor),
                        },
                        draw_index,
                        distance: delta.length(),
                        held: held
                            == Some(Attachment {
                                target: entity,
                                anchor,
                            }),
                        coordinate: 0.0,
                        delta,
                    },
                );
            }
            Ok(false)
        },
    )?;
    Ok(candidate.and_then(|found| {
        let Feature::ConnectorAnchor(anchor) = found.target.feature else {
            return None;
        };
        let point = (point + found.delta).as_vec2();
        point.is_finite().then_some(AnchorSnap {
            attachment: Attachment {
                target: found.target.entity,
                anchor,
            },
            point,
        })
    }))
}

pub(crate) fn drag_connector_anchor(
    world: &mut World,
    view: Entity,
    point: Vec2,
) -> CanvasResult<Option<AnchorSnap>> {
    let held = world.get::<DragSnap>(view).and_then(|state| state.anchor);
    let snapped = connector_anchor(world, view, point, held)?;
    if let Some(hit) = snapped
        && let Some(mut drag) = world.get_mut::<DragSnap>(view)
    {
        *drag = DragSnap {
            anchor: Some(hit.attachment),
            ..default()
        };
    }
    Ok(snapped)
}

fn axes(bounds: Option<Rect>, point: DVec2, axis: usize) -> [f64; 3] {
    bounds.map_or([point[axis]; 3], |bounds| {
        let min = f64::from(bounds.min[axis]);
        let max = f64::from(bounds.max[axis]);
        [min, f64::midpoint(min, max), max]
    })
}

struct EdgeSource {
    point: DVec2,
    bounds: Option<Rect>,
    previous: [Option<Target>; 2],
}

impl EdgeSource {
    fn collect(
        &self,
        matches: &mut Matches,
        other: Rect,
        entity: Entity,
        draw_index: usize,
        tolerance: Tolerance,
    ) {
        let source_count = if self.bounds.is_some() { 3 } else { 1 };
        for axis in 0..2 {
            let targets = axes(Some(other), DVec2::ZERO, axis);
            for (source, at) in axes(self.bounds, self.point, axis)
                .into_iter()
                .take(source_count)
                .enumerate()
            {
                for (target_axis, coordinate) in targets.into_iter().enumerate() {
                    let target = Target {
                        entity,
                        feature: Feature::Edge {
                            source,
                            target: target_axis,
                        },
                    };
                    let mut delta = DVec2::ZERO;
                    delta[axis] = coordinate - at;
                    tolerance.consider(
                        &mut matches.axes[axis],
                        Candidate {
                            target,
                            draw_index,
                            distance: delta[axis].abs(),
                            held: self.previous[axis] == Some(target),
                            coordinate,
                            delta,
                        },
                    );
                }
            }
        }
    }
}

fn matches(
    world: &World,
    view: Entity,
    point: DVec2,
    bounds: Option<Rect>,
    previous: DragSnap,
) -> CanvasResult<Matches> {
    let mut matches = Matches::default();
    let Some(state) = world.get::<ViewState>(view) else {
        return Ok(matches);
    };
    let settings = state.object_snap;
    let Some(document) = state
        .document
        .filter(|&document| world.get::<crate::CanvasDocument>(document).is_some())
    else {
        return Ok(matches);
    };
    if !point.is_finite() || !(settings.edges || settings.path_nodes) {
        return Ok(matches);
    }
    let tolerance = Tolerance::for_view(state);
    let selected: EntityHashSet = state.selection.iter().copied().collect();
    let mut ancestors = EntityHashSet::default();
    for &item in &selected {
        let mut item = item;
        while let Some(parent) = world.get::<ChildOf>(item).map(ChildOf::parent) {
            if parent == document || !ancestors.insert(parent) {
                break;
            }
            item = parent;
        }
    }
    let components = crate::order::OrderComponents::new(world);
    let mut draw_index = 0;
    let mut items = Vec::new();
    let layers = crate::order::stack(world, document)?.layers();
    crate::order::visit_draw_order(
        world,
        layers.iter().filter(|layer| layer.visible()),
        |entity, container, entry| -> CanvasResult<bool> {
            let Some(entry) =
                entry.filter(|entry| components.is_valid(entry, container.parent(document)))
            else {
                return Ok(false);
            };
            if selected.contains(&entity) {
                return Ok(false);
            }
            if !visible_paint(entry) {
                return Ok(false);
            }
            let is_group = entry.contains::<crate::CanvasGroup>();
            draw_index += 1;
            let path = entry
                .get::<CanvasGeometry>()
                .and_then(|geometry| match geometry {
                    CanvasGeometry::Path(path) if settings.path_nodes => Some(path),
                    _ => None,
                });
            let bounds = if !is_group && (settings.edges || path.is_some()) {
                crate::groups::document_bounds(world, entity)?
            } else {
                None
            };
            if let Some(path) = path.filter(|_| bounds.is_some()) {
                let projection = crate::groups::document_transform(world, entity)?;
                for (index, command) in path.commands.iter().enumerate() {
                    use crate::CanvasPathCommand::*;
                    let local = match *command {
                        MoveTo(to)
                        | LineTo(to)
                        | QuadTo { to, .. }
                        | CubicTo { to, .. }
                        | ArcTo { to, .. } => to,
                        Close => continue,
                    };
                    let delta = project(projection, local.as_dvec2()) - point;
                    let target = Target {
                        entity,
                        feature: Feature::Node(index),
                    };
                    tolerance.consider(
                        &mut matches.node,
                        Candidate {
                            target,
                            draw_index,
                            distance: delta.length(),
                            held: previous.node == Some(target),
                            coordinate: 0.0,
                            delta,
                        },
                    );
                }
            }
            if settings.edges && (bounds.is_some() || is_group && !ancestors.contains(&entity)) {
                items.push((entity, container, bounds, draw_index));
            }
            Ok(is_group)
        },
    )?;
    let edges = EdgeSource {
        bounds,
        point,
        previous: previous.axes,
    };
    let mut group_bounds = EntityHashMap::<Rect>::default();
    for (entity, container, leaf_bounds, draw_index) in items.into_iter().rev() {
        let other = leaf_bounds.or_else(|| group_bounds.remove(&entity));
        let Some(other) = other else {
            continue;
        };
        if let crate::CanvasContainer::Group(parent) = container
            && !ancestors.contains(&parent)
        {
            group_bounds
                .entry(parent)
                .and_modify(|bounds| *bounds = bounds.union(other))
                .or_insert(other);
        }
        edges.collect(&mut matches, other, entity, draw_index, tolerance);
    }
    Ok(matches)
}

pub(crate) fn point(world: &World, view: Entity, point: Vec2) -> CanvasResult<Option<Vec2>> {
    let point = point.as_dvec2();
    Ok(matches(world, view, point, None, DragSnap::default())?
        .correction()
        .map(|delta| (point + delta).as_vec2())
        .filter(|point| point.is_finite()))
}

pub(crate) fn project(projection: Affine3A, point: DVec2) -> DVec2 {
    let [x, y] = crate::groups::planar_axes(&projection);
    projection.translation.truncate().as_dvec2() + x * point.x + y * point.y
}

pub(crate) fn local_delta(
    world: &mut World,
    view: Entity,
    projection: Affine3A,
    point: DVec2,
    bounds: Option<Rect>,
) -> CanvasResult<Option<DVec2>> {
    let previous = world.get::<DragSnap>(view).copied().unwrap_or_default();
    let found = matches(world, view, project(projection, point), bounds, previous)?;
    if let Some(mut state) = world.get_mut::<DragSnap>(view) {
        *state = found.drag_state();
    }
    Ok(found.correction().map(|delta| {
        let [x, y] = crate::groups::planar_axes(&projection);
        let determinant = x.perp_dot(y);
        DVec2::new(
            delta.perp_dot(y) / determinant,
            x.perp_dot(delta) / determinant,
        )
    }))
}

pub(crate) fn moving_bounds(
    world: &World,
    view: Entity,
    hit: Entity,
    parent: Affine3A,
    proposed: DVec2,
) -> CanvasResult<Option<Rect>> {
    let Some(state) = world
        .get::<ViewState>(view)
        .filter(|state| state.object_snap.edges)
    else {
        return Ok(None);
    };
    let (mut combined, mut offset) = (None::<Rect>, Vec2::ZERO);
    for &item in &state.selection {
        if item == hit {
            let current = crate::groups::local_transform(world, hit)?
                .translation
                .truncate()
                .as_dvec2();
            let [x, y] = crate::groups::planar_axes(&parent);
            let delta = proposed - current;
            offset = (x * delta.x + y * delta.y).as_vec2();
        }
        if let Some(bounds) = crate::groups::document_bounds(world, item)? {
            combined = Some(combined.map_or(bounds, |combined| combined.union(bounds)));
        }
    }
    Ok(combined.map(|bounds| Rect {
        min: bounds.min + offset,
        max: bounds.max + offset,
    }))
}
