const HANDLE_HALF_SIZE: f32 = 3.0;
const HANDLE_COLOR: Color = Color::srgb(1.0, 0.75, 0.1);
const PREVIEW_DEPTH: f32 = 102.0;
const HANDLE_DEPTH_OFFSET: f32 = 0.02;

use super::paint::{GradientMaterial, PaintMaterial};
use crate::{CanvasFill, error::CanvasResult as Result};
use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
};

#[derive(Component, FromTemplate)]
struct Parts {
    fill: Entity,
    stroke: Entity,
    handles: Entity,
}

#[derive(Component)]
struct Preview {
    data: crate::pen::Preview,
    zoom: f32,
    meshes: [Handle<Mesh>; 3],
    materials: [PaintMaterial; 3],
    nodes: [Entity; 3],
}

pub(super) fn install(app: &mut App) {
    app.add_observer(release);
}

fn meshes(data: &crate::pen::Preview, zoom: f32) -> Result<[Mesh; 3]> {
    let empty = || crate::geometry::mesh(Vec::new(), Vec::new());
    let (fill, stroke) = if data.path.commands.is_empty() {
        (empty(), empty())
    } else {
        crate::geometry::meshes_with_arrows(
            &crate::CanvasGeometry::Path(data.path.clone()),
            data.paint,
            data.arrows.as_ref(),
        )?
    };
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for p in &data.points {
        let base = positions.len() as u32;
        for d in [Vec2::NEG_X, Vec2::NEG_Y, Vec2::X, Vec2::Y] {
            positions.push((*p + d * (HANDLE_HALF_SIZE / zoom)).extend(0.0).to_array());
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Ok([fill, stroke, crate::geometry::mesh(positions, indices)])
}

fn fills(data: &crate::pen::Preview) -> [Option<CanvasFill>; 3] {
    [
        data.paint.fill,
        data.paint
            .stroke
            .map(|stroke| CanvasFill::Solid(stroke.color)),
        Some(CanvasFill::Solid(HANDLE_COLOR)),
    ]
}

fn release_assets(world: &mut World, meshes: &[Handle<Mesh>; 3], materials: &[PaintMaterial; 3]) {
    for h in meshes {
        world.resource_mut::<Assets<Mesh>>().remove(h.id());
    }
    for h in materials {
        h.release(world);
    }
}

pub(super) fn sync(
    world: &mut World,
    view: Entity,
    content: Entity,
    layer: usize,
    current: Option<Entity>,
) -> Result<Option<Entity>> {
    (|| -> Result<Option<Entity>> {
    let data = match crate::pen::preview(world, view) {
        Some(data) => Some(data),
        None => crate::basic_path::preview(world, view)?,
    };
    let Some(data) = data else {
        if let Some(e) = current {
            world.despawn(e);
        }
        return Ok(None);
    };
    let zoom = crate::CanvasView::_zoom(world, view)?;
    if let Some(e) = current {
        let p = world
            .get::<Preview>(e)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::PenPreview))?;
        if p.data == data && p.zoom == zoom {
            return Ok(Some(e));
        }
        let (handles, mut materials, nodes) = (p.meshes.clone(), p.materials.clone(), p.nodes);
        let previous_fills = fills(&p.data);
        if p.data.name != data.name {
            world.entity_mut(e).insert(Name::new(data.name));
        }
        for (i, mesh) in meshes(&data, zoom)?.into_iter().enumerate() {
            let visible = mesh.count_vertices() > 0;
            *world
                .resource_mut::<Assets<Mesh>>()
                .get_mut(&handles[i])
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::PenPreviewMesh))? = mesh;
            world.entity_mut(nodes[i]).insert(if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
        for (index, fill) in fills(&data).into_iter().enumerate() {
            if fill != previous_fills[index] {
                materials[index].update(world, nodes[index], fill, ColorMaterial::from)?;
            }
        }
        let mut p = world
            .get_mut::<Preview>(e)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::PenPreview))?;
        p.data = data;
        p.zoom = zoom;
        p.materials = materials;
        return Ok(Some(e));
    }
    let generated = meshes(&data, zoom)?;
    let visibility = generated.each_ref().map(|m| {
        if m.count_vertices() > 0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    });
    let meshes = generated.map(|m| world.resource_mut::<Assets<Mesh>>().add(m));
    let materials = fills(&data).map(|fill| PaintMaterial::add(world, fill, ColorMaterial::from));
    let [fill, stroke, handles] = meshes.clone();
    let [fv, sv, hv] = visibility;
    let name = data.name;
    let result = bevy_widgetry_core::scene::spawn_scene(
        world,
        bsn! {
            Parts {fill:#Fill,stroke:#Stroke,handles:#Handles}
            template(move |_|Ok(Name::new(name))) template(move |_|Ok(ChildOf(content))) Transform::from_xyz(0.0,0.0,PREVIEW_DEPTH) Visibility::default()
            Children [
                #Fill template(move |_|Ok(Mesh2d(fill.clone()))) template(move |_|Ok(RenderLayers::none().with(layer))) template(move |_|Ok(fv)) Transform::default() Pickable::IGNORE NoFrustumCulling
            --
                #Stroke template(move |_|Ok(Mesh2d(stroke.clone()))) template(move |_|Ok(RenderLayers::none().with(layer))) template(move |_|Ok(sv)) Transform::from_xyz(0.0,0.0,super::STROKE_DEPTH_OFFSET) Pickable::IGNORE NoFrustumCulling
            --
                #Handles template(move |_|Ok(Mesh2d(handles.clone()))) template(move |_|Ok(RenderLayers::none().with(layer))) template(move |_|Ok(hv)) Transform::from_xyz(0.0,0.0,HANDLE_DEPTH_OFFSET) Pickable::IGNORE NoFrustumCulling
            ]
        },
    );
    match result {
        Ok(e) => {
            let Some(parts) = world.get::<Parts>(e) else {
                world.despawn(e);
                release_assets(world, &meshes, &materials);
                return Err(crate::error::missing(crate::CanvasPart::PenPreviewParts));
            };
            let nodes = [parts.fill, parts.stroke, parts.handles];
            for (node, material) in nodes.into_iter().zip(&materials) {
                material.attach(world, node);
            }
            world.entity_mut(e).insert(Preview {
                data,
                zoom,
                meshes,
                materials,
                nodes,
            });
            Ok(Some(e))
        }
        Err(e) => {
            release_assets(world, &meshes, &materials);
            Err(e.into_bevy_error().into())
        }
    }
})().map_err(|error| error.at("view", view).at("content", content))
}

fn release(
    event: On<Discard<Preview>>,
    previews: Query<&Preview>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut gradients: ResMut<Assets<GradientMaterial>>,
) {
    if let Ok(p) = previews.get(event.entity) {
        for h in &p.meshes {
            meshes.remove(h.id());
        }
        for h in &p.materials {
            h.release_from(&mut materials, &mut gradients);
        }
    }
}
