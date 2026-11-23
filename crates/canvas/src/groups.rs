use crate::error::{CanvasResult as Result, PublicResult, invalid, missing};
use crate::order::{CanvasItemOrder, container_items, item_container, valid_items};
use crate::{
    CanvasContainer, CanvasDocument, CanvasGeometry, CanvasIssue, CanvasItem, CanvasPaint,
    CanvasPart,
};
use bevy::ecs::world::EntityRef;
use bevy::{
    ecs::entity::EntityHashSet,
    math::{Affine3A, DVec2},
    prelude::*,
};
use bevy_widgetry_diagnostics::ErrorFrame;

// Allow up to a tenth of a logical pixel per axis at the maximum View zoom.
const POSITION_ROUND_TRIP_TOLERANCE: f32 = 0.1 / crate::view::MAX_ZOOM;
const TRANSFORM_ROUND_TRIP_TOLERANCE: f32 = 2.0 * f32::EPSILON;

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
#[require(CanvasItem, CanvasItemOrder, Transform)]
pub struct CanvasGroup {
    name: String,
}

impl CanvasGroup {
    pub(crate) fn new(name: String) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub(crate) fn has_group_components(item: &EntityRef) -> bool {
    item.contains::<CanvasGroup>()
        && item.contains::<CanvasItemOrder>()
        && item.contains::<Transform>()
}

impl CanvasDocument {
    /// Measures the item and its group descendants in document coordinates for export.
    /// Includes vector curve extrema and rendered mesh extents.
    pub fn item_bounds(
        world: &World,
        document: Entity,
        item: Entity,
    ) -> PublicResult<Option<Rect>> {
        item_parent(world, document, item)
            .and_then(|_| measure_document_bounds(world, item, ProjectionCheck::MeasureForExport))
            .map_err(|error| {
                error
                    .at("document", document)
                    .at("item", item)
                    .into_failure(ErrorFrame::new("canvas.CanvasDocument.item_bounds"))
            })
    }

    pub fn container_of(
        world: &World,
        document: Entity,
        item: Entity,
    ) -> PublicResult<CanvasContainer> {
        item_parent(world, document, item).map_err(|error| {
            error
                .at("document", document)
                .at("item", item)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.container_of"))
        })
    }

    pub fn group_items(
        world: &mut World,
        target: impl Into<crate::CanvasEditTarget>,
        items: &[Entity],
        name: impl Into<String>,
    ) -> PublicResult<Entity> {
        let target = target.into();
        let document = target.document();
        crate::history::apply(world, target, |world, document| {
            group_items(world, document, items, name.into())
        })
        .map_err(|error| {
            error
                .at("document", document)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.group_items"))
        })
    }

    pub fn ungroup(
        world: &mut World,
        target: impl Into<crate::CanvasEditTarget>,
        group: Entity,
    ) -> PublicResult<Vec<Entity>> {
        let target = target.into();
        let document = target.document();
        crate::history::apply(world, target, |world, document| {
            ungroup(world, document, group)
        })
        .map_err(|error| {
            error
                .at("document", document)
                .at("group", group)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.ungroup"))
        })
    }

    pub fn update_group_transform(
        world: &mut World,
        target: impl Into<crate::CanvasEditTarget>,
        group: Entity,
        transform: Transform,
    ) -> PublicResult {
        let target = target.into();
        let document = target.document();
        crate::history::apply(world, target, |world, document| {
            group_container(world, document, group)?;
            set_group_transform(world, document, group, transform)
        })
        .map_err(|error| {
            error
                .at("document", document)
                .at("group", group)
                .input(crate::CanvasInput::Transform(transform))
                .into_failure(ErrorFrame::new(
                    "canvas.CanvasDocument.update_group_transform",
                ))
        })
    }

