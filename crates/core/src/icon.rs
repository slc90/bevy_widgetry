mod svg;

use crate::{
    ForegroundColor,
    ui::{WidgetryUiPlugin, WidgetryUiSystems},
};
use bevy::window::RequestRedraw;
use bevy::{asset::AssetPath, platform::collections::HashMap, prelude::*};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// icon 的 Scene 入口与运行期 state；通过 BSN 的 @WidgetryIcon 和 [WidgetryIconProps] 一次性初始化。
/// 需先注册 AssetPlugin、ScenePlugin 和 WidgetryIconPlugin；展开后由本 component 维护 state，system 异步生成 image。
/// 公开实例接口仅提供只读查询；展示输入通过 entity API 更新，不发布变化、asset ready 或 replacement 完成 event。
/// 输入提交不代表 image、颜色或 layout 已同步；直接替换或移除 Component 属于 ECS 结构操作。
#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryIconProps)]
#[require(Node, IconRasterState)]
pub struct WidgetryIcon {
    /// 通过 AssetServer 异步加载的 SVG handle。
    svg: Handle<svg::SvgAsset>,
    /// 等比缩放的像素上限；None 使用 SVG 原始尺寸。
    max_size: Option<UVec2>,
    /// 显式颜色覆盖；None 使用继承的 foreground color 或白色。
    color: Option<Color>,
}

/// BSN @WidgetryIcon 的一次性初始化输入；展开后不保留 props 副本，运行期 state 由 WidgetryIcon 保存。
#[derive(Clone, Debug, Default)]
pub struct WidgetryIconProps {
    /// 调用方应提供 SVG asset 路径；展开时通过 AssetServer 加载，无需手动取得 AssetServer。
    pub path: AssetPath<'static>,
    /// SVG 等比缩放的像素上限；None 使用原始尺寸，任一维为零时不生成 image。
    pub max_size: Option<UVec2>,
    /// 显式颜色覆盖；None 使用继承的 foreground color，未提供 foreground color 时使用白色。
    pub color: Option<Color>,
}

/// 记录已生成的 image child entity 与实际显示的 SVG，用于延迟替换 asset。
#[derive(Component)]
struct IconMaterialized {
    /// 实际显示 raster image 的 child entity。
    image_entity: Entity,
    /// 当前显示或 cache 对应的 SVG asset 标识。
    svg_asset_id: AssetId<svg::SvgAsset>,
}

/// 标记需要重建 image 的 icon，使异步 asset 未就绪时可以逐帧重试。
#[derive(Component)]
struct IconPendingUpdate;

/// 区分由 icon system 创建的 image child entity，限制颜色更新的 query 范围。
#[derive(Component)]
struct IconImage;

/// 每个 icon 独立保存 rasterization 失败 state，asset 等待不清除异常，entity 销毁时自动回收。
#[derive(Component, Default)]
struct IconRasterState {
    /// 最近一次 rasterization 失败是否已经报告。
    raster_failed: bool,
}

/// 相同 SVG 与尺寸共享 raster image；颜色由 ImageNode 独立处理。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct IconImageCacheKey {
    /// 当前显示或 cache 对应的 SVG asset 标识。
    svg_asset_id: AssetId<svg::SvgAsset>,
    /// 决定 rasterization 比例的尺寸约束。
    raster_spec: IconRasterSpec,
}

/// 复用已完成 rasterization 的 image，避免多个相同 icon 重复生成像素。
#[derive(Resource, Default)]
struct IconImageCache {
    /// 按 SVG 和尺寸复用的 strong image handle。
    images: HashMap<IconImageCacheKey, Handle<Image>>,
}

/// 注册 SVG loader、image cache 和同步 system；必须在 AssetPlugin 之后注册。
pub struct WidgetryIconPlugin;

/// 此 query 集中表达 style 同步所需的数据访问与 entity filter 条件。
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

/// 区分原始尺寸和等比缩放上限，作为 raster image cache key 的一部分。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum IconRasterSpec {
    Intrinsic,
    MaxSize { width: u32, height: u32 },
}

/// 优先复用 cache，正常等待保持安静；像素失败记录日志并上抛，恢复按 state transition 记录。
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

    // 零尺寸属于调用方输入，保持原有不生成 image 的行为，不作为库异常。
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

/// 仅 SVG 标识变化时安排 image 替换，颜色变化无需重新 rasterize。
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

