use crate::{
    CanvasGrid, CanvasPart,
    error::{CanvasResult, missing},
};
use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
};
use bevy_widgetry_core::scene::spawn_scene;

const GRID_DEPTH: f32 = -1.0;
const HALF_LINE_WIDTH: f64 = 0.5;
const MIN_LINE_SPACING: f64 = 12.0;
const MAX_LINES_PER_AXIS: u32 = 512;
const MINOR_COLOR: Color = Color::srgba(0.55, 0.61, 0.69, 0.28);
const MAJOR_COLOR: Color = Color::srgba(0.55, 0.61, 0.69, 0.56);

#[derive(Component)]
struct Overlay {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    layout: Option<Layout>,
}

#[derive(Clone, Copy, PartialEq)]
struct Layout {
    center: Vec2,
    size: Vec2,
    zoom: f32,
    spacing: f32,
    major_every: Option<u32>,
}

impl Layout {
    fn period(self) -> (f64, Option<u32>) {
        let spacing = f64::from(self.spacing) * f64::from(self.zoom);
        let minimum = MIN_LINE_SPACING
            .max(f64::from(self.size.max_element()) / f64::from(MAX_LINES_PER_AXIS - 1));
        if spacing >= minimum {
            return (spacing, self.major_every);
        }
        let period = spacing * f64::from(self.major_every.unwrap_or(1));
        let stride = (minimum / period).log2().ceil().max(0.0).exp2();
        (period * stride, self.major_every.map(|_| 1))
    }

    fn lines(self, axis: usize) -> impl Iterator<Item = (f32, bool)> {
        let (step, major) = self.period();
        let cycle = f64::from(major.unwrap_or(1));
        let half = f64::from(self.size[axis]) * 0.5 + HALF_LINE_WIDTH;
        // Keep the line indices small as the view pans far from the document origin.
        let phase = (f64::from(self.center[axis]) * f64::from(self.zoom)).rem_euclid(step * cycle);
        let first = ((phase - half) / step).ceil();
        (0..MAX_LINES_PER_AXIS).map_while(move |offset| {
            let index = first + f64::from(offset);
            let position = index * step - phase;
            (position <= half).then_some((
                (position / f64::from(self.zoom)) as f32,
                major.is_some() && index.rem_euclid(cycle) == 0.0,
            ))
        })
    }

    fn mesh(self) -> Mesh {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();
        let minor = MINOR_COLOR.to_linear().to_f32_array();
        let major = MAJOR_COLOR.to_linear().to_f32_array();
        let half_line = (HALF_LINE_WIDTH / f64::from(self.zoom)) as f32;
        let half = self.size * (0.5 / self.zoom);
        for axis in 0..2 {
            let perpendicular = 1 - axis;
            for (at, emphasized) in self.lines(axis) {
                let base = positions.len() as u32;
                let mut min = Vec2::ZERO;
                let mut max = Vec2::ZERO;
                min[axis] = at - half_line;
                max[axis] = at + half_line;
                min[perpendicular] = -half[perpendicular];
                max[perpendicular] = half[perpendicular];
                positions.extend([
                    min.extend(0.0).to_array(),
                    [max.x, min.y, 0.0],
                    max.extend(0.0).to_array(),
                    [min.x, max.y, 0.0],
                ]);
                indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                colors.extend([if emphasized { major } else { minor }; 4]);
            }
        }
        crate::geometry::mesh(positions, indices)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    }
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
    if let Some(entity) = current {
        return Ok(Some(entity));
    }
    let camera_depth = world
        .get::<Transform>(camera)
        .ok_or_else(|| missing(CanvasPart::CameraTransform))?
        .translation
        .z;
    let transform = Transform::from_xyz(0.0, 0.0, GRID_DEPTH - camera_depth)
        .with_scale(Vec3::new(1.0, -1.0, 1.0));
    let mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(crate::geometry::mesh(Vec::new(), Vec::new()));
    let material = world
        .resource_mut::<Assets<ColorMaterial>>()
        .add(ColorMaterial::default());
    let material_handle = material.clone();
    let scene = spawn_scene(
        world,
        bsn! {
            Name("CanvasGrid") Pickable::IGNORE NoFrustumCulling Visibility::Inherited
            template(move |_| Ok(transform))
            template(move |_| Ok(ChildOf(camera)))
            template(move |_| Ok(RenderLayers::none().with(layer)))
            template(move |_| Ok(MeshMaterial2d(material_handle.clone())))
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

pub(super) fn update(
    world: &mut World,
    entity: Entity,
    grid: CanvasGrid,
    center: Vec2,
    size: Vec2,
    zoom: f32,
) -> CanvasResult {
    let layout = Layout {
        center,
        size,
        zoom,
        spacing: grid.spacing,
        major_every: grid.major_every,
    };
    let overlay = world
        .get::<Overlay>(entity)
        .ok_or_else(|| missing(CanvasPart::GridOverlay))?;
    if overlay.layout == Some(layout) {
        return Ok(());
    }
    let handle = overlay.mesh.clone();
    let mesh = layout.mesh();
    let has_lines = mesh.count_vertices() > 0;
    *world
        .resource_mut::<Assets<Mesh>>()
        .get_mut(&handle)
        .ok_or_else(|| missing(CanvasPart::GridMesh))? = mesh;
    if has_lines && world.get::<Mesh2d>(entity).is_none() {
        world.entity_mut(entity).insert(Mesh2d(handle));
    } else if !has_lines {
        world.entity_mut(entity).remove::<Mesh2d>();
    }
    world
        .get_mut::<Overlay>(entity)
        .ok_or_else(|| missing(CanvasPart::GridOverlay))?
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

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn pan_keeps_grid_phase_and_major_lines_in_document_space() {
        let layout = Layout {
            center: Vec2::new(-13.0, 9.0),
            size: Vec2::new(100.0, 100.0),
            zoom: 2.0,
            spacing: 20.0,
            major_every: Some(2),
        };
        assert_eq!(
            layout.lines(0).collect::<Vec<_>>(),
            [(-7.0, false), (13.0, true)]
        );
        assert_eq!(
            layout.lines(1).collect::<Vec<_>>(),
            [(-9.0, true), (11.0, false)]
        );
    }

    #[test]
    fn dense_grid_adapts_to_zoom_and_has_a_fixed_geometry_budget() {
        let dense = Layout {
            center: Vec2::new(-10.0, 10.0),
            size: Vec2::new(7680.0, 4320.0),
            zoom: 0.1,
            spacing: 10.0,
            major_every: Some(4),
        };
        let (step, major) = dense.period();
        assert_eq!(major, Some(1));
        assert!((step - 16.0).abs() < 0.001);
        for spacing in [dense.spacing, f32::from_bits(1)] {
            let view = Layout { spacing, ..dense };
            let count = view.mesh().count_vertices();
            assert!(count > 0 && count <= MAX_LINES_PER_AXIS as usize * 2 * 4);
        }
    }
}