    pub fn document_transform(
        world: &World,
        document: Entity,
        item: Entity,
    ) -> PublicResult<Affine3A> {
        item_parent(world, document, item)
            .and_then(|_| document_transform(world, item))
            .map_err(|error| {
                error
                    .at("document", document)
                    .at("item", item)
                    .into_failure(ErrorFrame::new("canvas.CanvasDocument.document_transform"))
            })
    }
}

pub(crate) fn update_item_transform(
    world: &mut World,
    document: Entity,
    item: Entity,
    transform: Transform,
) -> Result {
    if world.get::<crate::CanvasText>(item).is_some()
        || world.get::<crate::CanvasConnector>(item).is_some()
    {
        item_parent(world, document, item)?;
        validate_item_transform(world, item, transform)?;
        return crate::history::set_transform(world, document, item, transform);
    }
    if world.get::<CanvasGroup>(item).is_none() {
        return CanvasDocument::_update_transform(world, document, item, transform);
    }
    group_container(world, document, item)?;
    set_group_transform(world, document, item, transform)
}

fn set_group_transform(
    world: &mut World,
    document: Entity,
    group: Entity,
    transform: Transform,
) -> Result {
    validate_item_transform(world, group, transform)?;
    crate::history::set_transform(world, document, group, transform)
}

pub(crate) fn item_parent(
    world: &World,
    document: Entity,
    item: Entity,
) -> Result<CanvasContainer> {
    crate::model::validate_document(world, document)?;
    item_container(world, document, item)
        .filter(|container| {
            valid_items(world, container.parent(document), &[item])
                .next()
                .is_some()
        })
        .ok_or_else(|| crate::model::invalid_ownership(world, item).at("item", item))
}

fn group_container(world: &World, document: Entity, group: Entity) -> Result<CanvasContainer> {
    if world.get::<CanvasGroup>(group).is_none() {
        return Err(invalid(CanvasIssue::GroupTarget));
    }
    item_parent(world, document, group)
}

pub(crate) fn local_transform(world: &World, item: Entity) -> Result<Transform> {
    world
        .get::<Transform>(item)
        .copied()
        .ok_or_else(|| missing(CanvasPart::Transform).at("item", item))
}

pub(crate) fn validate_group_transform(transform: Transform) -> Result {
    let scale = transform.scale;
    if !transform.translation.is_finite()
        || !scale.is_finite()
        || scale.x <= 0.0
        || scale.x != scale.y
        || scale.z != 1.0
        || !transform.rotation.is_normalized()
        || transform.rotation.x != 0.0
        || transform.rotation.y != 0.0
    {
        return Err(
            invalid(CanvasIssue::GroupTransform).input(crate::CanvasInput::Transform(transform))
        );
    }
    Ok(())
}

pub(crate) fn parent_transform(world: &World, item: Entity) -> Result<Affine3A> {
    let mut ancestors = Vec::new();
    let mut current = item;
    let mut visited = EntityHashSet::default();
    while let Some(parent) = world.get::<ChildOf>(current).map(ChildOf::parent) {
        if world.get::<CanvasDocument>(parent).is_some()
            || world.get::<CanvasGroup>(parent).is_none()
        {
            break;
        }
        if !visited.insert(parent) {
            return Err(crate::model::invalid_ownership(world, current));
        }
        let transform = local_transform(world, parent)?;
        validate_group_transform(transform)?;
        ancestors.push(transform.compute_affine());
        current = parent;
    }
    // Compose in hierarchy order, as Transform propagation does for the View proxies.
    Ok(ancestors
        .into_iter()
        .rev()
        .fold(Affine3A::IDENTITY, |parent, local| parent * local))
}

pub(crate) fn document_transform(world: &World, item: Entity) -> Result<Affine3A> {
    Ok(parent_transform(world, item)? * local_transform(world, item)?.compute_affine())
}

pub(crate) fn planar_axes(transform: &Affine3A) -> [DVec2; 2] {
    [transform.matrix3.x_axis, transform.matrix3.y_axis]
        .map(|axis| DVec2::new(f64::from(axis.x), f64::from(axis.y)))
}

fn preserves_projection(expected: Affine3A, actual: Affine3A) -> bool {
    actual.is_finite()
        && actual
            .translation
            .abs_diff_eq(expected.translation, POSITION_ROUND_TRIP_TOLERANCE)
        && planar_axes(&expected)
            .into_iter()
            .zip(planar_axes(&actual))
            .all(|(expected, actual)| {
                let tolerance = expected.length() * f64::from(TRANSFORM_ROUND_TRIP_TOLERANCE);
                actual.abs_diff_eq(expected, tolerance)
            })
}

fn preserves_geometry(
    geometry: &CanvasGeometry,
    paint: &CanvasPaint,
    expected: Affine3A,
    actual: Affine3A,
) -> Result<bool> {
    let [expected_x, expected_y] = planar_axes(&expected);
    let [actual_x, actual_y] = planar_axes(&actual);
    if expected_x == actual_x && expected_y == actual_y {
        return Ok(true);
    }
    let axes = [actual_x - expected_x, actual_y - expected_y];
    let offset =
        actual.translation.truncate().as_dvec2() - expected.translation.truncate().as_dvec2();
    let centered = |extent| (offset - extent, offset + extent);
    let (minimum, maximum) = match geometry {
        CanvasGeometry::Rectangle(rectangle) => centered(rectangle.projection_extent(axes, paint)),
        CanvasGeometry::Ellipse(ellipse) => centered(ellipse.projection_extent(axes, paint)),
        CanvasGeometry::Path(path) => {
            let Some(bounds) = path.projected_curve_bounds(axes, offset, paint)? else {
                return Ok(true);
            };
            bounds
        }
    };
    let tolerance = DVec2::splat(f64::from(POSITION_ROUND_TRIP_TOLERANCE));
    Ok(minimum.cmpge(-tolerance).all() && maximum.cmple(tolerance).all())
}

pub(crate) fn relative_transform(
    parent: Affine3A,
    document: Transform,
    geometry: &CanvasGeometry,
    paint: &CanvasPaint,
) -> Result<Transform> {
    let [x, _] = planar_axes(&parent);
    let scale = x.length() as f32;
    let rotation = Quat::from_rotation_z(x.y.atan2(x.x) as f32);
    let point = crate::scale::local_point(parent, document.translation.truncate()).as_vec2();
    let local = Transform {
        translation: point.extend(document.translation.z - parent.translation.z),
        rotation: rotation.inverse() * document.rotation,
        scale: document.scale / Vec3::new(scale, scale, 1.0),
    };
    let expected = document.compute_affine();
    let actual = parent * local.compute_affine();
    if !preserves_projection(expected, actual)
        || !preserves_geometry(geometry, paint, expected, actual)?
    {
        return Err(invalid(CanvasIssue::GroupTransform));
    }
    Ok(local)
}

fn group_children(world: &World, group: Entity) -> &[Entity] {
    world
        .get::<CanvasItemOrder>(group)
        .map_or(&[], |order| order.0.as_slice())
}

enum ProjectionCheck {
    Measure,
    MeasureForExport,
    Validate,
    Preserve(Affine3A),
}

fn subtree_bounds(
    world: &World,
    item: Entity,
    local: Transform,
    transform: Affine3A,
    check: ProjectionCheck,
) -> Result<Option<Rect>> {
    let export = matches!(check, ProjectionCheck::MeasureForExport);
    let measure = export || matches!(check, ProjectionCheck::Measure);
    let previous = match check {
        ProjectionCheck::Preserve(previous) => Some(previous),
        _ => None,
    };
    let mut pending = vec![(item, local, transform, previous)];
    let mut bounds: Option<Rect> = None;
    while let Some((item, local, transform, previous)) = pending.pop() {
        if !transform.is_finite()
            || previous.is_some_and(|previous| !preserves_projection(previous, transform))
        {
            return Err(invalid(CanvasIssue::GroupTransform).at("item", item));
        }
        if world.get::<CanvasGroup>(item).is_some() {
            validate_group_transform(local).map_err(|error| error.at("group", item))?;
            let [x, y] = planar_axes(&transform);
            if x.perp_dot(y) == 0.0 {
                return Err(invalid(CanvasIssue::GroupTransform).at("group", item));
            }
            let children = group_children(world, item);
            for child in valid_items(world, item, children) {
                let local = local_transform(world, child)?;
                let affine = local.compute_affine();
                pending.push((
                    child,
                    local,
                    transform * affine,
                    previous.map(|previous| previous * affine),
                ));
            }
            continue;
        }
        if let Some(text) = world.get::<crate::CanvasText>(item) {
            let rect = text.bounds(&local, &transform)?;
            if let Some(previous) = previous
                && !preserves_geometry(
                    &text.enclosing_geometry(),
                    &CanvasPaint::default(),
                    previous,
                    transform,
                )?
            {
                return Err(invalid(CanvasIssue::GroupTransform).at("text", item));
            }
            if measure {
                bounds = Some(bounds.map_or(rect, |current| current.union(rect)));
            }
            continue;
        }
        if let Some(connector) = world.get::<crate::CanvasConnector>(item) {
            let vector_bounds = connector.bounds(&local, &transform)?;
            if let Some(previous) = previous {
                let style = connector.style();
                let (route, arrows) = connector.route().paths(style);
                let route_preserved = preserves_geometry(
                    &CanvasGeometry::Path(route),
                    &style.paint(),
                    previous,
                    transform,
                )?;
                let arrows_preserved = arrows.commands.is_empty()
                    || preserves_geometry(
                        &CanvasGeometry::Path(arrows),
                        &CanvasPaint::default(),
                        previous,
                        transform,
                    )?;
                let label_preserved = if let Some((label, at)) = connector.label_at() {
                    preserves_projection(
                        crate::CanvasConnector::label_projection(previous, at),
                        crate::CanvasConnector::label_projection(transform, at),
                    ) && preserves_geometry(
                        &label.enclosing_geometry(),
                        &CanvasPaint::default(),
                        crate::CanvasConnector::label_projection(previous, at),
                        crate::CanvasConnector::label_projection(transform, at),
                    )?
                } else {
                    true
                };
                if !(route_preserved && arrows_preserved && label_preserved) {
                    return Err(invalid(CanvasIssue::GroupTransform).at("connector", item));
                }
            }
            if measure && connector.drawn() {
                let (route, paint, arrows) = connector.geometry();
                let painted = crate::mesh_cache::painted_bounds(
                    world,
                    item,
                    &route,
                    paint,
                    &transform,
                    arrows.as_ref(),
                )
                .map_err(|error| error.at("connector", item))?;
                let label = connector.label_bounds(&local, &transform)?;
                for rect in [painted, label].into_iter().flatten() {
                    bounds = Some(bounds.map_or(rect, |current| current.union(rect)));
                }
                if export {
                    bounds =
                        Some(bounds.map_or(vector_bounds, |current| current.union(vector_bounds)));
                }
            }
            continue;
        }
        let geometry = world
            .get::<CanvasGeometry>(item)
            .ok_or_else(|| missing(CanvasPart::Geometry).at("item", item))?;
        let paint = world
            .get::<CanvasPaint>(item)
            .ok_or_else(|| missing(CanvasPart::Paint).at("item", item))?;
        let item_bounds = geometry
            .bounds_in_projection(&local, &transform, paint)
            .map_err(|error| error.at("shape", item))?;
        if let Some(previous) = previous
            && !preserves_geometry(geometry, paint, previous, transform)?
        {
            return Err(invalid(CanvasIssue::GroupTransform).at("shape", item));
        }
        if measure && (paint.fill.is_some() || paint.stroke.is_some()) {
            let item_bounds = if let CanvasGeometry::Path(path) = geometry {
                let mut painted = crate::mesh_cache::painted_bounds(
                    world, item, geometry, *paint, &transform, None,
                )
                .map_err(|error| error.at("shape", item))?;
                if export
                    && let Some((min, max)) = path
                        .projected_curve_bounds(
                            planar_axes(&transform),
                            transform.translation.truncate().as_dvec2(),
                            paint,
                        )
                        .map_err(|error| error.at("shape", item))?
                {
                    let vector_bounds = Rect {
                        min: min.as_vec2(),
                        max: max.as_vec2(),
                    };
                    painted = Some(painted.map_or(vector_bounds, |mesh| mesh.union(vector_bounds)));
                }
                let Some(rect) = painted else {
                    continue;
                };
                rect
            } else {
                item_bounds
            };
            bounds = Some(bounds.map_or(item_bounds, |current| current.union(item_bounds)));
        }
    }
    Ok(bounds)
}

pub(crate) fn local_bounds(world: &World, group: Entity) -> Result<Rect> {
    Ok(subtree_bounds(
        world,
        group,
        Transform::IDENTITY,
        Affine3A::IDENTITY,
        ProjectionCheck::Measure,
    )?
    .unwrap_or_else(|| Rect::from_corners(Vec2::ZERO, Vec2::ZERO)))
}

pub(crate) fn document_bounds(world: &World, item: Entity) -> Result<Option<Rect>> {
    measure_document_bounds(world, item, ProjectionCheck::Measure)
}

fn measure_document_bounds(
    world: &World,
    item: Entity,
    check: ProjectionCheck,
) -> Result<Option<Rect>> {
    subtree_bounds(
        world,
        item,
        local_transform(world, item)?,
        document_transform(world, item)?,
        check,
    )
}

pub(crate) fn validate_item_transform(world: &World, item: Entity, transform: Transform) -> Result {
    if let Some(connector) = world.get::<crate::CanvasConnector>(item) {
        connector.bounds(
            &transform,
            &(parent_transform(world, item)? * transform.compute_affine()),
        )?;
        return Ok(());
    }
    if let Some(text) = world.get::<crate::CanvasText>(item) {
        text.bounds(
            &transform,
            &(parent_transform(world, item)? * transform.compute_affine()),
        )?;
        return Ok(());
    }
    if world.get::<CanvasGroup>(item).is_some() {
        validate_group_transform(transform)?;
        let projected = parent_transform(world, item)? * transform.compute_affine();
        let _ = subtree_bounds(world, item, transform, projected, ProjectionCheck::Validate)?;
        Ok(())
    } else {
        let geometry = world
            .get::<CanvasGeometry>(item)
            .ok_or_else(|| missing(CanvasPart::Geometry).at("item", item))?;
        let paint = world
            .get::<CanvasPaint>(item)
            .ok_or_else(|| missing(CanvasPart::Paint).at("item", item))?;
        validate_shape_projection(world, item, geometry, transform, paint)
    }
}

pub(crate) fn validate_shape_projection(
    world: &World,
    item: Entity,
    geometry: &CanvasGeometry,
    transform: Transform,
    paint: &CanvasPaint,
) -> Result {
    geometry._validate(&transform, paint)?;
    let parent = parent_transform(world, item)?;
    if parent != Affine3A::IDENTITY {
        geometry.projected_bounds(&(parent * transform.compute_affine()), paint)?;
    }
    Ok(())
}

fn group_items(
    world: &mut World,
    document: Entity,
    items: &[Entity],
    name: String,
) -> Result<Entity> {
    let invalid_members = || {
        invalid(CanvasIssue::GroupMembers).input(crate::CanvasInput::Entities {
            field: "items",
            values: items.to_vec(),
        })
    };
    let selected: EntityHashSet = items.iter().copied().collect();
    let Some(&first) = items
        .first()
        .filter(|_| selected.len() == items.len() && items.len() >= 2)
    else {
        return Err(invalid_members());
    };
    let container = item_parent(world, document, first)?;
    let parent = container.parent(document);
    let siblings = container_items(world, document, container)?.to_vec();
    let members: Vec<_> = valid_items(world, parent, &siblings)
        .skip_while(|item| !selected.contains(item))
        .take_while(|item| selected.contains(item))
        .collect();
    if members.len() != selected.len() {
        return Err(invalid_members());
    }
    let index = siblings
        .iter()
        .position(|item| *item == members[0])
        .ok_or_else(invalid_members)?;
    let mut bounds: Option<Rect> = None;
    for &item in &members {
        let local = local_transform(world, item)?;
        if let Some(item_bounds) = subtree_bounds(
            world,
            item,
            local,
            local.compute_affine(),
            ProjectionCheck::Measure,
        )? {
            bounds = Some(bounds.map_or(item_bounds, |current| current.union(item_bounds)));
        }
    }
    let center = bounds.map_or(Vec2::ZERO, |bounds| {
        ((bounds.min.as_dvec2() + bounds.max.as_dvec2()) * 0.5).as_vec2()
    });
    let group_transform = Transform::from_translation(center.extend(0.0));
    let old_parent = parent_transform(world, first)?;
    let new_parent = old_parent * group_transform.compute_affine();
    let updates = members
        .iter()
        .map(|&item| {
            let original = local_transform(world, item)?;
            let mut local = original;
            local.translation -= group_transform.translation;
            let local_affine = local.compute_affine();
            let projection = new_parent * local_affine;
            let _ = subtree_bounds(world, item, local, local_affine, ProjectionCheck::Validate)?;
            let _ = subtree_bounds(
                world,
                item,
                local,
                projection,
                ProjectionCheck::Preserve(old_parent * original.compute_affine()),
            )?;
            Ok((item, local))
        })
        .collect::<Result<Vec<_>>>()?;

    let group = world.spawn_empty().id();
    crate::history::edit_order(world, document, container, index, |siblings| {
        siblings.retain(|item| !selected.contains(item));
        siblings.insert(index, group);
    })?;
    crate::history::record_created(world, document, group)?;
    world.insert_batch([(
        group,
        (
            CanvasGroup::new(name),
            CanvasItemOrder(members),
            group_transform,
            ChildOf(parent),
        ),
    )]);
    crate::history::reparent_items(world, document, group, updates)?;
    Ok(group)
}

fn ungroup(world: &mut World, document: Entity, group: Entity) -> Result<Vec<Entity>> {
    let container = group_container(world, document, group)?;
    let transform = local_transform(world, group)?;
    validate_group_transform(transform)?;
    let parent_projection = parent_transform(world, group)?;
    let group_projection = document_transform(world, group)?;
    let members: Vec<_> = valid_items(world, group, group_children(world, group)).collect();
    let updates = members
        .iter()
        .map(|&item| {
            let local = local_transform(world, item)?;
            let mut expanded = transform.mul_transform(local);
            expanded.translation = transform
                .compute_affine()
                .transform_point3(local.translation);
            let expanded_affine = expanded.compute_affine();
            let projection = parent_projection * expanded_affine;
            let _ = subtree_bounds(
                world,
                item,
                expanded,
                expanded_affine,
                ProjectionCheck::Validate,
            )?;
            let _ = subtree_bounds(
                world,
                item,
                expanded,
                projection,
                ProjectionCheck::Preserve(group_projection * local.compute_affine()),
            )?;
            Ok((item, expanded))
        })
        .collect::<Result<Vec<_>>>()?;
    let index = container_items(world, document, container)?
        .iter()
        .position(|&item| item == group)
        .ok_or_else(|| invalid(CanvasIssue::GroupTarget))?;
    crate::history::edit_order(world, document, container, index, |siblings| {
        drop(siblings.splice(index..index + 1, members.iter().copied()));
    })?;
    let parent = container.parent(document);
    crate::history::reparent_items(world, document, parent, updates)?;
    crate::history::detach_item(world, document, group)?;
    Ok(members)
}

pub(crate) fn prune_empty_groups(world: &mut World, document: Entity, mut group: Entity) -> Result {
    loop {
        if world.get::<CanvasGroup>(group).is_none()
            || valid_items(world, group, group_children(world, group))
                .next()
                .is_some()
        {
            return Ok(());
        }
        let Some(parent) = item_container(world, document, group) else {
            return Ok(());
        };
        crate::history::edit_order(world, document, parent, 0, |siblings| {
            siblings.retain(|&item| item != group);
        })?;
        crate::history::detach_item(world, document, group)?;
        let CanvasContainer::Group(ancestor) = parent else {
            return Ok(());
        };
        group = ancestor;
    }
}
