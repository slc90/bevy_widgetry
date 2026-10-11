use super::{SplashRevealMaterial, SplashSession, presentation::SplashPresentation};
use crate::assets::{SplashArtboard, SplashLayer, SplashLayerKind};
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy_widgetry::scene::WidgetrySceneCommandsExt;

#[derive(Component)]
pub(super) struct SplashCharacter;

#[derive(Component)]
pub(super) struct SplashLayerNode(pub(super) SplashLayerKind);

#[derive(Component)]
pub(super) struct SplashLayerImage(pub(super) SplashLayerKind);

pub(crate) fn setup(
    commands: &mut Commands,
    server: &AssetServer,
    images: &mut Assets<Image>,
    materials: &mut Assets<SplashRevealMaterial>,
    artboard: &SplashArtboard,
    window: Entity,
    gallery_camera: Entity,
) {
    let size = Vec2::from_array(artboard.artboard);
    let target = images.add(Image::new_target_texture(
        size.x as u32,
        size.y as u32,
        TextureFormat::Bgra8UnormSrgb,
        None,
    ));
    let camera = commands
        .spawn((
            Name::new("SplashOffscreenCamera"),
            Camera2d,
            Msaa::Off,
            Camera {
                order: -1,
                clear_color: ClearColorConfig::Custom(canvas_color()),
                ..default()
            },
            RenderTarget::Image(target.clone().into()),
        ))
        .id();
    let loaded: Vec<_> = artboard
        .layers
        .iter()
        .map(|layer| {
            let path = if layer.name == SplashLayerKind::BodyOpen
                && std::env::var("GALLERY_SPLASH_FAULT").as_deref() == Ok("asset")
            {
                crate::assets::splash_missing_image()
            } else {
                layer.name.path()
            };
            (layer.clone(), server.load::<Image>(path))
        })
        .collect();
    let children: Vec<_> = loaded
        .iter()
        .filter(|(layer, _)| layer.name != SplashLayerKind::BodyClosed)
        .map(|(layer, handle)| layer_scene(layer.clone(), handle.clone()))
        .collect();
    let scene_root = commands.spawn_scene_with_error_handler(bsn! {
        Name("SplashOffscreenScene")
        UiTargetCamera(camera)
        template(|_| Ok(Pickable::IGNORE))
        Node { width: px(size.x), height: px(size.y), overflow: Overflow::clip() }
        BackgroundColor({canvas_color()})
        BackgroundGradient({gradient()})
        Children [
            Name("SplashCharacter")
            template(|_| Ok(SplashCharacter))
            Node { position_type: PositionType::Absolute, width: px(size.x), height: px(size.y) }
            template(move |_| Ok(UiTransform { translation: Val2::px(0.0, size.y + 1.0), ..default() }))
            template(|_| Ok(Pickable::IGNORE))
            Children [{children}]
        ]
    }).id();
    let fallback = commands
        .spawn_scene_with_error_handler(bsn! {
            Name("SplashOpaqueCover")
            UiTargetCamera(gallery_camera)
            GlobalZIndex(1000000)
            @full_window()
            BackgroundColor({canvas_color()})
            BackgroundGradient({gradient()})
        })
        .id();
    let material = materials.add(SplashRevealMaterial {
        cover: Vec4::new(0.0, 0.0, 1.0, 1.0),
        scene: target.clone(),
        viewport_center: Vec4::ZERO,
        wave: Vec4::ZERO,
    });
    let overlay = commands
        .spawn_scene_with_error_handler(bsn! {
            Name("SplashDisplay")
            UiTargetCamera(gallery_camera)
            GlobalZIndex(1000001)
            @full_window()
            Visibility::Hidden
            MaterialNode::<SplashRevealMaterial>({material.clone()})
        })
        .id();
    let presentation = SplashPresentation::new(
        window,
        gallery_camera,
        camera,
        scene_root,
        overlay,
        material.id(),
        loaded.iter().map(|(_, handle)| handle.id()).collect(),
    );
    commands.insert_resource(presentation);
    commands.insert_resource(SplashSession {
        gallery_camera,
        camera,
        scene_root,
        fallback: Some(fallback),
        overlay,
        material,
        loaded,
        shader: server.load(crate::assets::splash_shader()),
        reveal_center: None,
        terminal_submitted: false,
    });
}

fn full_window() -> impl Scene {
    bsn! {
        Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: percent(100), height: percent(100) }
        template(|_| Ok(Pickable { should_block_lower: true, is_hoverable: false }))
    }
}

fn layer_scene(layer: SplashLayer, handle: Handle<Image>) -> impl Scene {
    let size = layer.draw_size.unwrap_or(layer.source_size);
    let rect = layer.source_rect.map(|r| Rect::new(r[0], r[1], r[2], r[3]));
    let name = format!("Splash{:?}", layer.name);
    let kind = layer.name;
    bsn! {
        Name({name})
        template(move |_| Ok(SplashLayerNode(kind)))
        template(|_| Ok(Pickable::IGNORE))
        Node { position_type: PositionType::Absolute, left: px(layer.pivot[0]), top: px(layer.pivot[1]), width: px(0), height: px(0) }
        ZIndex({layer.z})
        template(move |_| Ok(UiTransform { rotation: Rot2::radians(layer.rotation_degrees.to_radians()), scale: Vec2::splat(layer.scale), ..default() }))
        Children [
            Node { position_type: PositionType::Absolute, left: px(-layer.source_pivot[0]), top: px(-layer.source_pivot[1]), width: px(size[0]), height: px(size[1]) }
            template(move |_| Ok(ImageNode { image: handle.clone(), rect, image_mode: NodeImageMode::Stretch, ..default() }))
            template(move |_| Ok(SplashLayerImage(kind)))
            template(|_| Ok(Pickable::IGNORE))
        ]
    }
}

pub(super) fn canvas_color() -> Color {
    Color::srgb_u8(248, 238, 247)
}

fn gradient() -> Vec<Gradient> {
    vec![Gradient::Linear(LinearGradient::new(
        2.1,
        vec![
            ColorStop::percent(canvas_color(), 0),
            ColorStop::percent(Color::srgb_u8(228, 225, 246), 50),
            ColorStop::percent(Color::srgb_u8(244, 227, 239), 100),
        ],
    ))]
}
