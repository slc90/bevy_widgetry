mod svg;

use crate::ForegroundColor;
use bevy::window::RequestRedraw;
use bevy::{asset::AssetPath, platform::collections::HashMap, prelude::*};
use bevy_widgetry_log::{widgetry_info, widgetry_warn};

/// icon 的 Scene 入口与运行期 state；通过 BSN 的 @WidgetryIcon 和 [WidgetryIconProps] 一次性初始化。
/// 需先注册 AssetPlugin、ScenePlugin 和 WidgetryIconPlugin；展开后由本 component 维护 state，system 异步生成 image。
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

/// 优先复用 cache，正常等待保持安静；内部吸收的像素失败与恢复按 state transition 记录。
fn resolve_icon_image_handle(
    entity: Entity,
    icon: &WidgetryIcon,
    diagnostics: &mut IconRasterState,
    svg_assets: &Assets<svg::SvgAsset>,
    images: &mut Assets<Image>,
    cache: &mut IconImageCache,
) -> Option<Handle<Image>> {
    let svg_asset = svg_assets.get(&icon.svg)?;

    // 零尺寸属于调用方输入，保持原有不生成 image 的行为，不作为库异常。
    if icon.max_size.is_some_and(|size| size.x == 0 || size.y == 0) {
        return None;
    }

    let key = IconImageCacheKey {
        svg_asset_id: icon.svg.id(),
        raster_spec: icon.raster_spec(),
    };

    if let Some(handle) = cache.images.get(&key) {
        diagnostics.raster_recovered(entity);
        return Some(handle.clone());
    }

    let image = match key.raster_spec {
        IconRasterSpec::Intrinsic => svg_asset.render_intrinsic_to_image(),
        IconRasterSpec::MaxSize { width, height } => svg_asset.render_to_image(width, height),
    };
    let image = match image {
        Ok(image) => image,
        Err(error) => {
            if !diagnostics.raster_failed {
                widgetry_warn!(?entity, path = ?icon.svg.path(), width = error.width, height = error.height, ?error, "图标像素缓冲区创建失败");
                diagnostics.raster_failed = true;
            }
            return None;
        }
    };
    diagnostics.raster_recovered(entity);

    let handle = images.add(image);

    cache.images.insert(key, handle.clone());

    Some(handle)
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
) {
    for (entity, icon, mut node, foreground_color, mut diagnostics) in &mut icons {
        let Some(image_handle) = resolve_icon_image_handle(
            entity,
            icon,
            &mut diagnostics,
            &svg_assets,
            &mut images,
            &mut cache,
        ) else {
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
) {
    for (entity, icon, mut materialized, mut diagnostics) in icons {
        let Some(image_handle) = resolve_icon_image_handle(
            entity,
            icon,
            &mut diagnostics,
            &svg_assets,
            &mut images,
            &mut cache,
        ) else {
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

    /// 覆盖 icon 颜色，后续 style 同步会更新现有 image child entity。
    pub fn set_color(&mut self, color: Color) {
        self.color = Some(color);
    }

    /// 恢复使用继承的 foreground color；未提供 foreground color 时使用白色。
    pub fn clear_color(&mut self) {
        self.color = None;
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

    /// 请求新 SVG；加载完成前保留当前显示的 image。
    pub fn set_svg(&mut self, asset_server: &AssetServer, path: impl Into<AssetPath<'static>>) {
        self.svg = asset_server.load(path);
    }
}

impl Plugin for WidgetryIconPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<svg::SvgAsset>()
            .init_asset_loader::<svg::SvgAssetLoader>()
            .init_resource::<IconImageCache>()
            .add_message::<RequestRedraw>()
            .add_systems(
                // 等待 window 准备与无效 tree 清理，再创建 image，供同帧 hierarchy 传播和 layout 使用。
                PostUpdate,
                (materialize_icons, mark_changed_icons, update_pending_icons)
                    .chain()
                    .after(bevy::ui::UiSystems::Prepare)
                    .before(bevy::ui::UiSystems::Propagate),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy_widgetry_test_utils::LogCapture;

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

    // 未加载的 asset 保持安静；像素失败只警告一次，恢复后只记录一次，再次失败可重新报告。
    #[test]
    fn raster_failure_logs_state_edges() {
        let capture = LogCapture::default();
        capture.run(|| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), bevy::scene::ScenePlugin, WidgetryIconPlugin))
                .init_asset::<Image>()
                .edit_schedule(PostUpdate, |schedule| { schedule.set_executor(SingleThreadedExecutor::new()); });
            let handle = app.world().resource::<Assets<svg::SvgAsset>>().reserve_handle();
            // 通过 Scene 创建身份，再用保留的 handle 覆盖路径 template，以确定性地控制 asset 就绪时机。
            let entity = app.world_mut().spawn_scene(bsn! { @WidgetryIcon WidgetryIcon { svg: {handle.clone()} } }).unwrap().id();
            app.update();
            app.update();
            assert_eq!(capture.records().len(), 1);
            let oversized = resvg::usvg::Tree::from_str(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="4294967295" height="4294967295"/>"#,
                &resvg::usvg::Options::default(),
            ).unwrap();
            app.world_mut().resource_mut::<Assets<svg::SvgAsset>>().insert(handle.id(), svg::SvgAsset::from_tree(oversized)).unwrap();
            app.update();
            app.update();
            assert_eq!(capture.records().iter().filter(|r| r.level == bevy::log::Level::WARN).count(), 1);
            let asset = app.world_mut().resource_mut::<Assets<svg::SvgAsset>>().remove(handle.id()).unwrap();
            let before_wait = capture.records().len();
            app.update();
            app.update();
            assert_eq!(capture.records().len(), before_wait);
            app.world_mut().resource_mut::<Assets<svg::SvgAsset>>().insert(handle.id(), asset).unwrap();
            app.world_mut().get_mut::<WidgetryIcon>(entity).unwrap().max_size = Some(UVec2::splat(16));
            app.update();
            app.update();
            assert_eq!(capture.records().iter().filter(|r| r.fields["message"].contains("恢复")).count(), 1);
            app.world_mut().entity_mut(entity).remove::<IconMaterialized>();
            app.world_mut().get_mut::<WidgetryIcon>(entity).unwrap().max_size = None;
            app.update();
            assert_eq!(capture.records().iter().filter(|r| r.level == bevy::log::Level::WARN).count(), 2);
            let before_despawn = capture.records().len();
            app.world_mut().entity_mut(entity).despawn();
            app.update();
            assert_eq!(capture.records().len(), before_despawn);
        });
    }
}
