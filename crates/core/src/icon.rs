mod colors;
pub use colors::{WidgetryIconColorOverrides, WidgetryIconStateColorOverrides};
mod error;
mod svg;
use crate::diagnostics::FailureState;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, WidgetryEcsError, into_bevy_error},
};
use error::failure;
pub use error::{IconError, IconFailure};
use std::sync::Arc;

use crate::{
    disabled::WidgetryEffectiveDisabled,
    foreground::InheritedForeground,
    ui::{WidgetryUiPlugin, WidgetryUiSystems},
};
use bevy::ui::ComputedUiRenderTargetInfo;
use bevy::window::RequestRedraw;
use bevy::{asset::AssetPath, platform::collections::HashMap, prelude::*};
use bevy_widgetry_internal_log::widgetry_debug;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryIconProps)]
#[require(Node, IconRasterState, colors::ColorState)]
pub struct WidgetryIcon {
    svg: Handle<svg::SvgAsset>,
    max_size: Option<UVec2>,
}

/// Prepare the image subtree after glyph shape projection so UI target information can propagate into it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetryIconMaterialize;

#[derive(Clone, Debug, Default)]
pub struct WidgetryIconProps {
    pub path: AssetPath<'static>,
    pub max_size: Option<UVec2>,
    pub colors: WidgetryIconColorOverrides,
}

#[derive(Component)]
struct IconMaterialized {
    image_entity: Entity,
    svg_asset_id: AssetId<svg::SvgAsset>,
    raster_spec: IconRasterSpec,
}

struct ResolvedIconImage {
    handle: Handle<Image>,
    logical_size: Vec2,
    raster_spec: IconRasterSpec,
}

#[derive(Component)]
struct IconPendingUpdate;

#[derive(Component)]
struct IconImage;

#[derive(Component, Default)]
struct IconRasterState {
    failure: FailureState<IconFailureKey>,
    image: FailureState<Entity>,
}

enum IconFailureKey {
    Raster {
        image: IconImageCacheKey,
        width: u32,
        height: u32,
        scale: u32,
    },
    Load {
        asset: AssetId<svg::SvgAsset>,
        error: Arc<bevy::asset::AssetLoadError>,
    },
}

