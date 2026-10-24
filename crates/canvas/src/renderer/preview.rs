use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
};

#[derive(Component)]
struct Preview {
    fill: Handle<Mesh>,
    stroke: Handle<Mesh>,
    fill_material: Handle<ColorMaterial>,
    stroke_material: Handle<ColorMaterial>,
    size: Vec2,
    paint: crate::CanvasPaint,
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
    let Some((start, end, paint)) = crate::input::preview(world, view) else {
        if let Some(entity) = current {
            world.despawn(entity);
        }
        return Ok(None);
    };
    let size = (end - start).abs();
    let center = start + (end - start) * 0.5;
    if !size.is_finite() || !center.is_finite() {
        return Err(BevyError::error("Canvas preview bounds overflow"));
    }
    let transform = Transform::from_translation(center.extend(0.03));
    if let Some(entity) = current {
        let assets = world
            .get::<Preview>(entity)
            .ok_or_else(|| BevyError::error("Canvas preview assets missing"))?;
        let (fill_material, stroke_material) =
            (assets.fill_material.clone(), assets.stroke_material.clone());
        let paint_changed = assets.paint != paint;
        if assets.size != size
            || assets.paint.stroke.map(|stroke| stroke.width)
                != paint.stroke.map(|stroke| stroke.width)
        {
            let (fill, stroke) = (assets.fill.clone(), assets.stroke.clone());
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            *meshes
                .get_mut(&fill)
                .ok_or_else(|| BevyError::error("Canvas preview fill missing"))? =
                crate::geometry::fill(size);
            *meshes
                .get_mut(&stroke)
                .ok_or_else(|| BevyError::error("Canvas preview stroke missing"))? =
                crate::geometry::stroke(size, paint.stroke.map_or(0.0, |stroke| stroke.width));
            world
                .get_mut::<Preview>(entity)
                .ok_or_else(|| BevyError::error("Canvas preview assets missing"))?
                .size = size;
        }
        if paint_changed {
            for (material, color) in [
                (&fill_material, paint.fill.unwrap_or(Color::NONE)),
                (
                    &stroke_material,
                    paint.stroke.map_or(Color::NONE, |stroke| stroke.color),
                ),
            ] {
                let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
                let mut material = materials
                    .get_mut(material)
                    .ok_or_else(|| BevyError::error("Canvas preview material missing"))?;
                *material = ColorMaterial::from(color);
            }
            world
                .get_mut::<Preview>(entity)
                .ok_or_else(|| BevyError::error("Canvas preview assets missing"))?
                .paint = paint;
        }
        world
            .get_mut::<Transform>(entity)
            .ok_or_else(|| BevyError::error("Canvas preview transform missing"))?
            .set_if_neq(transform);
        return Ok(Some(entity));
    }
    let assets =
        Preview {
            fill: world
                .resource_mut::<Assets<Mesh>>()
                .add(crate::geometry::fill(size)),
            stroke: world
                .resource_mut::<Assets<Mesh>>()
                .add(crate::geometry::stroke(
                    size,
                    paint.stroke.map_or(0.0, |stroke| stroke.width),
                )),
            fill_material: world
                .resource_mut::<Assets<ColorMaterial>>()
                .add(ColorMaterial::from(paint.fill.unwrap_or(Color::NONE))),
            stroke_material: world.resource_mut::<Assets<ColorMaterial>>().add(
                ColorMaterial::from(paint.stroke.map_or(Color::NONE, |stroke| stroke.color)),
            ),
            size,
            paint,
        };
    let (fill, stroke, fill_material, stroke_material) = (
        assets.fill.clone(),
        assets.stroke.clone(),
        assets.fill_material.clone(),
        assets.stroke_material.clone(),
    );
    let result = bevy_widgetry_core::scene::spawn_scene(
        world,
        bsn! {
            Name("CanvasRectanglePreview")
            template(move |_|Ok(ChildOf(content)))
            template(move |_|Ok(transform)) Visibility::default()
            Children [
                template(move |_|Ok(Mesh2d(fill.clone())))
                template(move |_|Ok(MeshMaterial2d(fill_material.clone())))
                template(move |_|Ok(RenderLayers::none().with(layer)))
                Transform::default() Visibility::default() Pickable::IGNORE NoFrustumCulling
            --
                template(move |_|Ok(Mesh2d(stroke.clone())))
                template(move |_|Ok(MeshMaterial2d(stroke_material.clone())))
                template(move |_|Ok(RenderLayers::none().with(layer)))
                Transform::from_xyz(0.0,0.0,0.01) Visibility::default() Pickable::IGNORE NoFrustumCulling
            ]
        },
    );
    match result {
        Ok(entity) => {
            world.entity_mut(entity).insert(assets);
            Ok(Some(entity))
        }
        Err(error) => {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            meshes.remove(assets.fill.id());
            meshes.remove(assets.stroke.id());
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            materials.remove(assets.fill_material.id());
            materials.remove(assets.stroke_material.id());
            Err(BevyError::error(error))
        }
    }
}

fn release(
    event: On<Discard<Preview>>,
    assets: Query<&Preview>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok(assets) = assets.get(event.entity) {
        meshes.remove(assets.fill.id());
        meshes.remove(assets.stroke.id());
        materials.remove(assets.fill_material.id());
        materials.remove(assets.stroke_material.id());
    }
}
