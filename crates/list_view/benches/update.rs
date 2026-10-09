use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_widgetry_list_view::*;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};

struct Fixture {
    app: App,
    source: Entity,
    root: Entity,
    viewport: Entity,
}

fn main() -> Result {
    let mut harness = Harness::new("list_view-criterion")?;
    for (items, height, rich) in [
        (1_000, 384, false),
        (10_000, 384, false),
        (100_000, 384, false),
        (10_000, 768, false),
        (10_000, 384, true),
    ] {
        for action in [
            "first_scene",
            "idle",
            "scroll",
            "visible_mutation",
            "offscreen_mutation",
            "resize",
            "hidden_idle",
            "rebuild",
            "destroy",
        ] {
            run(
                &mut harness,
                &format!("list/n{items}/v640x{height}/rich{rich}/{action}"),
                matches!(action, "first_scene" | "destroy"),
                || {
                    let mut app = ui_app()?;
                    app.add_plugins(WidgetryListViewPlugin);
                    app.register_widgetry_list_view::<String>()?;
                    let mut model = WidgetryListModel::default();
                    for index in 0..items {
                        model.push(format!("Item {index:06}: value = 123.45"))?;
                    }
                    let source = app.world_mut().spawn(model).id();
                    let mut fixture = Fixture {
                        app,
                        source,
                        root: Entity::PLACEHOLDER,
                        viewport: Entity::PLACEHOLDER,
                    };
                    if action != "first_scene" {
                        spawn_view(&mut fixture, height, rich)?;
                        if action == "hidden_idle" {
                            fixture
                                .app
                                .world_mut()
                                .get_mut::<Node>(fixture.root)
                                .ok_or_else(|| missing("Node"))?
                                .display = Display::None;
                        }
                        settle(&mut fixture.app);
                        let world = fixture.app.world_mut();
                        if world.query::<&WidgetryListViewItem>().iter(world).count() == 0
                            && action != "hidden_idle"
                        {
                            return Err(missing("ListView visible rows"));
                        }
                    }
                    Ok(fixture)
                },
                |fixture, index| {
                    let app = &mut fixture.app;
                    match action {
                        "first_scene" => {
                            spawn_view(fixture, height, rich)?;
                            settle(&mut fixture.app);
                        }
                        "scroll" => {
                            app.world_mut()
                                .get_mut::<ScrollPosition>(fixture.viewport)
                                .ok_or_else(|| missing("ScrollPosition"))?
                                .0
                                .y = if index.is_multiple_of(2) { 320.0 } else { 0.0 };
                            app.update();
                        }
                        "visible_mutation" | "offscreen_mutation" => {
                            let row = if action == "visible_mutation" {
                                0
                            } else {
                                items - 1
                            };
                            {
                                let mut model = app
                                    .world_mut()
                                    .get_mut::<WidgetryListModel<String>>(fixture.source)
                                    .ok_or_else(|| missing("ListModel"))?;
                                *model.get_mut(row)?.ok_or_else(|| missing("item"))? =
                                    format!("Changed {index:06}: value = 123.45");
                            }
                            app.update();
                        }
                        "resize" => {
                            app.world_mut()
                                .get_mut::<Node>(fixture.root)
                                .ok_or_else(|| missing("Node"))?
                                .height =
                                px(height as f32
                                    + if index.is_multiple_of(2) { 64.0 } else { 0.0 });
                            settle(app);
                        }
                        "rebuild" => {
                            app.world_mut().despawn(fixture.root);
                            spawn_view(fixture, height, rich)?;
                            settle(&mut fixture.app);
                        }
                        "destroy" => {
                            app.world_mut().despawn(fixture.root);
                            app.world_mut().despawn(fixture.source);
                            app.update();
                        }
                        _ => app.update(),
                    }
                    Ok(())
                },
                |fixture| {
                    if fixture.root != Entity::PLACEHOLDER
                        && !matches!(action, "hidden_idle" | "destroy")
                    {
                        validate_text(&mut fixture.app)?;
                    }
                    Ok(fixture.app.world().entities().count_spawned())
                },
            )?;
        }
    }
    harness.finish()
}

fn spawn_view(fixture: &mut Fixture, height: u32, rich: bool) -> Result {
    let source = fixture.source;
    fixture.root = fixture.app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(move |_, value: &String| bsn_list![(Node { column_gap: px(8) } Children [Text({value.clone()}) bevy_widgetry_core::text::WidgetryText, {rich.then(|| bsn! { Text("status: active") bevy_widgetry_core::text::WidgetryText })}])])},
        }
        Node { width: px(640), height: px(height as f32) }
    })?.id();
    fixture.viewport = fixture
        .app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(fixture.app.world())?;
    Ok(())
}
