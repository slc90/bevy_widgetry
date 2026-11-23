use crate::{
    CanvasPart,
    error::{CanvasResult, missing},
};
use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
};
use bevy_widgetry_core::scene::spawn_scene;

const GUIDE_DEPTH: f32 = 104.0;
const HALF_WIDTH_PX: f32 = 0.75;
const GUIDE_COLOR: Color = Color::srgb(0.08, 0.65, 0.95);

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Layout {
    pub(super) lines: [Option<f32>; 2],
    pub(super) center: Vec2,
    pub(super) pixels: Vec2,
    pub(super) zoom: f32,
}

impl Layout {
    fn mesh(self) -> Mesh {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let half_size = self.pixels * (0.5 / self.zoom);
        let width = HALF_WIDTH_PX / self.zoom;
        for (axis, at) in self.lines.into_iter().enumerate() {
            let Some(at) = at else {
                continue;
            };
            let at = at - self.center[axis];
            if at.abs() > half_size[axis] {
                continue;
            }
            let mut min = -half_size;
            let mut max = half_size;
            min[axis] = at - width;
            max[axis] = at + width;
            let base = positions.len() as u32;
            positions.extend([
                min.extend(0.0).to_array(),
                [max.x, min.y, 0.0],
                max.extend(0.0).to_array(),
                [min.x, max.y, 0.0],
            ]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        crate::geometry::mesh(positions, indices)
    }
}

#[derive(Component)]
struct Overlay {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    layout: Option<Layout>,
}

pub(super) fn sync(
    world: &mut World,
    camera: Entity,
    layer: usize,
    enabled: bool,
    current: Option<Entity>,
) -> CanvasResult<Option<Entity>> {
    if !enabled {
        if let Some(entity) = current {
            world.despawn(entity);
        }
        return Ok(None);
    }
    if current.is_some() {
        return Ok(current);
    }
    let camera_depth = world
        .get::<Transform>(camera)
        .ok_or_else(|| missing(CanvasPart::CameraTransform))?
        .translation
        .z;
    let transform = Transform::from_xyz(0.0, 0.0, GUIDE_DEPTH - camera_depth)
        .with_scale(Vec3::new(1.0, -1.0, 1.0));
    let mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(crate::geometry::mesh(Vec::new(), Vec::new()));
    let material = world
        .resource_mut::<Assets<ColorMaterial>>()
        .add(GUIDE_COLOR);
    let display_mesh = mesh.clone();
    let display_material = material.clone();
    let scene = spawn_scene(
        world,
        bsn! {
            Name("CanvasSnapGuide") Pickable::IGNORE NoFrustumCulling Visibility::Inherited
            template(move |_| Ok(transform))
            template(move |_| Ok(ChildOf(camera)))
            template(move |_| Ok(RenderLayers::none().with(layer)))
            template(move |_| Ok(Mesh2d(display_mesh.clone())))
            template(move |_| Ok(MeshMaterial2d(display_material.clone())))
        },
    );
    match scene {
        Ok(entity) => {
            world.entity_mut(entity).insert(Overlay {
                mesh,
                material,
                layout: None,
            });
            Ok(Some(entity))
        }
        Err(error) => {
            world.resource_mut::<Assets<Mesh>>().remove(mesh.id());
            world
                .resource_mut::<Assets<ColorMaterial>>()
                .remove(material.id());
            Err(error.into_bevy_error().into())
        }
    }
}

pub(super) fn update(world: &mut World, entity: Entity, layout: Layout) -> CanvasResult {
    let overlay = world
        .get::<Overlay>(entity)
        .ok_or_else(|| missing(CanvasPart::SnapGuide))?;
    if overlay.layout == Some(layout) {
        return Ok(());
    }
    let handle = overlay.mesh.clone();
    *world
        .resource_mut::<Assets<Mesh>>()
        .get_mut(&handle)
        .ok_or_else(|| missing(CanvasPart::SnapGuide))? = layout.mesh();
    world
        .get_mut::<Overlay>(entity)
        .ok_or_else(|| missing(CanvasPart::SnapGuide))?
        .layout = Some(layout);
    Ok(())
}

pub(super) fn install(app: &mut App) {
    app.add_observer(
        |event: On<Discard<Overlay>>,
         overlays: Query<&Overlay>,
         mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<ColorMaterial>>| {
            if let Ok(overlay) = overlays.get(event.entity) {
                meshes.remove(overlay.mesh.id());
                materials.remove(overlay.material.id());
            }
        },
    );
}
