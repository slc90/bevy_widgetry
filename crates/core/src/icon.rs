mod svg;

use crate::{
    ForegroundColor,
    ui::{WidgetryUiPlugin, WidgetryUiSystems},
};
use bevy::window::RequestRedraw;
use bevy::{asset::AssetPath, platform::collections::HashMap, prelude::*};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryIconProps)]
#[require(Node, IconRasterState)]
pub struct WidgetryIcon {
    svg: Handle<svg::SvgAsset>,
    max_size: Option<UVec2>,
    color: Option<Color>,
}

#[derive(Clone, Debug, Default)]
pub struct WidgetryIconProps {
    pub path: AssetPath<'static>,
    pub max_size: Option<UVec2>,
    pub color: Option<Color>,
}

#[derive(Component)]
struct IconMaterialized {
    image_entity: Entity,
    svg_asset_id: AssetId<svg::SvgAsset>,
}

#[derive(Component)]
struct IconPendingUpdate;

#[derive(Component)]
struct IconImage;

#[derive(Component, Default)]
struct IconRasterState {
    raster_failed: bool,
}

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

type IconColorQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static WidgetryIcon,
        Option<&'static ForegroundColor>,
        &'static IconMaterialized,
    ),
    Or<(Changed<WidgetryIcon>, Changed<ForegroundColor>)>,
>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum IconRasterSpec {
    Intrinsic,
    MaxSize { width: u32, height: u32 },
}

fn resolve_icon_image_handle(
    entity: Entity,
    icon: &WidgetryIcon,
    diagnostics: &mut IconRasterState,
    svg_assets: &Assets<svg::SvgAsset>,
    images: &mut Assets<Image>,
    cache: &mut IconImageCache,
) -> Result<Option<Handle<Image>>, BevyError> {
    let Some(svg_asset) = svg_assets.get(&icon.svg) else {
        return Ok(None);
    };

    if icon.max_size.is_some_and(|size| size.x == 0 || size.y == 0) {
        return Ok(None);
    }

    let key = IconImageCacheKey {
        svg_asset_id: icon.svg.id(),
        raster_spec: icon.raster_spec(),
    };

    if let Some(handle) = cache.images.get(&key) {
        diagnostics.raster_recovered(entity);
        return Ok(Some(handle.clone()));
    }

    let image = match key.raster_spec {
        IconRasterSpec::Intrinsic => svg_asset.render_intrinsic_to_image(),
        IconRasterSpec::MaxSize { width, height } => svg_asset.render_to_image(width, height),
    };
    let image = match image {
        Ok(image) => image,
        Err(error) => {
            if !diagnostics.raster_failed {
                widgetry_error!(?entity, path = ?icon.svg.path(), width = error.width, height = error.height, ?error, "图标像素缓冲区创建失败");
                diagnostics.raster_failed = true;
            }
            return Err(BevyError::error(format!(
                "图标像素缓冲区创建失败: {}x{}",
                error.width, error.height
            )));
        }
    };
    diagnostics.raster_recovered(entity);

    let handle = images.add(image);

    cache.images.insert(key, handle.clone());

    Ok(Some(handle))
}

