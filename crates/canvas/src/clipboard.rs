use crate::error::{CanvasResult, PublicResult, invalid, missing};
use crate::{
    CanvasConnector, CanvasContainer, CanvasDocument, CanvasGeometry, CanvasGroup, CanvasIssue,
    CanvasLayerId, CanvasPaint, CanvasPart, CanvasText,
};
use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    math::Affine3A,
    prelude::*,
};
use bevy_widgetry_diagnostics::ErrorFrame;

const PASTE_OFFSET: Vec2 = Vec2::splat(16.0);

/// Application-local snapshots of editable Canvas items.
#[derive(Clone, Default, Resource)]
pub struct CanvasClipboard {
    items: Vec<Item>,
}

#[derive(Clone)]
struct Item {
    source: Entity,
    parent: Option<usize>,
    transform: Transform,
    artwork: Artwork,
}

#[derive(Clone)]
enum Artwork {
    Shape(CanvasGeometry, CanvasPaint),
    Group(String),
    Text(CanvasText),
    Connector(CanvasConnector),
}

impl CanvasClipboard {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Captures the selected roots and their descendants, returning the root count.
    pub fn copy(world: &mut World, document: Entity, items: &[Entity]) -> PublicResult<usize> {
        let clipboard = Self::capture(world, document, items).map_err(|error| {
            error
                .at("document", document)
                .into_failure(ErrorFrame::new("canvas.CanvasClipboard.copy"))
        })?;
        Ok(clipboard.save(world))
    }

    /// Removes the copied roots in one history entry and returns their count.
    pub fn cut(world: &mut World, document: Entity, items: &[Entity]) -> PublicResult<usize> {
        (|| -> CanvasResult<usize> {
            let clipboard = Self::capture(world, document, items)?;
            let roots = clipboard
                .items
                .iter()
                .filter(|item| item.parent.is_none())
                .map(|item| item.source)
                .collect::<Vec<_>>();
            if !roots.is_empty() {
                crate::batch::remove_selected_items(world, document, &roots)?;
            }
            Ok(clipboard.save(world))
        })()
        .map_err(|error| {
            error
                .at("document", document)
                .into_failure(ErrorFrame::new("canvas.CanvasClipboard.cut"))
        })
    }

    /// Pastes into an editable layer and returns the newly created roots in draw order.
    pub fn paste(
        world: &mut World,
        document: Entity,
        layer: Option<CanvasLayerId>,
    ) -> PublicResult<Vec<Entity>> {
        let clipboard = world.get_resource::<Self>().cloned().unwrap_or_default();
        clipboard.insert(world, document, layer).map_err(|error| {
            error
                .at("document", document)
                .into_failure(ErrorFrame::new("canvas.CanvasClipboard.paste"))
        })
    }

    /// Inserts a copy with the same offset as paste, preserving the clipboard contents.
    pub fn duplicate(
        world: &mut World,
        document: Entity,
        layer: Option<CanvasLayerId>,
        items: &[Entity],
    ) -> PublicResult<Vec<Entity>> {
        Self::capture(world, document, items)
            .and_then(|clipboard| clipboard.insert(world, document, layer))
            .map_err(|error| {
                error
                    .at("document", document)
                    .into_failure(ErrorFrame::new("canvas.CanvasClipboard.duplicate"))
            })
    }

    fn capture(world: &World, document: Entity, items: &[Entity]) -> CanvasResult<Self> {
        for &item in items {
            crate::groups::item_parent(world, document, item)?;
        }
        let selected: EntityHashSet = items.iter().copied().collect();
        let mut copied = EntityHashMap::default();
        let mut clipboard = Self::default();
        for source in CanvasDocument::_draw_order(world, document)? {
            let parent = world
                .get::<ChildOf>(source)
                .and_then(|child| copied.get(&child.parent()).copied());
            if parent.is_none() && !selected.contains(&source) {
                continue;
            }
            let local = crate::groups::local_transform(world, source)?;
            let transform = if parent.is_some() {
                local
            } else {
                document_pose(world, source, local)?
            };
            let artwork = if let Some(group) = world.get::<CanvasGroup>(source) {
                Artwork::Group(group.name().to_owned())
            } else if let Some(text) = world.get::<CanvasText>(source) {
                Artwork::Text(text.clone())
            } else if let Some(connector) = world.get::<CanvasConnector>(source) {
                Artwork::Connector(connector.clone())
            } else {
                let geometry = world
                    .get::<CanvasGeometry>(source)
                    .ok_or_else(|| missing(CanvasPart::Geometry).at("item", source))?;
                let paint = world
                    .get::<CanvasPaint>(source)
                    .ok_or_else(|| missing(CanvasPart::Paint).at("item", source))?;
                Artwork::Shape(geometry.clone(), *paint)
            };
            copied.insert(source, clipboard.items.len());
            clipboard.items.push(Item {
                source,
                parent,
                transform,
                artwork,
            });
        }
        clipboard.prepare(&copied)
    }

    fn save(self, world: &mut World) -> usize {
        let count = self
            .items
            .iter()
            .filter(|item| item.parent.is_none())
            .count();
        if count > 0 {
            world.insert_resource(self);
        }
        count
    }

