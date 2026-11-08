const ASYNC_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
use bevy::prelude::*;
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use bevy_widgetry_core::icon::{WidgetryIcon, WidgetryIconPlugin};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run};
use bevy_widgetry_test_utils::{add_ui_plugins, advance_until, scene_app, spawn_ui_camera};
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
    let entity_counts = [10, 100, 1000];
    for count in entity_counts {
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
                            return Err(missing(&format!(
                                "expected Icon projection: color {:?} != {:?}, image {:?} != {:?}",
                                image.color,
                                fixture.color,
                                image.image.id(),
                                fixture.images[fixture.source].id()
                            )));
                        }
                    }
                    Ok(world.entities().len())
                },
            )?;
        }
        run(
            &mut harness,
            &format!("icon/n{count}/density"),
            false,
            || density_fixture(count),
            |fixture, index| {
                let scale = if index.is_multiple_of(2) { 2.0 } else { 1.0 };
                fixture
                    .base
                    .app
                    .world_mut()
                    .get_mut::<Camera>(fixture.camera)
                    .ok_or_else(|| missing("density camera"))?
                    .computed
                    .target_info
                    .as_mut()
                    .ok_or_else(|| missing("density target"))?
                    .scale_factor = scale;
                fixture.base.app.update();
                fixture.base.source = usize::from(index.is_multiple_of(2));
                Ok(())
            },
            |fixture| {
                let world = fixture.base.app.world();
                if world.resource::<Assets<Image>>().len() != fixture.image_count {
                    return Err(missing("density cache reuse"));
                }
                for &icon in &fixture.base.icons {
                    let child = world
                        .get::<Children>(icon)
                        .ok_or_else(|| missing("density child"))?[0];
                    let image = world
                        .get::<ImageNode>(child)
                        .ok_or_else(|| missing("density image"))?;
                    let node = world
                        .get::<Node>(child)
                        .ok_or_else(|| missing("density layout"))?;
                    if image.image != fixture.base.images[fixture.base.source]
                        || node.width != px(16)
                        || node.height != px(16)
                    {
                        return Err(missing("density projection with fixed logical size"));
                    }
                }
                Ok(world.entities().len())
            },
        )?;
    }
    harness.finish()
}

struct DensityFixture {
    base: Fixture,
    camera: Entity,
    image_count: usize,
}

fn density_fixture(count: usize) -> Result<DensityFixture> {
    let mut base = fixture(count)?;
    add_ui_plugins(&mut base.app);
    let camera = spawn_ui_camera(&mut base.app, UVec2::new(1920, 1080), 1.0);
    for &icon in &base.icons {
        base.app
            .world_mut()
            .entity_mut(icon)
            .insert(UiTargetCamera(camera));
    }
    let mut images = Vec::new();
    for scale in [1.0, 2.0] {
        base.app
            .world_mut()
            .get_mut::<Camera>(camera)
            .ok_or_else(|| missing("density camera"))?
            .computed
            .target_info
            .as_mut()
            .ok_or_else(|| missing("density target"))?
            .scale_factor = scale;
        base.app.update();
        let child = base
            .app
            .world()
            .get::<Children>(base.icons[0])
            .ok_or_else(|| missing("density child"))?[0];
        images.push(
            base.app
                .world()
                .get::<ImageNode>(child)
                .ok_or_else(|| missing("density image"))?
                .image
                .clone(),
        );
    }
    let mut images = images.into_iter();
    base.images = [
        images.next().ok_or_else(|| missing("density 1x"))?,
        images.next().ok_or_else(|| missing("density 2x"))?,
    ];
    // Begin at 1x so every measured operation changes density instead of repeating the warmup state.
    base.app
        .world_mut()
        .get_mut::<Camera>(camera)
        .ok_or_else(|| missing("density camera"))?
        .computed
        .target_info
        .as_mut()
        .ok_or_else(|| missing("density target"))?
        .scale_factor = 1.0;
    base.app.update();
    let image_count = base.app.world().resource::<Assets<Image>>().len();
    Ok(DensityFixture {
        base,
        camera,
        image_count,
    })
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
        ASYNC_WAIT_TIMEOUT,
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
    // Keep strong handles to both SVGs in preload roots so replacements do not trigger asset eviction and reloading.
    icons.truncate(count);
    app.update();
    let color = app
        .world()
        .resource::<bevy_widgetry_theme::WidgetryTheme>()
        .icon
        .normal
        .foreground;
    Ok(Fixture {
        app,
        icons,
        images,
        color,
        source: 0,
    })
}
