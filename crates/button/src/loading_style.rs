const RGBA_CHANNELS: u32 = 4;
const OPAQUE_CHANNEL: u8 = 255;
const RING_HALF_WIDTH: f32 = 4.5;
const RING_RADIUS: f32 = 40.0;
const ARC_GAP_HALF_ANGLE: f32 = std::f32::consts::PI / 6.0;
const ARC_EDGE_SHARPNESS: f32 = 40.0;
const TRACK_ALPHA_FACTOR: f32 = 0.22;
const SPIN_PERIOD_SECONDS: f32 = 1.2;

use crate::{ButtonError, ButtonFailure, WidgetryButton, WidgetryButtonLoading, error::failure};
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::ui::WidgetryUiSystems;

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

fn missing(root: Entity, part: &'static str) -> ButtonFailure {
    failure(
        ButtonError::MissingPart { root, part },
        "button.loading_style",
    )
}

fn parts(world: &World, entity: Entity) -> Result<Parts, ButtonFailure> {
    let children = world
        .get::<Children>(entity)
        .ok_or_else(|| missing(entity, "content"))?;
    let content = children
        .iter()
        .find(|&child| world.get::<ButtonContent>(child).is_some())
        .ok_or_else(|| missing(entity, "content"))?;
    if world.get::<Visibility>(content).is_none() {
        return Err(missing(entity, "content.Visibility"));
    }
    if world.get::<UiTransform>(content).is_none() {
        return Err(missing(entity, "content.UiTransform"));
    }
    let spinner = children
        .iter()
        .find(|&child| world.get::<Spinner>(child).is_some())
        .ok_or_else(|| missing(entity, "spinner"))?;
    if world.get::<Visibility>(spinner).is_none() {
        return Err(missing(entity, "spinner.Visibility"));
    }
    Ok(Parts { content, spinner })
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<SpinnerImages>()
        .add_systems(
            PostUpdate,
            sync_presentation.in_set(WidgetryUiSystems::Materialize),
        )
        .add_systems(
            PostUpdate,
            (update_spinner_images, animate_spinners)
                .chain()
                .in_set(WidgetryUiSystems::ContentColors),
        );
}