fn mark_changed_icons(
    mut commands: Commands,
    icons: Query<(Entity, &WidgetryIcon, &IconMaterialized), Changed<WidgetryIcon>>,
) {
    for (entity, icon, materialized) in &icons {
        if icon.svg.id() != materialized.svg_asset_id {
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
            &mut Node,
            Option<&ForegroundColor>,
            &mut IconRasterState,
        ),
        Without<IconMaterialized>,
    >,
    svg_assets: Res<Assets<svg::SvgAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<IconImageCache>,
    server: Res<AssetServer>,
    mut redraw: MessageWriter<RequestRedraw>,
) -> Result<(), BevyError> {
    let mut failure = None;
    for (entity, icon, mut node, foreground_color, mut diagnostics) in &mut icons {
        let Some(image_handle) = (match resolve_icon_image_handle(
            entity,
            icon,
            &mut diagnostics,
            &svg_assets,
            &mut images,
            &mut cache,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                if failure.is_none() {
                    failure = Some(error);
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

        let mut image_node = ImageNode::new(image_handle);

        let color = icon
            .color
            .or_else(|| foreground_color.map(|foreground| foreground.0))
            .unwrap_or(Color::WHITE);

        image_node.color = color;

        // Icon image 覆盖 parent 的可见区域时会截走 Widget click 或 title bar drag。
        // 设为 Pickable::IGNORE，让输入仍能命中 parent。
        let image_entity = commands
            .spawn((IconImage, image_node, Pickable::IGNORE))
            .id();

        commands
            .entity(entity)
            .add_child(image_entity)
            .insert(IconMaterialized {
                image_entity,
                svg_asset_id: icon.svg.id(),
            });
        // Reactive App 可能在新 Image 的 asset event 与 render preparation 完成前停止 update。
        // 请求 redraw，让跨帧准备继续执行。
        redraw.write(RequestRedraw);
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn update_pending_icons(
    mut commands: Commands,
    icons: Query<
        (
            Entity,
            &WidgetryIcon,
            &mut IconMaterialized,
            &mut IconRasterState,
        ),
        With<IconPendingUpdate>,
    >,
    svg_assets: Res<Assets<svg::SvgAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<IconImageCache>,
    mut image_nodes: Query<&mut ImageNode, With<IconImage>>,
    server: Res<AssetServer>,
    mut redraw: MessageWriter<RequestRedraw>,
) -> Result<(), BevyError> {
    let mut failure = None;
    for (entity, icon, mut materialized, mut diagnostics) in icons {
        let Some(image_handle) = (match resolve_icon_image_handle(
            entity,
            icon,
            &mut diagnostics,
            &svg_assets,
            &mut images,
            &mut cache,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                if failure.is_none() {
                    failure = Some(error);
                }
                continue;
            }
        }) else {
            // 替换 SVG 尚在 loading 时无法生成新 image。
            // 保留 pending 标记供后续帧重试，避免请求被提前清除而永远保留旧图。
            if server.load_state(icon.svg.id()).is_loading() {
                redraw.write(RequestRedraw);
            }
            continue;
        };

        let Ok(mut image_node) = image_nodes.get_mut(materialized.image_entity) else {
            continue;
        };

        image_node.image = image_handle;

        materialized.svg_asset_id = icon.svg.id();

        commands.entity(entity).remove::<IconPendingUpdate>();
        redraw.write(RequestRedraw);
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn sync_icon_color(
    icons: IconColorQuery<'_, '_>,
    mut image_nodes: Query<&mut ImageNode, With<IconImage>>,
) {
    for (icon, foreground_color, materialized) in &icons {
        let color = icon
            .color
            .or_else(|| foreground_color.map(|foreground| foreground.0))
            .unwrap_or(Color::WHITE);

        let Ok(mut image_node) = image_nodes.get_mut(materialized.image_entity) else {
            continue;
        };

        image_node.color = color;
    }
}

#[cold]
fn invalid_icon_target(entity: Entity) -> BevyError {
    widgetry_error!(?entity, "Icon 更新目标不存在或缺失 WidgetryIcon");
    BevyError::error("Icon 更新目标不存在或缺失 WidgetryIcon")
}

#[inline]
fn set_icon_color(
    world: &mut World,
    entity: Entity,
    color: Option<Color>,
) -> Result<bool, BevyError> {
    let mut icon = world
        .get_mut::<WidgetryIcon>(entity)
        .ok_or_else(|| invalid_icon_target(entity))?;
    if icon.color == color {
        return Ok(false);
    }
    icon.color = color;
    Ok(true)
}

impl IconRasterState {
    fn raster_recovered(&mut self, entity: Entity) {
        if self.raster_failed {
            widgetry_info!(?entity, "图标栅格化恢复正常");
            self.raster_failed = false;
        }
    }
}

impl WidgetryIcon {
    fn scene(props: WidgetryIconProps) -> impl Scene {
        bsn! {
            WidgetryIcon {
                svg: {props.path},
                max_size: {props.max_size},
                color: {props.color},
            }
        }
    }

    pub fn path(&self) -> Option<&AssetPath<'static>> {
        self.svg.path()
    }

    pub fn max_size(&self) -> Option<UVec2> {
        self.max_size
    }

    pub fn color_override(&self) -> Option<Color> {
        self.color
    }

    #[inline]
    pub fn set_color_in_world(
        world: &mut World,
        entity: Entity,
        color: Color,
    ) -> Result<bool, BevyError> {
        set_icon_color(world, entity, Some(color))
    }

    #[inline]
    pub fn clear_color_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        set_icon_color(world, entity, None)
    }

    pub fn set_svg_in_world(
        world: &mut World,
        entity: Entity,
        path: impl Into<AssetPath<'static>>,
    ) -> Result<bool, BevyError> {
        let current = world
            .get::<WidgetryIcon>(entity)
            .ok_or_else(|| invalid_icon_target(entity))?
            .svg
            .id();
        let Some(server) = world.get_resource::<AssetServer>() else {
            widgetry_error!(?entity, "Icon SVG 更新缺失 AssetServer");
            return Err(BevyError::error("Icon SVG 更新缺失 AssetServer"));
        };
        let svg = server.load(path);
        if current == svg.id() {
            return Ok(false);
        }
        world
            .get_mut::<WidgetryIcon>(entity)
            .ok_or_else(|| invalid_icon_target(entity))?
            .svg = svg;
        Ok(true)
    }

    pub fn set_color(commands: &mut Commands, entity: Entity, color: Color) {
        commands.queue(move |world: &mut World| {
            Self::set_color_in_world(world, entity, color).map(|_| ())
        });
    }

    pub fn clear_color(commands: &mut Commands, entity: Entity) {
        commands
            .queue(move |world: &mut World| Self::clear_color_in_world(world, entity).map(|_| ()));
    }

    pub fn set_svg(commands: &mut Commands, entity: Entity, path: impl Into<AssetPath<'static>>) {
        let path = path.into();
        commands.queue(move |world: &mut World| {
            Self::set_svg_in_world(world, entity, path).map(|_| ())
        });
    }

    fn raster_spec(&self) -> IconRasterSpec {
        match self.max_size {
            Some(size) => IconRasterSpec::MaxSize {
                width: size.x,
                height: size.y,
            },
            None => IconRasterSpec::Intrinsic,
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
            .add_systems(
                // window 尚未准备或旧 tree 尚未清理时创建 image 会错过正确的 UI 准备。
                // 在 Materialize 阶段创建，使其赶上同帧 propagation 与 layout。
                PostUpdate,
                (materialize_icons, mark_changed_icons, update_pending_icons)
                    .chain()
                    .in_set(WidgetryUiSystems::Materialize),
            )
            .add_systems(
                PostUpdate,
                sync_icon_color
                    .after(bevy::ui::UiSystems::Propagate)
                    .before(bevy::ui::UiSystems::Content),
            );
        widgetry_info!("WidgetryIconPlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
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
        assert_eq!(icon.color, None);
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
                    @color: { Some(Color::BLACK) },
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
        assert_eq!(icon.color, Some(Color::BLACK));
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
            let entity = app.world_mut().spawn_scene(bsn! { @WidgetryIcon WidgetryIcon { svg: {handle.clone()} } }).unwrap().id();
            app.update();
            app.update();
            assert!(capture.records().iter().all(|record| record.level == bevy::log::Level::INFO));
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
                @WidgetryIcon WidgetryIcon { svg: {a.clone()} }
            })
            .unwrap()
            .id();
        app.update();
        let child = app.world().get::<Children>(icon).unwrap()[0];
        let image_a = app.world().get::<ImageNode>(child).unwrap().image.clone();
        assert_display(&app, icon, child, &image_a, Color::WHITE);
        patch_test_icon(&mut app, icon, |icon| icon.svg = b.clone());
        for _ in 0..3 {
            app.update();
            assert_display(&app, icon, child, &image_a, Color::WHITE);
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
            .spawn_scene(bsn! { @WidgetryIcon WidgetryIcon { svg: {a.clone()} } })
            .unwrap()
            .id();
        app.update();
        let child = app.world().get::<Children>(icon).unwrap()[0];
        let image_a = app.world().get::<ImageNode>(child).unwrap().image.clone();
        patch_test_icon(&mut app, icon, |icon| icon.svg = b.clone());
        app.update();
        assert_display(&app, icon, child, &image_a, Color::WHITE);
        patch_test_icon(&mut app, icon, |icon| icon.svg = a.clone());
        app.update();
        assert_display(&app, icon, child, &image_a, Color::WHITE);
        make_ready(&mut app, &b, 16);
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        app.update();
        assert_display(&app, icon, child, &image_a, Color::WHITE);
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
                .spawn_scene(bsn! { @WidgetryIcon WidgetryIcon { svg: {a.clone()} } })
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
                    @WidgetryIcon { @max_size: {Some(size)} }
                    WidgetryIcon { svg: {handle.clone()} }
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
                    !app.world()
                        .get::<IconRasterState>(icon)
                        .unwrap()
                        .raster_failed
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
                // AssetPlugin 初始化后再注册自定义 source 不会更新已建立的 AssetServer。
                // 提前注册内存 source，避免 loader 误读磁盘路径。
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
                    .spawn_scene(bsn! { @WidgetryIcon WidgetryIcon { svg: {a.clone()} } })
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
                        assert_display(&app, icon, *child, image, Color::WHITE);
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
