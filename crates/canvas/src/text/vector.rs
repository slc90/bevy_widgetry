use super::{CanvasText, CanvasTextStyle, GlyphLayout, GlyphRun};
use crate::error::{CanvasResult, glyph_outline, missing};
use bevy::camera::visibility::RenderLayers;
use bevy::math::{Affine3A, DVec2};
use bevy::prelude::*;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers,
};
use skrifa::instance::{NormalizedCoord, Size};
use skrifa::outline::{DrawError, DrawSettings, OutlinePen, pen::PathStyle};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::collections::HashMap;
use std::sync::Arc;

const MESH_EM: f32 = 64.0;
const CURVE_ERROR_PIXELS: f32 = 0.125;
const MIN_PIXELS_PER_EM_LOG2: f64 = 3.0;
const MAX_PIXELS_PER_EM_LOG2: f64 = 17.0;
const CACHE_ENTRIES: usize = 8192;
const CACHE_GEOMETRY_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Precision(i32);

impl Precision {
    fn new(font_size: f32, projection: Affine3A, density: DVec2) -> Self {
        let [x, y] = crate::groups::planar_axes(&projection).map(|axis| axis * density);
        let (xx, yy, xy) = (x.length_squared(), y.length_squared(), x.dot(y));
        let largest_scale = ((xx + yy + (xx - yy).hypot(2.0 * xy)) * 0.5).sqrt();
        let pixels_per_em = largest_scale * f64::from(font_size);
        // Round up so a bucket covers every scale it serves. The maximum keeps Lyon above f32 coordinate noise.
        Self(
            pixels_per_em
                .log2()
                .ceil()
                .clamp(MIN_PIXELS_PER_EM_LOG2, MAX_PIXELS_PER_EM_LOG2) as i32,
        )
    }

    fn tolerance(self) -> f32 {
        CURVE_ERROR_PIXELS * MESH_EM / 2.0_f32.powi(self.0)
    }
}

#[derive(PartialEq, Eq, Hash)]
struct GlyphKey {
    blob: u64,
    face: u32,
    coords: Arc<[NormalizedCoord]>,
    id: GlyphId,
    precision: Precision,
}

struct Entry {
    mesh: Option<Handle<Mesh>>,
    bytes: usize,
    last_used: u64,
}

#[derive(Resource, Default)]
pub(crate) struct GlyphMeshCache {
    entries: HashMap<GlyphKey, Entry>,
    bytes: usize,
    access: u64,
}

impl GlyphMeshCache {
    fn mesh(
        &mut self,
        assets: &mut Assets<Mesh>,
        run: &GlyphRun,
        id: GlyphId,
        precision: Precision,
    ) -> CanvasResult<Option<Handle<Mesh>>> {
        use std::collections::hash_map::Entry as Slot;
        self.access += 1;
        let entry = match self.entries.entry(GlyphKey {
            blob: run.font.data.id(),
            face: run.font.index,
            coords: run.coords.clone(),
            id,
            precision,
        }) {
            Slot::Occupied(entry) => entry.into_mut(),
            Slot::Vacant(entry) => {
                let mesh = tessellate(run, id, precision).map_err(|error| {
                    error.input(crate::CanvasInput::Index {
                        field: "glyph",
                        value: id.to_u32() as usize,
                    })
                })?;
                let bytes = mesh.as_ref().map_or(0, |mesh| {
                    mesh.count_vertices() * size_of::<[f32; 3]>()
                        + mesh.indices().map_or(0, |indices| indices.len()) * size_of::<u32>()
                });
                self.bytes += bytes;
                entry.insert(Entry {
                    mesh: mesh.map(|mesh| assets.add(mesh)),
                    bytes,
                    last_used: 0,
                })
            }
        };
        entry.last_used = self.access;
        Ok(entry.mesh.clone())
    }

