//! 仅 harness 指定环境变量时安装 readiness 观测；不修改 desktop_app 或 renderer 配置。

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::text::TextLayoutInfo;
use bevy::ui::{InteractionDisabled, UiSystems};
use bevy::ui_widgets::Button;
use bevy::window::RequestRedraw;
use bevy_widgetry::icon::WidgetryIcon;
use std::fs;
use std::path::PathBuf;

/// GPU readback 对应的可见内容区域；逐个验证 Text/Icon 已产生实际像素。
#[derive(Resource)]
struct Readiness {
    /// 每个样本独占目录，禁止复用旧 ready 文件。
    output: PathBuf,
    /// screenshot 对应帧的内容区域，坐标是固定 scale factor=1 的物理像素。
    regions: Vec<(Vec2, Vec2)>,
    /// 一次最多一个在途 screenshot，失败的空帧继续等待。
    pending: bool,
    /// 正常关闭前只发布一次 readiness。
    published: bool,
}

/// 默认启动完全不安装观测 system；readiness 位于真实 layout 与 picking stack 更新之后。
pub(crate) fn install(app: &mut App) -> Result {
    let Some(output) = std::env::var_os("GALLERY_STARTUP_BENCH_OUTPUT") else {
        return Ok(());
    };
    let output = PathBuf::from(output);
    fs::create_dir_all(&output)?;
    if output.join("ready.txt").exists() {
        return Err(BevyError::error(
            "startup benchmark output contains stale readiness",
        ));
    }
    app.insert_resource(Readiness {
        output,
        regions: Vec::new(),
        pending: false,
        published: false,
    })
    .add_systems(
        PostUpdate,
        capture_when_ready
            .after(UiSystems::Stack)
            .after(UiSystems::PostLayout)
            .after(VisibilitySystems::VisibilityPropagate),
    );
    Ok(())
}

/// 通过公开 UI state 确认可见性，显式排除 display:none 的页面及零尺寸内容。
fn visible(world: &World, entity: Entity) -> bool {
    if !world
        .get::<InheritedVisibility>(entity)
        .is_some_and(|visibility| visibility.get())
    {
        return false;
    }
    let mut ancestor = Some(entity);
    while let Some(current) = ancestor {
        if world
            .get::<Node>(current)
            .is_some_and(|node| node.display == Display::None)
        {
            return false;
        }
        ancestor = world.get::<ChildOf>(current).map(ChildOf::parent);
    }
    world
        .get::<ComputedNode>(entity)
        .is_some_and(|node| node.size().min_element() > 0.0)
}

/// Text 必须完成 glyph layout，Icon 必须生成已加载 ImageNode，ButtonNav 必须可 picking 且 enabled。
/// screenshot GPU readback 的像素验证仍是发布 readiness 的最终前置条件。
fn capture_when_ready(world: &mut World) -> Result {
    let readiness = world.resource::<Readiness>();
    if readiness.published {
        return Ok(());
    }
    if readiness.pending {
        // GPU readback 在途时继续推进必要帧，与 BRP screenshot 的 activity contract 一致。
        world.write_message(RequestRedraw);
        return Ok(());
    }
    let named = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .map(|(entity, name)| (entity, name.as_str().to_owned()))
        .collect::<Vec<_>>();
    for name in ["GalleryRoot", "ButtonPage", "ButtonNav", "ButtonDemo"] {
        let Some((entity, _)) = named.iter().find(|(_, current)| current == name) else {
            return Ok(());
        };
        if !visible(world, *entity) {
            return Ok(());
        }
        if name == "ButtonNav"
            && (world.get::<Button>(*entity).is_none()
                || world.get::<InteractionDisabled>(*entity).is_some())
        {
            return Err(BevyError::error("startup ButtonNav is not interactive"));
        }
    }
    let content = world
        .query::<(Entity, Option<&Text>, Option<&WidgetryIcon>)>()
        .iter(world)
        .filter(|(_, text, icon)| text.is_some() || icon.is_some())
        .map(|(entity, text, icon)| {
            (
                entity,
                text.is_some_and(|text| !text.0.is_empty()),
                icon.is_some(),
            )
        })
        .collect::<Vec<_>>();
    let mut regions = Vec::new();
    let mut text_count = 0;
    let mut icon_count = 0;
    for (entity, text, icon) in content {
        if !visible(world, entity) {
            continue;
        }
        if text {
            if world
                .get::<TextLayoutInfo>(entity)
                .is_none_or(|layout| layout.glyphs.is_empty())
            {
                return Ok(());
            }
            text_count += 1;
        }
        if icon {
            let loaded = world.get::<Children>(entity).is_some_and(|children| {
                children.iter().any(|child| {
                    world.get::<ImageNode>(child).is_some_and(|image| {
                        world.resource::<Assets<Image>>().contains(&image.image)
                    })
                })
            });
            if !loaded {
                return Ok(());
            }
            icon_count += 1;
        }
        let node = world
            .get::<ComputedNode>(entity)
            .ok_or_else(|| BevyError::error("startup content has no layout"))?;
        let transform = world
            .get::<UiGlobalTransform>(entity)
            .ok_or_else(|| BevyError::error("startup content has no transform"))?;
        let center = transform.to_scale_angle_translation().2;
        regions.push((center - node.size() * 0.5, center + node.size() * 0.5));
    }
    // Button 页、sidebar、标题至少有这些内容；不能把缺失主要 Scene 的帧视为已启动。
    if text_count < 25 || icon_count < 5 {
        return Ok(());
    }
    let mut readiness = world.resource_mut::<Readiness>();
    readiness.regions = regions;
    readiness.pending = true;
    world
        .spawn(Screenshot::primary_window())
        .observe(on_capture);
    world.write_message(RequestRedraw);
    Ok(())
}

/// 每个内容区域都必须呈现像素变化，避免只读到背景或尚未就绪的 UI pipeline。
/// 保存 PNG 和原子发布 ready 的开销计入 harness 所观测的上界，不宣称精确首次 present 时间。
fn on_capture(
    event: On<ScreenshotCaptured>,
    mut readiness: ResMut<Readiness>,
    mut redraw: MessageWriter<RequestRedraw>,
) -> Result {
    readiness.pending = false;
    let image = &event.image;
    for &(min, max) in &readiness.regions {
        let x0 = min.x.max(0.0) as u32;
        let y0 = min.y.max(0.0) as u32;
        let x1 = (max.x.ceil() as u32).min(image.width());
        let y1 = (max.y.ceil() as u32).min(image.height());
        if x0 >= x1 || y0 >= y1 {
            redraw.write(RequestRedraw);
            return Ok(());
        }
        let background = image.get_color_at(x0, y0)?;
        let mut changed = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                if image.get_color_at(x, y)? != background {
                    changed += 1;
                }
            }
        }
        if changed < 3 {
            redraw.write(RequestRedraw);
            return Ok(());
        }
    }
    image
        .clone()
        .try_into_dynamic()?
        .save(readiness.output.join("ready.png"))?;
    fs::write(
        readiness.output.join("ready.tmp"),
        format!(
            "Button page; {} rendered Text/Icon regions; ButtonNav enabled; 1920x1080; DX12\n",
            readiness.regions.len()
        ),
    )?;
    fs::rename(
        readiness.output.join("ready.tmp"),
        readiness.output.join("ready.txt"),
    )?;
    readiness.published = true;
    Ok(())
}