/// 为已就绪的 SVG 创建 image child entity，并应用 layout 与初始颜色。
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

        // image 只是 WidgetryIcon 的视觉实现，不能挡住 parent Widget 或 title bar 底层 drag 区域的 picking。
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
        // 新 Image 的 asset event 和 render preparation 可能跨帧，按需刷新模式也必须完成提交。
        redraw.write(RequestRedraw);
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// 在新 SVG 就绪后替换已有 image，再清除待更新标记。
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
            // 新 SVG 可能还没加载完成。
            // 保留 IconPendingUpdate，下一帧继续尝试。
            if server.load_state(icon.svg.id()).is_loading() {
                redraw.write(RequestRedraw);
            }
            continue;
        };

        let Ok(mut image_node) = image_nodes.get_mut(materialized.image_entity) else {
            continue;
        };

        image_node.image = image_handle;

        // 记录当前真正已经显示出来的 SVG。
        materialized.svg_asset_id = icon.svg.id();

        // 更新成功，清掉 pending。
        commands.entity(entity).remove::<IconPendingUpdate>();
        redraw.write(RequestRedraw);
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// 将 icon 显式颜色或继承的 foreground color 同步到已生成的 image。
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

/// entity 更新入口共用无效目标诊断，普通等待与合法同值不走此路径。
#[cold]
fn invalid_icon_target(entity: Entity) -> BevyError {
    widgetry_error!(?entity, "Icon 更新目标不存在或缺失 WidgetryIcon");
    BevyError::error("Icon 更新目标不存在或缺失 WidgetryIcon")
}

/// 显式颜色与继承色共用提交路径；合法同值不标记 Component changed，也不触发后续颜色同步。
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
    /// 只有此前确实报告过失败才输出恢复，等待期间不会误报恢复。
    fn raster_recovered(&mut self, entity: Entity) {
        if self.raster_failed {
            widgetry_info!(?entity, "图标栅格化恢复正常");
            self.raster_failed = false;
        }
    }
}

impl WidgetryIcon {
    /// 将 props 写入 component template；SVG handle template 在展开时取得 AssetServer，异步处理仍由 system 负责。
    fn scene(props: WidgetryIconProps) -> impl Scene {
        bsn! {
            WidgetryIcon {
                svg: {props.path},
                max_size: {props.max_size},
                color: {props.color},
            }
        }
    }

    /// 读取当前请求的 SVG 路径，不表示该 SVG 已加载或已显示；无路径 handle 返回 None。
    pub fn path(&self) -> Option<&AssetPath<'static>> {
        self.svg.path()
    }

    /// 读取构造时的尺寸上限；None 使用 SVG 原始尺寸，任一维为零时不生成 image。
    /// 不提供 runtime max_size setter。
    pub fn max_size(&self) -> Option<UVec2> {
        self.max_size
    }

    /// 读取显式颜色覆盖，None 表示消费 ForegroundColor / 白色；不是当前实际显示颜色。
    pub fn color_override(&self) -> Option<Color> {
        self.color
    }

    /// 立即提交颜色覆盖；后续 style 同步更新 image，不发布变化或显示完成 event。
    /// Ok(true) 表示输入改变，Ok(false) 表示合法同值；失效或非 Icon entity 返回 Severity::Error。
    #[inline]
    pub fn set_color_in_world(
        world: &mut World,
        entity: Entity,
        color: Color,
    ) -> Result<bool, BevyError> {
        set_icon_color(world, entity, Some(color))
    }

    /// 立即清除显式颜色，恢复 ForegroundColor；未提供时使用白色，不表示 image 已同步。
    /// 已无覆盖返回 Ok(false)，错误目标与通知边界同 set_color_in_world。
    #[inline]
    pub fn clear_color_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        set_icon_color(world, entity, None)
    }

    /// 立即提交新 SVG 请求；加载或 rasterization 未完成时保留已有 image，失败不清空旧图。
    /// Ok(true) 只表示请求改变，Ok(false) 表示同一 SVG；不发布 asset ready、失败或 replacement 完成 event。
    /// 失效或非 Icon entity、缺失必需 AssetServer 返回 Severity::Error；加载失败由 asset pipeline 异步反馈。
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

    /// 排队提交颜色覆盖，在 Commands 执行时校验并读取最新输入；错误交给宿主 error handler。
    /// 同值、颜色继承及显示时机与 set_color_in_world 相同，入队时尚未修改 Component。
    pub fn set_color(commands: &mut Commands, entity: Entity, color: Color) {
        commands.queue(move |world: &mut World| {
            Self::set_color_in_world(world, entity, color).map(|_| ())
        });
    }

    /// 排队清除显式颜色，在 Commands 执行时恢复继承输入；已清除不修改，不发布清空 event。
    /// 错误反馈与执行时机同 set_color，颜色 projection 仍由后续 system 同步。
    pub fn clear_color(commands: &mut Commands, entity: Entity) {
        commands
            .queue(move |world: &mut World| Self::clear_color_in_world(world, entity).map(|_| ()));
    }

    /// 排队请求新 SVG，在 Commands 执行时使用 AssetServer 并提交输入；错误交给宿主 error handler。
    /// 旧图保留、同值与异步失败边界同 set_svg_in_world，不把入队或输入提交视为 replacement 完成。
    pub fn set_svg(commands: &mut Commands, entity: Entity, path: impl Into<AssetPath<'static>>) {
        let path = path.into();
        commands.queue(move |world: &mut World| {
            Self::set_svg_in_world(world, entity, path).map(|_| ())
        });
    }

    /// 将可选尺寸转换为 cache 使用的明确尺寸语义。
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
                // 等待 window 准备与无效 tree 清理，再创建 image，供同帧 hierarchy 传播和 layout 使用。
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