    pub(crate) fn trim(&mut self) {
        if self.entries.len() <= CACHE_ENTRIES && self.bytes <= CACHE_GEOMETRY_BYTES {
            return;
        }
        let mut ages: Vec<_> = self
            .entries
            .values()
            .map(|entry| (entry.last_used, entry.bytes))
            .collect();
        ages.sort_unstable();
        let mut bytes = self.bytes;
        let mut count = ages.len();
        let mut cutoff = 0;
        for (age, size) in ages {
            if count <= CACHE_ENTRIES && bytes <= CACHE_GEOMETRY_BYTES {
                break;
            }
            bytes -= size;
            count -= 1;
            cutoff = age;
        }
        self.entries.retain(|_, entry| entry.last_used > cutoff);
        self.bytes = bytes;
    }
}

struct GlyphPen(lyon::path::path::Builder);

impl OutlinePen for GlyphPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.begin(point(x, -y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(point(x, -y));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.quadratic_bezier_to(point(cx, -cy), point(x, -y));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0
            .cubic_bezier_to(point(cx0, -cy0), point(cx1, -cy1), point(x, -y));
    }

    fn close(&mut self) {
        self.0.close();
    }
}

fn tessellate(run: &GlyphRun, id: GlyphId, precision: Precision) -> CanvasResult<Option<Mesh>> {
    let font = FontRef::from_index(run.font.data.as_ref(), run.font.index)
        .map_err(|error| glyph_outline(DrawError::Read(error)))?;
    let outline = font
        .outline_glyphs()
        .get(id)
        .ok_or_else(|| glyph_outline(DrawError::GlyphNotFound(id)))?;
    let mut pen = GlyphPen(Path::builder());
    let settings = DrawSettings::unhinted(Size::new(MESH_EM), run.coords.as_ref())
        .with_path_style(PathStyle::HarfBuzz);
    outline.draw(settings, &mut pen).map_err(glyph_outline)?;
    let mut triangles = VertexBuffers::new();
    FillTessellator::new().tessellate_path(
        &pen.0.build(),
        &FillOptions::default()
            .with_fill_rule(FillRule::NonZero)
            .with_tolerance(precision.tolerance()),
        &mut BuffersBuilder::new(&mut triangles, |vertex: FillVertex| {
            let position = vertex.position();
            [position.x, position.y, 0.0]
        }),
    )?;
    Ok((!triangles.indices.is_empty())
        .then(|| crate::geometry::mesh(triangles.vertices, triangles.indices)))
}

pub(crate) struct TextDraw {
    layout: DrawLayout,
    color: Color,
}

enum DrawLayout {
    Measured(Arc<GlyphLayout>),
    Reflected(CanvasTextStyle),
}

impl TextDraw {
    pub(crate) fn from_text(text: &CanvasText) -> Self {
        let layout = match &text.glyphs {
            Some(layout) => DrawLayout::Measured(layout.clone()),
            None => DrawLayout::Reflected(text.style.clone()),
        };
        Self {
            layout,
            color: text.style.color,
        }
    }

    pub(crate) fn project(
        self,
        world: &mut World,
        existing: Option<Entity>,
        projection: Projection,
    ) -> CanvasResult<Entity> {
        let Projection {
            parent,
            layer,
            local,
            document,
            density,
            cull,
        } = projection;
        let layout = match self.layout {
            DrawLayout::Measured(layout) => layout,
            // Reflect deserialization reconstructs the model fields without the derived layout.
            DrawLayout::Reflected(style) => super::measure(world, style)?
                .glyphs
                .ok_or_else(|| missing(crate::CanvasPart::Text))?,
        };
        let precision = Precision::new(layout.max_font_size, document, density);
        let meshes = if existing
            .and_then(|entity| world.get::<TextProxy>(entity))
            .is_some_and(|proxy| {
                Arc::ptr_eq(&proxy.layout, &layout) && proxy.precision == precision
            }) {
            None
        } else {
            Some(glyph_meshes(world, &layout, precision)?)
        };
        let proxy = if let Some(entity) = existing {
            if world.get::<ChildOf>(entity).map(ChildOf::parent) != Some(parent) {
                world.entity_mut(entity).insert(ChildOf(parent));
            }
            world
                .get_mut::<Transform>(entity)
                .ok_or_else(|| missing(crate::CanvasPart::ProxyTransform))?
                .set_if_neq(local);
            let proxy = world
                .get::<TextProxy>(entity)
                .ok_or_else(|| missing(crate::CanvasPart::Text))?;
            let material = proxy.material.clone();
            let color_changed = proxy.color != self.color;
            if color_changed {
                world
                    .resource_mut::<Assets<ColorMaterial>>()
                    .get_mut(&material)
                    .ok_or_else(|| missing(crate::CanvasPart::Material))?
                    .color = self.color;
            }
            if meshes.is_some() || color_changed {
                world.entity_mut(entity).insert(TextProxy {
                    layout,
                    precision,
                    material: material.clone(),
                    color: self.color,
                });
            }
            (entity, material)
        } else {
            let material = world
                .resource_mut::<Assets<ColorMaterial>>()
                .add(ColorMaterial {
                    color: self.color,
                    alpha_mode: bevy::sprite_render::AlphaMode2d::Blend,
                    ..default()
                });
            let entity = world
                .spawn((
                    Name::new("CanvasTextProxy"),
                    ChildOf(parent),
                    local,
                    Visibility::Inherited,
                    TextProxy {
                        layout,
                        precision,
                        material: material.clone(),
                        color: self.color,
                    },
                ))
                .id();
            (entity, material)
        };
        if let Some(meshes) = meshes {
            place_glyphs(world, proxy.0, layer, &proxy.1, meshes, cull);
        }
        Ok(proxy.0)
    }
}

