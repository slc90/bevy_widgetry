use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{Scrollbar, ScrollbarThumb};
use bevy_widgetry_scroll_area::{
    ScrollAxis, WidgetryScrollArea, WidgetryScrollAreaColorOverrides, WidgetryScrollAreaPlugin,
};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app};
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Resource, Default)]
struct HoverRequest(Option<(Entity, bool)>);

struct Fixture {
    app: App,
    root: Entity,
    thumb: Entity,
    expected: Color,
}

fn apply_hover(world: &mut World) -> Result {
    // Apply after Picking so the benchmark's style stimulus survives the shared hover projection.
    if let Some((entity, hovered)) = world.resource_mut::<HoverRequest>().0.take() {
        let mut entity = world.get_entity_mut(entity).map_err(BevyError::error)?;
        if entity
            .get::<Hovered>()
            .is_none_or(|current| current.0 != hovered)
        {
            entity.insert(Hovered(hovered));
        }
    }
    Ok(())
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.add_plugins(WidgetryScrollAreaPlugin)
        .init_resource::<HoverRequest>()
        .add_systems(Update, apply_hover);
    let mut roots = Vec::new();
    for _ in 0..count {
        roots.push(
            app.world_mut()
                .spawn_scene(bsn! {
                    @WidgetryScrollArea {
                        @axis: ScrollAxis::Both,
                        @children: bsn_list! { Node { width: px(500), height: px(500), flex_shrink: 0.0 } },
                    }
                    Node { width: px(120), height: px(120) }
                })
                .map_err(BevyError::error)?
                .id(),
        );
    }
    settle(&mut app);
    let root = roots[0];
    let world = app.world();
    let bar = world
        .get::<Children>(root)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<Scrollbar>(child).is_some())
        })
        .ok_or_else(|| missing("Scrollbar"))?;
    let thumb = world
        .get::<Children>(bar)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<ScrollbarThumb>(child).is_some())
        })
        .ok_or_else(|| missing("ScrollbarThumb"))?;
    if world
        .get::<ComputedNode>(thumb)
        .is_none_or(|node| node.size().min_element() <= 0.0)
    {
        return Err(missing("laid out Thumb"));
    }
    Ok(Fixture {
        app,
        root,
        thumb,
        expected: WidgetryThemeMode::Dark
            .colors()
            .scroll_area
            .vertical
            .thumb
            .normal
            .background,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("scroll-area-style-criterion")?;
    for count in [10, 100, 1000] {
        for action in ["idle", "hover_one", "override_one"] {
            run(
                &mut harness,
                &format!("style/n{count}/{action}"),
                false,
                || fixture(count),
                |fixture, index| {
                    let active = index.is_multiple_of(2);
                    match action {
                        "hover_one" => {
                            fixture.app.world_mut().resource_mut::<HoverRequest>().0 =
                                Some((fixture.thumb, active));
                            let colors =
                                &WidgetryThemeMode::Dark.colors().scroll_area.vertical.thumb;
                            fixture.expected = if active {
                                colors.hovered.background
                            } else {
                                colors.normal.background
                            };
                        }
                        "override_one" => {
                            let mut colors = WidgetryScrollAreaColorOverrides::default();
                            fixture.expected = if active { Color::BLACK } else { Color::WHITE };
                            colors.horizontal.thumb.normal.background = Some(fixture.expected);
                            colors.vertical.thumb.normal.background = Some(fixture.expected);
                            WidgetryScrollAreaColorOverrides::set_in_world(
                                fixture.app.world_mut(),
                                fixture.root,
                                colors,
                            )?;
                        }
                        _ => {}
                    }
                    // This measures a headless App update with real UI layout, excluding GPU and native presentation.
                    fixture.app.update();
                    Ok(())
                },
                |fixture| {
                    if fixture
                        .app
                        .world()
                        .get::<BackgroundColor>(fixture.thumb)
                        .is_none_or(|color| color.0 != fixture.expected)
                    {
                        return Err(missing("expected Thumb color"));
                    }
                    Ok(fixture.app.world().entities().len())
                },
            )?;
        }
    }
    harness.finish()
}
