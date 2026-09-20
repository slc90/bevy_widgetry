use bevy::prelude::*;
use bevy::window::RequestRedraw;
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin};
use std::time::{Duration, Instant};

// props 只初始化一次；image 生成后，颜色覆盖、清除与 SVG 替换都由 WidgetryIcon 运行期 state 驱动。
#[test]
fn runtime_mutations_survive_scene_initialization() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        WidgetryAssetPlugin,
        WidgetryIconPlugin,
    ))
    .init_asset::<Image>();
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryIcon {
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
        .get_mut::<WidgetryIcon>(entity)
        .unwrap()
        .set_color(Color::srgb(1.0, 0.0, 0.0));
    app.update();
    assert_eq!(
        app.world().get::<ImageNode>(child).unwrap().color,
        Color::srgb(1.0, 0.0, 0.0)
    );
    app.world_mut()
        .get_mut::<WidgetryIcon>(entity)
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
        .get_mut::<WidgetryIcon>(entity)
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

// 按需刷新时，仅允许 WidgetryIcon 发出的请求推进后续帧；首次加载和替换均应完成，稳定后停止请求。
#[test]
fn asynchronous_icons_request_redraw_until_ready() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        WidgetryAssetPlugin,
        WidgetryIconPlugin,
    ))
    .init_asset::<Image>()
    .add_message::<RequestRedraw>();
    let icon = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()} }
        })
        .unwrap()
        .id();
    let server = app.world().resource::<AssetServer>().clone();
    let mut previous = None;
    for path in [BuiltinIcon::WindowClose, BuiltinIcon::WindowRestore] {
        app.world_mut()
            .get_mut::<WidgetryIcon>(icon)
            .unwrap()
            .set_svg(&server, path.path());
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            app.world_mut()
                .resource_mut::<Messages<RequestRedraw>>()
                .clear();
            app.update();
            let requested = !app.world().resource::<Messages<RequestRedraw>>().is_empty();
            let image = app.world().get::<Children>(icon).and_then(|children| {
                app.world()
                    .get::<ImageNode>(children[0])
                    .map(|node| node.image.clone())
            });
            assert!(requested, "图标尚需加载或新图像尚需提交时必须请求刷新");
            if image.is_some() && image != previous {
                previous = image;
                break;
            }
            assert!(Instant::now() < deadline, "仅由刷新请求驱动时图标未能完成");
            std::thread::yield_now();
        }
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        app.update();
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }
}
