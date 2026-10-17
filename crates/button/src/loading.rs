use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui_widgets::Button;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_cursor::internal::LoadingCursor;
use bevy_widgetry_internal_log::widgetry_error;

use crate::WidgetryButton;

#[derive(Component, Default, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub(crate) struct LoadingState(pub(crate) bool);

#[derive(EntityEvent)]
pub struct WidgetryButtonLoadingChanged {
    pub entity: Entity,
    pub loading: bool,
}

#[derive(Component, Default, Clone)]
pub(crate) struct ButtonContent {
    hidden: Option<(Visibility, UiTransform)>,
}

#[derive(Component, Default, Clone)]
pub(crate) struct Spinner;

#[derive(Component, Default, Clone)]
pub(crate) struct SpinnerArc;

#[derive(Component, Default, Clone)]
pub(crate) struct SpinnerTrack;

#[derive(Resource, Default)]
struct SpinnerImages(Option<(Handle<Image>, Handle<Image>)>);

struct Parts {
    content: Entity,
    spinner: Entity,
}

fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "Button Loading 目标不存在、类型不匹配或内部结构缺失"
    );
    BevyError::error("Button Loading 目标不存在、类型不匹配或内部结构缺失")
}

fn parts(world: &World, entity: Entity) -> Result<Parts, BevyError> {
    if world.get::<WidgetryButton>(entity).is_none()
        || world.get::<Button>(entity).is_none()
        || world.get::<LoadingState>(entity).is_none()
    {
        return Err(invalid_target(entity));
    }
    let children = world
        .get::<Children>(entity)
        .ok_or_else(|| invalid_target(entity))?;
    let content = children
        .iter()
        .find(|&child| {
            world.get::<ButtonContent>(child).is_some()
                && world.get::<Visibility>(child).is_some()
                && world.get::<UiTransform>(child).is_some()
        })
        .ok_or_else(|| invalid_target(entity))?;
    let spinner = children
        .iter()
        .find(|&child| {
            world.get::<Spinner>(child).is_some() && world.get::<Visibility>(child).is_some()
        })
        .ok_or_else(|| invalid_target(entity))?;
    Ok(Parts { content, spinner })
}

impl WidgetryButton {
    pub fn is_loading(world: &World, entity: Entity) -> Result<bool, BevyError> {
        parts(world, entity)?;
        world
            .get::<LoadingState>(entity)
            .map(|state| state.0)
            .ok_or_else(|| invalid_target(entity))
    }

    pub fn set_loading_in_world(
        world: &mut World,
        entity: Entity,
        loading: bool,
    ) -> Result<bool, BevyError> {
        let parts = parts(world, entity)?;
        if Self::is_loading(world, entity)? == loading {
            return Ok(false);
        }
        let presentation = if loading {
            let visibility = *world
                .get::<Visibility>(parts.content)
                .ok_or_else(|| invalid_target(entity))?;
            let transform = *world
                .get::<UiTransform>(parts.content)
                .ok_or_else(|| invalid_target(entity))?;
            world
                .get_mut::<ButtonContent>(parts.content)
                .ok_or_else(|| invalid_target(entity))?
                .hidden = Some((visibility, transform));
            // Explicit Visible descendants bypass visibility inheritance.
            // Zero scale suppresses their geometry while preserving layout and their own Visibility.
            (
                Visibility::Hidden,
                UiTransform {
                    scale: Vec2::ZERO,
                    ..transform
                },
            )
        } else {
            world
                .get_mut::<ButtonContent>(parts.content)
                .ok_or_else(|| invalid_target(entity))?
                .hidden
                .take()
                .ok_or_else(|| invalid_target(entity))?
        };
        world.entity_mut(entity).insert(LoadingState(loading));
        world.entity_mut(parts.content).insert(presentation);
        world.entity_mut(parts.spinner).insert(if loading {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if loading {
            world.entity_mut(entity).insert(LoadingCursor);
        } else {
            world.entity_mut(entity).remove::<LoadingCursor>();
        }
        bevy_widgetry_core::disabled::set_intrinsic_disabled_for::<LoadingState>(
            world, entity, loading,
        );
        world.trigger(WidgetryButtonLoadingChanged { entity, loading });
        Ok(true)
    }

    pub fn set_loading(commands: &mut Commands, entity: Entity, loading: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_loading_in_world(world, entity, loading).map(|_| ())
        });
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<SpinnerImages>()
        .register_type::<LoadingState>()
        .add_systems(
            PostUpdate,
            (update_spinner_images, animate_spinners)
                .chain()
                .in_set(WidgetryUiSystems::ContentColors),
        );
}

fn mask(arc: bool) -> Image {
    const SIZE: u32 = 96;
    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - Vec2::splat(SIZE as f32 / 2.0);
            let ring = (4.5 - (point.length() - 40.0).abs()).clamp(0.0, 1.0);
            let angle = point.y.atan2(point.x).abs();
            let coverage = if arc {
                ((angle - std::f32::consts::PI / 6.0) * 40.0 + 0.5).clamp(0.0, 1.0)
            } else {
                1.0
            };
            data.extend_from_slice(&[255, 255, 255, (ring * coverage * 255.0).round() as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn update_spinner_images(
    mut masks: ResMut<SpinnerImages>,
    images: Option<ResMut<Assets<Image>>>,
    roots: Query<&ResolvedForeground, With<WidgetryButton>>,
    spinners: Query<&ChildOf, With<Spinner>>,
    mut nodes: Query<
        (&ChildOf, &mut ImageNode, Has<SpinnerArc>),
        Or<(With<SpinnerArc>, With<SpinnerTrack>)>,
    >,
) {
    if masks.0.is_none() {
        let Some(mut images) = images else {
            return;
        };
        masks.0 = Some((images.add(mask(false)), images.add(mask(true))));
    }
    let Some((track, arc)) = &masks.0 else {
        return;
    };
    for (parent, mut image, is_arc) in &mut nodes {
        let Ok(owner) = spinners.get(parent.parent()) else {
            continue;
        };
        let Ok(foreground) = roots.get(owner.parent()) else {
            continue;
        };
        let color = if is_arc {
            foreground.0
        } else {
            foreground.0.with_alpha(foreground.0.alpha() * 0.22)
        };
        let handle = if is_arc { arc } else { track };
        if image.image != *handle {
            image.image = handle.clone();
        }
        if image.color != color {
            image.color = color;
        }
    }
}

fn animate_spinners(
    time: Option<Res<Time>>,
    roots: Query<&Children, (With<WidgetryButton>, With<LoadingCursor>)>,
    spinners: Query<&Children, With<Spinner>>,
    mut arcs: Query<&mut UiTransform, With<SpinnerArc>>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    let Some(time) = time else {
        return;
    };
    let step = Rot2::radians(time.delta_secs() * std::f32::consts::TAU / 1.2);
    for children in &roots {
        for child in children {
            if let Ok(images) = spinners.get(*child) {
                for image in images {
                    if let Ok(mut transform) = arcs.get_mut(*image) {
                        transform.rotation *= step;
                    }
                }
            }
        }
    }
    if !roots.is_empty() {
        redraw.write(bevy::window::RequestRedraw);
    }
}