impl PartialEq for IconFailureKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Raster {
                    image: a,
                    width: aw,
                    height: ah,
                    scale: ascale,
                },
                Self::Raster {
                    image: b,
                    width: bw,
                    height: bh,
                    scale: bscale,
                },
            ) => (a, aw, ah, ascale) == (b, bw, bh, bscale),
            (
                Self::Load {
                    asset: a,
                    error: ae,
                },
                Self::Load {
                    asset: b,
                    error: be,
                },
            ) => a == b && Arc::ptr_eq(ae, be),
            _ => false,
        }
    }
}
impl Eq for IconFailureKey {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct IconImageCacheKey {
    svg_asset_id: AssetId<svg::SvgAsset>,
    raster_spec: IconRasterSpec,
}

#[derive(Resource, Default)]
struct IconImageCache {
    images: HashMap<IconImageCacheKey, Handle<Image>>,
}

pub struct WidgetryIconPlugin;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum IconRasterSpec {
    Intrinsic { scale: u32 },
    MaxSize { width: u32, height: u32 },
}

fn resolve_icon_image_handle(
    entity: Entity,
    icon: &WidgetryIcon,
    scale: f32,
    diagnostics: &mut IconRasterState,
    svg_assets: &Assets<svg::SvgAsset>,
    server: &AssetServer,
    images: &mut Assets<Image>,
    cache: &mut IconImageCache,
) -> Result<Option<ResolvedIconImage>, WidgetryEcsError> {
    let Some(svg_asset) = svg_assets.get(&icon.svg) else {
        if let bevy::asset::LoadState::Failed(error) = server.load_state(icon.svg.id()) {
            let key = IconFailureKey::Load {
                asset: icon.svg.id(),
                error: Arc::clone(&error),
            };
            diagnostics.failure.failure(key, || {
                if let bevy::asset::AssetLoadError::AssetLoaderError(loader) = error.as_ref()
                    && let Some(managed) = loader.error().downcast_ref::<WidgetryEcsError>()
                    && let Some(diagnostic) = managed.shared_diagnostics().next()
                {
                    return diagnostic;
                }
                Arc::new(failure(
                    IconError::AssetLoad {
                        entity,
                        path: icon.path().cloned().map(Box::new),
                        source: error,
                    },
                    "load icon SVG",
                ))
            });
            if let Some(diagnostic) = diagnostics.failure.active() {
                diagnostic.record_handled(
                    "retain icon projection",
                    "keep the existing image or empty initial state after SVG load failure",
                );
            }
        }
        return Ok(None);
    };

    if icon.max_size.is_some_and(|size| size.x == 0 || size.y == 0) {
        return Ok(None);
    }

    let key = IconImageCacheKey {
        svg_asset_id: icon.svg.id(),
        raster_spec: icon.raster_spec(scale),
    };
    let logical_size = svg_asset.logical_size(icon.max_size);

    if let Some(handle) = cache.images.get(&key) {
        diagnostics.raster_recovered(entity);
        return Ok(Some(ResolvedIconImage {
            handle: handle.clone(),
            logical_size,
            raster_spec: key.raster_spec,
        }));
    }

    let image = match key.raster_spec {
        IconRasterSpec::Intrinsic { scale } => {
            svg_asset.render_intrinsic_to_image(f32::from_bits(scale))
        }
        IconRasterSpec::MaxSize { width, height } => svg_asset.render_to_image(width, height),
    };
    let image = match image {
        Ok(image) => image,
        Err(error) => {
            let failure_key = IconFailureKey::Raster {
                image: key,
                width: error.width,
                height: error.height,
                scale: error.scale.to_bits(),
            };
            return Err(diagnostics.failure.failure(failure_key, || {
                Arc::new(failure(
                    IconError::Rasterize {
                        entity,
                        path: icon.path().cloned().map(Box::new),
                        requested_size: icon.max_size,
                        width: error.width,
                        height: error.height,
                        scale: error.scale,
                    },
                    "rasterize icon",
                ))
            }));
        }
    };
    diagnostics.raster_recovered(entity);

    let handle = images.add(image);

    cache.images.insert(key, handle.clone());

    Ok(Some(ResolvedIconImage {
        handle,
        logical_size,
        raster_spec: key.raster_spec,
    }))
}

fn mark_changed_icons(
    mut commands: Commands,
    icons: Query<
        (
            Entity,
            &WidgetryIcon,
            &IconMaterialized,
            &ComputedUiRenderTargetInfo,
        ),
        Or<(Changed<WidgetryIcon>, Changed<ComputedUiRenderTargetInfo>)>,
    >,
) {
    for (entity, icon, materialized, target) in &icons {
        if icon.svg.id() != materialized.svg_asset_id
            || icon.raster_spec(target.scale_factor()) != materialized.raster_spec
        {
            commands.entity(entity).insert(IconPendingUpdate);
        }
    }
}

fn materialize_icons(
    mut commands: Commands,
    mut icons: Query<
        (
            Entity,
            &WidgetryIcon,
            &ComputedUiRenderTargetInfo,
            &mut Node,
            &colors::ColorState,
            Option<&InheritedForeground>,
            Option<&WidgetryEffectiveDisabled>,
            &mut IconRasterState,
        ),
        Without<IconMaterialized>,
    >,
    svg_assets: Res<Assets<svg::SvgAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<IconImageCache>,
    server: Res<AssetServer>,
    mut redraw: MessageWriter<RequestRedraw>,
    mode: Res<bevy_widgetry_theme::WidgetryThemeMode>,
) -> Result<(), BevyError> {
    let mut failures = DiagnosticBatch::default();
    for (entity, icon, target, mut node, overrides, foreground, disabled, mut diagnostics) in
        &mut icons
    {
        let Some(image_handle) = (match resolve_icon_image_handle(
            entity,
            icon,
            target.scale_factor(),
            &mut diagnostics,
            &svg_assets,
            &server,
            &mut images,
            &mut cache,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                for diagnostic in error.shared_diagnostics() {
                    failures.push_shared(diagnostic);
                }
                continue;
            }
        }) else {
            if server.load_state(icon.svg.id()).is_loading() {
                redraw.write(RequestRedraw);
            }
            continue;
        };

        node.justify_content = JustifyContent::Center;
        node.align_items = AlignItems::Center;

        if let Some(size) = icon.max_size {
            node.width = Val::Px(size.x as f32);
            node.height = Val::Px(size.y as f32);
        }

        let mut image_node = ImageNode::new(image_handle.handle);

        let color = resolve_icon_color(&overrides.0, foreground, disabled, mode.colors());

        image_node.color = color;

        // An icon image covering its parent can intercept widget clicks or title bar drags.
        // Use Pickable::IGNORE so input still reaches the parent.
        let image_entity = commands
            .spawn((
                IconImage,
                image_node,
                Node {
                    width: px(image_handle.logical_size.x),
                    height: px(image_handle.logical_size.y),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();

        commands
            .entity(entity)
            .add_child(image_entity)
            .insert(IconMaterialized {
                image_entity,
                svg_asset_id: icon.svg.id(),
                raster_spec: image_handle.raster_spec,
            });
        // A reactive App can stop updating before the new Image's asset events and render preparation finish.
        // Request a redraw to continue preparation across frames.
        redraw.write(RequestRedraw);
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}

fn update_pending_icons(
    mut commands: Commands,
    icons: Query<
        (
            Entity,
            &WidgetryIcon,
            &ComputedUiRenderTargetInfo,
            &mut IconMaterialized,
            &mut IconRasterState,
        ),
        With<IconPendingUpdate>,
    >,
    svg_assets: Res<Assets<svg::SvgAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<IconImageCache>,
    mut image_nodes: Query<(&mut ImageNode, &mut Node), With<IconImage>>,
    server: Res<AssetServer>,
    mut redraw: MessageWriter<RequestRedraw>,
) -> Result<(), BevyError> {
    let mut failures = DiagnosticBatch::default();
    for (entity, icon, target, mut materialized, mut diagnostics) in icons {
        let Some(image_handle) = (match resolve_icon_image_handle(
            entity,
            icon,
            target.scale_factor(),
            &mut diagnostics,
            &svg_assets,
            &server,
            &mut images,
            &mut cache,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                for diagnostic in error.shared_diagnostics() {
                    failures.push_shared(diagnostic);
                }
                continue;
            }
        }) else {
            // A replacement SVG cannot produce an image while it is still loading.
            // Keep the pending marker for retries so the request cannot be lost and leave the old image indefinitely.
            if server.load_state(icon.svg.id()).is_loading() {
                redraw.write(RequestRedraw);
            }
            continue;
        };

        let Ok((mut image_node, mut node)) = image_nodes.get_mut(materialized.image_entity) else {
            let error = diagnostics.image.failure(materialized.image_entity, || {
                Arc::new(failure(
                    IconError::MissingImageNode {
                        entity,
                        image_entity: materialized.image_entity,
                    },
                    "update icon image",
                ))
            });
            for diagnostic in error.shared_diagnostics() {
                failures.push_shared(diagnostic);
            }
            continue;
        };
        diagnostics
            .image
            .recover(|| ErrorFrame::new("icon image restored").field("entity", entity));

        image_node.image = image_handle.handle;
        node.width = px(image_handle.logical_size.x);
        node.height = px(image_handle.logical_size.y);

        materialized.svg_asset_id = icon.svg.id();
        materialized.raster_spec = image_handle.raster_spec;

        commands.entity(entity).remove::<IconPendingUpdate>();
        redraw.write(RequestRedraw);
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}

fn resolve_icon_color(
    overrides: &WidgetryIconColorOverrides,
    inherited: Option<&InheritedForeground>,
    disabled: Option<&WidgetryEffectiveDisabled>,
    theme: &bevy_widgetry_theme::WidgetryTheme,
) -> Color {
    let disabled = disabled.is_some_and(WidgetryEffectiveDisabled::is_disabled);
    let state = if disabled {
        &overrides.disabled
    } else {
        &overrides.normal
    };
    let fallback = overrides.resolve(&theme.icon);
    state
        .foreground
        .or_else(|| {
            inherited
                .filter(|c| c.disabled == disabled)
                .map(|c| c.color)
        })
        .unwrap_or(if disabled {
            fallback.disabled.foreground
        } else {
            fallback.normal.foreground
        })
}
fn sync_icon_color(
    mode: Res<bevy_widgetry_theme::WidgetryThemeMode>,
    mut icons: Query<(
        Entity,
        &colors::ColorState,
        Option<&InheritedForeground>,
        Option<&WidgetryEffectiveDisabled>,
        &IconMaterialized,
        &mut IconRasterState,
    )>,
    mut image_nodes: Query<&mut ImageNode, With<IconImage>>,
) -> Result<(), BevyError> {
    let mut failures = DiagnosticBatch::default();
    for (entity, overrides, inherited, disabled, materialized, mut state) in &mut icons {
        let color = resolve_icon_color(&overrides.0, inherited, disabled, mode.colors());
        let Ok(mut image) = image_nodes.get_mut(materialized.image_entity) else {
            let error = state.image.failure(materialized.image_entity, || {
                Arc::new(failure(
                    IconError::MissingImageNode {
                        entity,
                        image_entity: materialized.image_entity,
                    },
                    "sync icon color",
                ))
            });
            for diagnostic in error.shared_diagnostics() {
                failures.push_shared(diagnostic);
            }
            continue;
        };
        state
            .image
            .recover(|| ErrorFrame::new("icon image restored").field("entity", entity));
        if image.color != color {
            image.color = color;
        }
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}

#[cold]
fn invalid_icon_target(entity: Entity, path: Option<AssetPath<'static>>) -> IconFailure {
    failure(
        IconError::InvalidTarget {
            entity,
            missing: "WidgetryIcon",
            path: path.map(Box::new),
        },
        "update icon",
    )
}

impl IconRasterState {
    fn raster_recovered(&mut self, entity: Entity) {
        self.failure
            .recover(|| ErrorFrame::new("icon raster ready").field("entity", entity));
    }
}

impl WidgetryIcon {
    fn scene(props: WidgetryIconProps) -> impl Scene {
        let empty_path = props.path.path().as_os_str().is_empty();
        let initial_path = props.path.clone();
        bsn! {
            template(move |context| {
                if empty_path {
                    return Err(into_bevy_error(failure(IconError::EmptyPath { entity: Some(context.entity.id()), path: Box::new(initial_path.clone()) }, "initialize icon")));
                }
                props.colors.clone().initial(context.entity.id()).map_err(into_bevy_error)
            })
            WidgetryIcon {
                svg: {props.path},
                max_size: {props.max_size},

            }
        }
    }

    pub fn path(&self) -> Option<&AssetPath<'static>> {
        self.svg.path()
    }

    pub fn max_size(&self) -> Option<UVec2> {
        self.max_size
    }

    #[inline]
    pub fn set_color_in_world(
        world: &mut World,
        entity: Entity,
        color: Color,
    ) -> Result<bool, IconFailure> {
        WidgetryIconColorOverrides::set_in_world(
            world,
            entity,
            WidgetryIconColorOverrides {
                normal: WidgetryIconStateColorOverrides {
                    foreground: Some(color),
                },
                disabled: WidgetryIconStateColorOverrides {
                    foreground: Some(color),
                },
            },
        )
    }

    #[inline]
    pub fn clear_color_in_world(world: &mut World, entity: Entity) -> Result<bool, IconFailure> {
        WidgetryIconColorOverrides::clear_in_world(world, entity)
    }

    pub fn set_svg_in_world(
        world: &mut World,
        entity: Entity,
        path: impl Into<AssetPath<'static>>,
    ) -> Result<bool, IconFailure> {
        let path = path.into();
        let current = world
            .get::<WidgetryIcon>(entity)
            .ok_or_else(|| invalid_icon_target(entity, Some(path.clone())))?
            .svg
            .id();
        let Some(server) = world.get_resource::<AssetServer>() else {
            return Err(failure(
                IconError::MissingAssetServer {
                    entity,
                    path: Box::new(path),
                },
                "set icon SVG",
            ));
        };
        let svg = server.load(path.clone());
        if current == svg.id() {
            return Ok(false);
        }
        world
            .get_mut::<WidgetryIcon>(entity)
            .ok_or_else(|| invalid_icon_target(entity, Some(path)))?
            .svg = svg;
        Ok(true)
    }

    pub fn set_color(commands: &mut Commands, entity: Entity, color: Color) {
        commands.queue(move |world: &mut World| {
            Self::set_color_in_world(world, entity, color)
                .map(|_| ())
                .map_err(into_bevy_error)
        });
    }

    pub fn clear_color(commands: &mut Commands, entity: Entity) {
        commands.queue(move |world: &mut World| {
            Self::clear_color_in_world(world, entity)
                .map(|_| ())
                .map_err(into_bevy_error)
        });
    }

    pub fn set_svg(commands: &mut Commands, entity: Entity, path: impl Into<AssetPath<'static>>) {
        let path = path.into();
        commands.queue(move |world: &mut World| {
            Self::set_svg_in_world(world, entity, path)
                .map(|_| ())
                .map_err(into_bevy_error)
        });
    }

    fn raster_spec(&self, scale: f32) -> IconRasterSpec {
        match self.max_size {
            Some(size) => IconRasterSpec::MaxSize {
                width: (size.x as f32 * scale).ceil() as u32,
                height: (size.y as f32 * scale).ceil() as u32,
            },
            None => IconRasterSpec::Intrinsic {
                scale: scale.to_bits(),
            },
        }
    }
}

impl Plugin for WidgetryIconPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        app.init_asset::<svg::SvgAsset>()
            .init_asset_loader::<svg::SvgAssetLoader>()
            .init_resource::<IconImageCache>()
            .add_message::<RequestRedraw>()
            .configure_sets(
                PostUpdate,
                WidgetryIconMaterialize
                    .after(WidgetryUiSystems::Materialize)
                    .before(WidgetryUiSystems::Disabled)
                    .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                    .before(bevy::ui::UiSystems::Stack),
            )
            .add_systems(
                // Prepare the subtree and glyph shape first so the image reaches propagation and layout in the same frame.
                PostUpdate,
                materialize_icons.in_set(WidgetryIconMaterialize),
            )
            .add_systems(
                PostUpdate,
                (mark_changed_icons, update_pending_icons)
                    .chain()
                    .after(bevy::ui::UiSystems::Propagate)
                    .before(bevy::ui::UiSystems::Content),
            )
            .add_systems(
                PostUpdate,
                sync_icon_color.in_set(WidgetryUiSystems::ContentColors),
            );
        widgetry_debug!("WidgetryIconPlugin 注册完成");
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::asset::io::{
        AssetSourceBuilder, AssetSourceId,
        memory::{Dir, MemoryAssetReader},
    };
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, advance_until, scene_app};
    use std::{path::Path, time::Duration};

    #[test]
    fn scene_constructs_icon_with_defaults() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::scene::ScenePlugin,
        ))
        .init_asset::<svg::SvgAsset>();
        let entity = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                @WidgetryIcon { @path: "icons/default.svg" }
            })
            .id();
        app.world_mut().flush();
        let icon = app.world().get::<WidgetryIcon>(entity).unwrap();
        assert_eq!(
            icon.svg.path().unwrap(),
            &AssetPath::from("icons/default.svg")
        );
        assert_eq!(icon.max_size, None);
        assert_eq!(
            *WidgetryIconColorOverrides::get(app.world(), entity).unwrap(),
            WidgetryIconColorOverrides::default()
        );
        assert!(app.world().get::<Node>(entity).is_some());
    }

    #[test]
    fn scene_constructs_icon_with_props() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::scene::ScenePlugin,
        ))
        .init_asset::<svg::SvgAsset>();
        let entity = app
            .world_mut()
            .commands()
            .spawn_scene(bsn! {
                @WidgetryIcon {
                    @path: { String::from("icons/configured.svg") },
                    @max_size: { Some(UVec2::new(24, 16)) },
                    @colors: { WidgetryIconColorOverrides { normal: WidgetryIconStateColorOverrides { foreground: Some(Color::BLACK) }, ..default() } },
                }
            })
            .id();
        app.world_mut().flush();
        let icon = app.world().get::<WidgetryIcon>(entity).unwrap();
        assert_eq!(
            icon.svg.path().unwrap(),
            &AssetPath::from("icons/configured.svg")
        );
        assert_eq!(icon.max_size, Some(UVec2::new(24, 16)));
        assert_eq!(
            WidgetryIconColorOverrides::get(app.world(), entity)
                .unwrap()
                .normal
                .foreground,
            Some(Color::BLACK)
        );
    }

    #[test]
    fn raster_failure_logs_state_edges() {
        let capture = LogCapture::default();
        let errors = ErrorCapture::default();
        errors.run(|| capture.run(|| {
            let mut app = App::new();
            app.set_error_handler(ErrorCapture::handler());
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), bevy::scene::ScenePlugin, WidgetryIconPlugin))
                .init_asset::<Image>()
                .edit_schedule(PostUpdate, |schedule| { schedule.set_executor(SingleThreadedExecutor::new()); });
            let handle = app.world().resource::<Assets<svg::SvgAsset>>().reserve_handle();
            let entity = app.world_mut().spawn_scene(bsn! { WidgetryIcon { svg: {handle.clone()} } }).unwrap().id();
            app.update();
            app.update();
            assert!(capture.records().iter().all(|record| record.level == bevy::log::Level::DEBUG));
            let oversized = resvg::usvg::Tree::from_str(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="4294967295" height="4294967295"/>"#,
                &resvg::usvg::Options::default(),
            ).unwrap();
            app.world_mut().resource_mut::<Assets<svg::SvgAsset>>().insert(handle.id(), svg::SvgAsset::from_tree(oversized)).unwrap();
            app.update();
            app.update();
            assert_eq!(capture.records().iter().filter(|r| r.level == bevy::log::Level::ERROR).count(), 1);
            assert!(app.world().get::<Children>(entity).is_none());
            assert!(app.world().resource::<Assets<Image>>().is_empty());
            let asset = app.world_mut().resource_mut::<Assets<svg::SvgAsset>>().remove(handle.id()).unwrap();
            let before_wait = capture.records().len();
            app.update();
            app.update();
            assert_eq!(capture.records().len(), before_wait);
            app.world_mut().resource_mut::<Assets<svg::SvgAsset>>().insert(handle.id(), asset).unwrap();
            patch_test_icon(&mut app, entity, |icon| icon.max_size = Some(UVec2::splat(16)));
            app.update();
            app.update();
            assert_eq!(capture.records().iter().filter(|r| r.fields["message"].contains("恢复")).count(), 1);
            app.world_mut().entity_mut(entity).remove::<IconMaterialized>();
            patch_test_icon(&mut app, entity, |icon| icon.max_size = None);
            app.update();
            assert_eq!(capture.records().iter().filter(|r| r.level == bevy::log::Level::ERROR).count(), 2);
            let before_despawn = capture.records().len();
            app.world_mut().entity_mut(entity).despawn();
            app.update();
            assert_eq!(capture.records().len(), before_despawn);
        }));
        let failures = errors.take();
        assert_eq!(failures.len(), 3);
        assert!(
            failures
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
    }

    fn controlled_app() -> App {
        let mut app = scene_app();
        app.add_plugins(WidgetryIconPlugin);
        app
    }

    #[test]
    fn target_density_changes_raster_pixels_without_changing_logical_image_size() {
        use bevy_widgetry_test_utils::{add_ui_plugins, spawn_ui_camera};
        for max_size in [None, Some(UVec2::splat(16))] {
            let mut app = controlled_app();
            add_ui_plugins(&mut app);
            let camera = spawn_ui_camera(&mut app, UVec2::new(400, 200), 1.0);
            let svg = app
                .world()
                .resource::<Assets<svg::SvgAsset>>()
                .reserve_handle();
            make_ready(&mut app, &svg, 16);
            let icon = app
                .world_mut()
                .spawn_scene(bsn! {
                    WidgetryIcon {svg: {svg}, max_size: {max_size}} UiTargetCamera(camera)
                })
                .unwrap()
                .id();
            let mut first = None;
            for scale in [1.0_f32, 1.25, 1.5, 2.0, 1.0] {
                app.world_mut()
                    .get_mut::<Camera>(camera)
                    .unwrap()
                    .computed
                    .target_info
                    .as_mut()
                    .unwrap()
                    .scale_factor = scale;
                app.update();
                let child = app.world().get::<Children>(icon).unwrap()[0];
                let image = app.world().get::<ImageNode>(child).unwrap();
                let pixels = app
                    .world()
                    .resource::<Assets<Image>>()
                    .get(&image.image)
                    .unwrap();
                assert_eq!(pixels.width(), (16.0 * scale).ceil() as u32);
                assert_eq!(pixels.height(), (8.0 * scale).ceil() as u32);
                assert_eq!(
                    app.world().get::<ComputedNode>(child).unwrap().size(),
                    Vec2::new(16.0, 8.0) * scale
                );
                if scale == 1.0 {
                    if let Some(first) = &first {
                        assert_eq!(&image.image, first);
                    } else {
                        first = Some(image.image.clone());
                    }
                }
            }
            assert_eq!(app.world().resource::<Assets<Image>>().len(), 4);
            for _ in 0..3 {
                app.update();
            }
            assert_eq!(app.world().resource::<Assets<Image>>().len(), 4);
        }
    }

    #[test]
    fn density_uses_each_target_camera_and_includes_ui_scale_once() {
        use bevy_widgetry_test_utils::{add_ui_plugins, spawn_ui_camera};
        let mut app = controlled_app();
        add_ui_plugins(&mut app);
        let a = spawn_ui_camera(&mut app, UVec2::new(400, 200), 1.0);
        let b = spawn_ui_camera(&mut app, UVec2::new(400, 200), 1.5);
        let svg = app
            .world()
            .resource::<Assets<svg::SvgAsset>>()
            .reserve_handle();
        make_ready(&mut app, &svg, 16);
        let icons = [a, a, b].map(|camera| app.world_mut().spawn_scene(bsn! {
            WidgetryIcon {svg: {svg.clone()},max_size: {Some(UVec2::splat(16))}} UiTargetCamera(camera)
        }).unwrap().id());
        app.world_mut().resource_mut::<bevy::ui::UiScale>().0 = 2.0;
        app.update();
        let handles = icons.map(|icon| {
            let child = app.world().get::<Children>(icon).unwrap()[0];
            app.world().get::<ImageNode>(child).unwrap().image.clone()
        });
        assert_eq!(handles[0], handles[1]);
        assert_ne!(handles[0], handles[2]);
        for (image, expected) in handles.iter().zip([32, 32, 48]) {
            assert_eq!(
                app.world()
                    .resource::<Assets<Image>>()
                    .get(image)
                    .unwrap()
                    .width(),
                expected
            );
        }
    }

    fn patch_test_icon(app: &mut App, entity: Entity, patch: impl FnOnce(&mut WidgetryIcon)) {
        patch(&mut app.world_mut().get_mut::<WidgetryIcon>(entity).unwrap());
    }

    fn make_ready(app: &mut App, handle: &Handle<svg::SvgAsset>, width: u32) {
        let tree = resvg::usvg::Tree::from_str(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="8"><rect width="100%" height="100%" fill="white"/></svg>"#
        ), &resvg::usvg::Options::default()).unwrap();
        app.world_mut()
            .resource_mut::<Assets<svg::SvgAsset>>()
            .insert(handle.id(), svg::SvgAsset::from_tree(tree))
            .unwrap();
    }

    fn assert_display(app: &App, icon: Entity, child: Entity, image: &Handle<Image>, color: Color) {
        let children = app.world().get::<Children>(icon).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], child);
        assert_eq!(app.world().get::<ChildOf>(child).unwrap().parent(), icon);
        let node = app.world().get::<ImageNode>(child).unwrap();
        assert_eq!(&node.image, image);
        assert_eq!(node.color, color);
        assert!(app.world().resource::<Assets<Image>>().contains(image));
        let picking = app.world().get::<Pickable>(child).unwrap();
        assert!(!picking.should_block_lower && !picking.is_hoverable);
    }

    #[test]
    fn pending_replacement_keeps_image_and_latest_request_wins() {
        let mut app = controlled_app();
        let handles: [_; 3] = std::array::from_fn(|_| {
            app.world()
                .resource::<Assets<svg::SvgAsset>>()
                .reserve_handle()
        });
        let [a, b, c] = &handles;
        make_ready(&mut app, a, 12);
        let icon = app
            .world_mut()
            .spawn_scene(bsn! {
                WidgetryIcon { svg: {a.clone()} }
            })
            .unwrap()
            .id();
        app.update();
        let child = app.world().get::<Children>(icon).unwrap()[0];
        let image_a = app.world().get::<ImageNode>(child).unwrap().image.clone();
        assert_display(
            &app,
            icon,
            child,
            &image_a,
            bevy_widgetry_theme::WidgetryThemeMode::Dark
                .colors()
                .icon
                .normal
                .foreground,
        );
        patch_test_icon(&mut app, icon, |icon| icon.svg = b.clone());
        for _ in 0..3 {
            app.update();
            assert_display(
                &app,
                icon,
                child,
                &image_a,
                bevy_widgetry_theme::WidgetryThemeMode::Dark
                    .colors()
                    .icon
                    .normal
                    .foreground,
            );
        }
        WidgetryIcon::set_color_in_world(app.world_mut(), icon, Color::BLACK).unwrap();
        app.update();
        assert_display(&app, icon, child, &image_a, Color::BLACK);
        patch_test_icon(&mut app, icon, |icon| icon.svg = c.clone());
        app.update();
        assert_display(&app, icon, child, &image_a, Color::BLACK);
        make_ready(&mut app, c, 20);
        app.update();
        let image_c = app.world().get::<ImageNode>(child).unwrap().image.clone();
        assert_ne!(image_c, image_a);
        assert_eq!(
            app.world()
                .resource::<Assets<Image>>()
                .get(&image_c)
                .unwrap()
                .width(),
            20
        );
        assert_display(&app, icon, child, &image_c, Color::BLACK);
        make_ready(&mut app, b, 16);
        for _ in 0..2 {
            app.update();
            assert_display(&app, icon, child, &image_c, Color::BLACK);
        }
    }

    #[test]
    fn returning_to_displayed_source_cancels_pending_replacement() {
        let mut app = controlled_app();
        let a = app
            .world()
            .resource::<Assets<svg::SvgAsset>>()
            .reserve_handle();
        let b = app
            .world()
            .resource::<Assets<svg::SvgAsset>>()
            .reserve_handle();
        make_ready(&mut app, &a, 12);
        let icon = app
            .world_mut()
            .spawn_scene(bsn! { WidgetryIcon { svg: {a.clone()} } })
            .unwrap()
            .id();
        app.update();
        let child = app.world().get::<Children>(icon).unwrap()[0];
        let image_a = app.world().get::<ImageNode>(child).unwrap().image.clone();
        patch_test_icon(&mut app, icon, |icon| icon.svg = b.clone());
        app.update();
        assert_display(
            &app,
            icon,
            child,
            &image_a,
            bevy_widgetry_theme::WidgetryThemeMode::Dark
                .colors()
                .icon
                .normal
                .foreground,
        );
        patch_test_icon(&mut app, icon, |icon| icon.svg = a.clone());
        app.update();
        assert_display(
            &app,
            icon,
            child,
            &image_a,
            bevy_widgetry_theme::WidgetryThemeMode::Dark
                .colors()
                .icon
                .normal
                .foreground,
        );
        make_ready(&mut app, &b, 16);
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        app.update();
        assert_display(
            &app,
            icon,
            child,
            &image_a,
            bevy_widgetry_theme::WidgetryThemeMode::Dark
                .colors()
                .icon
                .normal
                .foreground,
        );
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }

    #[test]
    fn despawning_waiting_icons_does_not_leave_or_create_image_children() {
        for materialized in [false, true] {
            let mut app = controlled_app();
            let a = app
                .world()
                .resource::<Assets<svg::SvgAsset>>()
                .reserve_handle();
            let b = app
                .world()
                .resource::<Assets<svg::SvgAsset>>()
                .reserve_handle();
            let icon = app
                .world_mut()
                .spawn_scene(bsn! { WidgetryIcon { svg: {a.clone()} } })
                .unwrap()
                .id();
            if materialized {
                make_ready(&mut app, &a, 12);
            }
            app.update();
            let child = app
                .world()
                .get::<Children>(icon)
                .map(|children| children[0]);
            assert_eq!(child.is_some(), materialized);
            patch_test_icon(&mut app, icon, |icon| icon.svg = b.clone());
            app.update();
            app.world_mut().despawn(icon);
            make_ready(&mut app, &a, 12);
            make_ready(&mut app, &b, 16);
            for _ in 0..2 {
                app.world_mut()
                    .resource_mut::<Messages<RequestRedraw>>()
                    .clear();
                app.update();
                assert!(app.world().get_entity(icon).is_err());
                if let Some(child) = child {
                    assert!(app.world().get_entity(child).is_err());
                }
                assert_eq!(
                    app.world_mut()
                        .query_filtered::<Entity, With<IconImage>>()
                        .iter(app.world())
                        .count(),
                    0
                );
                assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
            }
        }
    }

    #[test]
    fn zero_size_never_materializes_an_image() {
        for size in [UVec2::new(0, 16), UVec2::new(16, 0), UVec2::ZERO] {
            let mut app = controlled_app();
            let handle = app
                .world()
                .resource::<Assets<svg::SvgAsset>>()
                .reserve_handle();
            make_ready(&mut app, &handle, 12);
            let icon = app
                .world_mut()
                .spawn_scene(bsn! {
                    WidgetryIcon { svg: {handle.clone()}, max_size: {Some(size)} }
                })
                .unwrap()
                .id();
            for _ in 0..2 {
                app.world_mut()
                    .resource_mut::<Messages<RequestRedraw>>()
                    .clear();
                app.update();
                assert!(app.world().get::<Children>(icon).is_none());
                assert!(app.world().resource::<Assets<Image>>().is_empty());
                assert!(
                    app.world()
                        .get::<IconRasterState>(icon)
                        .unwrap()
                        .failure
                        .active()
                        .is_none()
                );
                assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
            }
        }
    }

    #[test]
    fn failed_svg_loading_preserves_existing_image_or_empty_state() {
        for path in ["invalid.svg", "missing.svg"] {
            for has_image in [false, true] {
                let directory = Dir::default();
                directory.insert_asset_text(Path::new("invalid.svg"), "this is not SVG");
                let mut app = App::new();
                // Registering a source after AssetPlugin initializes does not update the existing AssetServer.
                // Register the memory source first so the loader cannot read a disk path instead.
                app.register_asset_source(
                    AssetSourceId::Default,
                    AssetSourceBuilder::new(move || {
                        Box::new(MemoryAssetReader {
                            root: directory.clone(),
                        })
                    }),
                );
                app.add_plugins((
                    MinimalPlugins,
                    AssetPlugin::default(),
                    bevy::scene::ScenePlugin,
                    WidgetryIconPlugin,
                ))
                .init_asset::<Image>();
                let a = app
                    .world()
                    .resource::<Assets<svg::SvgAsset>>()
                    .reserve_handle();
                let icon = app
                    .world_mut()
                    .spawn_scene(bsn! { WidgetryIcon { svg: {a.clone()} } })
                    .unwrap()
                    .id();
                if has_image {
                    make_ready(&mut app, &a, 12);
                }
                app.update();
                let original = app.world().get::<Children>(icon).map(|children| {
                    let child = children[0];
                    (
                        child,
                        app.world().get::<ImageNode>(child).unwrap().image.clone(),
                    )
                });
                assert_eq!(original.is_some(), has_image);
                WidgetryIcon::set_svg_in_world(app.world_mut(), icon, path).unwrap();
                let failed = app.world().get::<WidgetryIcon>(icon).unwrap().svg.clone();
                advance_until(
                    &mut app,
                    Duration::from_secs(2),
                    &format!("{path} 的 SVG load failure"),
                    |world| {
                        world
                            .resource::<AssetServer>()
                            .load_state(failed.id())
                            .is_failed()
                    },
                )
                .expect("内存 source 应在期限内报告失败");
                for _ in 0..2 {
                    app.world_mut()
                        .resource_mut::<Messages<RequestRedraw>>()
                        .clear();
                    app.update();
                    if let Some((child, image)) = &original {
                        assert_display(
                            &app,
                            icon,
                            *child,
                            image,
                            bevy_widgetry_theme::WidgetryThemeMode::Dark
                                .colors()
                                .icon
                                .normal
                                .foreground,
                        );
                    } else {
                        assert!(app.world().get::<Children>(icon).is_none());
                        assert!(app.world().resource::<Assets<Image>>().is_empty());
                    }
                    assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
                }
            }
        }
    }
}

#[cfg(test)]
mod diagnostic_tests;
