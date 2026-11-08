const HANDLE_COLOR: Color = Color::srgb(1.0, 0.75, 0.1);
const NODE_DEPTH: f32 = 103.0;

use crate::error::CanvasResult as Result;
use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
};

#[derive(Component)]
struct Overlay {
    data: crate::nodes::Overlay,
    zoom: f32,
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

pub(super) fn sync(
    world: &mut World,
    view: Entity,
    content: Entity,
    layer: usize,
    current: Option<Entity>,
) -> Result<Option<Entity>> {
    (|| -> Result<Option<Entity>> {
    let data = if crate::CanvasView::_mode(world, view)? == crate::CanvasMode::PathNodeEdit {
        crate::CanvasView::_single_selection(world, view)?
            .and_then(|shape| crate::nodes::overlay(world, shape))
    } else {
        None
    };
    let Some(data) = data else {
        if let Some(e) = current {
            world.despawn(e);
        }
        return Ok(None);
    };
    let zoom = crate::CanvasView::_zoom(world, view)?;
    if let Some(e) = current {
        let previous = world
            .get::<Overlay>(e)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::NodeOverlay))?;
        if previous.data == data && previous.zoom == zoom {
            return Ok(Some(e));
        }
        let handle = previous.mesh.clone();
        *world
            .resource_mut::<Assets<Mesh>>()
            .get_mut(&handle)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::NodeOverlayMesh))? =
            crate::nodes::mesh(&data, zoom);
        let mut previous = world
            .get_mut::<Overlay>(e)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::NodeOverlay))?;
        previous.data = data;
        previous.zoom = zoom;
        return Ok(Some(e));
    }
    let mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(crate::nodes::mesh(&data, zoom));
    let material = world
        .resource_mut::<Assets<ColorMaterial>>()
        .add(ColorMaterial::from(HANDLE_COLOR));
    let (m, c) = (mesh.clone(), material.clone());
    match bevy_widgetry_core::scene::spawn_scene(
        world,
        bsn! {
            Name("CanvasPathNodes") template(move |_|Ok(Mesh2d(m.clone()))) template(move |_|Ok(MeshMaterial2d(c.clone())))
            template(move |_|Ok(RenderLayers::none().with(layer))) template(move |_|Ok(ChildOf(content)))
            Transform::from_xyz(0.0,0.0,NODE_DEPTH) Visibility::default() Pickable::IGNORE NoFrustumCulling
        },
    ) {
        Ok(e) => {
            world.entity_mut(e).insert(Overlay {
                data,
                zoom,
                mesh,
                material,
            });
            Ok(Some(e))
        }
        Err(e) => {
            world.resource_mut::<Assets<Mesh>>().remove(mesh.id());
            world
                .resource_mut::<Assets<ColorMaterial>>()
                .remove(material.id());
            Err(e.into_bevy_error().into())
        }
    }
})().map_err(|error| error.at("view", view).at("content", content))
}

pub(super) fn install(app: &mut App) {
    app.add_observer(
        |event: On<Discard<Overlay>>,
         overlays: Query<&Overlay>,
         mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<ColorMaterial>>| {
            if let Ok(o) = overlays.get(event.entity) {
                meshes.remove(o.mesh.id());
                materials.remove(o.material.id());
            }
        },
    );
}