// 测试 module 中的断言用于验证 contract，生产代码仍禁止。
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

    // 通过 Scene 创建 icon，无需调用方取得 AssetServer，并保持默认尺寸和继承颜色语义。
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

    // Scene 将调用方的尺寸上限与显式颜色写入运行期 component，并接受 owned 路径。
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

    // 未加载的 asset 保持安静；像素失败上抛错误并只记录一次 ERROR，恢复后只记录一次，再次失败可重新报告。
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
            // 通过 Scene 创建身份，再用保留的 handle 覆盖路径 template，以确定性地控制 asset 就绪时机。
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

    /// 独立 App 运行真实 Icon systems，以保留 handle 控制资源就绪，不装配 native window。
    fn controlled_app() -> App {
        let mut app = scene_app();
        app.add_plugins(WidgetryIconPlugin);
        app
    }

    /// 保留 handle / 尺寸控制仅用于局部 raster/等待测试，不作为公开 Widget state 更新入口。
    fn patch_test_icon(app: &mut App, entity: Entity, patch: impl FnOnce(&mut WidgetryIcon)) {
        patch(&mut app.world_mut().get_mut::<WidgetryIcon>(entity).unwrap());
    }

    /// 将测试矩形插入保留的 handle，不触发磁盘读取或异步 loader。
    fn make_ready(app: &mut App, handle: &Handle<svg::SvgAsset>, width: u32) {
        let tree = resvg::usvg::Tree::from_str(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="8"><rect width="100%" height="100%" fill="white"/></svg>"#
        ), &resvg::usvg::Options::default()).unwrap();
        app.world_mut()
            .resource_mut::<Assets<svg::SvgAsset>>()
            .insert(handle.id(), svg::SvgAsset::from_tree(tree))
            .unwrap();
    }

    /// 观察实际 image child、颜色与资源；每步同时保护唯一 child 和 hierarchy/picking 合同。
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

    /// B 等待期间一直保留 A 且颜色可变；当前请求 C 先就绪，B 后到不能回退显示资源。
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

    /// A→B pending→A 取消替换后仍保留原图；B 后到不再触发替换或持续 redraw。
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

    /// 首次等待和已有图的替换等待中销毁 Icon；资源后来就绪也不能生成孤儿 image child。
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

    /// 任一尺寸为零时，已就绪 SVG 也不生成 image，不产生 raster failure 或重复 redraw。
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

    /// 真实 loader 解析失败或读取缺失资源时，不生成新 image；已有图仍保留，稳定失败后不持续 redraw。
    #[test]
    fn failed_svg_loading_preserves_existing_image_or_empty_state() {
        for path in ["invalid.svg", "missing.svg"] {
            for has_image in [false, true] {
                let directory = Dir::default();
                directory.insert_asset_text(Path::new("invalid.svg"), "this is not SVG");
                let mut app = App::new();
                // 自定义内存 source 必须在 AssetPlugin 前注册，避免依赖磁盘或真实桌面。
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
