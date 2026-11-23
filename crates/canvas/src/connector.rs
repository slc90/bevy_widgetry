use crate::error::{CanvasResult, PublicResult, invalid};
use crate::{
    CanvasContainer, CanvasDocument, CanvasEditTarget, CanvasGeometry, CanvasIssue, CanvasLayerId,
    CanvasPaint, CanvasPath, CanvasPathCommand, CanvasStroke, CanvasText, CanvasTextAlign,
    CanvasTextStyle,
};
use bevy::{math::Affine3A, prelude::*};
use bevy_widgetry_diagnostics::ErrorFrame;

const ARROW_LENGTH_IN_STROKE_WIDTHS: f64 = 4.0;
const ARROW_HALF_WIDTH_RATIO: f64 = 0.4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum CanvasArrow {
    #[default]
    None,
    Triangle,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CanvasConnectorRoute {
    pub start: Vec2,
    pub waypoints: Vec<Vec2>,
    pub end: Vec2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub struct CanvasConnectorStyle {
    pub stroke: CanvasStroke,
    pub start_arrow: CanvasArrow,
    pub end_arrow: CanvasArrow,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CanvasConnectorLabel {
    pub text: String,
    pub font_size: f32,
    pub color: Color,
    /// Normalized distance along the route, from the start at 0 to the end at 1.
    pub position: f32,
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
#[require(crate::CanvasItem, Transform)]
pub struct CanvasConnector {
    route: CanvasConnectorRoute,
    style: CanvasConnectorStyle,
    label: Option<CanvasConnectorLabel>,
    layout: Option<CanvasText>,
}

impl CanvasConnectorRoute {
    pub(crate) fn from_vertices(vertices: &[Vec2]) -> Option<Self> {
        let (&start, rest) = vertices.split_first()?;
        let (&end, waypoints) = rest.split_last()?;
        Some(Self {
            start,
            waypoints: waypoints.to_vec(),
            end,
        })
    }

    pub(crate) fn vertices(&self) -> impl Iterator<Item = Vec2> + '_ {
        std::iter::once(self.start)
            .chain(self.waypoints.iter().copied())
            .chain(std::iter::once(self.end))
    }

    fn segments(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.vertices()
            .zip(self.vertices().skip(1))
            .filter(|(from, to)| from != to)
    }

    pub(crate) fn drawable(&self) -> bool {
        self.vertices().all(Vec2::is_finite) && self.segments().next().is_some()
    }

    pub(crate) fn point(&self, index: usize) -> Option<Vec2> {
        self.vertices().nth(index)
    }

    pub(crate) fn set_point(&mut self, index: usize, point: Vec2) {
        if index == 0 {
            self.start = point;
        } else if index == self.waypoints.len() + 1 {
            self.end = point;
        } else if let Some(waypoint) = self.waypoints.get_mut(index - 1) {
            *waypoint = point;
        }
    }

    fn at_fraction(&self, fraction: f32) -> Vec2 {
        let segments = || {
            self.segments().map(|(a, b)| {
                let start = a.as_dvec2();
                let end = b.as_dvec2();
                (start, end, start.distance(end))
            })
        };
        let length: f64 = segments().map(|(_, _, length)| length).sum();
        let mut remaining = length * f64::from(fraction);
        for (start, end, length) in segments() {
            if remaining <= length {
                return start.lerp(end, remaining / length).as_vec2();
            }
            remaining -= length;
        }
        self.end
    }

    pub(crate) fn paths(&self, style: CanvasConnectorStyle) -> (CanvasPath, CanvasPath) {
        use CanvasPathCommand::{Close, LineTo, MoveTo};
        let route = CanvasPath {
            commands: std::iter::once(MoveTo(self.start))
                .chain(self.vertices().skip(1).map(LineTo))
                .collect(),
        };
        let mut arrows = CanvasPath {
            commands: Vec::new(),
        };
        let mut add_arrow = |tip: Vec2, from: Vec2| {
            let tip = tip.as_dvec2();
            let delta = tip - from.as_dvec2();
            let length =
                (f64::from(style.stroke.width) * ARROW_LENGTH_IN_STROKE_WIDTHS).min(delta.length());
            let direction = delta.normalize();
            let base = tip - direction * length;
            let wing = direction.perp() * length * ARROW_HALF_WIDTH_RATIO;
            arrows.commands.extend([
                MoveTo(tip.as_vec2()),
                LineTo((base + wing).as_vec2()),
                LineTo((base - wing).as_vec2()),
                Close,
            ]);
        };
        if style.start_arrow == CanvasArrow::Triangle
            && let Some((_, next)) = self.segments().next()
        {
            add_arrow(self.start, next);
        }
        if style.end_arrow == CanvasArrow::Triangle
            && let Some((previous, _)) = self.segments().last()
        {
            add_arrow(self.end, previous);
        }
        (route, arrows)
    }
}

impl CanvasConnectorStyle {
    pub(crate) fn paint(self) -> CanvasPaint {
        CanvasPaint {
            fill: (self.start_arrow != CanvasArrow::None || self.end_arrow != CanvasArrow::None)
                .then_some(self.stroke.color),
            stroke: Some(self.stroke),
            ..default()
        }
    }
}

impl CanvasConnector {
    pub fn route(&self) -> &CanvasConnectorRoute {
        &self.route
    }

    pub fn style(&self) -> CanvasConnectorStyle {
        self.style
    }

    pub fn label(&self) -> Option<&CanvasConnectorLabel> {
        self.label.as_ref()
    }

    pub(crate) fn geometry(&self) -> (CanvasGeometry, CanvasPaint, Option<CanvasPath>) {
        let (route, arrows) = self.route.paths(self.style);
        (
            CanvasGeometry::Path(route),
            self.style.paint(),
            Some(arrows),
        )
    }

    pub(crate) fn label_at(&self) -> Option<(&CanvasText, Vec2)> {
        Some((
            self.layout.as_ref()?,
            self.route.at_fraction(self.label.as_ref()?.position),
        ))
    }

    pub(crate) fn label_transform(transform: Transform, at: Vec2, depth: f32) -> Transform {
        let mut label = transform;
        label.translation = transform.transform_point(at.extend(0.0));
        CanvasText::projected_transform(label, depth)
    }

    pub(crate) fn label_projection(projection: Affine3A, at: Vec2) -> Affine3A {
        projection * Affine3A::from_translation(at.extend(0.0))
    }

    pub(crate) fn label_bounds(
        &self,
        local: &Transform,
        projection: &Affine3A,
    ) -> CanvasResult<Option<Rect>> {
        self.label_at()
            .map(|(label, at)| label.bounds(local, &Self::label_projection(*projection, at)))
            .transpose()
    }

    pub(crate) fn bounds(&self, local: &Transform, projection: &Affine3A) -> CanvasResult<Rect> {
        if !self.route.drawable() {
            return Err(invalid(CanvasIssue::ConnectorRoute));
        }
        let (route, arrows) = self.route.paths(self.style);
        let paint = self.style.paint();
        let mut bounds =
            CanvasGeometry::Path(route).bounds_in_projection(local, projection, &paint)?;
        if !arrows.commands.is_empty() {
            bounds = bounds.union(arrows.projected_bounds(projection, &CanvasPaint::default())?);
        }
        if let Some(label) = self.label_bounds(local, projection)? {
            bounds = bounds.union(label);
        }
        if !bounds.size().is_finite() {
            return Err(invalid(CanvasIssue::ConnectorRoute));
        }
        Ok(bounds)
    }

    fn new(
        world: &mut World,
        route: CanvasConnectorRoute,
        style: CanvasConnectorStyle,
        mut label: Option<CanvasConnectorLabel>,
    ) -> CanvasResult<Self> {
        let layout = label
            .as_mut()
            .map(|label| {
                if !(0.0..=1.0).contains(&label.position) {
                    return Err(invalid(CanvasIssue::ConnectorLabelPosition));
                }
                let layout = crate::text::measure(
                    world,
                    CanvasTextStyle {
                        content: label.text.clone(),
                        font_size: label.font_size,
                        color: label.color,
                        align: CanvasTextAlign::Center,
                    },
                )?;
                label.text.clone_from(&layout.style().content);
                Ok(layout)
            })
            .transpose()?;
        Ok(Self {
            route,
            style,
            label,
            layout,
        })
    }
}

impl CanvasDocument {
    pub fn create_connector(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        route: CanvasConnectorRoute,
        style: CanvasConnectorStyle,
        label: Option<CanvasConnectorLabel>,
    ) -> PublicResult<Entity> {
        crate::history::apply(world, target, |world, document| {
            let layer = crate::order::stack(world, document)?.default_layer;
            Self::insert_connector(
                world,
                document,
                CanvasContainer::Layer(layer),
                route,
                style,
                label,
            )
        })
        .map_err(|e| e.into_failure(ErrorFrame::new("canvas.CanvasDocument.create_connector")))
    }

    pub fn create_connector_in_layer(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        layer: CanvasLayerId,
        route: CanvasConnectorRoute,
        style: CanvasConnectorStyle,
        label: Option<CanvasConnectorLabel>,
    ) -> PublicResult<Entity> {
        crate::history::apply(world, target, |world, document| {
            Self::insert_connector(
                world,
                document,
                CanvasContainer::Layer(layer),
                route,
                style,
                label,
            )
        })
        .map_err(|e| {
            e.into_failure(ErrorFrame::new(
                "canvas.CanvasDocument.create_connector_in_layer",
            ))
        })
    }

    fn insert_connector(
        world: &mut World,
        document: Entity,
        container: CanvasContainer,
        route: CanvasConnectorRoute,
        style: CanvasConnectorStyle,
        label: Option<CanvasConnectorLabel>,
    ) -> CanvasResult<Entity> {
        let connector = CanvasConnector::new(world, route, style, label)?;
        let projection = match container {
            CanvasContainer::Layer(_) => Affine3A::IDENTITY,
            CanvasContainer::Group(group) => crate::groups::document_transform(world, group)?,
        };
        connector.bounds(&Transform::IDENTITY, &projection)?;
        Self::insert_item(world, document, container, (connector, Transform::IDENTITY))
    }

    pub(crate) fn draw_connector(
        world: &mut World,
        document: Entity,
        container: CanvasContainer,
        mut route: CanvasConnectorRoute,
        mut style: CanvasConnectorStyle,
    ) -> CanvasResult<Entity> {
        if let CanvasContainer::Group(group) = container {
            let parent = crate::groups::document_transform(world, group)?;
            route = CanvasConnectorRoute {
                start: crate::scale::local_point(parent, route.start).as_vec2(),
                end: crate::scale::local_point(parent, route.end).as_vec2(),
                waypoints: route
                    .waypoints
                    .into_iter()
                    .map(|point| crate::scale::local_point(parent, point).as_vec2())
                    .collect(),
            };
            let [x, _] = crate::groups::planar_axes(&parent);
            style.stroke.width = (f64::from(style.stroke.width) / x.length()) as f32;
        }
        crate::history::apply(world, document, |world, document| {
            Self::insert_connector(world, document, container, route, style, None)
        })
    }

    pub fn update_connector(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        item: Entity,
        route: CanvasConnectorRoute,
        style: CanvasConnectorStyle,
        label: Option<CanvasConnectorLabel>,
    ) -> PublicResult {
        crate::history::apply(world, target, |world, document| {
            crate::groups::item_parent(world, document, item)?;
            let previous = connector(world, item)?;
            if previous.route == route && previous.style == style && previous.label == label {
                return Ok(());
            }
            let value = CanvasConnector::new(world, route, style, label)?;
            Self::set_connector(world, document, item, value)
        })
        .map_err(|e| {
            e.at("connector", item)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.update_connector"))
        })
    }

    pub(crate) fn move_connector_point(
        world: &mut World,
        document: Entity,
        item: Entity,
        index: usize,
        point: Vec2,
    ) -> CanvasResult {
        let mut value = connector(world, item)?.clone();
        value.route.set_point(index, point);
        if !value.route.drawable() {
            return Ok(());
        }
        Self::set_connector(world, document, item, value)
    }

    fn set_connector(
        world: &mut World,
        document: Entity,
        item: Entity,
        value: CanvasConnector,
    ) -> CanvasResult {
        value.bounds(
            &crate::groups::local_transform(world, item)?,
            &crate::groups::document_transform(world, item)?,
        )?;
        crate::history::set_connector(world, document, item, value)
    }

    pub fn duplicate_connector(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        item: Entity,
    ) -> PublicResult<Entity> {
        crate::history::apply(world, target, |world, document| {
            let container = crate::groups::item_parent(world, document, item)?;
            let value = connector(world, item)?.clone();
            let transform = crate::groups::local_transform(world, item)?;
            Self::insert_item(world, document, container, (value, transform))
        })
        .map_err(|e| {
            e.at("connector", item)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.duplicate_connector"))
        })
    }

    pub fn remove_connector(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        item: Entity,
    ) -> PublicResult {
        crate::history::apply(world, target, |world, document| {
            connector(world, item)?;
            crate::batch::remove_items(world, document, &[item], crate::groups::item_parent)
        })
        .map_err(|e| {
            e.at("connector", item)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.remove_connector"))
        })
    }
}

fn connector(world: &World, item: Entity) -> CanvasResult<&CanvasConnector> {
    world
        .get::<CanvasConnector>(item)
        .ok_or_else(|| crate::model::invalid_ownership(world, item))
}
