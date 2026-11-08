const FIRST_OFFSCREEN_LAYER: usize = 1024;

use crate::diagnostics::FailureState;
use bevy::asset::RenderAssetUsages;
use bevy::camera::{RenderTarget, visibility::VisibilitySystems};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::ui::UiSystems;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_internal_log::widgetry_debug;
mod error;
pub use error::{
    OffscreenBinding, OffscreenBindingRejection, OffscreenError, OffscreenFailure,
    OffscreenFailures, OffscreenResource, OffscreenTargetIdentity,
};
use error::{binding_frame, failure, push, single};

#[derive(Resource)]
struct Layers {
    next: usize,
    free: Vec<usize>,
}

#[must_use]
pub struct OffscreenTarget {
    owner: Entity,
    image: Handle<Image>,
    layer: usize,
}

#[derive(Component)]
pub struct OffscreenViewport {
    target: OffscreenTarget,
    viewport: Entity,
    camera: Entity,
    scene: Entity,
    failure: FailureState<UpdateFailure>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AllocationFailure {
    Resource(OffscreenResource),
    Exhausted(usize),
}
#[derive(Component, Default)]
struct OwnerFailures {
    allocation: FailureState<AllocationFailure>,
    missing_viewport: FailureState<()>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum UpdateFailure {
    Owner(Entity),
    Association(Option<Entity>),
    ComputedNode,
    Camera,
    Scene,
    Images,
    Image,
}

#[derive(Component, Clone, Copy)]
struct SceneClaim(OffscreenBinding);
#[derive(Component)]
struct CameraClaim(OffscreenBinding);
#[derive(Component)]
struct ViewportClaim(OffscreenBinding);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OffscreenUpdate;
pub struct WidgetryOffscreenPlugin;

impl Default for Layers {
    fn default() -> Self {
        Self {
            next: FIRST_OFFSCREEN_LAYER,
            free: Vec::new(),
        }
    }
}

impl OffscreenTarget {
    pub fn allocate(world: &mut World, owner: Entity) -> Result<Self, OffscreenFailures> {
        if let Err(source) = world.get_entity_mut(owner) {
            return Err(single(
                OffscreenError::EntityAccess {
                    owner,
                    entity: owner,
                    source,
                },
                "offscreen.allocate",
            ));
        }
        let invalid = if !world.contains_resource::<Layers>() {
            Some((
                AllocationFailure::Resource(OffscreenResource::Layers),
                OffscreenError::MissingResource {
                    owner,
                    target: None,
                    resource: OffscreenResource::Layers,
                },
            ))
        } else if !world.contains_resource::<Assets<Image>>() {
            Some((
                AllocationFailure::Resource(OffscreenResource::Images),
                OffscreenError::MissingResource {
                    owner,
                    target: None,
                    resource: OffscreenResource::Images,
                },
            ))
        } else {
            let layers = world.resource::<Layers>();
            (layers.free.is_empty() && layers.next == usize::MAX).then_some((
                AllocationFailure::Exhausted(layers.next),
                OffscreenError::LayerExhausted {
                    owner,
                    next: layers.next,
                },
            ))
        };
        if let Some((key, kind)) = invalid {
            let mut entity = world.entity_mut(owner);
            let mut state = entity.entry::<OwnerFailures>().or_default().into_mut();
            return Err(OffscreenFailures(
                state
                    .allocation
                    .failure(key, || failure(kind, "offscreen.allocate")),
            ));
        }
        let mut layers = world.resource_mut::<Layers>();
        let layer = if let Some(layer) = layers.free.pop() {
            layer
        } else {
            let layer = layers.next;
            layers.next += 1;
            layer
        };
        let mut image = Image::new_uninit(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;
        let image = world.resource_mut::<Assets<Image>>().add(image);
        if let Some(mut state) = world.get_mut::<OwnerFailures>(owner) {
            state
                .allocation
                .recover(|| ErrorFrame::new("offscreen.allocate").field("owner", owner));
        }
        Ok(Self {
            owner,
            image,
            layer,
        })
    }

    pub fn layer(&self) -> usize {
        self.layer
    }
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }
    fn identity(&self) -> OffscreenTargetIdentity {
        OffscreenTargetIdentity {
            image: self.image.id(),
            layer: self.layer,
        }
    }

