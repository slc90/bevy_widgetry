use crate::error::{CanvasResult, PublicResult, invalid};
use crate::{
    CanvasAnchor, CanvasConnectorEnd, CanvasConnectorEndpoint, CanvasContainer, CanvasDocument,
    CanvasEditTarget, CanvasGeometry, CanvasIssue, CanvasLayerId, CanvasPaint, CanvasPath,
    CanvasPathCommand, CanvasStroke, CanvasText, CanvasTextAlign, CanvasTextStyle,
    anchors::Attachment,
};
use bevy::{math::Affine3A, prelude::*};
use bevy_widgetry_diagnostics::ErrorFrame;

const ARROW_LENGTH_IN_STROKE_WIDTHS: f64 = 4.0;
const ARROW_HALF_WIDTH_RATIO: f64 = 0.4;
const COLLAPSED_MARKER_HALF_SIZE: f32 = 3.0;

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
    attachments: [Option<Attachment>; 2],
    hidden_by_anchor: bool,
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

    fn endpoint(&self, end: CanvasConnectorEnd, local: Transform) -> CanvasConnectorEndpoint {
        let point = match end {
            CanvasConnectorEnd::Start => self.route.start,
            CanvasConnectorEnd::End => self.route.end,
        };
        self.attachments[end.index()].map_or_else(
            || {
                CanvasConnectorEndpoint::Free(
                    crate::snapping::project(local.compute_affine(), point.as_dvec2()).as_vec2(),
                )
            },
            |attachment| CanvasConnectorEndpoint::Attached {
                target: attachment.target,
                anchor: attachment.anchor,
            },
        )
    }

    pub(crate) fn drawn(&self) -> bool {
        self.visible() && self.route.drawable()
    }

    pub(crate) fn visible(&self) -> bool {
        !self.hidden_by_anchor
    }

    pub(crate) fn movement_anchor(&self) -> Vec2 {
        if self.attachments[CanvasConnectorEnd::Start.index()].is_none() {
            self.route.start
        } else if let Some(&waypoint) = self.route.waypoints.first() {
            waypoint
        } else if self.attachments[CanvasConnectorEnd::End.index()].is_none() {
            self.route.end
        } else {
            Vec2::ZERO
        }
    }

    fn has_attachments(&self) -> bool {
        self.attachments.iter().any(Option::is_some)
    }

    pub(crate) fn remap_attachments(&mut self, mut target: impl FnMut(Entity) -> Option<Entity>) {
        self.attachments = self.attachments.map(|attachment| {
            let attachment = attachment?;
            Some(Attachment {
                target: target(attachment.target)?,
                ..attachment
            })
        });
        self.hidden_by_anchor = false;
    }

    fn set_route_endpoint(&mut self, end: CanvasConnectorEnd, point: Vec2) {
        match end {
            CanvasConnectorEnd::Start => self.route.start = point,
            CanvasConnectorEnd::End => self.route.end = point,
        }
    }

    fn refresh_targets(&mut self, world: &World, document: Entity, item: Entity) -> CanvasResult {
        let projection = crate::groups::document_transform(world, item)?;
        self.refresh_in_projection(world, document, projection)
    }

    fn refresh_in_projection(
        &mut self,
        world: &World,
        document: Entity,
        projection: Affine3A,
    ) -> CanvasResult {
        self.refresh_endpoints(projection, |attachment| {
            shape_anchor(world, document, attachment)
        })
    }

    pub(crate) fn refresh_endpoints(
        &mut self,
        projection: Affine3A,
        mut resolve: impl FnMut(Attachment) -> CanvasResult<Option<(bool, Vec2)>>,
    ) -> CanvasResult {
        self.hidden_by_anchor = false;
        if !self.has_attachments() {
            return Ok(());
        }
        for end in CanvasConnectorEnd::ALL {
            let Some(attachment) = self.attachments[end.index()] else {
                continue;
            };
            if let Some((visible, point)) = resolve(attachment)? {
                self.hidden_by_anchor |= !visible;
                self.set_route_endpoint(end, local_anchor(projection, point, attachment.target)?);
            } else {
                self.attachments[end.index()] = None;
            }
        }
        Ok(())
    }

    pub fn style(&self) -> CanvasConnectorStyle {
        self.style
    }

    pub fn label(&self) -> Option<&CanvasConnectorLabel> {
        self.label.as_ref()
    }

    pub(crate) fn geometry(&self) -> (CanvasGeometry, CanvasPaint, Option<CanvasPath>) {
        if !self.route.drawable() {
            let point = self.route.start;
            let radius = COLLAPSED_MARKER_HALF_SIZE;
            let marker = CanvasPath {
                commands: vec![
                    CanvasPathCommand::MoveTo(point - Vec2::X * radius),
                    CanvasPathCommand::LineTo(point - Vec2::Y * radius),
                    CanvasPathCommand::LineTo(point + Vec2::X * radius),
                    CanvasPathCommand::LineTo(point + Vec2::Y * radius),
                    CanvasPathCommand::Close,
                ],
            };
            return (
                CanvasGeometry::Path(marker),
                CanvasPaint {
                    fill: Some(self.style.stroke.color),
                    ..Default::default()
                },
                None,
            );
        }
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
            if self.route.vertices().all(Vec2::is_finite) {
                crate::model::validate_transform_and_paint(local, &self.style.paint())?;
                let point =
                    crate::snapping::project(*projection, self.route.start.as_dvec2()).as_vec2();
                if point.is_finite() {
                    return Ok(Rect::from_center_size(point, Vec2::ZERO));
                }
            }
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
            attachments: [None; crate::CanvasConnectorEnd::ALL.len()],
            hidden_by_anchor: false,
            style,
            label,
            layout,
        })
    }
}

