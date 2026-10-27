use crate::{CanvasPaint, CanvasView};
use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
};

#[derive(Component)]
struct Outline {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    points: [Vec2; 4],
    zoom: f32,
}

pub(super) fn install(app: &mut App) {
    app.add_observer(release);
}

pub(super) fn sync(
    world: &mut World,
    view: Entity,
    content: Entity,
    layer: usize,
    current: Option<Entity>,
) -> Result<Option<Entity>> {
    let Some(shape) = CanvasView::selection(world, view)? else {
        if let Some(entity) = current {
            world.despawn(entity);
        }
        return Ok(None);
    };
    let geometry = world
        .get::<crate::CanvasGeometry>(shape)
        .ok_or_else(|| BevyError::error("Canvas selection geometry missing"))?
        .clone();
    let transform = *world
        .get::<Transform>(shape)
        .ok_or_else(|| BevyError::error("Canvas selection transform missing"))?;
    let paint = *world
        .get::<CanvasPaint>(shape)
        .ok_or_else(|| BevyError::error("Canvas selection paint missing"))?;
    let points = crate::selection::frame(world, shape)?;
    let zoom = CanvasView::zoom(world, view)?;
    let (controls, rotation) = if let Some(rectangle) = geometry.rectangle() {
        (
            crate::resize::controls(rectangle, &transform, &paint).map(|(_, point)| point),
            crate::rotate::control(rectangle, &transform, &paint, zoom),
        )
    } else {
        let middle = |a: Vec2, b: Vec2| ((a.as_dvec2() + b.as_dvec2()) * 0.5).as_vec2();
        let controls = [
            points[0],
            middle(points[0], points[1]),
            points[1],
            middle(points[1], points[2]),
            points[2],
            middle(points[2], points[3]),
            points[3],
            middle(points[3], points[0]),
        ];
        let top = controls[1];
        let center = middle(points[0], points[2]);
        (
            controls,
            top + (top - center).normalize_or_zero() * (crate::rotate::OFFSET / zoom),
        )
    };
    let depth = 101.0;
    if let Some(entity) = current {
        let outline = world
            .get::<Outline>(entity)
            .ok_or_else(|| BevyError::error("Canvas outline assets missing"))?;
        if outline.points != points || outline.zoom != zoom {
            let handle = outline.mesh.clone();
            *world
                .resource_mut::<Assets<Mesh>>()
                .get_mut(&handle)
                .ok_or_else(|| BevyError::error("Canvas outline mesh missing"))? =
                crate::geometry::outline(
                    points,
                    2.0 / zoom,
                    3.0 / zoom,
                    controls,
                    crate::resize::HANDLE_SIZE / zoom,
                    rotation,
                    crate::rotate::HANDLE_SIZE / zoom,
                );
            let mut outline = world
                .get_mut::<Outline>(entity)
                .ok_or_else(|| BevyError::error("Canvas outline assets missing"))?;
            outline.points = points;
            outline.zoom = zoom;
        }
        world
            .get_mut::<Transform>(entity)
            .ok_or_else(|| BevyError::error("Canvas outline transform missing"))?
            .set_if_neq(Transform::from_xyz(0.0, 0.0, depth));
        return Ok(Some(entity));
    }
    let assets = Outline {
        mesh: world
            .resource_mut::<Assets<Mesh>>()
            .add(crate::geometry::outline(
                points,
                2.0 / zoom,
                3.0 / zoom,
                controls,
                crate::resize::HANDLE_SIZE / zoom,
                rotation,
                crate::rotate::HANDLE_SIZE / zoom,
            )),
        material: world
            .resource_mut::<Assets<ColorMaterial>>()
            .add(ColorMaterial::from(Color::srgb(0.0, 0.55, 1.0))),
        points,
        zoom,
    };
    let (mesh, material) = (assets.mesh.clone(), assets.material.clone());
    let result = bevy_widgetry_core::scene::spawn_scene(
        world,
        bsn! {
            Name("CanvasSelectionOutline")
            template(move |_| Ok(Mesh2d(mesh.clone())))
            template(move |_| Ok(MeshMaterial2d(material.clone())))
            template(move |_| Ok(RenderLayers::none().with(layer)))
            template(move |_| Ok(ChildOf(content)))
            Transform::from_xyz(0.0,0.0,depth) Visibility::default() Pickable::IGNORE NoFrustumCulling
        },
    );
    match result {
        Ok(entity) => {
            world.entity_mut(entity).insert(assets);
            Ok(Some(entity))
        }
        Err(error) => {
            release_assets(world, &assets);
            Err(BevyError::error(error))
        }
    }
}

fn release_assets(world: &mut World, assets: &Outline) {
    world
        .resource_mut::<Assets<Mesh>>()
        .remove(assets.mesh.id());
    world
        .resource_mut::<Assets<ColorMaterial>>()
        .remove(assets.material.id());
}

fn release(
    event: On<Discard<Outline>>,
    assets: Query<&Outline>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok(assets) = assets.get(event.entity) {
        meshes.remove(assets.mesh.id());
        materials.remove(assets.material.id());
    }
}
