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
    materials: [Handle<ColorMaterial>; 3],
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
        crate::geometry::meshes(&crate::CanvasGeometry::Path(data.path.clone()), data.paint)?
    };
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for p in &data.points {
        let base = positions.len() as u32;
        for d in [Vec2::NEG_X, Vec2::NEG_Y, Vec2::X, Vec2::Y] {
            positions.push((*p + d * (3.0 / zoom)).extend(0.0).to_array());
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Ok([fill, stroke, crate::geometry::mesh(positions, indices)])
}

fn colors(data: &crate::pen::Preview) -> [Color; 3] {
    [
        data.paint.fill.unwrap_or(Color::NONE),
        data.paint.stroke.map_or(Color::NONE, |s| s.color),
        Color::srgb(1.0, 0.75, 0.1),
    ]
}

fn release_assets(
    world: &mut World,
    meshes: &[Handle<Mesh>; 3],
    materials: &[Handle<ColorMaterial>; 3],
) {
    for h in meshes {
        world.resource_mut::<Assets<Mesh>>().remove(h.id());
    }
    for h in materials {
        world.resource_mut::<Assets<ColorMaterial>>().remove(h.id());
    }
}

pub(super) fn sync(
    world: &mut World,
    view: Entity,
    content: Entity,
    layer: usize,
    current: Option<Entity>,
) -> Result<Option<Entity>> {
    let Some(data) = crate::pen::preview(world, view) else {
        if let Some(e) = current {
            world.despawn(e);
        }
        return Ok(None);
    };
    let zoom = crate::CanvasView::zoom(world, view)?;
    if let Some(e) = current {
        let p = world
            .get::<Preview>(e)
            .ok_or_else(|| BevyError::error("Canvas Pen preview missing"))?;
        if p.data == data && p.zoom == zoom {
            return Ok(Some(e));
        }
        let (handles, materials, nodes) = (p.meshes.clone(), p.materials.clone(), p.nodes);
        for (i, mesh) in meshes(&data, zoom)?.into_iter().enumerate() {
            let visible = mesh.count_vertices() > 0;
            *world
                .resource_mut::<Assets<Mesh>>()
                .get_mut(&handles[i])
                .ok_or_else(|| BevyError::error("Canvas Pen preview mesh missing"))? = mesh;
            world.entity_mut(nodes[i]).insert(if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
        for (h, color) in materials.iter().zip(colors(&data)) {
            *world
                .resource_mut::<Assets<ColorMaterial>>()
                .get_mut(h)
                .ok_or_else(|| BevyError::error("Canvas Pen preview material missing"))? =
                ColorMaterial::from(color);
        }
        let mut p = world
            .get_mut::<Preview>(e)
            .ok_or_else(|| BevyError::error("Canvas Pen preview missing"))?;
        p.data = data;
        p.zoom = zoom;
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
    let materials = colors(&data).map(|c| {
        world
            .resource_mut::<Assets<ColorMaterial>>()
            .add(ColorMaterial::from(c))
    });
    let [fill, stroke, handles] = meshes.clone();
    let [fill_material, stroke_material, handle_material] = materials.clone();
    let [fv, sv, hv] = visibility;
    let result = bevy_widgetry_core::scene::spawn_scene(
        world,
        bsn! {
            Parts {fill:#Fill,stroke:#Stroke,handles:#Handles}
            Name("CanvasPenPreview") template(move |_|Ok(ChildOf(content))) Transform::from_xyz(0.0,0.0,102.0) Visibility::default()
            Children [
                #Fill template(move |_|Ok(Mesh2d(fill.clone()))) template(move |_|Ok(MeshMaterial2d(fill_material.clone()))) template(move |_|Ok(RenderLayers::none().with(layer))) template(move |_|Ok(fv)) Transform::default() Pickable::IGNORE NoFrustumCulling
            --
                #Stroke template(move |_|Ok(Mesh2d(stroke.clone()))) template(move |_|Ok(MeshMaterial2d(stroke_material.clone()))) template(move |_|Ok(RenderLayers::none().with(layer))) template(move |_|Ok(sv)) Transform::from_xyz(0.0,0.0,0.01) Pickable::IGNORE NoFrustumCulling
            --
                #Handles template(move |_|Ok(Mesh2d(handles.clone()))) template(move |_|Ok(MeshMaterial2d(handle_material.clone()))) template(move |_|Ok(RenderLayers::none().with(layer))) template(move |_|Ok(hv)) Transform::from_xyz(0.0,0.0,0.02) Pickable::IGNORE NoFrustumCulling
            ]
        },
    );
    match result {
        Ok(e) => {
            let Some(parts) = world.get::<Parts>(e) else {
                world.despawn(e);
                release_assets(world, &meshes, &materials);
                return Err(BevyError::error("Canvas Pen preview parts missing"));
            };
            let nodes = [parts.fill, parts.stroke, parts.handles];
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
            Err(BevyError::error(e))
        }
    }
}

fn release(
    event: On<Discard<Preview>>,
    previews: Query<&Preview>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok(p) = previews.get(event.entity) {
        for h in &p.meshes {
            meshes.remove(h.id());
        }
        for h in &p.materials {
            materials.remove(h.id());
        }
    }
}
