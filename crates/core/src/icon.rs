mod svg;

use bevy::{asset::AssetPath, platform::collections::HashMap, prelude::*};

use crate::ForegroundColor;

#[derive(Component)]
#[require(Node)]
pub struct Icon {
    svg: Handle<svg::SvgAsset>,
    max_size: Option<UVec2>,
    color: Option<Color>,
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

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = Some(color);
    }

    pub fn clear_color(&mut self) {
        self.color = None;
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

    pub fn set_svg(&mut self, asset_server: &AssetServer, path: impl Into<AssetPath<'static>>) {
        self.svg = asset_server.load(path);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum IconRasterSpec {
    Intrinsic,
    MaxSize { width: u32, height: u32 },
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

pub struct IconPlugin;

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

fn sync_icon_color(
    icons: Query<
        (&Icon, Option<&ForegroundColor>, &IconMaterialized),
        Or<(Changed<Icon>, Changed<ForegroundColor>)>,
    >,
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
