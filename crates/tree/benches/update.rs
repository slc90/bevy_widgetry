use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_tree::*;

#[derive(Component)]
struct Label(String);

struct Fixture {
    app: App,
    source: Entity,
    hierarchy: Entity,
    branch: Entity,
    leaf: Entity,
    root: Entity,
    viewport: Entity,
}

fn main() -> Result {
    let mut harness = Harness::new("tree-criterion")?;
    for (nodes, shape, height, rich) in [
        (1_000, "wide", 384, false),
        (10_000, "wide", 384, false),
        (100_000, "wide", 384, false),
        (10_000, "collapsed", 384, false),
        (1_000, "deep", 384, false),
        (10_000, "wide", 768, false),
        (10_000, "wide", 384, true),
    ] {
        for action in [
            "first_scene",
            "idle",
            "scroll",
            "resize",
            "visible_mutation",
            "offscreen_mutation",
            "expand_collapse",
            "hidden_idle",
            "rebuild",
            "destroy",
        ] {
            if action == "scroll" && shape != "wide" {
                continue;
            }
            run(
                &mut harness,
                &format!("tree/n{nodes}/{shape}/v640x{height}/rich{rich}/{action}"),
                matches!(action, "first_scene" | "destroy"),
                || {
                    let mut fixture = fixture(nodes, shape, rich)?;
                    if action != "first_scene" {
                        spawn_view(&mut fixture, height)?;
                        if action == "hidden_idle" {
                            fixture
                                .app
                                .world_mut()
                                .get_mut::<Node>(fixture.root)
                                .ok_or_else(|| missing("Node"))?
                                .display = Display::None;
                        }
                        settle(&mut fixture.app);
                        if fixture
                            .app
                            .world()
                            .get::<WidgetryTreeModel>(fixture.source)
                            .ok_or_else(|| missing("TreeModel"))?
                            .visible_items()
                            .is_empty()
                        {
                            return Err(missing("Tree projection"));
                        }
                    }
                    Ok(fixture)
                },
                |fixture, index| {
                    let app = &mut fixture.app;
                    match action {
                        "first_scene" => {
                            spawn_view(fixture, height)?;
                            settle(&mut fixture.app);
                        }
                        "visible_mutation" | "offscreen_mutation" => {
                            let entity = if action == "visible_mutation" {
                                fixture.branch
                            } else {
                                fixture.leaf
                            };
                            app.world_mut()
                                .get_mut::<Label>(entity)
                                .ok_or_else(|| missing("Label"))?
                                .0 = format!("Changed {index:06}");
                            app.update();
                        }
                        "expand_collapse" => {
                            WidgetryTreeModel::toggle_expand(
                                app.world_mut(),
                                fixture.source,
                                fixture.branch,
                            )?;
                            settle(app);
                        }
                        "scroll" => {
                            app.world_mut()
                                .get_mut::<ScrollPosition>(fixture.viewport)
                                .ok_or_else(|| missing("ScrollPosition"))?
                                .0
                                .y = if index.is_multiple_of(2) { 320.0 } else { 0.0 };
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
                            spawn_view(fixture, height)?;
                            settle(&mut fixture.app);
                        }
                        "destroy" => {
                            app.world_mut().despawn(fixture.root);
                            app.world_mut().despawn(fixture.source);
                            app.world_mut().despawn(fixture.hierarchy);
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

fn fixture(nodes: usize, shape: &str, rich: bool) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.add_plugins(WidgetryTreePlugin);
    app.register_renderer::<Label>(WidgetryTreeRenderer::new(move |_, label: &Label| bsn_list![(Node { column_gap: px(8) } Children [Text({label.0.clone()}), {rich.then(|| bsn! { Text("node metadata") })}])]))?;
    let hierarchy = app.world_mut().spawn_empty().id();
    let branch = app
        .world_mut()
        .spawn((WidgetryTreeNode, Label("Branch".into()), ChildOf(hierarchy)))
        .id();
    let mut leaf = branch;
    for index in 1..nodes {
        let parent = match shape {
            "deep" => leaf,
            "collapsed" => branch,
            _ if index == 1 => branch,
            _ => hierarchy,
        };
        leaf = app
            .world_mut()
            .spawn((
                WidgetryTreeNode,
                Label(format!("Node {index:06}")),
                ChildOf(parent),
            ))
            .id();
    }
    let source = app
        .world_mut()
        .spawn(WidgetryTreeModel::new(hierarchy))
        .id();
    Ok(Fixture {
        app,
        source,
        hierarchy,
        branch,
        leaf,
        root: Entity::PLACEHOLDER,
        viewport: Entity::PLACEHOLDER,
    })
}

fn spawn_view(fixture: &mut Fixture, height: u32) -> Result {
    let source = fixture.source;
    fixture.root = fixture.app.world_mut().spawn_scene(bsn! { @WidgetryTreeView { @source: source } Node { width: px(640), height: px(height as f32) } })?.id();
    fixture.viewport = fixture
        .app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(fixture.app.world())?;
    fixture
        .app
        .world_mut()
        .get_mut::<ScrollPosition>(fixture.viewport)
        .ok_or_else(|| missing("ScrollPosition"))?
        .0 = Vec2::ZERO;
    Ok(())
}
