mod svg;

use crate::ForegroundColor;
use bevy::{asset::AssetPath, platform::collections::HashMap, prelude::*};
use bevy_widgetry_log::{widgetry_error, widgetry_info, widgetry_warn};

/// 异步加载 SVG 并生成图像子实体；需先注册 AssetPlugin 和 IconPlugin。
#[derive(Component)]
#[require(Node, IconDiagnostics)]
pub struct Icon {
    /// 通过资产服务器异步加载的 SVG 句柄。
    svg: Handle<svg::SvgAsset>,
    /// 等比缩放的像素上限；None 使用 SVG 原始尺寸。
    max_size: Option<UVec2>,
    /// 显式颜色覆盖；None 使用继承前景色或白色。
    color: Option<Color>,
}

/// 记录已生成的图像子实体与实际显示的 SVG，用于延迟替换资源。
#[derive(Component)]
struct IconMaterialized {
    /// 实际显示栅格图像的子实体。
    image_entity: Entity,
    /// 当前显示或缓存对应的 SVG 资产标识。
    svg_asset_id: AssetId<svg::SvgAsset>,
}

/// 标记需要重建图像的图标，使异步资源未就绪时可以逐帧重试。
#[derive(Component)]
struct IconPendingUpdate;

/// 区分由图标系统创建的图像子实体，限制颜色更新的查询范围。
#[derive(Component)]
struct IconImage;

/// 每个图标独立保存已报告异常，资源等待不清除异常，实体销毁时自动回收。
#[derive(Component, Default)]
struct IconDiagnostics {
    /// 最近一次栅格化失败是否已经报告。
    raster_failed: bool,
    /// 已生成图像缺失或脱离所属图标是否已经报告。
    image_missing: bool,
    /// 布局组件缺失会阻止首次生成图像，需独立于加载状态诊断。
    layout_missing: bool,
}

/// 相同 SVG 与尺寸共享栅格图像；颜色由 ImageNode 独立处理。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct IconImageCacheKey {
    /// 当前显示或缓存对应的 SVG 资产标识。
    svg_asset_id: AssetId<svg::SvgAsset>,
    /// 决定栅格化比例的尺寸约束。
    raster_spec: IconRasterSpec,
}

/// 复用已栅格化图像，避免多个相同图标重复生成像素。
#[derive(Resource, Default)]
struct IconImageCache {
    /// 按 SVG 和尺寸复用的强图像句柄。
    images: HashMap<IconImageCacheKey, Handle<Image>>,
}

/// 注册 SVG 加载器、图像缓存和同步系统；必须在 AssetPlugin 之后注册。
pub struct IconPlugin;

/// 此查询集中表达样式同步所需的数据访问与实体过滤条件。
type IconColorQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Icon,
        Option<&'static ForegroundColor>,
        &'static IconMaterialized,
    ),
    Or<(Changed<Icon>, Changed<ForegroundColor>)>,
>;

/// 区分原始尺寸和等比缩放上限，作为栅格图像缓存键的一部分。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum IconRasterSpec {
    Intrinsic,
    MaxSize { width: u32, height: u32 },
}

/// 优先复用缓存，正常等待保持安静；内部吸收的像素失败与恢复按状态边沿记录。
fn resolve_icon_image_handle(
    entity: Entity,
    icon: &Icon,
    diagnostics: &mut IconDiagnostics,
    svg_assets: &Assets<svg::SvgAsset>,
    images: &mut Assets<Image>,
    cache: &mut IconImageCache,
) -> Option<Handle<Image>> {
    let svg_asset = svg_assets.get(&icon.svg)?;

    // 零尺寸属于调用方输入，保持原有不生成图像的行为，不作为库异常。
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

/// 仅 SVG 标识变化时安排图像替换，颜色变化无需重新栅格化。
fn mark_changed_icons(
    mut commands: Commands,
    icons: Query<(Entity, &Icon, &IconMaterialized), Changed<Icon>>,
) {
    for (entity, icon, materialized) in &icons {
        if icon.svg.id() != materialized.svg_asset_id {
            commands.entity(entity).insert(IconPendingUpdate);
        }
    }
}

/// 为已就绪的 SVG 创建图像子实体，并应用布局与初始颜色。
fn materialize_icons(
    mut commands: Commands,
    mut icons: Query<
        (
            Entity,
            &Icon,
            &mut Node,
            Option<&ForegroundColor>,
            &mut IconDiagnostics,
        ),
        Without<IconMaterialized>,
    >,
    svg_assets: Res<Assets<svg::SvgAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<IconImageCache>,
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

        // 图像只是 Icon 的视觉实现，不能挡住父控件或标题栏底层拖动区的拾取。
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
    }
}

