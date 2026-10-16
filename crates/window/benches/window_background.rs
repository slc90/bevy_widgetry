use bevy::asset::RenderAssetUsages;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::Pressed;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle};
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowImageBackground,
    WidgetryWindowImageMode, WidgetryWindowPlugin, owned_widgetry_window,
};

struct Fixture {
    app: App,
    roots: Vec<Entity>,
    button: Entity,
}

fn main() -> Result {
    let mut harness = Harness::new("window-background")?;
    for count in [1, 12, 100] {
        for action in [
            "idle",
            "resize",
            "pending",
            "theme_idle",
            "hover_one",
            "press_one",
        ] {
            run(
                &mut harness,
                &format!("window-background/n{count}/{action}"),
                false,
                || fixture(count, action),
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
                    if action == "hover_one" {
                        fixture
                            .app
                            .world_mut()
                            .entity_mut(fixture.button)
                            .insert(Hovered(index.is_multiple_of(2)));
                    }
                    if action == "press_one" {
                        if index.is_multiple_of(2) {
                            fixture
                                .app
                                .world_mut()
                                .entity_mut(fixture.button)
                                .insert(Pressed);
                        } else {
                            fixture
                                .app
                                .world_mut()
                                .entity_mut(fixture.button)
                                .remove::<Pressed>();
                        }
                    }
                    fixture.app.update();
                    for &root in &fixture.roots {
                        if matches!(action, "theme_idle" | "hover_one" | "press_one") {
                            let colors =
                                bevy_widgetry_theme::WidgetryThemeMode::Dark.colors().window;
                            let background = fixture
                                .app
                                .world()
                                .get::<BackgroundColor>(root)
                                .ok_or_else(|| missing("WindowRoot BackgroundColor"))?;
                            if background.0 != colors.frame.normal.background {
                                return Err(BevyError::error(
                                    "Theme Window background invariant failed",
                                ));
                            }
                            continue;
                        }
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
                    let colors = bevy_widgetry_theme::WidgetryThemeMode::Dark
                        .colors()
                        .window
                        .close;
                    let expected = if action == "press_one" && index.is_multiple_of(2) {
                        colors.pressed
                    } else if action == "hover_one" && index.is_multiple_of(2) {
                        colors.hovered
                    } else {
                        colors.normal
                    };
                    let background = fixture
                        .app
                        .world()
                        .get::<BackgroundColor>(fixture.button)
                        .ok_or_else(|| missing("CloseButton BackgroundColor"))?;
                    let foreground = fixture
                        .app
                        .world()
                        .get::<bevy_widgetry_core::foreground::ResolvedForeground>(fixture.button)
                        .ok_or_else(|| missing("CloseButton ResolvedForeground"))?;
                    if background.0 != expected.background || foreground.0 != expected.foreground {
                        return Err(BevyError::error(
                            "Window button input/color invariant failed",
                        ));
                    }
                    Ok(())
                },
                |fixture| Ok(fixture.app.world().entities().len()),
            )?;
        }
    }
    harness.finish()
}

fn fixture(count: usize, action: &str) -> Result<Fixture> {
    let mut app = scene_app();
    app.add_plugins(WidgetryWindowPlugin);
    let image = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        if action == "pending" {
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
        let image_background = WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
            image: image.clone(),
            mode: WidgetryWindowImageMode::Cover,
            opacity: 0.5,
        });
        let background = if matches!(action, "theme_idle" | "hover_one" | "press_one") {
            WidgetryWindowBackground::Theme
        } else {
            image_background
        };
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
    let title = app
        .world()
        .get::<Children>(roots[0])
        .and_then(|children| children.first())
        .copied()
        .ok_or_else(|| missing("TitleBar"))?;
    let controls = app
        .world()
        .get::<Children>(title)
        .and_then(|children| children.last())
        .copied()
        .ok_or_else(|| missing("WindowControls"))?;
    let button = app
        .world()
        .get::<Children>(controls)
        .and_then(|children| children.last())
        .copied()
        .ok_or_else(|| missing("CloseButton"))?;
    Ok(Fixture { app, roots, button })
}
