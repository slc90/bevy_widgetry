use super::{SplashSession, presentation::SplashPresentation};
use bevy::prelude::*;
use bevy_widgetry::scene::WidgetrySceneCommandsExt;

pub(crate) fn setup(
    commands: &mut Commands,
    server: &AssetServer,
    window: Entity,
    gallery_camera: Entity,
) {
    let loaded: Vec<Handle<Image>> = crate::assets::splash_frames()
        .map(|path| server.load(path))
        .collect();
    let fallback = commands
        .spawn_scene_with_error_handler(bsn! {
            Name("SplashOpaqueCover")
            UiTargetCamera(gallery_camera)
            GlobalZIndex(1000000)
            @full_window()
            BackgroundColor({canvas_color()})
        })
        .id();
    let first = loaded[0].clone();
    let overlay = commands.spawn_scene_with_error_handler(bsn! {
        Name("SplashDisplay")
        UiTargetCamera(gallery_camera)
        GlobalZIndex(1000001)
        @full_window()
        template(move |_| Ok(ImageNode { image: first.clone(), image_mode: NodeImageMode::Stretch, ..default() }))
    }).id();
    commands.insert_resource(SplashPresentation::new(
        window,
        gallery_camera,
        overlay,
        loaded.iter().map(Handle::id).collect(),
    ));
    commands.insert_resource(SplashSession {
        gallery_camera,
        overlay,
        fallback: Some(fallback),
        loaded,
    });
}

fn full_window() -> impl Scene {
    bsn! {
        Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: percent(100), height: percent(100) }
        template(|_| Ok(Pickable { should_block_lower: true, is_hoverable: false }))
    }
}

pub(super) fn canvas_color() -> Color {
    Color::srgb_u8(248, 238, 247)
}
