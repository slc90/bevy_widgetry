use crate::{
    ButtonError, ButtonFailure, WidgetryButton,
    error::{core, failure},
};
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui_widgets::Button;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_cursor::internal::LoadingCursor;

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

fn invalid_target(root: Entity) -> ButtonFailure {
    failure(ButtonError::InvalidTarget { root }, "button.loading")
}

fn missing(root: Entity, part: &'static str) -> ButtonFailure {
    failure(ButtonError::MissingPart { root, part }, "button.loading")
}

fn parts(world: &World, entity: Entity) -> Result<Parts, ButtonFailure> {
    if world.get::<WidgetryButton>(entity).is_none() {
        return Err(invalid_target(entity));
    }
    if world.get::<Button>(entity).is_none() {
        return Err(missing(entity, "Button"));
    }
    if world.get::<LoadingState>(entity).is_none() {
        return Err(missing(entity, "LoadingState"));
    }
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

impl WidgetryButton {
    pub fn is_loading(world: &World, entity: Entity) -> Result<bool, ButtonFailure> {
        parts(world, entity)?;
        world
            .get::<LoadingState>(entity)
            .map(|state| state.0)
            .ok_or_else(|| missing(entity, "LoadingState"))
    }

    pub fn set_loading_in_world(
        world: &mut World,
        entity: Entity,
        loading: bool,
    ) -> Result<bool, ButtonFailure> {
        let parts = parts(world, entity)?;
        if Self::is_loading(world, entity)? == loading {
            return Ok(false);
        }
        bevy_widgetry_core::disabled::validate_intrinsic_disabled_target(world, entity)
            .map_err(|error| core(error, entity, "button.set_loading.disabled"))?;
        let presentation = if loading {
            let visibility = *world
                .get::<Visibility>(parts.content)
                .ok_or_else(|| missing(entity, "content.Visibility"))?;
            let transform = *world
                .get::<UiTransform>(parts.content)
                .ok_or_else(|| missing(entity, "content.UiTransform"))?;
            world
                .get_mut::<ButtonContent>(parts.content)
                .ok_or_else(|| missing(entity, "content"))?
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
                .ok_or_else(|| missing(entity, "content"))?
                .hidden
                .take()
                .ok_or_else(|| missing(entity, "content.presentation"))?
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
        )
        .map_err(|error| core(error, entity, "button.set_loading.disabled"))?;
        world.trigger(WidgetryButtonLoadingChanged { entity, loading });
        Ok(true)
    }

    pub fn set_loading(commands: &mut Commands, entity: Entity, loading: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_loading_in_world(world, entity, loading)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
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

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{ButtonError, WidgetryButtonPlugin};
    use bevy_widgetry_core::diagnostics::CoreError;

    #[test]
    fn missing_loading_parts_are_classified_before_state_or_notifications_change() {
        for missing in [
            "Button",
            "LoadingState",
            "content",
            "spinner",
            "content.Visibility",
            "content.UiTransform",
        ] {
            let mut app = bevy_widgetry_test_utils::scene_app();
            app.add_plugins(WidgetryButtonPlugin);
            let root = app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryButton })
                .unwrap()
                .id();
            let parts = parts(app.world(), root).unwrap();
            match missing {
                "Button" => {
                    app.world_mut().entity_mut(root).remove::<Button>();
                }
                "LoadingState" => {
                    app.world_mut().entity_mut(root).remove::<LoadingState>();
                }
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
                _ => {
                    app.world_mut()
                        .entity_mut(parts.content)
                        .remove::<UiTransform>();
                }
            }
            #[derive(Resource, Default)]
            struct Changes(usize);
            app.init_resource::<Changes>().add_observer(
                |_: On<WidgetryButtonLoadingChanged>, mut changes: ResMut<Changes>| changes.0 += 1,
            );
            let failure =
                WidgetryButton::set_loading_in_world(app.world_mut(), root, true).unwrap_err();
            assert!(
                matches!(failure.kind(), ButtonError::MissingPart { root: target, part } if *target == root && *part == missing)
            );
            assert!(
                !app.world()
                    .get::<LoadingState>(root)
                    .is_some_and(|state| state.0)
            );
            assert!(app.world().get::<LoadingCursor>(root).is_none());
            assert_eq!(app.world().resource::<Changes>().0, 0);
        }
    }

    #[test]
    fn unavailable_disabled_runtime_is_preserved_before_loading_presentation_is_changed() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<bevy::scene::ScenePatch>();
        let world = app.world_mut();
        let root = world.spawn_scene(bsn! { @WidgetryButton }).unwrap().id();
        let parts = parts(world, root).unwrap();
        let initial = *world.get::<Visibility>(parts.content).unwrap();
        assert!(!WidgetryButton::set_loading_in_world(world, root, false).unwrap());
        let failure = WidgetryButton::set_loading_in_world(world, root, true).unwrap_err();
        assert!(
            matches!(failure.kind(), ButtonError::Core { source: CoreError::MissingResource { entity, resource: "DisabledRuntime" } } if *entity == root)
        );
        assert!(!world.get::<LoadingState>(root).unwrap().0);
        assert!(world.get::<LoadingCursor>(root).is_none());
        assert!(
            world
                .get::<ButtonContent>(parts.content)
                .unwrap()
                .hidden
                .is_none()
        );
        assert_eq!(*world.get::<Visibility>(parts.content).unwrap(), initial);
        assert_eq!(
            *world.get::<Visibility>(parts.spinner).unwrap(),
            Visibility::Hidden
        );
    }
}
