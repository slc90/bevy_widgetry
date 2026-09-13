use bevy::prelude::*;
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_core::icon::{Icon, IconPlugin};
use std::time::{Duration, Instant};

// props 只初始化一次；图像生成后，颜色覆盖、清除与 SVG 替换都由 Icon 运行期状态驱动。
#[test]
fn runtime_mutations_survive_scene_initialization() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        WidgetryAssetPlugin,
        IconPlugin,
    ))
    .init_asset::<Image>();
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @Icon {
                @path: {BuiltinIcon::WindowClose.path()},
                @max_size: { Some(UVec2::new(16, 16)) },
                @color: { Some(Color::BLACK) },
            }
        })
        .unwrap()
        .id();
    let deadline = Instant::now() + Duration::from_secs(2);
    while app.world().get::<Children>(entity).is_none() {
        assert!(Instant::now() < deadline, "初始图标未生成图像");
        app.update();
        std::thread::yield_now();
    }
    let child = app.world().get::<Children>(entity).unwrap()[0];
    let initial_image = app.world().get::<ImageNode>(child).unwrap().image.clone();
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::BLACK
    );
    app.world_mut()
        .get_mut::<Icon>(entity)
        .unwrap()
        .set_color(Color::srgb(1.0, 0.0, 0.0));
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::srgb(1.0, 0.0, 0.0)
    );
    app.world_mut()
        .get_mut::<Icon>(entity)
        .unwrap()
        .clear_color();
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::WHITE
    );
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().image,
        initial_image
    );
    let asset_server = app.world().resource::<AssetServer>().clone();
    app.world_mut()
        .get_mut::<Icon>(entity)
        .unwrap()
        .set_svg(&asset_server, BuiltinIcon::WindowRestore.path());
    let deadline = Instant::now() + Duration::from_secs(2);
    while app.world().get::<ImageNode>(child).unwrap().image == initial_image {
        assert!(Instant::now() < deadline, "运行期 SVG 替换未完成");
        app.update();
        std::thread::yield_now();
    }
    app.update();
    assert_eq!(app.world().get::<Children>(entity).unwrap()[0], child);
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::WHITE
    );
    let node = app.world().get::<Node>(entity).unwrap();
    assert_eq!(node.width, px(16));
    assert_eq!(node.height, px(16));
}
