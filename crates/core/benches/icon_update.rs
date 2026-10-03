use bevy::prelude::*;
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run};
use bevy_widgetry_test_utils::{advance_until, scene_app};
use std::time::Duration;

struct Fixture {
    app: App,
    icons: Vec<Entity>,
    images: [Handle<Image>; 2],
    color: Color,
    source: usize,
}

fn main() -> Result {
    let mut harness = Harness::new("icon-update-criterion")?;
    for count in [10, 100, 1000] {
        for action in ["idle", "color", "svg", "queued_color", "queued_svg"] {
            run(
                &mut harness,
                &format!("icon/n{count}/{action}"),
                false,
                || fixture(count),
                |fixture, index| {
                    let world = fixture.app.world_mut();
                    for &entity in &fixture.icons {
                        match action {
                            "color" => {
                                WidgetryIcon::set_color_in_world(
                                    world,
                                    entity,
                                    if index.is_multiple_of(2) {
                                        Color::BLACK
                                    } else {
                                        Color::WHITE
                                    },
                                )?;
                            }
                            "svg" => {
                                WidgetryIcon::set_svg_in_world(
                                    world,
                                    entity,
                                    if index.is_multiple_of(2) {
                                        BuiltinIcon::WindowClose
                                    } else {
                                        BuiltinIcon::WindowRestore
                                    }
                                    .path(),
                                )?;
                            }
                            "queued_color" => {
                                WidgetryIcon::set_color(
                                    &mut world.commands(),
                                    entity,
                                    if index.is_multiple_of(2) {
                                        Color::BLACK
                                    } else {
                                        Color::WHITE
                                    },
                                );
                            }
                            "queued_svg" => {
                                WidgetryIcon::set_svg(
                                    &mut world.commands(),
                                    entity,
                                    if index.is_multiple_of(2) {
                                        BuiltinIcon::WindowClose
                                    } else {
                                        BuiltinIcon::WindowRestore
                                    }
                                    .path(),
                                );
                            }
                            _ => {}
                        }
                    }
                    fixture.app.update();
                    if matches!(action, "color" | "queued_color") {
                        fixture.color = if index.is_multiple_of(2) {
                            Color::BLACK
                        } else {
                            Color::WHITE
                        };
                    }
                    if matches!(action, "svg" | "queued_svg") {
                        fixture.source = usize::from(!index.is_multiple_of(2));
                    }
                    Ok(())
                },
                |fixture| {
                    let world = fixture.app.world();
                    for &icon in &fixture.icons {
                        let child = world
                            .get::<Children>(icon)
                            .ok_or_else(|| missing("Icon child"))?[0];
                        let image = world
                            .get::<ImageNode>(child)
                            .ok_or_else(|| missing("ImageNode"))?;
                        if image.color != fixture.color
                            || image.image != fixture.images[fixture.source]
                        {
                            return Err(missing("expected Icon projection"));
                        }
                    }
                    Ok(world.entities().len())
                },
            )?;
        }
    }
    harness.finish()
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut app = scene_app();
    app.add_plugins((WidgetryAssetPlugin, WidgetryIconPlugin));
    let mut icons = Vec::with_capacity(count + 2);
    for index in 0..count + 2 {
        let path = if index == count + 1 {
            BuiltinIcon::WindowRestore
        } else {
            BuiltinIcon::WindowClose
        }
        .path();
        let entity = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryIcon { @path: {path}, @max_size: {Some(UVec2::splat(16))} }
            })?
            .id();
        icons.push(entity);
    }
    advance_until(
        &mut app,
        Duration::from_secs(5),
        "benchmark Icon images",
        |world| {
            icons.iter().all(|entity| {
                world.get::<Children>(*entity).is_some_and(|children| {
                    children
                        .iter()
                        .any(|child| world.get::<ImageNode>(child).is_some())
                })
            })
        },
    )?;
    let images = [icons[count], icons[count + 1]].map(|icon| {
        let child = app
            .world()
            .get::<Children>(icon)
            .ok_or_else(|| missing("Icon child"))?[0];
        Ok::<_, BevyError>(
            app.world()
                .get::<ImageNode>(child)
                .ok_or_else(|| missing("ImageNode"))?
                .image
                .clone(),
        )
    });
    let [close, restore] = images;
    let images = [close?, restore?];
    // 两个 preload root 保持两种 SVG 的 strong handle，避免持续替换触发 asset 回收与再次加载。
    icons.truncate(count);
    app.update();
    Ok(Fixture {
        app,
        icons,
        images,
        color: Color::WHITE,
        source: 0,
    })
}
