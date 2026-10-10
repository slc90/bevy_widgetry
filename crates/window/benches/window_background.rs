use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle};
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowImageBackground,
    WidgetryWindowImageMode, WidgetryWindowPlugin, owned_widgetry_window,
};

struct Fixture {
    app: App,
    roots: Vec<Entity>,
}

fn main() -> Result {
    let mut harness = Harness::new("window-background")?;
    for count in [1, 12, 100] {
        for action in ["idle", "resize", "pending"] {
            run(
                &mut harness,
                &format!("window-background/n{count}/{action}"),
                false,
                || fixture(count, action == "pending"),
                |fixture, index| {
                    if action == "resize" {
                        let size = if index.is_multiple_of(2) {
                            Vec2::new(1920.0, 1080.0)
                        } else {
                            Vec2::new(800.0, 1400.0)
                        };
                        for &root in &fixture.roots {
                            fixture
                                .app
                                .world_mut()
                                .get_mut::<ComputedNode>(root)
                                .ok_or_else(|| missing("WindowRoot ComputedNode"))?
                                .size = size;
                        }
                    }
                    fixture.app.update();
                    for &root in &fixture.roots {
                        let image = fixture
                            .app
                            .world()
                            .get::<ImageNode>(root)
                            .ok_or_else(|| missing("WindowRoot ImageNode"))?;
                        if (action == "pending") != image.rect.is_none() {
                            return Err(BevyError::error("Cover readiness invariant failed"));
                        }
                        std::hint::black_box(image.rect);
                    }
                    Ok(())
                },
                |fixture| Ok(fixture.app.world().entities().len()),
            )?;
        }
    }
    harness.finish()
}

fn fixture(count: usize, pending: bool) -> Result<Fixture> {
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    let image = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        if pending {
            images.reserve_handle()
        } else {
            images.add(Image::new_fill(
                Extent3d {
                    width: 1600,
                    height: 1091,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[255; 4],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ))
        }
    };
    let mut roots = Vec::with_capacity(count);
    for _ in 0..count {
        let background = WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
            image: image.clone(),
            mode: WidgetryWindowImageMode::Cover,
            opacity: 0.5,
        });
        let root = app.world_mut().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), background, Default::default(),  bsn_list!{}, bsn_list!{})
        }).map_err(BevyError::error)?.id();
        app.world_mut()
            .get_mut::<ComputedNode>(root)
            .ok_or_else(|| missing("WindowRoot ComputedNode"))?
            .size = Vec2::new(1920.0, 1080.0);
        roots.push(root);
    }
    settle(&mut app);
    Ok(Fixture { app, roots })
}
