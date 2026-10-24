use bevy::asset::RenderAssetUsages;
use bevy::camera::{RenderTarget, visibility::VisibilitySystems};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::ui::UiSystems;
use bevy_widgetry_internal_log::widgetry_info;

#[derive(Resource)]
struct Layers {
    next: usize,
    free: Vec<usize>,
}

#[must_use]
pub struct OffscreenTarget {
    image: Handle<Image>,
    layer: usize,
}

#[derive(Component)]
pub struct OffscreenViewport {
    target: OffscreenTarget,
    viewport: Entity,
    camera: Entity,
    scene: Entity,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OffscreenUpdate;

pub struct WidgetryOffscreenPlugin;

impl Default for Layers {
    fn default() -> Self {
        Self {
            next: 1024,
            free: Vec::new(),
        }
    }
}

impl OffscreenTarget {
    pub fn allocate(world: &mut World) -> Result<Self> {
        let mut layers = world.resource_mut::<Layers>();
        let layer = if let Some(layer) = layers.free.pop() {
            layer
        } else {
            let layer = layers.next;
            layers.next = layer
                .checked_add(1)
                .ok_or_else(|| BevyError::error("Offscreen layer overflow"))?;
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
        Ok(Self { image, layer })
    }

    pub fn layer(&self) -> usize {
        self.layer
    }

    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub fn release(self, world: &mut World) {
        world
            .resource_mut::<Assets<Image>>()
            .remove(self.image.id());
        world.resource_mut::<Layers>().free.push(self.layer);
    }

    pub fn bind(
        self,
        world: &mut World,
        viewport: Entity,
        camera: Entity,
        scene: Entity,
    ) -> Result<OffscreenViewport> {
        if world.get::<Node>(viewport).is_none()
            || world.get::<Camera>(camera).is_none()
            || !world.entities().contains(scene)
        {
            self.release(world);
            world.despawn(scene);
            return Err(BevyError::error(
                "Offscreen binding requires a UI viewport, camera and owned scene",
            ));
        }
        world
            .entity_mut(camera)
            .insert(RenderTarget::Image(self.image.clone().into()));
        world.entity_mut(viewport).insert(ViewportNode::new(camera));
        Ok(OffscreenViewport {
            target: self,
            viewport,
            camera,
            scene,
        })
    }
}

impl OffscreenViewport {
    pub fn update(world: &mut World, owner: Entity) -> Result<Option<UVec2>> {
        let target = world
            .get::<Self>(owner)
            .ok_or_else(|| BevyError::error("Offscreen viewport missing"))?;
        let (viewport, camera) = (target.viewport, target.camera);
        if world
            .get::<ViewportNode>(viewport)
            .is_none_or(|node| node.camera != Some(camera))
        {
            return Err(BevyError::error("Offscreen viewport association broken"));
        }
        let size = world
            .get::<ComputedNode>(viewport)
            .ok_or_else(|| BevyError::error("Offscreen ComputedNode missing"))?
            .size()
            .as_uvec2();
        let active = size.x > 0
            && size.y > 0
            && world
                .get::<InheritedVisibility>(owner)
                .is_some_and(|visibility| visibility.get());
        let mut state = world
            .get_mut::<Camera>(camera)
            .ok_or_else(|| BevyError::error("Offscreen camera missing"))?;
        if state.is_active != active {
            state.is_active = active;
        }
        if !active {
            return Ok(None);
        }
        // Camera target metadata precedes UI layout, while ViewportNode resizes the image after this set.
        // Keep projection and picking on the current physical size during a resize.
        if let Some(info) = &mut state.computed.target_info
            && info.physical_size != size
        {
            info.physical_size = size;
        }
        Ok(Some(size))
    }
}

fn release(
    event: On<Discard<OffscreenViewport>>,
    targets: Query<&OffscreenViewport>,
    mut images: ResMut<Assets<Image>>,
    mut layers: ResMut<Layers>,
    mut commands: Commands,
) {
    if let Ok(target) = targets.get(event.entity) {
        images.remove(target.target.image.id());
        layers.free.push(target.target.layer);
        let viewport = target.viewport;
        let camera = target.camera;
        commands.queue(move |world: &mut World| {
            if world
                .get::<ViewportNode>(viewport)
                .is_some_and(|node| node.camera == Some(camera))
            {
                world.entity_mut(viewport).remove::<ViewportNode>();
            }
        });
        commands.entity(target.scene).try_despawn();
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
        widgetry_info!("WidgetryOffscreenPlugin 注册完成");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::scene_app;

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
        let other = OffscreenTarget::allocate(world).unwrap();
        let target = OffscreenTarget::allocate(world).unwrap();
        let old_image = target.image().clone();
        let old_layer = target.layer();
        assert_ne!(old_layer, other.layer());
        let (old_scene, camera) = scene(world, target.layer());
        let bound = target.bind(world, viewport, camera, old_scene).unwrap();
        let owner = world.spawn(bound).id();
        let target = OffscreenTarget::allocate(world).unwrap();
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
        let recycled = OffscreenTarget::allocate(world).unwrap();
        assert_eq!(recycled.layer(), old_layer);
        recycled.release(world);
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
        other.release(world);
        assert_eq!(world.resource::<Assets<Image>>().len(), 1);
    }

    #[test]
    fn invalid_binding_reclaims_target_and_owned_scene() {
        let mut app = scene_app();
        app.add_plugins(WidgetryOffscreenPlugin);
        let world = app.world_mut();
        let viewport = world.spawn_scene(bsn! { Node::default() }).unwrap().id();
        let scene = world.spawn(Name::new("InvalidOffscreenScene")).id();
        let target = OffscreenTarget::allocate(world).unwrap();
        let image = target.image().clone();
        let layer = target.layer();
        let result = target.bind(world, viewport, scene, scene);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().severity(),
            bevy::ecs::error::Severity::Error
        );
        assert!(!world.entities().contains(scene));
        assert!(world.resource::<Assets<Image>>().get(&image).is_none());
        assert!(world.get::<ViewportNode>(viewport).is_none());
        let replacement = OffscreenTarget::allocate(world).unwrap();
        assert_eq!(replacement.layer(), layer);
        replacement.release(world);
    }
}