    pub fn release(self, world: &mut World) -> Result<(), OffscreenFailures> {
        let identity = self.identity();
        let mut failures = None;
        if let Some(mut images) = world.get_resource_mut::<Assets<Image>>() {
            images.remove(self.image.id());
        } else {
            push(
                &mut failures,
                single(
                    OffscreenError::MissingResource {
                        owner: self.owner,
                        target: Some(identity),
                        resource: OffscreenResource::Images,
                    },
                    "offscreen.release",
                ),
            );
        }
        if let Some(mut layers) = world.get_resource_mut::<Layers>() {
            layers.free.push(self.layer);
        } else {
            push(
                &mut failures,
                single(
                    OffscreenError::MissingResource {
                        owner: self.owner,
                        target: Some(identity),
                        resource: OffscreenResource::Layers,
                    },
                    "offscreen.release",
                ),
            );
        }
        failures.map_or(Ok(()), Err)
    }

    pub fn bind(
        self,
        world: &mut World,
        viewport: Entity,
        camera: Entity,
        scene: Entity,
    ) -> Result<OffscreenViewport, OffscreenFailures> {
        let binding = OffscreenBinding {
            owner: self.owner,
            viewport,
            camera,
            scene,
            target: self.identity(),
        };
        // A scene is transferred only when its subtree contains no host or existing lease.
        let scene_owned = scene_is_transferable(world, scene, self.owner, viewport);
        let reason = if !world.entities().contains(self.owner) {
            Some(OffscreenBindingRejection::MissingOwner)
        } else if scene == self.owner || scene == viewport {
            Some(OffscreenBindingRejection::SceneIsOwner)
        } else if !world.entities().contains(scene) {
            Some(OffscreenBindingRejection::MissingScene)
        } else if !scene_owned {
            Some(OffscreenBindingRejection::SceneInUse)
        } else if world.get::<Node>(viewport).is_none() {
            Some(OffscreenBindingRejection::MissingViewportNode)
        } else if world.get::<Camera>(camera).is_none() {
            Some(OffscreenBindingRejection::MissingCamera)
        } else if !within_scene(world, camera, scene) {
            Some(OffscreenBindingRejection::CameraOutsideScene)
        } else if world.get::<CameraClaim>(camera).is_some() {
            Some(OffscreenBindingRejection::CameraInUse)
        } else if world
            .get::<ViewportClaim>(viewport)
            .is_some_and(|claim| claim.0.owner != self.owner)
            || (world
                .get::<ViewportNode>(viewport)
                .is_some_and(|node| node.camera.is_some())
                && world.get::<ViewportClaim>(viewport).is_none())
        {
            Some(OffscreenBindingRejection::ViewportInUse)
        } else {
            None
        };
        let invalid = reason
            .map(|reason| OffscreenError::InvalidBinding { binding, reason })
            .or_else(|| {
                let resource = if !world.contains_resource::<Assets<Image>>() {
                    Some(OffscreenResource::Images)
                } else if !world.contains_resource::<Layers>() {
                    Some(OffscreenResource::Layers)
                } else {
                    None
                };
                resource.map(|resource| OffscreenError::MissingResource {
                    owner: self.owner,
                    target: Some(binding.target),
                    resource,
                })
            });
        if let Some(kind) = invalid {
            let mut error = single(kind, "offscreen.bind")
                .with_context(binding_frame(&binding, "offscreen.bind"));
            if let Err(cleanup) = self.release(world) {
                error = error.append(
                    cleanup.with_context(binding_frame(&binding, "offscreen.bind.cleanup")),
                );
            }
            if scene_owned {
                world.despawn(scene);
            }
            return Err(error);
        }
        world.entity_mut(scene).insert(SceneClaim(binding));
        world.entity_mut(camera).insert((
            CameraClaim(binding),
            RenderTarget::Image(self.image.clone().into()),
        ));
        world
            .entity_mut(viewport)
            .insert((ViewportClaim(binding), ViewportNode::new(camera)));
        Ok(OffscreenViewport {
            target: self,
            viewport,
            camera,
            scene,
            failure: FailureState::default(),
        })
    }
}

fn within_scene(world: &World, mut entity: Entity, scene: Entity) -> bool {
    loop {
        if entity == scene {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return false;
        };
        entity = parent.parent();
    }
}

fn scene_is_transferable(world: &World, scene: Entity, owner: Entity, viewport: Entity) -> bool {
    if !world.entities().contains(scene) {
        return false;
    }
    let mut ancestor = world.get::<ChildOf>(scene).map(ChildOf::parent);
    while let Some(entity) = ancestor {
        if world.get::<SceneClaim>(entity).is_some() {
            return false;
        }
        ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    let mut pending = vec![scene];
    while let Some(entity) = pending.pop() {
        if entity == owner
            || entity == viewport
            || world.get::<OffscreenViewport>(entity).is_some()
            || world.get::<SceneClaim>(entity).is_some()
            || world.get::<CameraClaim>(entity).is_some()
            || world.get::<ViewportClaim>(entity).is_some()
        {
            return false;
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    true
}

impl OffscreenViewport {
    fn binding(&self) -> OffscreenBinding {
        OffscreenBinding {
            owner: self.target.owner,
            viewport: self.viewport,
            camera: self.camera,
            scene: self.scene,
            target: self.target.identity(),
        }
    }

    pub fn update(world: &mut World, owner: Entity) -> Result<Option<UVec2>, OffscreenFailures> {
        if let Err(source) = world.get_entity_mut(owner) {
            return Err(single(
                OffscreenError::EntityAccess {
                    owner,
                    entity: owner,
                    source,
                },
                "offscreen.update",
            ));
        }
        let Some(target) = world.get::<Self>(owner) else {
            let mut entity = world.entity_mut(owner);
            let mut state = entity.entry::<OwnerFailures>().or_default().into_mut();
            return Err(OffscreenFailures(state.missing_viewport.failure(
                (),
                || {
                    failure(
                        OffscreenError::MissingViewport { owner },
                        "offscreen.update",
                    )
                },
            )));
        };
        let binding = target.binding();
        let actual_camera = world
            .get::<ViewportNode>(binding.viewport)
            .and_then(|node| node.camera);
        let invalid = if binding.owner != owner {
            Some((
                UpdateFailure::Owner(owner),
                OffscreenError::OwnerMismatch {
                    binding,
                    actual_owner: owner,
                },
            ))
        } else if actual_camera != Some(binding.camera) {
            Some((
                UpdateFailure::Association(actual_camera),
                OffscreenError::AssociationBroken {
                    binding,
                    actual_camera,
                },
            ))
        } else if world.get::<ComputedNode>(binding.viewport).is_none() {
            Some((
                UpdateFailure::ComputedNode,
                OffscreenError::MissingComputedNode { binding },
            ))
        } else if world.get::<Camera>(binding.camera).is_none() {
            Some((
                UpdateFailure::Camera,
                OffscreenError::MissingCamera { binding },
            ))
        } else if !world.entities().contains(binding.scene) {
            Some((
                UpdateFailure::Scene,
                OffscreenError::MissingScene { binding },
            ))
        } else if let Some(images) = world.get_resource::<Assets<Image>>() {
            images.get(binding.target.image).is_none().then_some((
                UpdateFailure::Image,
                OffscreenError::MissingImage { binding },
            ))
        } else {
            Some((
                UpdateFailure::Images,
                OffscreenError::MissingResource {
                    owner,
                    target: Some(binding.target),
                    resource: OffscreenResource::Images,
                },
            ))
        };
        if let Some((key, kind)) = invalid {
            let mut target = world.get_mut::<Self>(owner).ok_or_else(|| {
                single(
                    OffscreenError::MissingViewport { owner },
                    "offscreen.update",
                )
            })?;
            return Err(OffscreenFailures(
                target
                    .failure
                    .failure(key, || failure(kind, "offscreen.update")),
            ));
        }
        if world
            .get::<Self>(owner)
            .is_some_and(|target| target.failure.active().is_some())
            && let Some(mut target) = world.get_mut::<Self>(owner)
        {
            target
                .failure
                .recover(|| binding_frame(&binding, "offscreen.update"));
        }
        if world
            .get::<OwnerFailures>(owner)
            .is_some_and(|state| state.missing_viewport.active().is_some())
            && let Some(mut state) = world.get_mut::<OwnerFailures>(owner)
        {
            state
                .missing_viewport
                .recover(|| binding_frame(&binding, "offscreen.update"));
        }
        let size = world
            .get::<ComputedNode>(binding.viewport)
            .map(|node| node.size().as_uvec2())
            .ok_or_else(|| {
                single(
                    OffscreenError::MissingComputedNode { binding },
                    "offscreen.update",
                )
            })?;
        let active = size.x > 0
            && size.y > 0
            && world
                .get::<InheritedVisibility>(owner)
                .is_some_and(|visibility| visibility.get());
        {
            let mut state = world.get_mut::<Camera>(binding.camera).ok_or_else(|| {
                single(
                    OffscreenError::MissingCamera { binding },
                    "offscreen.update",
                )
            })?;
            if state.is_active != active {
                state.is_active = active;
            }
            // Camera target metadata precedes UI layout, while ViewportNode resizes the image after this set.
            // Keep projection and picking on the current physical size during a resize.
            if active
                && let Some(info) = &mut state.computed.target_info
                && info.physical_size != size
            {
                info.physical_size = size;
            }
        }
        Ok(active.then_some(size))
    }
}

fn release(
    event: On<Discard<OffscreenViewport>>,
    targets: Query<&OffscreenViewport>,
    mut commands: Commands,
) {
    if let Ok(target) = targets.get(event.entity) {
        let binding = target.binding();
        let target = OffscreenTarget {
            owner: target.target.owner,
            image: target.target.image.clone(),
            layer: target.target.layer,
        };
        commands.queue(move |world: &mut World| -> Result {
            let result = target.release(world);
            if world
                .get::<ViewportClaim>(binding.viewport)
                .is_some_and(|claim| claim.0.target == binding.target)
            {
                if world
                    .get::<ViewportNode>(binding.viewport)
                    .is_some_and(|node| node.camera == Some(binding.camera))
                {
                    world.entity_mut(binding.viewport).remove::<ViewportNode>();
                }
                world.entity_mut(binding.viewport).remove::<ViewportClaim>();
            }
            if world
                .get::<SceneClaim>(binding.scene)
                .is_some_and(|claim| claim.0.target == binding.target)
            {
                world.despawn(binding.scene);
            }
            if world
                .get::<CameraClaim>(binding.camera)
                .is_some_and(|claim| claim.0.target == binding.target)
            {
                if world
                    .get::<RenderTarget>(binding.camera)
                    .and_then(RenderTarget::as_image)
                    .is_some_and(|image| image.id() == binding.target.image)
                {
                    world.entity_mut(binding.camera).remove::<RenderTarget>();
                }
                world.entity_mut(binding.camera).remove::<CameraClaim>();
            }
            result.map_err(|error| {
                error
                    .with_context(binding_frame(&binding, "offscreen.discard"))
                    .into_bevy_error()
            })
        });
    }
}

impl Plugin for WidgetryOffscreenPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Assets<Image>>() {
            app.init_asset::<Image>();
        }
        app.init_resource::<Layers>()
            .add_observer(release)
            .configure_sets(
                PostUpdate,
                OffscreenUpdate
                    .after(UiSystems::Layout)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .before(VisibilitySystems::CalculateBounds)
                    .before(VisibilitySystems::CheckVisibility)
                    .before(bevy::ui::widget::update_viewport_render_target_size),
            );
        widgetry_debug!("WidgetryOffscreenPlugin 注册完成");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::scene_app;

    #[test]
    fn exhaustion_is_lazy_and_recycled_layers_still_allocate() {
        let mut app = scene_app();
        app.add_plugins(WidgetryOffscreenPlugin);
        let world = app.world_mut();
        let owner = world.spawn_empty().id();
        world.resource_mut::<Layers>().next = usize::MAX;
        let first = OffscreenTarget::allocate(world, owner).err().unwrap();
        let second = OffscreenTarget::allocate(world, owner).err().unwrap();
        let id = first.diagnostics().next().unwrap().origin().id();
        assert_eq!(id, second.diagnostics().next().unwrap().origin().id());
        world.resource_mut::<Layers>().free.push(1024);
        let recycled = OffscreenTarget::allocate(world, owner).unwrap();
        assert_eq!(recycled.layer(), 1024);
        assert_eq!(world.resource::<Layers>().next, usize::MAX);
        recycled.release(world).unwrap();
    }

    #[test]
    fn binding_and_both_cleanup_failures_keep_independent_diagnostics() {
        let mut app = scene_app();
        app.add_plugins(WidgetryOffscreenPlugin);
        let world = app.world_mut();
        let owner = world.spawn_empty().id();
        let scene = world.spawn_empty().id();
        let target = OffscreenTarget::allocate(world, owner).unwrap();
        let identity = target.identity();
        world.remove_resource::<Layers>();
        world.remove_resource::<Assets<Image>>();
        let error = target.bind(world, owner, scene, scene).err().unwrap();
        let diagnostics = error.diagnostics().collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 3);
        let ids = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.origin().id())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), 3);
        for diagnostic in diagnostics {
            assert_eq!(
                diagnostic.origin().backtrace().status(),
                std::backtrace::BacktraceStatus::Captured
            );
            assert!(
                diagnostic
                    .contexts()
                    .iter()
                    .flat_map(ErrorFrame::fields)
                    .any(|(name, value)| name == "image"
                        && value == &format!("{:?}", identity.image))
            );
        }
        assert!(world.get_entity(scene).is_err());
    }

