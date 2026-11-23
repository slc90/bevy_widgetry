use crate::error::{CanvasResult, missing};
use crate::{CanvasFill, CanvasGradientStops, CanvasPart};
use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};
use bevy_widgetry_asset::BuiltinShader;

#[derive(Clone, Copy, PartialEq, ShaderType)]
struct GradientUniform {
    origin: Vec2,
    axis: Vec2,
    size: f32,
    radial: u32,
    count: u32,
    offsets: [Vec4; 2],
    colors: [Vec4; CanvasGradientStops::MAX],
}

#[derive(Clone, Asset, TypePath, AsBindGroup)]
pub(crate) struct GradientMaterial {
    #[uniform(0)]
    gradient: GradientUniform,
}

impl Material2d for GradientMaterial {
    fn vertex_shader() -> ShaderRef {
        BuiltinShader::CanvasGradient.path().into()
    }

    fn fragment_shader() -> ShaderRef {
        BuiltinShader::CanvasGradient.path().into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

impl GradientMaterial {
    fn from_fill(fill: Option<CanvasFill>) -> Option<Self> {
        let (origin, axis, size, radial, stops) = match fill? {
            CanvasFill::Solid(_) => return None,
            CanvasFill::LinearGradient { start, end, stops } => {
                let axis = end.as_dvec2() - start.as_dvec2();
                let length = axis.length();
                (start, (axis / length).as_vec2(), length as f32, 0, stops)
            }
            CanvasFill::RadialGradient {
                center,
                radius,
                stops,
            } => (center, Vec2::ZERO, radius, 1, stops),
        };
        let mut gradient = GradientUniform {
            origin,
            axis,
            size,
            radial,
            count: stops.as_slice().len() as u32,
            offsets: [Vec4::ZERO; 2],
            colors: [Vec4::ZERO; CanvasGradientStops::MAX],
        };
        for (index, stop) in stops.as_slice().iter().enumerate() {
            gradient.offsets[index / 4][index % 4] = stop.offset;
            gradient.colors[index] = Vec4::from_array(stop.color.to_linear().to_f32_array());
        }
        Some(Self { gradient })
    }
}

#[derive(Clone)]
pub(crate) enum PaintMaterial {
    Solid(Handle<ColorMaterial>),
    Gradient(Handle<GradientMaterial>),
}

pub(crate) fn blended(color: Color) -> ColorMaterial {
    ColorMaterial { color, ..default() }
}

impl PaintMaterial {
    pub(crate) fn add(
        world: &mut World,
        fill: Option<CanvasFill>,
        solid: impl FnOnce(Color) -> ColorMaterial,
    ) -> Self {
        if let Some(material) = GradientMaterial::from_fill(fill) {
            Self::Gradient(
                world
                    .resource_mut::<Assets<GradientMaterial>>()
                    .add(material),
            )
        } else {
            let color = fill.map_or(Color::NONE, CanvasFill::first_color);
            Self::Solid(
                world
                    .resource_mut::<Assets<ColorMaterial>>()
                    .add(solid(color)),
            )
        }
    }

    pub(crate) fn attach(&self, world: &mut World, entity: Entity) {
        match self {
            Self::Solid(handle) => {
                world
                    .entity_mut(entity)
                    .insert(MeshMaterial2d(handle.clone()));
            }
            Self::Gradient(handle) => {
                world
                    .entity_mut(entity)
                    .insert(MeshMaterial2d(handle.clone()));
            }
        }
    }

    #[cfg(feature = "png_export")]
    pub(crate) fn id(&self) -> bevy::asset::UntypedAssetId {
        match self {
            Self::Solid(handle) => handle.id().untyped(),
            Self::Gradient(handle) => handle.id().untyped(),
        }
    }

    pub(crate) fn update(
        &mut self,
        world: &mut World,
        entity: Entity,
        fill: Option<CanvasFill>,
        solid: impl FnOnce(Color) -> ColorMaterial,
    ) -> CanvasResult {
        match (&*self, GradientMaterial::from_fill(fill)) {
            (Self::Solid(handle), None) => {
                let color = fill.map_or(Color::NONE, CanvasFill::first_color);
                let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
                let mut material = materials
                    .get_mut(handle)
                    .ok_or_else(|| missing(CanvasPart::Material))?;
                *material = solid(color);
            }
            (Self::Gradient(handle), Some(material)) => {
                let mut materials = world.resource_mut::<Assets<GradientMaterial>>();
                let mut previous = materials
                    .get_mut(handle)
                    .ok_or_else(|| missing(CanvasPart::Material))?;
                *previous = material;
            }
            _ => {
                let mut entity = world.entity_mut(entity);
                match self {
                    Self::Solid(_) => {
                        entity.remove::<MeshMaterial2d<ColorMaterial>>();
                    }
                    Self::Gradient(_) => {
                        entity.remove::<MeshMaterial2d<GradientMaterial>>();
                    }
                }
                let entity = entity.id();
                self.release(world);
                *self = Self::add(world, fill, solid);
                self.attach(world, entity);
            }
        }
        Ok(())
    }

    pub(crate) fn release(&self, world: &mut World) {
        match self {
            Self::Solid(handle) => {
                world
                    .resource_mut::<Assets<ColorMaterial>>()
                    .remove(handle.id());
            }
            Self::Gradient(handle) => {
                world
                    .resource_mut::<Assets<GradientMaterial>>()
                    .remove(handle.id());
            }
        }
    }

    pub(crate) fn release_from(
        &self,
        solids: &mut Assets<ColorMaterial>,
        gradients: &mut Assets<GradientMaterial>,
    ) {
        match self {
            Self::Solid(handle) => {
                solids.remove(handle.id());
            }
            Self::Gradient(handle) => {
                gradients.remove(handle.id());
            }
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.add_plugins(Material2dPlugin::<GradientMaterial>::default());
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        CanvasColorStop, CanvasDocument, CanvasMode, CanvasPaint, CanvasRectangle, CanvasView,
    };
    use bevy::{picking::pointer::*, ui::UiGlobalTransform};
    use bevy_widgetry_test_utils::{
        ErrorCapture, picking_app, queue_pointer, spawn_picking_camera,
    };

    fn frame(app: &mut App) {
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
    }

    fn asset(
        world: &World,
        view: Entity,
        shape: Entity,
    ) -> (AssetId<GradientMaterial>, GradientUniform) {
        let proxy = world.get::<super::super::Renderer>(view).unwrap().proxies[&shape];
        let fill = world.get::<super::super::ProxyParts>(proxy).unwrap().fill;
        let material = &world
            .get::<MeshMaterial2d<GradientMaterial>>(fill)
            .unwrap()
            .0;
        let uniform = world
            .resource::<Assets<GradientMaterial>>()
            .get(material)
            .unwrap()
            .gradient;
        (material.id(), uniform)
    }

    #[test]
    fn views_own_independent_gradient_materials_through_preview_commit_and_cleanup() {
        let mut app = picking_app();
        app.add_plugins((TransformPlugin, crate::CanvasPlugin));
        app.set_error_handler(ErrorCapture::handler());
        let window_size = UVec2::new(480, 240);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(window_size.x, window_size.y),
                    ..default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();
        let camera = spawn_picking_camera(&mut app, window, window_size, 1.0);
        let document = app.world_mut().spawn(CanvasDocument).id();
        let shape = CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            CanvasRectangle {
                size: Vec2::splat(48.0),
                corner_radius: 0.0,
            },
            Transform::IDENTITY,
            CanvasPaint {
                fill: Some(CanvasFill::LinearGradient {
                    start: Vec2::new(-12.0, 0.0),
                    end: Vec2::new(12.0, 0.0),
                    stops: CanvasGradientStops::new(&[
                        CanvasColorStop {
                            offset: 0.0,
                            color: Color::BLACK,
                        },
                        CanvasColorStop {
                            offset: 1.0,
                            color: Color::WHITE,
                        },
                    ])
                    .unwrap(),
                }),
                ..default()
            },
        )
        .unwrap();
        let [main, secondary] = [20.0, 220.0].map(|left| {
            app.world_mut().spawn_scene(bsn! {
                @CanvasView { @document:{Some(document)} }
                Node { position_type:PositionType::Absolute, left:px(left), top:px(20), width:px(160), height:px(140) }
                template(move |_| Ok(UiTargetCamera(camera)))
            }).unwrap().id()
        });
        CanvasView::set_selection_in_world(app.world_mut(), main, [shape]).unwrap();
        CanvasView::set_mode_in_world(app.world_mut(), main, CanvasMode::GradientEdit).unwrap();
        frame(&mut app);
        frame(&mut app);
        let (main_id, initial) = asset(app.world(), main, shape);
        let (secondary_id, other) = asset(app.world(), secondary, shape);
        assert_ne!(main_id, secondary_id);
        assert!(initial == other);
        let target = bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap();
        let center = app
            .world()
            .get::<UiGlobalTransform>(main)
            .unwrap()
            .translation;
        let mut at = Location {
            target,
            position: center + Vec2::X * 12.0,
        };
        let pointer = PointerId::Mouse;
        queue_pointer(
            &mut app,
            pointer,
            at.clone(),
            PointerAction::Move { delta: Vec2::ONE },
        );
        frame(&mut app);
        queue_pointer(
            &mut app,
            pointer,
            at.clone(),
            PointerAction::Press(PointerButton::Primary),
        );
        frame(&mut app);
        let delta = Vec2::new(4.0, 15.0);
        at.position += delta;
        queue_pointer(&mut app, pointer, at.clone(), PointerAction::Move { delta });
        frame(&mut app);
        let (preview_id, preview) = asset(app.world(), main, shape);
        assert_eq!(preview_id, main_id);
        assert!(preview != initial);
        assert!(asset(app.world(), secondary, shape).1 == initial);
        queue_pointer(
            &mut app,
            pointer,
            at,
            PointerAction::Release(PointerButton::Primary),
        );
        frame(&mut app);
        assert!(asset(app.world(), secondary, shape).1 == preview);
        app.world_mut().despawn(main);
        frame(&mut app);
        let materials = app.world().resource::<Assets<GradientMaterial>>();
        assert!(materials.get(main_id).is_none());
        assert!(materials.get(secondary_id).unwrap().gradient == preview);
    }
}
