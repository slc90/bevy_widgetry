use crate::CanvasView;
use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
};

#[derive(Component)]
struct Outline {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    points: Vec<[Vec2; 4]>,
    zoom: f32,
    parameters: Vec<(crate::parameters::ParameterHandle, Vec2)>,
}

pub(super) fn install(app: &mut App) {
    app.add_observer(release);
}

fn mesh(
    points: &[[Vec2; 4]],
    zoom: f32,
    parameters: &[(crate::parameters::ParameterHandle, Vec2)],
) -> Result<Mesh> {
    if let [points] = points {
        crate::parameters::decorate(
            crate::geometry::outline(
                *points,
                2.0 / zoom,
                3.0 / zoom,
                crate::scale::controls(*points).map(|(_, p)| p),
                crate::scale::HANDLE_SIZE / zoom,
                crate::rotate::control(*points, zoom),
                crate::rotate::HANDLE_SIZE / zoom,
            ),
            parameters,
            zoom,
        )
    } else {
        Ok(crate::geometry::reference_frames(
            points,
            2.0 / zoom,
            3.0 / zoom,
        ))
    }
}

pub(super) fn sync(
    world: &mut World,
    view: Entity,
    content: Entity,
    layer: usize,
    current: Option<Entity>,
) -> Result<Option<Entity>> {
    let selection = CanvasView::selection(world, view)?;
    if selection.is_empty()
        || (selection.len() == 1
            && CanvasView::mode(world, view)? == crate::CanvasMode::PathNodeEdit)
    {
        if let Some(e) = current {
            world.despawn(e);
        }
        return Ok(None);
    }
    let zoom = CanvasView::zoom(world, view)?;
    let parameters = if let [shape] = selection.as_slice() {
        crate::parameters::controls(world, *shape, zoom)?
    } else {
        Vec::new()
    };
    let mut points = Vec::with_capacity(selection.len());
    for shape in selection {
        points.push(crate::selection::frame(world, shape)?);
    }
    if let Some(e) = current {
        let previous = world
            .get::<Outline>(e)
            .ok_or_else(|| BevyError::error("Canvas outline assets missing"))?;
        if previous.points != points || previous.zoom != zoom || previous.parameters != parameters {
            let handle = previous.mesh.clone();
            *world
                .resource_mut::<Assets<Mesh>>()
                .get_mut(&handle)
                .ok_or_else(|| BevyError::error("Canvas outline mesh missing"))? =
                mesh(&points, zoom, &parameters)?;
            let mut previous = world
                .get_mut::<Outline>(e)
                .ok_or_else(|| BevyError::error("Canvas outline assets missing"))?;
            previous.points = points;
            previous.zoom = zoom;
            previous.parameters = parameters;
        }
        return Ok(Some(e));
    }
    let assets = Outline {
        mesh: world
            .resource_mut::<Assets<Mesh>>()
            .add(mesh(&points, zoom, &parameters)?),
        material: world
            .resource_mut::<Assets<ColorMaterial>>()
            .add(ColorMaterial::from(Color::srgb(0.0, 0.55, 1.0))),
        points,
        zoom,
        parameters,
    };
    let (mesh, material) = (assets.mesh.clone(), assets.material.clone());
    let result = bevy_widgetry_core::scene::spawn_scene(
        world,
        bsn! {
            Name("CanvasSelectionOutline") template(move |_|Ok(Mesh2d(mesh.clone()))) template(move |_|Ok(MeshMaterial2d(material.clone())))
            template(move |_|Ok(RenderLayers::none().with(layer))) template(move |_|Ok(ChildOf(content)))
            Transform::from_xyz(0.0,0.0,101.0) Visibility::default() Pickable::IGNORE NoFrustumCulling
        },
    );
    match result {
        Ok(e) => {
            world.entity_mut(e).insert(assets);
            Ok(Some(e))
        }
        Err(e) => {
            release_assets(world, &assets);
            Err(BevyError::error(e))
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