    fn prepare(mut self, sources: &EntityHashMap<usize>) -> CanvasResult<Self> {
        let mut projections = Vec::with_capacity(self.items.len());
        for item in &mut self.items {
            if item.parent.is_none() {
                item.transform.translation += PASTE_OFFSET.extend(0.0);
            }
            let parent = item
                .parent
                .map_or(Affine3A::IDENTITY, |index| projections[index]);
            let projection = parent * item.transform.compute_affine();
            if !matches!(&item.artwork, Artwork::Connector(_)) {
                item.artwork.validate(&item.transform, &projection)?;
            }
            projections.push(projection);
        }
        for (index, item) in self.items.iter().enumerate() {
            if let Artwork::Connector(connector) = &item.artwork {
                let mut resolved = connector.clone();
                resolved.refresh_endpoints(projections[index], |attachment| {
                    let Some(&target) = sources.get(&attachment.target) else {
                        return Ok(None);
                    };
                    let Artwork::Shape(geometry, _) = &self.items[target].artwork else {
                        return Ok(None);
                    };
                    Ok(crate::anchors::projected_point(
                        geometry,
                        projections[target],
                        attachment.anchor,
                    )?
                    .map(|point| (true, point)))
                })?;
                resolved.bounds(&item.transform, &projections[index])?;
            }
        }
        Ok(self)
    }

    fn insert(
        self,
        world: &mut World,
        document: Entity,
        layer: Option<CanvasLayerId>,
    ) -> CanvasResult<Vec<Entity>> {
        let stack = crate::order::stack(world, document)?;
        if self.is_empty() {
            return Ok(Vec::new());
        }
        let layer = stack
            .editable_layer(
                layer
                    .filter(|&id| stack.layer(id).is_some_and(crate::CanvasLayer::editable))
                    .or(Some(stack.default_layer)),
            )
            .ok_or_else(|| invalid(CanvasIssue::LayerUnavailable))?;
        crate::history::apply(world, document, |world, document| {
            let mut roots = Vec::new();
            let mut inserted = Vec::with_capacity(self.items.len());
            let mut entities = EntityHashMap::default();
            let mut connectors = Vec::new();
            for item in self.items {
                let container = item.parent.map_or(CanvasContainer::Layer(layer), |index| {
                    CanvasContainer::Group(inserted[index])
                });
                let transform = item.transform;
                let entity = match item.artwork {
                    Artwork::Shape(geometry, paint) => CanvasDocument::insert_item(
                        world,
                        document,
                        container,
                        (geometry, paint, transform),
                    )?,
                    Artwork::Group(name) => CanvasDocument::insert_item(
                        world,
                        document,
                        container,
                        (CanvasGroup::new(name), transform),
                    )?,
                    Artwork::Text(text) => {
                        CanvasDocument::insert_item(world, document, container, (text, transform))?
                    }
                    Artwork::Connector(connector) => {
                        let mut free = connector.clone();
                        free.remap_attachments(|_| None);
                        let copy = CanvasDocument::insert_item(
                            world,
                            document,
                            container,
                            (free, transform),
                        )?;
                        connectors.push((copy, connector));
                        copy
                    }
                };
                inserted.push(entity);
                entities.insert(item.source, entity);
                if item.parent.is_none() {
                    roots.push(entity);
                }
            }
            for (entity, mut connector) in connectors {
                connector.remap_attachments(|target| entities.get(&target).copied());
                crate::history::set_connector(world, document, entity, connector)?;
            }
            Ok(roots)
        })
    }
}

impl Artwork {
    fn validate(&self, local: &Transform, projection: &Affine3A) -> CanvasResult {
        match self {
            Self::Shape(geometry, paint) => geometry
                .bounds_in_projection(local, projection, paint)
                .map(|_| ()),
            Self::Text(text) => text.bounds(local, projection).map(|_| ()),
            Self::Connector(connector) => connector.bounds(local, projection).map(|_| ()),
            Self::Group(_) => {
                crate::groups::validate_group_transform(*local)?;
                let [x, y] = crate::groups::planar_axes(projection);
                if !projection.is_finite() || x.perp_dot(y) == 0.0 {
                    return Err(invalid(CanvasIssue::GroupTransform));
                }
                Ok(())
            }
        }
    }
}

fn document_pose(
    world: &World,
    source: Entity,
    mut transform: Transform,
) -> CanvasResult<Transform> {
    let parent = crate::groups::parent_transform(world, source)?;
    if parent != Affine3A::IDENTITY {
        let [x, _] = crate::groups::planar_axes(&parent);
        let scale = x.length();
        let rotation = Quat::from_rotation_z(x.y.atan2(x.x) as f32);
        transform.translation = parent.transform_point3(transform.translation);
        transform.rotation = (rotation * transform.rotation).normalize();
        transform.scale.x = (f64::from(transform.scale.x) * scale) as f32;
        transform.scale.y = (f64::from(transform.scale.y) * scale) as f32;
    }
    Ok(transform)
}