fn sync_presentation(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<(Entity, &WidgetryButtonLoading), (
            With<WidgetryButton>,
            Or<(Changed<WidgetryButtonLoading>, Added<WidgetryButton>)>,
        )>()
        .iter(world)
        .map(|(entity, state)| (entity, state.is_active()))
        .collect::<Vec<_>>();
    let mut errors = bevy_widgetry_diagnostics::bevy::DiagnosticBatch::default();
    for (root, active) in roots {
        if let Err(error) = present(world, root, active) {
            errors.push(error);
        }
    }
    match errors.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn present(world: &mut World, root: Entity, active: bool) -> Result<(), ButtonFailure> {
    {
        let mut entity = world
            .get_entity_mut(root)
            .map_err(|_| failure(ButtonError::InvalidTarget { root }, "button.loading_style"))?;
        if active {
            entity.insert(bevy_widgetry_cursor::WidgetryCursorLoading);
        } else {
            entity.remove::<bevy_widgetry_cursor::WidgetryCursorLoading>();
        }
    }
    let parts = parts(world, root)?;
    let presentation = if active {
        let visibility = *world
            .get::<Visibility>(parts.content)
            .ok_or_else(|| missing(root, "content.Visibility"))?;
        let transform = *world
            .get::<UiTransform>(parts.content)
            .ok_or_else(|| missing(root, "content.UiTransform"))?;
        let mut content = world
            .get_mut::<ButtonContent>(parts.content)
            .ok_or_else(|| missing(root, "content"))?;
        if content.hidden.is_none() {
            content.hidden = Some((visibility, transform));
        }
        // Explicit Visible descendants bypass inherited visibility.
        // Suppress their geometry without changing their layout or Visibility.
        Some((
            Visibility::Hidden,
            UiTransform {
                scale: Vec2::ZERO,
                ..transform
            },
        ))
    } else {
        world
            .get_mut::<ButtonContent>(parts.content)
            .ok_or_else(|| missing(root, "content"))?
            .hidden
            .take()
    };
    if let Some(presentation) = presentation {
        world.entity_mut(parts.content).insert(presentation);
    }
    world.entity_mut(parts.spinner).insert(if active {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    Ok(())
}

fn mask(arc: bool) -> Image {
    const SIZE: u32 = 96;
    let mut data = Vec::with_capacity((SIZE * SIZE * RGBA_CHANNELS) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - Vec2::splat(SIZE as f32 / 2.0);
            let ring = (RING_HALF_WIDTH - (point.length() - RING_RADIUS).abs()).clamp(0.0, 1.0);
            let angle = point.y.atan2(point.x).abs();
            let coverage = if arc {
                ((angle - ARC_GAP_HALF_ANGLE) * ARC_EDGE_SHARPNESS + 0.5).clamp(0.0, 1.0)
            } else {
                1.0
            };
            data.extend_from_slice(&[
                OPAQUE_CHANNEL,
                OPAQUE_CHANNEL,
                OPAQUE_CHANNEL,
                (ring * coverage * f32::from(OPAQUE_CHANNEL)).round() as u8,
            ]);
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
    if nodes.is_empty() {
        return;
    }
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
            foreground
                .0
                .with_alpha(foreground.0.alpha() * TRACK_ALPHA_FACTOR)
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
    roots: Query<(&Children, &WidgetryButtonLoading), With<WidgetryButton>>,
    spinners: Query<&Children, With<Spinner>>,
    mut arcs: Query<&mut UiTransform, With<SpinnerArc>>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    let Some(time) = time else {
        return;
    };
    let step = Rot2::radians(time.delta_secs() * std::f32::consts::TAU / SPIN_PERIOD_SECONDS);
    let mut active = false;
    for (children, loading) in &roots {
        if !loading.is_active() {
            continue;
        }
        active = true;
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
    if active {
        redraw.write(bevy::window::RequestRedraw);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::WidgetryButtonPlugin;
    use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

    #[test]
    fn damaged_default_visuals_do_not_reject_loading_and_report_through_the_style_boundary() {
        for missing in [
            "content",
            "spinner",
            "content.Visibility",
            "content.UiTransform",
            "spinner.Visibility",
        ] {
            let mut app = scene_app();
            app.add_plugins(WidgetryButtonPlugin);
            app.set_error_handler(ErrorCapture::handler());
            let root = app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryButton })
                .unwrap()
                .id();
            let parts = parts(app.world(), root).unwrap();
            match missing {
                "content" => {
                    app.world_mut().despawn(parts.content);
                }
                "spinner" => {
                    app.world_mut().despawn(parts.spinner);
                }
                "content.Visibility" => {
                    app.world_mut()
                        .entity_mut(parts.content)
                        .remove::<Visibility>();
                }
                "content.UiTransform" => {
                    app.world_mut()
                        .entity_mut(parts.content)
                        .remove::<UiTransform>();
                }
                _ => {
                    app.world_mut()
                        .entity_mut(parts.spinner)
                        .remove::<Visibility>();
                }
            }
            assert!(WidgetryButtonLoading::set_in_world(app.world_mut(), root, true).unwrap());
            assert!(
                WidgetryButtonLoading::get(app.world(), root)
                    .unwrap()
                    .is_active()
            );
            let errors = ErrorCapture::default();
            errors.run(|| app.update());
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            assert!(
                app.world()
                    .get::<bevy_widgetry_cursor::WidgetryCursorLoading>(root)
                    .is_some()
            );
            let error = records[0]
                .error
                .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                .unwrap();
            assert!(
                matches!(error.diagnostics().next().unwrap().source().unwrap().downcast_ref::<ButtonError>().unwrap(), ButtonError::MissingPart { root: target, part } if *target == root && *part == missing)
            );
            WidgetryButtonLoading::set_in_world(app.world_mut(), root, false).unwrap();
            errors.run(|| app.update());
            assert!(
                app.world()
                    .get::<bevy_widgetry_cursor::WidgetryCursorLoading>(root)
                    .is_none()
            );
            assert_eq!(errors.take_records().len(), 1);
        }
    }
}