/// 在新 SVG 就绪后替换已有图像，再清除待更新标记。
fn update_pending_icons(
    mut commands: Commands,
    icons: Query<
        (Entity, &Icon, &mut IconMaterialized, &mut IconDiagnostics),
        With<IconPendingUpdate>,
    >,
    svg_assets: Res<Assets<svg::SvgAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<IconImageCache>,
    mut image_nodes: Query<&mut ImageNode, With<IconImage>>,
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
    }
}

/// 将图标显式颜色或继承前景色同步到已生成的图像。
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

/// 无论资源是否已就绪，布局缺失都不能被图像构造查询静默过滤。
fn diagnose_icon_layout(mut icons: Query<(Entity, Has<Node>, &mut IconDiagnostics), With<Icon>>) {
    for (entity, has_node, mut diagnostics) in &mut icons {
        let missing = !has_node;
        if missing != diagnostics.layout_missing {
            if missing {
                widgetry_error!(?entity, "Icon 必需布局组件缺失");
            } else {
                widgetry_info!(?entity, "Icon 必需布局组件恢复正常");
            }
            diagnostics.layout_missing = missing;
        }
    }
}

/// 图像必须仍是所属 Icon 的直接子节点；仅重新出现组件不代表层级已恢复。
fn diagnose_icon_images(
    mut icons: Query<(Entity, &IconMaterialized, &mut IconDiagnostics)>,
    images: Query<Option<&ChildOf>, (With<IconImage>, With<ImageNode>)>,
) {
    for (entity, materialized, mut diagnostics) in &mut icons {
        let missing = !images
            .get(materialized.image_entity)
            .is_ok_and(|parent| parent.is_some_and(|parent| parent.parent() == entity));
        if missing && !diagnostics.image_missing {
            widgetry_error!(?entity, image_entity = ?materialized.image_entity, "Icon 内部图像缺失或父子归属异常");
        } else if !missing && diagnostics.image_missing {
            widgetry_info!(?entity, "Icon 内部图像恢复正常");
        }
        diagnostics.image_missing = missing;
    }
}

impl IconDiagnostics {
    /// 只有此前确实报告过失败才输出恢复，等待期间不会误报恢复。
    fn raster_recovered(&mut self, entity: Entity) {
        if self.raster_failed {
            widgetry_info!(?entity, "图标栅格化恢复正常");
            self.raster_failed = false;
        }
    }
}

impl Icon {
    /// 使用 SVG 自身尺寸，scale = 1.0。
    pub fn new(asset_server: &AssetServer, path: impl Into<AssetPath<'static>>) -> Self {
        Self {
            svg: asset_server.load(path),
            max_size: None,
            color: None,
        }
    }

    /// 将 SVG 等比缩放到指定范围内。
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some(UVec2::new(width, height));
        self
    }

    /// 设置图标专用颜色，优先于继承的 ForegroundColor。
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// 覆盖图标颜色，后续样式同步会更新现有图像子实体。
    pub fn set_color(&mut self, color: Color) {
        self.color = Some(color);
    }

    /// 恢复使用继承前景色；未提供前景色时使用白色。
    pub fn clear_color(&mut self) {
        self.color = None;
    }

    /// 将可选尺寸转换为缓存使用的明确尺寸语义。
    fn raster_spec(&self) -> IconRasterSpec {
        match self.max_size {
            Some(size) => IconRasterSpec::MaxSize {
                width: size.x,
                height: size.y,
            },
            None => IconRasterSpec::Intrinsic,
        }
    }

    /// 请求新 SVG；加载完成前保留当前显示的图像。
    pub fn set_svg(&mut self, asset_server: &AssetServer, path: impl Into<AssetPath<'static>>) {
        self.svg = asset_server.load(path);
    }
}