impl CanvasDocument {
    pub fn connector_endpoint(
        world: &World,
        document: Entity,
        item: Entity,
        end: CanvasConnectorEnd,
    ) -> PublicResult<CanvasConnectorEndpoint> {
        (|| -> CanvasResult<_> {
            crate::groups::item_parent(world, document, item)?;
            let local = crate::groups::local_transform(world, item)?;
            Ok(connector(world, item)?.endpoint(end, local))
        })()
        .map_err(|error| {
            error
                .at("connector", item)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.connector_endpoint"))
        })
    }

    pub fn shape_anchor(
        world: &World,
        document: Entity,
        shape: Entity,
        anchor: CanvasAnchor,
    ) -> PublicResult<Option<Vec2>> {
        (|| -> CanvasResult<Option<Vec2>> {
            crate::model::validate_document(world, document)?;
            if crate::order::item_placement(world, document, shape).is_none()
                || world.get::<crate::CanvasShape>(shape).is_none()
            {
                return Err(crate::model::invalid_ownership(world, shape));
            }
            crate::anchors::document_point(world, shape, anchor)
        })()
        .map_err(|error| {
            error
                .at("document", document)
                .at("shape", shape)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.shape_anchor"))
        })
    }

    pub fn set_connector_endpoint(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        item: Entity,
        end: CanvasConnectorEnd,
        endpoint: CanvasConnectorEndpoint,
    ) -> PublicResult<bool> {
        crate::history::apply(world, target, |world, document| {
            crate::groups::item_parent(world, document, item)?;
            let previous = connector(world, item)?;
            let mut value = previous.clone();
            let local = crate::groups::local_transform(world, item)?;
            if value.endpoint(end, local) == endpoint {
                return Ok(false);
            }
            let (point, attachment) = match endpoint {
                CanvasConnectorEndpoint::Free(point) => (
                    crate::scale::local_point(local.compute_affine(), point).as_vec2(),
                    None,
                ),
                CanvasConnectorEndpoint::Attached { target, anchor } => {
                    let attachment = Attachment { target, anchor };
                    let projection = crate::groups::document_transform(world, item)?;
                    let (_, point) = resolve_attachment(world, document, projection, attachment)?
                        .ok_or_else(|| {
                        invalid(CanvasIssue::ConnectorAnchor).at("shape", target)
                    })?;
                    (point, Some(attachment))
                }
            };
            value.attachments[end.index()] = attachment;
            value.set_route_endpoint(end, point);
            value.refresh_targets(world, document, item)?;
            if previous == &value {
                return Ok(false);
            }
            if attachment.is_none() && !value.route.drawable() {
                return Err(invalid(CanvasIssue::ConnectorRoute));
            }
            Self::set_connector(world, document, item, value)?;
            Ok(true)
        })
        .map_err(|error| {
            error.at("connector", item).into_failure(ErrorFrame::new(
                "canvas.CanvasDocument.set_connector_endpoint",
            ))
        })
    }

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
                [None; CanvasConnectorEnd::ALL.len()],
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
                [None; CanvasConnectorEnd::ALL.len()],
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
        attachments: [Option<Attachment>; 2],
    ) -> CanvasResult<Entity> {
        if !route.drawable() {
            return Err(invalid(CanvasIssue::ConnectorRoute));
        }
        let mut connector = CanvasConnector::new(world, route, style, label)?;
        connector.attachments = attachments;
        let projection = match container {
            CanvasContainer::Layer(_) => Affine3A::IDENTITY,
            CanvasContainer::Group(group) => crate::groups::document_transform(world, group)?,
        };
        connector.refresh_in_projection(world, document, projection)?;
        if !connector.route.drawable() {
            return Err(invalid(CanvasIssue::ConnectorRoute));
        }
        connector.bounds(&Transform::IDENTITY, &projection)?;
        Self::insert_item(world, document, container, (connector, Transform::IDENTITY))
    }

    pub(crate) fn draw_connector(
        world: &mut World,
        document: Entity,
        container: CanvasContainer,
        mut route: CanvasConnectorRoute,
        mut style: CanvasConnectorStyle,
        attachments: [Option<Attachment>; 2],
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
            Self::insert_connector(world, document, container, route, style, None, attachments)
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
            if !route.drawable() && previous.route != route {
                return Err(invalid(CanvasIssue::ConnectorRoute));
            }
            let attachments = previous.attachments;
            let mut value = CanvasConnector::new(world, route, style, label)?;
            value.attachments = attachments;
            value.refresh_targets(world, document, item)?;
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
        attachment: Option<Attachment>,
    ) -> CanvasResult<bool> {
        let mut value = connector(world, item)?.clone();
        value.route.set_point(index, point);
        if !value.route.drawable() {
            return Ok(false);
        }
        if let Some(end) = CanvasConnectorEnd::for_point(index, value.route.waypoints.len()) {
            value.attachments[end.index()] = attachment;
            value.refresh_targets(world, document, item)?;
        }
        Self::set_connector(world, document, item, value)?;
        Ok(true)
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

fn resolve_attachment(
    world: &World,
    document: Entity,
    projection: Affine3A,
    attachment: Attachment,
) -> CanvasResult<Option<(bool, Vec2)>> {
    shape_anchor(world, document, attachment)?
        .map(|(visible, point)| Ok((visible, local_anchor(projection, point, attachment.target)?)))
        .transpose()
}

fn shape_anchor(
    world: &World,
    document: Entity,
    attachment: Attachment,
) -> CanvasResult<Option<(bool, Vec2)>> {
    let Some((_, layer)) = crate::order::item_placement(world, document, attachment.target)
        .filter(|_| world.get::<crate::CanvasShape>(attachment.target).is_some())
    else {
        return Ok(None);
    };
    Ok(
        crate::anchors::document_point(world, attachment.target, attachment.anchor)?
            .map(|point| (layer.visible(), point)),
    )
}

fn local_anchor(projection: Affine3A, point: Vec2, target: Entity) -> CanvasResult<Vec2> {
    let local = crate::scale::local_point(projection, point).as_vec2();
    if !local.is_finite() {
        return Err(invalid(CanvasIssue::ConnectorAnchor).at("shape", target));
    }
    Ok(local)
}

fn attached_connectors(world: &mut World, document: Entity) -> Vec<(Entity, CanvasConnector)> {
    if world.component_id::<CanvasConnector>().is_none() {
        return Vec::new();
    }
    world
        .query::<(Entity, &CanvasConnector)>()
        .iter(world)
        .filter(|(item, value)| {
            value.has_attachments()
                && crate::order::item_placement(world, document, *item).is_some()
        })
        .map(|(item, value)| (item, value.clone()))
        .collect()
}

fn attachment_updates(
    world: &World,
    document: Entity,
    attached: Vec<(Entity, CanvasConnector)>,
) -> CanvasResult<Vec<(Entity, CanvasConnector)>> {
    let mut updates = Vec::new();
    for (item, mut value) in attached {
        value.refresh_targets(world, document, item)?;
        if connector(world, item)? != &value {
            value
                .bounds(
                    &crate::groups::local_transform(world, item)?,
                    &crate::groups::document_transform(world, item)?,
                )
                .map_err(|error| error.at("connector", item))?;
            updates.push((item, value));
        }
    }
    Ok(updates)
}

pub(crate) fn validate_following(
    world: &mut World,
    document: Entity,
    transforms: &[(Entity, Transform)],
    geometry: Option<(Entity, &CanvasGeometry)>,
) -> CanvasResult {
    preview_following(world, document, transforms, geometry, |_, _| Ok(()))
}

pub(crate) fn validate_following_layout(
    world: &mut World,
    document: Entity,
    transforms: &[(Entity, Transform)],
    validate: impl FnOnce(&World) -> CanvasResult,
) -> CanvasResult {
    preview_following(world, document, transforms, None, |world, mut routes| {
        if routes.is_empty() {
            return Ok(());
        }
        let swap_routes = |world: &mut World, routes: &mut [(Entity, CanvasConnector)]| {
            for (entity, route) in routes {
                if let Some(mut current) = world.get_mut::<CanvasConnector>(*entity) {
                    std::mem::swap(current.bypass_change_detection(), route);
                }
            }
        };
        swap_routes(world, &mut routes);
        let result = validate(world);
        swap_routes(world, &mut routes);
        result
    })
}

fn preview_following(
    world: &mut World,
    document: Entity,
    transforms: &[(Entity, Transform)],
    geometry: Option<(Entity, &CanvasGeometry)>,
    validate: impl FnOnce(&mut World, Vec<(Entity, CanvasConnector)>) -> CanvasResult,
) -> CanvasResult {
    let attached = attached_connectors(world, document);
    if attached.is_empty() {
        return Ok(());
    }
    let previous = transforms
        .iter()
        .map(|&(item, _)| {
            crate::groups::local_transform(world, item).map(|transform| (item, transform))
        })
        .collect::<CanvasResult<Vec<_>>>()?;
    let previous_geometry = geometry
        .map(|(item, _)| {
            world
                .get::<CanvasGeometry>(item)
                .cloned()
                .map(|geometry| (item, geometry))
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::Geometry).at("shape", item))
        })
        .transpose()?;
    // Preview the complete edit without running observers or changing the live component ticks.
    let set_transforms = |world: &mut World, values: &[(Entity, Transform)]| {
        for &(item, transform) in values {
            if let Some(mut current) = world.get_mut::<Transform>(item) {
                *current.bypass_change_detection() = transform;
            }
        }
    };
    set_transforms(world, transforms);
    if let Some((item, geometry)) = geometry
        && let Some(mut current) = world.get_mut::<CanvasGeometry>(item)
    {
        current.bypass_change_detection().clone_from(geometry);
    }
    let validation =
        attachment_updates(world, document, attached).and_then(|routes| validate(world, routes));
    set_transforms(world, &previous);
    if let Some((item, geometry)) = previous_geometry
        && let Some(mut current) = world.get_mut::<CanvasGeometry>(item)
    {
        *current.bypass_change_detection() = geometry;
    }
    validation
}

pub(crate) fn refresh_attachments(world: &mut World, document: Entity) -> CanvasResult {
    let attached = attached_connectors(world, document);
    for (item, value) in attachment_updates(world, document, attached)? {
        crate::history::set_connector(world, document, item, value)?;
    }
    Ok(())
}

fn connector(world: &World, item: Entity) -> CanvasResult<&CanvasConnector> {
    world
        .get::<CanvasConnector>(item)
        .ok_or_else(|| crate::model::invalid_ownership(world, item))
}