fn glyph_meshes(
    world: &mut World,
    layout: &GlyphLayout,
    precision: Precision,
) -> CanvasResult<Vec<(Handle<Mesh>, Transform)>> {
    world.resource_scope(|world, mut cache: Mut<GlyphMeshCache>| {
        let mut assets = world.resource_mut::<Assets<Mesh>>();
        let mut meshes = Vec::new();
        for run in &layout.runs {
            let scale = Vec2::splat(run.font_size / MESH_EM).extend(1.0);
            for glyph in &run.glyphs {
                if let Some(mesh) = cache.mesh(&mut assets, run, glyph.id, precision)? {
                    let transform =
                        Transform::from_translation(glyph.origin.extend(0.0)).with_scale(scale);
                    meshes.push((mesh, transform));
                }
            }
        }
        Ok(meshes)
    })
}

pub(crate) struct Projection {
    pub parent: Entity,
    pub layer: usize,
    pub local: Transform,
    pub document: Affine3A,
    pub density: DVec2,
    pub cull: bool,
}

#[derive(Component)]
struct TextProxy {
    layout: Arc<GlyphLayout>,
    precision: Precision,
    material: Handle<ColorMaterial>,
    color: Color,
}

fn place_glyphs(
    world: &mut World,
    root: Entity,
    layer: usize,
    material: &Handle<ColorMaterial>,
    meshes: Vec<(Handle<Mesh>, Transform)>,
    cull: bool,
) {
    let existing: Vec<_> = world
        .get::<Children>(root)
        .into_iter()
        .flat_map(|children| children.iter())
        .collect();
    for child in existing.iter().skip(meshes.len()) {
        world.despawn(*child);
    }
    for (index, (mesh, transform)) in meshes.into_iter().enumerate() {
        if let Some(&entity) = existing.get(index) {
            world.entity_mut(entity).insert((Mesh2d(mesh), transform));
        } else {
            let mut entity = world.spawn((
                Name::new("CanvasGlyph"),
                ChildOf(root),
                Mesh2d(mesh),
                MeshMaterial2d(material.clone()),
                transform,
                RenderLayers::none().with(layer),
                Pickable::IGNORE,
            ));
            // PNG prepares pipelines even when a glyph first appears in a later tile.
            if !cull {
                entity.insert(bevy::camera::visibility::NoFrustumCulling);
            }
        }
    }
}

#[cfg(feature = "png_export")]
pub(crate) fn assets(
    world: &World,
    proxy: Entity,
) -> (
    Option<AssetId<ColorMaterial>>,
    impl Iterator<Item = AssetId<Mesh>> + '_,
) {
    let material = world.get::<TextProxy>(proxy).map(|text| text.material.id());
    let meshes = world
        .get::<Children>(proxy)
        .into_iter()
        .flat_map(|children| children.iter())
        .filter_map(|entity| world.get::<Mesh2d>(entity).map(|mesh| mesh.0.id()));
    (material, meshes)
}
