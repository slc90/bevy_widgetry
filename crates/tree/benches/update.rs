use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::color::WidgetryStyleOwner;
use bevy_widgetry_list_view::WidgetryListView;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_tree::*;

#[derive(Component)]
struct Label(String);

#[derive(Resource, Default)]
struct HoverRequest(Option<(Entity, bool)>);

fn apply_hover(world: &mut World) -> Result {
    // Picking projects Hovered before Update; inject only this local style stimulus afterwards.
    if let Some((button, hovered)) = world.resource_mut::<HoverRequest>().0.take() {
        let mut button = world.get_entity_mut(button).map_err(BevyError::error)?;
        button.insert(Hovered(hovered));
    }
    Ok(())
}

struct Fixture {
    app: App,
    source: Entity,
    hierarchy: Entity,
    branch: Entity,
    leaf: Entity,
    root: Entity,
    viewport: Entity,
    expected_hover: Option<(Entity, bool)>,
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
            "hover_one",
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
                            .get::<WidgetryTreeState>(fixture.root)
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
                        "hover_one" => {
                            let button = app.world_mut().query_filtered::<Entity, (With<WidgetryButton>, With<InheritedVisibility>)>()
                                .iter(app.world()).find(|button| app.world().get::<InheritedVisibility>(*button).is_some_and(|visibility| visibility.get()))
                                .ok_or_else(|| missing("visible Tree Expander"))?;
                            let hovered = index.is_multiple_of(2);
                            app.world_mut().resource_mut::<HoverRequest>().0 =
                                Some((button, hovered));
                            fixture.expected_hover = Some((button, hovered));
                            app.update();
                        }
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
                            WidgetryTreeView::toggle_expand(
                                app.world_mut(),
                                fixture.root,
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
                        validate_owners(&mut fixture.app, fixture.root)?;
                        if let Some((button, hovered)) = fixture.expected_hover {
                            let colors = fixture
                                .app
                                .world()
                                .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
                                .colors()
                                .tree
                                .expander
                                .collapsed;
                            let colors = if hovered {
                                colors.hovered
                            } else {
                                colors.normal
                            };
                            if fixture
                                .app
                                .world()
                                .get::<BackgroundColor>(button)
                                .is_none_or(|background| background.0 != colors.background)
                                || fixture
                                    .app
                                    .world()
                                    .get::<bevy_widgetry_core::foreground::ResolvedForeground>(
                                        button,
                                    )
                                    .is_none_or(|foreground| foreground.0 != colors.foreground)
                            {
                                return Err(missing("Tree Expander hover palette"));
                            }
                        }
                    }
                    Ok(fixture.app.world().entities().count_spawned())
                },
            )?;
        }
    }
    for action in ["idle", "expand_collapse"] {
        run(
            &mut harness,
            &format!("tree/shared/n10000/{action}"),
            false,
            || {
                let mut fixture = fixture(10_000, "collapsed", false)?;
                spawn_view(&mut fixture, 384)?;
                let source = fixture.source;
                let second = fixture
                    .app
                    .world_mut()
                    .spawn_scene(bsn! {
                        @WidgetryTreeView { @source: source }
                        Node { width: px(640), height: px(194) }
                    })?
                    .id();
                settle(&mut fixture.app);
                Ok((fixture, second))
            },
            |(fixture, _), _| {
                if action == "expand_collapse" {
                    WidgetryTreeView::toggle_expand(
                        fixture.app.world_mut(),
                        fixture.root,
                        fixture.branch,
                    )?;
                }
                fixture.app.update();
                Ok(())
            },
            |(fixture, second)| {
                validate_text(&mut fixture.app)?;
                validate_owners(&mut fixture.app, fixture.root)?;
                validate_owners(&mut fixture.app, *second)?;
                if WidgetryTreeView::state(fixture.app.world(), *second)?
                    .visible_items()
                    .len()
                    != 1
                {
                    return Err(missing("independent collapsed shared TreeView"));
                }
                Ok(fixture.app.world().entities().count_spawned())
            },
        )?;
    }
    harness.finish()
}

fn validate_owners(app: &mut App, root: Entity) -> Result {
    let world = app.world_mut();
    let list = world
        .get::<Children>(root)
        .and_then(|children| {
            children.iter().find(|child| {
                world
                    .get::<WidgetryListView<WidgetryTreeVisibleItem, Entity>>(*child)
                    .is_some()
            })
        })
        .ok_or_else(|| missing("internal Tree ListView"))?;
    if world
        .get::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem, Entity>>>(list)
        .is_none_or(|owner| owner.entity != root)
    {
        return Err(missing("TreeView ListView style owner"));
    }
    for (entity, owner) in world
        .query_filtered::<(Entity, Option<&WidgetryStyleOwner<WidgetryButton>>), With<WidgetryButton>>()
        .iter(world)
    {
        let mut current = entity;
        while world.get::<WidgetryTreeView>(current).is_none() {
            let Some(parent) = world.get::<ChildOf>(current) else { break; };
            current = parent.parent();
        }
        if current == root && owner.is_none_or(|owner| owner.entity != root) {
            return Err(missing("TreeView Expander style owner"));
        }
    }
    Ok(())
}

fn fixture(nodes: usize, shape: &str, rich: bool) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.add_plugins(WidgetryTreePlugin);
    app.init_resource::<HoverRequest>()
        .add_systems(Update, apply_hover);
    app.register_renderer::<Label>(WidgetryTreeRenderer::new(move |_, label: &Label| bsn_list!{Node { column_gap: px(8) } Children [Text({label.0.clone()}) bevy_widgetry_core::text::WidgetryText-- {rich.then_some(bsn! { Text("node metadata") bevy_widgetry_core::text::WidgetryText })}]}))?;
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
        expected_hover: None,
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
