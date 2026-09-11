mod svg;

use crate::ForegroundColor;
use bevy::{asset::AssetPath, platform::collections::HashMap, prelude::*};

/// 异步加载 SVG 并生成图像子实体；需先注册 AssetPlugin 和 IconPlugin。
#[derive(Component)]
#[require(Node)]
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

/// 优先复用缓存；资源尚未就绪或无法分配像素时返回 None。
fn resolve_icon_image_handle(
    icon: &Icon,
    svg_assets: &Assets<svg::SvgAsset>,
    images: &mut Assets<Image>,
    cache: &mut IconImageCache,
) -> Option<Handle<Image>> {
    let svg_asset = svg_assets.get(&icon.svg)?;

    let key = IconImageCacheKey {
        svg_asset_id: icon.svg.id(),
        raster_spec: icon.raster_spec(),
    };

    if let Some(handle) = cache.images.get(&key) {
        return Some(handle.clone());
    }

    let image = match key.raster_spec {
        IconRasterSpec::Intrinsic => svg_asset.render_intrinsic_to_image(),
        IconRasterSpec::MaxSize { width, height } => svg_asset.render_to_image(width, height),
    }?;

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
        (Entity, &Icon, &mut Node, Option<&ForegroundColor>),
        Without<IconMaterialized>,
    >,
    svg_assets: Res<Assets<svg::SvgAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<IconImageCache>,
) {
    for (entity, icon, mut node, foreground_color) in &mut icons {
        let Some(image_handle) =
            resolve_icon_image_handle(icon, &svg_assets, &mut images, &mut cache)
        else {
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

        let image_entity = commands.spawn((IconImage, image_node)).id();

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
    icons: Query<(Entity, &Icon, &mut IconMaterialized), With<IconPendingUpdate>>,
    svg_assets: Res<Assets<svg::SvgAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<IconImageCache>,
    mut image_nodes: Query<&mut ImageNode, With<IconImage>>,
) {
    for (entity, icon, mut materialized) in icons {
        let Some(image_handle) =
            resolve_icon_image_handle(icon, &svg_assets, &mut images, &mut cache)
        else {
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
            .add_systems(
                Update,
                (
                    materialize_icons,
                    mark_changed_icons,
                    update_pending_icons,
                    sync_icon_color,
                ),
            );
    }
}