impl Plugin for IconPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<svg::SvgAsset>()
            .init_asset_loader::<svg::SvgAssetLoader>()
            .init_resource::<IconImageCache>()
            .add_systems(PostUpdate, (diagnose_icon_layout, diagnose_icon_images))
            .add_systems(
                Update,
                (
                    materialize_icons,
                    mark_changed_icons,
                    update_pending_icons,
                    sync_icon_color,
                ),
            );
        widgetry_info!("IconPlugin 注册完成");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy_widgetry_test_utils::LogCapture;

    // 资源仍在等待时布局组件缺失也属于内部异常；恢复、再次损坏和销毁各遵循边沿语义。
    #[test]
    fn missing_layout_logs_state_edges() {
        let capture = LogCapture::default();
        capture.run(|| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), IconPlugin))
                .init_asset::<Image>()
                .edit_schedule(PostUpdate, |schedule| {
                    schedule.set_executor(SingleThreadedExecutor::new());
                });
            let root = app
                .world_mut()
                .spawn(Icon {
                    svg: Handle::default(),
                    max_size: None,
                    color: None,
                })
                .id();
            app.update();
            let baseline = capture.records().len();
            app.world_mut().entity_mut(root).remove::<Node>();
            app.update();
            app.update();
            assert_eq!(capture.records().len(), baseline + 1);
            assert_eq!(capture.records()[baseline].level, bevy::log::Level::ERROR);
            app.world_mut().entity_mut(root).insert(Node::default());
            app.update();
            app.update();
            assert_eq!(capture.records().len(), baseline + 2);
            assert_eq!(
                capture.records()[baseline + 1].level,
                bevy::log::Level::INFO
            );
            app.world_mut().entity_mut(root).remove::<Node>();
            app.update();
            assert_eq!(capture.records().len(), baseline + 3);
            app.world_mut().entity_mut(root).despawn();
            app.update();
            assert_eq!(capture.records().len(), baseline + 3);
        });
    }

    // 未加载资源保持安静；像素失败只警告一次，恢复后只记录一次，再次失败可重新报告。
    #[test]
    fn raster_failure_logs_state_edges() {
        let capture = LogCapture::default();
        capture.run(|| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), IconPlugin))
                .init_asset::<Image>()
                .edit_schedule(Update, |schedule| { schedule.set_executor(SingleThreadedExecutor::new()); });
            let handle = app.world().resource::<Assets<svg::SvgAsset>>().reserve_handle();
            let entity = app.world_mut().spawn(Icon { svg: handle.clone(), max_size: None, color: None }).id();
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
            app.world_mut().get_mut::<Icon>(entity).unwrap().max_size = Some(UVec2::splat(16));
            app.update();
            app.update();
            assert_eq!(capture.records().iter().filter(|r| r.fields["message"].contains("恢复")).count(), 1);
            app.world_mut().entity_mut(entity).remove::<IconMaterialized>();
            app.world_mut().get_mut::<Icon>(entity).unwrap().max_size = None;
            app.update();
            assert_eq!(capture.records().iter().filter(|r| r.level == bevy::log::Level::WARN).count(), 2);
            let before_despawn = capture.records().len();
            app.world_mut().entity_mut(entity).despawn();
            app.update();
            assert_eq!(capture.records().len(), before_despawn);
        });
    }

    // 已生成图像缺失时只报一次 ERROR，恢复后报 INFO；正常颜色变更不留下执行日志。
    #[test]
    fn missing_image_logs_state_edges() {
        let capture = LogCapture::default();
        capture.run(|| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), IconPlugin))
                .init_asset::<Image>()
                .edit_schedule(Update, |schedule| {
                    schedule.set_executor(SingleThreadedExecutor::new());
                })
                .edit_schedule(PostUpdate, |schedule| {
                    schedule.set_executor(SingleThreadedExecutor::new());
                });
            let tree = resvg::usvg::Tree::from_str(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"/>"#,
                &resvg::usvg::Options::default(),
            )
            .unwrap();
            let handle = app
                .world_mut()
                .resource_mut::<Assets<svg::SvgAsset>>()
                .add(svg::SvgAsset::from_tree(tree));
            let root = app
                .world_mut()
                .spawn(Icon {
                    svg: handle,
                    max_size: None,
                    color: None,
                })
                .id();
            app.update();
            let image = app
                .world()
                .get::<IconMaterialized>(root)
                .unwrap()
                .image_entity;
            app.world_mut()
                .get_mut::<Icon>(root)
                .unwrap()
                .set_color(Color::BLACK);
            app.update();
            assert_eq!(capture.records().len(), 1);
            app.world_mut().entity_mut(image).remove::<IconImage>();
            app.update();
            app.update();
            assert_eq!(capture.records().len(), 2);
            assert_eq!(capture.records()[1].level, bevy::log::Level::ERROR);
            app.world_mut().entity_mut(image).insert(IconImage);
            app.update();
            app.update();
            assert_eq!(capture.records().len(), 3);
            assert!(capture.records()[2].fields["message"].contains("恢复"));
            app.world_mut().entity_mut(image).remove::<ChildOf>();
            app.update();
            app.update();
            assert_eq!(capture.records().len(), 4);
            assert_eq!(capture.records()[3].level, bevy::log::Level::ERROR);
            let other = app.world_mut().spawn_empty().id();
            app.world_mut().entity_mut(other).add_child(image);
            app.update();
            assert_eq!(capture.records().len(), 4);
            app.world_mut().entity_mut(root).add_child(image);
            app.update();
            app.update();
            assert_eq!(capture.records().len(), 5);
            assert!(capture.records()[4].fields["message"].contains("恢复"));
            app.world_mut().entity_mut(image).despawn();
            app.update();
            assert_eq!(capture.records().len(), 6);
            assert_eq!(capture.records()[5].level, bevy::log::Level::ERROR);
            app.world_mut().entity_mut(root).despawn();
            app.update();
            assert_eq!(capture.records().len(), 6);
        });
    }
}