    #[derive(Component, FromTemplate)]
    struct SceneCamera {
        camera: Entity,
    }

    fn scene(world: &mut World, layer: usize) -> (Entity, Entity) {
        let scene = world.spawn_scene(bsn! {
            SceneCamera { camera: #Camera }
            Transform::default()
            Visibility::default()
            Children [
                #Camera Camera2d Camera { is_active: false }
                template(move |_| Ok(bevy::camera::visibility::RenderLayers::none().with(layer)))
            ]
        }).unwrap().id();
        (scene, world.get::<SceneCamera>(scene).unwrap().camera)
    }

    #[test]
    fn replacing_and_removing_target_preserves_other_owners() {
        let mut app = scene_app();
        app.add_plugins(WidgetryOffscreenPlugin);
        let world = app.world_mut();
        let host_image = world.resource_mut::<Assets<Image>>().add(Image::default());
        let viewport = world.spawn_scene(bsn! { Node::default() }).unwrap().id();
        let owner = world.spawn_empty().id();
        let other_owner = world.spawn_empty().id();
        let other = OffscreenTarget::allocate(world, other_owner).unwrap();
        let target = OffscreenTarget::allocate(world, owner).unwrap();
        let old_image = target.image().clone();
        let old_layer = target.layer();
        assert_ne!(old_layer, other.layer());
        let (old_scene, camera) = scene(world, target.layer());
        let bound = target.bind(world, viewport, camera, old_scene).unwrap();
        world.entity_mut(owner).insert(bound);
        let target = OffscreenTarget::allocate(world, owner).unwrap();
        let new_image = target.image().clone();
        let (new_scene, new_camera) = scene(world, target.layer());
        let bound = target.bind(world, viewport, new_camera, new_scene).unwrap();
        world.entity_mut(owner).insert(bound);
        world.flush();
        assert!(!world.entities().contains(old_scene));
        assert!(world.resource::<Assets<Image>>().get(&old_image).is_none());
        assert_eq!(
            world.get::<ViewportNode>(viewport).unwrap().camera,
            Some(new_camera)
        );
        assert_eq!(
            world
                .get::<RenderTarget>(new_camera)
                .unwrap()
                .as_image()
                .unwrap()
                .id(),
            new_image.id()
        );
        let recycled = OffscreenTarget::allocate(world, owner).unwrap();
        assert_eq!(recycled.layer(), old_layer);
        recycled.release(world).unwrap();
        world.despawn(owner);
        world.flush();
        assert!(!world.entities().contains(new_scene));
        assert!(world.get::<ViewportNode>(viewport).is_none());
        assert!(world.resource::<Assets<Image>>().get(&new_image).is_none());
        assert!(
            world
                .resource::<Assets<Image>>()
                .get(other.image())
                .is_some()
        );
        assert!(world.resource::<Assets<Image>>().get(&host_image).is_some());
        other.release(world).unwrap();
        assert_eq!(world.resource::<Assets<Image>>().len(), 1);
    }

    #[test]
    fn invalid_binding_reclaims_target_and_owned_scene() {
        let mut app = scene_app();
        app.add_plugins(WidgetryOffscreenPlugin);
        let world = app.world_mut();
        let viewport = world.spawn_scene(bsn! { Node::default() }).unwrap().id();
        let owner = viewport;
        let scene = world.spawn(Name::new("InvalidOffscreenScene")).id();
        let target = OffscreenTarget::allocate(world, owner).unwrap();
        let image = target.image().clone();
        let layer = target.layer();
        let result = target.bind(world, viewport, scene, scene);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().into_bevy_error().severity(),
            bevy::ecs::error::Severity::Error
        );
        assert!(!world.entities().contains(scene));
        assert!(world.resource::<Assets<Image>>().get(&image).is_none());
        assert!(world.get::<ViewportNode>(viewport).is_none());
        let replacement = OffscreenTarget::allocate(world, owner).unwrap();
        assert_eq!(replacement.layer(), layer);
        replacement.release(world).unwrap();
    }
}
