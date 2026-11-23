use crate::error::{CanvasResult, PublicResult, invalid, missing};
use crate::{CanvasConnector, CanvasInput, CanvasIssue, CanvasPart, CanvasView, view::ViewState};
use bevy::{
    math::{Affine3A, DVec2},
    prelude::*,
};
use bevy_widgetry_diagnostics::ErrorFrame;

const SNAP_RADIUS: f64 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CanvasGrid {
    pub visible: bool,
    pub snap: CanvasGridSnap,
    pub spacing: f32,
    pub major_every: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub struct CanvasGridSnap {
    pub drawing: bool,
    pub movement: bool,
    pub path_nodes: bool,
    pub connector_points: bool,
}

impl CanvasGridSnap {
    pub const ALL: Self = Self {
        drawing: true,
        movement: true,
        path_nodes: true,
        connector_points: true,
    };
}

impl Default for CanvasGrid {
    fn default() -> Self {
        Self {
            visible: false,
            snap: default(),
            spacing: 25.0,
            major_every: Some(4),
        }
    }
}

#[derive(EntityEvent)]
pub struct CanvasGridChanged {
    pub entity: Entity,
    pub grid: CanvasGrid,
}

impl CanvasGrid {
    pub(crate) fn validate(self) -> CanvasResult {
        if !self.spacing.is_finite() || self.spacing <= 0.0 {
            return Err(invalid(CanvasIssue::GridSpacing).input(CanvasInput::Float {
                field: "spacing",
                value: self.spacing,
            }));
        }
        if self.major_every == Some(0) {
            return Err(invalid(CanvasIssue::GridMajorInterval));
        }
        Ok(())
    }

    fn snap_point(self, zoom: f32, point: DVec2) -> Option<DVec2> {
        let step = f64::from(self.spacing);
        let nearest = (point / step).round() * step;
        (nearest.as_vec2().is_finite()
            && (nearest - point).length() * f64::from(zoom) <= SNAP_RADIUS)
            .then_some(nearest)
    }
}

impl CanvasView {
    pub fn grid(world: &World, view: Entity) -> PublicResult<CanvasGrid> {
        world
            .get::<ViewState>(view)
            .map(|state| state.grid)
            .ok_or_else(|| {
                missing(CanvasPart::View)
                    .at("view", view)
                    .into_failure(ErrorFrame::new("canvas.CanvasView.grid"))
            })
    }

    pub fn set_grid_in_world(
        world: &mut World,
        view: Entity,
        grid: CanvasGrid,
    ) -> PublicResult<bool> {
        set_grid(world, view, grid)
            .map_err(|error| error.into_failure(ErrorFrame::new("canvas.CanvasView.set_grid")))
    }

    pub fn set_grid(commands: &mut Commands, view: Entity, grid: CanvasGrid) {
        commands.queue(move |world: &mut World| {
            set_grid(world, view, grid).map(|_| ()).map_err(|error| {
                error.into_bevy_error(ErrorFrame::new("canvas.CanvasView.set_grid"))
            })
        });
    }
}

fn set_grid(world: &mut World, view: Entity, grid: CanvasGrid) -> CanvasResult<bool> {
    (|| -> CanvasResult<bool> {
        grid.validate()?;
        let mut state = world
            .get_mut::<ViewState>(view)
            .ok_or_else(|| missing(CanvasPart::View))?;
        if state.grid == grid {
            return Ok(false);
        }
        state.grid = grid;
        world.trigger(CanvasGridChanged { entity: view, grid });
        Ok(true)
    })()
    .map_err(|err| err.at("view", view))
}

pub(crate) enum SnapTarget {
    Drawing,
    Movement,
    PathNode,
    ConnectorPoint,
}

fn snap_state(world: &World, view: Entity, target: SnapTarget) -> Option<&ViewState> {
    world.get::<ViewState>(view).filter(|state| match target {
        SnapTarget::Drawing => state.grid.snap.drawing,
        SnapTarget::Movement => state.grid.snap.movement,
        SnapTarget::PathNode => state.grid.snap.path_nodes,
        SnapTarget::ConnectorPoint => state.grid.snap.connector_points,
    })
}

pub(crate) fn point(
    world: &World,
    view: Entity,
    target: SnapTarget,
    point: Vec2,
) -> CanvasResult<Option<Vec2>> {
    if let Some(point) = crate::snapping::point(world, view, point)? {
        return Ok(Some(point));
    }
    Ok(grid_point(world, view, target, point))
}

pub(crate) fn drag_point(
    world: &mut World,
    view: Entity,
    point: Vec2,
) -> CanvasResult<Option<Vec2>> {
    let object =
        crate::snapping::local_delta(world, view, Affine3A::IDENTITY, point.as_dvec2(), None)?;
    Ok(object
        .map(|delta| (point.as_dvec2() + delta).as_vec2())
        .or_else(|| grid_point(world, view, SnapTarget::Drawing, point)))
}

fn grid_point(world: &World, view: Entity, target: SnapTarget, point: Vec2) -> Option<Vec2> {
    let state = snap_state(world, view, target)?;
    state
        .grid
        .snap_point(state.zoom, point.as_dvec2())
        .map(DVec2::as_vec2)
}

pub(crate) fn local_delta(
    world: &mut World,
    view: Entity,
    target: SnapTarget,
    projection: Affine3A,
    point: DVec2,
    bounds: Option<Rect>,
) -> CanvasResult<DVec2> {
    if let Some(delta) = crate::snapping::local_delta(world, view, projection, point, bounds)? {
        return Ok(delta);
    }
    let Some(state) = snap_state(world, view, target) else {
        return Ok(DVec2::ZERO);
    };
    let [x, y] = crate::groups::planar_axes(&projection);
    let document = projection.translation.truncate().as_dvec2() + x * point.x + y * point.y;
    let Some(nearest) = state.grid.snap_point(state.zoom, document) else {
        return Ok(DVec2::ZERO);
    };
    let delta = nearest - document;
    let determinant = x.perp_dot(y);
    Ok(DVec2::new(
        delta.perp_dot(y) / determinant,
        x.perp_dot(delta) / determinant,
    ))
}

pub(crate) fn move_anchor_offset(world: &World, item: Entity, transform: Transform) -> DVec2 {
    world
        .get::<CanvasConnector>(item)
        .map_or(DVec2::ZERO, |connector| {
            let start = connector.movement_anchor().as_dvec2();
            let [x, y] = crate::groups::planar_axes(&transform.compute_affine());
            x * start.x + y * start.y
        })
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn snapping_uses_the_same_document_grid_with_a_logical_pixel_radius() {
        let grid = CanvasGrid::default();
        for (point, zoom, expected) in [
            (DVec2::new(-47.0, 26.0), 1.0, DVec2::new(-50.0, 25.0)),
            (DVec2::new(-47.0, 26.0), 2.0, DVec2::new(-47.0, 26.0)),
            (DVec2::new(-49.0, 26.0), 2.0, DVec2::new(-50.0, 25.0)),
            (DVec2::new(-44.0, 25.0), 1.0, DVec2::new(-50.0, 25.0)),
        ] {
            assert_eq!(grid.snap_point(zoom, point).unwrap_or(point), expected);
        }
    }
}
