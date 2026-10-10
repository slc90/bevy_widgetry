use bevy::prelude::*;
use bevy::ui::{ComputedNode, GridPlacement};
use bevy::window::RequestRedraw;

use crate::headless::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollAreaContent,
    WidgetryScrollAreaViewport,
};

pub(crate) const DEFAULT_SCROLLBAR_THICKNESS: f32 = 12.0;

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ScrollAreaConfig {
    pub axis: ScrollAxis,
    pub scrollbar_visibility: ScrollbarVisibility,
    pub scrollbar_thickness: f32,
    pub keyboard_scroll: bool,
}

impl Default for ScrollAreaConfig {
    fn default() -> Self {
        Self {
            axis: ScrollAxis::default(),
            scrollbar_visibility: ScrollbarVisibility::default(),
            scrollbar_thickness: DEFAULT_SCROLLBAR_THICKNESS,
            keyboard_scroll: true,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct HorizontalScrollbar;

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct VerticalScrollbar;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Measurement {
    root: Vec2,
    viewport: Vec2,
    scrollable: Vec2,
    content: Vec2,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
enum Convergence {
    Solving,
    Stable(Measurement),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct VisibleBars {
    horizontal: bool,
    vertical: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Decision {
    bars: VisibleBars,
    state: Convergence,
    redraw: bool,
}

#[derive(Clone, Copy)]
struct Parts {
    viewport: Entity,
    content: Entity,
    horizontal: Option<Entity>,
    vertical: Option<Entity>,
}

fn has_horizontal(axis: ScrollAxis) -> bool {
    matches!(axis, ScrollAxis::Horizontal | ScrollAxis::Both)
}

fn has_vertical(axis: ScrollAxis) -> bool {
    matches!(axis, ScrollAxis::Vertical | ScrollAxis::Both)
}

fn initial_bars(config: ScrollAreaConfig) -> VisibleBars {
    VisibleBars {
        horizontal: has_horizontal(config.axis)
            && config.scrollbar_visibility.horizontal == ScrollbarPolicy::Always,
        vertical: has_vertical(config.axis)
            && config.scrollbar_visibility.vertical == ScrollbarPolicy::Always,
    }
}

fn advance(
    config: ScrollAreaConfig,
    current: VisibleBars,
    state: Convergence,
    measured: Measurement,
) -> Decision {
    if let Convergence::Stable(previous) = state {
        if previous != measured {
            return Decision {
                bars: initial_bars(config),
                state: Convergence::Solving,
                redraw: true,
            };
        }
        return Decision {
            bars: current,
            state,
            redraw: false,
        };
    }

    let mut next = current;
    if has_vertical(config.axis)
        && config.scrollbar_visibility.vertical == ScrollbarPolicy::Auto
        && !next.vertical
        && measured.scrollable.y > measured.viewport.y
    {
        next.vertical = true;
    }
    if has_horizontal(config.axis)
        && config.scrollbar_visibility.horizontal == ScrollbarPolicy::Auto
        && !next.horizontal
        && measured.scrollable.x > measured.viewport.x
    {
        next.horizontal = true;
    }
    let changed = next != current;
    Decision {
        bars: next,
        state: if changed {
            Convergence::Solving
        } else {
            Convergence::Stable(measured)
        },
        redraw: changed,
    }
}

fn parts(world: &World, root: Entity, axis: ScrollAxis) -> Option<Parts> {
    let children = world.get::<Children>(root)?;
    let viewport = children
        .iter()
        .find(|&entity| world.get::<WidgetryScrollAreaViewport>(entity).is_some())?;
    let content = world
        .get::<Children>(viewport)?
        .iter()
        .find(|&entity| world.get::<WidgetryScrollAreaContent>(entity).is_some())?;
    let horizontal = children
        .iter()
        .find(|&entity| world.get::<HorizontalScrollbar>(entity).is_some());
    let vertical = children
        .iter()
        .find(|&entity| world.get::<VerticalScrollbar>(entity).is_some());
    if (has_horizontal(axis) && horizontal.is_none()) || (has_vertical(axis) && vertical.is_none())
    {
        return None;
    }
    Some(Parts {
        viewport,
        content,
        horizontal,
        vertical,
    })
}

fn measurement(world: &World, root: Entity, parts: Parts) -> Option<Measurement> {
    let root_node = world.get::<ComputedNode>(root)?;
    let viewport_node = world.get::<ComputedNode>(parts.viewport)?;
    let content_node = world.get::<ComputedNode>(parts.content)?;
    Some(Measurement {
        root: root_node.size(),
        viewport: viewport_node.size(),
        scrollable: viewport_node.content_size(),
        content: content_node.size(),
    })
}

fn set_display(world: &mut World, entity: Option<Entity>, visible: bool) {
    if let Some(entity) = entity
        && let Some(mut node) = world.get_mut::<Node>(entity)
    {
        let display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

pub(crate) fn configure_geometry(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, (With<ScrollAreaConfig>, Without<Convergence>)>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let Some(&config) = world.get::<ScrollAreaConfig>(root) else {
            continue;
        };
        let Some(parts) = parts(world, root, config.axis) else {
            continue;
        };
        if world.get::<Node>(root).is_none()
            || world.get::<Node>(parts.viewport).is_none()
            || parts
                .horizontal
                .is_some_and(|entity| world.get::<Node>(entity).is_none())
            || parts
                .vertical
                .is_some_and(|entity| world.get::<Node>(entity).is_none())
        {
            continue;
        }
        if let Some(mut node) = world.get_mut::<Node>(parts.viewport) {
            node.grid_column = GridPlacement::start(1);
            node.grid_row = GridPlacement::start(1);
            node.overflow = config.axis.overflow();
            node.scrollbar_width = 0.0;
        }
        if let Some(horizontal) = parts.horizontal
            && let Some(mut node) = world.get_mut::<Node>(horizontal)
        {
            node.grid_column = GridPlacement::start(1);
            node.grid_row = GridPlacement::start(2);
            node.height = px(config.scrollbar_thickness);
        }
        if let Some(horizontal) = parts.horizontal {
            if let Some(mut scrollbar) = world.get_mut::<bevy::ui_widgets::Scrollbar>(horizontal) {
                scrollbar.target = parts.viewport;
            } else if let Some(mut scrollbar) =
                world.get_mut::<crate::disabled::SuspendedScrollbar>(horizontal)
            {
                scrollbar.0.target = parts.viewport;
            }
            crate::disabled::sync(world, horizontal);
        }
        if let Some(vertical) = parts.vertical
            && let Some(mut node) = world.get_mut::<Node>(vertical)
        {
            node.grid_column = GridPlacement::start(2);
            node.grid_row = GridPlacement::start_span(1, 2);
            node.width = px(config.scrollbar_thickness);
        }
        if let Some(vertical) = parts.vertical {
            if let Some(mut scrollbar) = world.get_mut::<bevy::ui_widgets::Scrollbar>(vertical) {
                scrollbar.target = parts.viewport;
            } else if let Some(mut scrollbar) =
                world.get_mut::<crate::disabled::SuspendedScrollbar>(vertical)
            {
                scrollbar.0.target = parts.viewport;
            }
            crate::disabled::sync(world, vertical);
        }
        let bars = initial_bars(config);
        set_display(world, parts.horizontal, bars.horizontal);
        set_display(world, parts.vertical, bars.vertical);
        world.entity_mut(root).insert(Convergence::Solving);
    }
}

pub(crate) fn solve_visibility(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, (With<ScrollAreaConfig>, With<Convergence>)>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let Some(&config) = world.get::<ScrollAreaConfig>(root) else {
            continue;
        };
        let Some(parts) = parts(world, root, config.axis) else {
            continue;
        };
        let Some(measured) = measurement(world, root, parts) else {
            continue;
        };
        let Some(&state) = world.get::<Convergence>(root) else {
            continue;
        };
        let current = VisibleBars {
            horizontal: parts.horizontal.is_some_and(|entity| {
                world
                    .get::<Node>(entity)
                    .is_some_and(|node| node.display != Display::None)
            }),
            vertical: parts.vertical.is_some_and(|entity| {
                world
                    .get::<Node>(entity)
                    .is_some_and(|node| node.display != Display::None)
            }),
        };
        let decision = advance(config, current, state, measured);
        set_display(world, parts.horizontal, decision.bars.horizontal);
        set_display(world, parts.vertical, decision.bars.vertical);
        if decision.state != state {
            world.entity_mut(root).insert(decision.state);
        }
        if decision.redraw {
            world.write_message(RequestRedraw);
        }
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::ui::RepeatedGridTrack;
    use bevy::ui_widgets::ScrollArea;
    use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};

    fn config(
        axis: ScrollAxis,
        horizontal: ScrollbarPolicy,
        vertical: ScrollbarPolicy,
    ) -> ScrollAreaConfig {
        ScrollAreaConfig {
            axis,
            scrollbar_visibility: ScrollbarVisibility {
                horizontal,
                vertical,
            },
            ..default()
        }
    }

    fn measured(viewport: Vec2, scrollable: Vec2) -> Measurement {
        Measurement {
            root: Vec2::splat(120.0),
            viewport,
            scrollable,
            content: scrollable,
        }
    }

    fn solve_once(config: ScrollAreaConfig, measured: Measurement) -> Decision {
        advance(config, initial_bars(config), Convergence::Solving, measured)
    }

    #[test]
    fn auto_stays_hidden_without_strict_overflow() {
        let config = config(
            ScrollAxis::Both,
            ScrollbarPolicy::Auto,
            ScrollbarPolicy::Auto,
        );
        for content in [Vec2::splat(80.0), Vec2::splat(100.0)] {
            let size = measured(Vec2::splat(100.0), content);
            assert_eq!(
                solve_once(config, size),
                Decision {
                    bars: VisibleBars::default(),
                    state: Convergence::Stable(size),
                    redraw: false
                }
            );
        }
    }

    #[test]
    fn auto_and_axis_policies_select_only_existing_overflow() {
        let both = config(
            ScrollAxis::Both,
            ScrollbarPolicy::Auto,
            ScrollbarPolicy::Auto,
        );
        for (content, expected) in [
            (
                Vec2::new(150.0, 80.0),
                VisibleBars {
                    horizontal: true,
                    vertical: false,
                },
            ),
            (
                Vec2::new(80.0, 150.0),
                VisibleBars {
                    horizontal: false,
                    vertical: true,
                },
            ),
            (
                Vec2::splat(150.0),
                VisibleBars {
                    horizontal: true,
                    vertical: true,
                },
            ),
        ] {
            let decision = solve_once(both, measured(Vec2::splat(100.0), content));
            assert_eq!(decision.bars, expected);
            assert_eq!(decision.state, Convergence::Solving);
            assert!(decision.redraw);
        }
        let vertical = config(
            ScrollAxis::Vertical,
            ScrollbarPolicy::Always,
            ScrollbarPolicy::Hidden,
        );
        assert_eq!(initial_bars(vertical), VisibleBars::default());
        assert_eq!(
            solve_once(vertical, measured(Vec2::splat(100.0), Vec2::splat(150.0))).bars,
            VisibleBars::default()
        );
    }

    #[test]
    fn always_and_hidden_do_not_follow_overflow() {
        let config = config(
            ScrollAxis::Both,
            ScrollbarPolicy::Hidden,
            ScrollbarPolicy::Always,
        );
        let size = measured(Vec2::splat(100.0), Vec2::splat(100.0));
        assert_eq!(
            solve_once(config, size).bars,
            VisibleBars {
                horizontal: false,
                vertical: true
            }
        );
        let overflowing = measured(Vec2::splat(100.0), Vec2::splat(200.0));
        assert_eq!(
            solve_once(config, overflowing).bars,
            VisibleBars {
                horizontal: false,
                vertical: true
            }
        );
    }

    #[test]
    fn vertical_gutter_can_induce_horizontal_auto() {
        let config = config(
            ScrollAxis::Both,
            ScrollbarPolicy::Auto,
            ScrollbarPolicy::Auto,
        );
        let first = solve_once(
            config,
            measured(Vec2::splat(100.0), Vec2::new(100.0, 150.0)),
        );
        assert_eq!(
            first.bars,
            VisibleBars {
                horizontal: false,
                vertical: true
            }
        );
        let second = advance(
            config,
            first.bars,
            first.state,
            measured(Vec2::new(88.0, 100.0), Vec2::new(100.0, 150.0)),
        );
        assert_eq!(
            second.bars,
            VisibleBars {
                horizontal: true,
                vertical: true
            }
        );
        let third = advance(
            config,
            second.bars,
            second.state,
            measured(Vec2::splat(88.0), Vec2::new(100.0, 150.0)),
        );
        assert_eq!(
            third.state,
            Convergence::Stable(measured(Vec2::splat(88.0), Vec2::new(100.0, 150.0)))
        );
        assert!(!third.redraw);
    }

    #[test]
    fn horizontal_gutter_can_induce_vertical_auto() {
        let config = config(
            ScrollAxis::Both,
            ScrollbarPolicy::Auto,
            ScrollbarPolicy::Auto,
        );
        let first = solve_once(
            config,
            measured(Vec2::splat(100.0), Vec2::new(150.0, 100.0)),
        );
        assert_eq!(
            first.bars,
            VisibleBars {
                horizontal: true,
                vertical: false
            }
        );
        let second = advance(
            config,
            first.bars,
            first.state,
            measured(Vec2::new(100.0, 88.0), Vec2::new(150.0, 100.0)),
        );
        assert_eq!(
            second.bars,
            VisibleBars {
                horizontal: true,
                vertical: true
            }
        );
        assert_eq!(second.state, Convergence::Solving);
        assert!(second.redraw);
    }

    #[test]
    fn stable_geometry_change_restarts_from_minimum() {
        let config = config(
            ScrollAxis::Both,
            ScrollbarPolicy::Auto,
            ScrollbarPolicy::Auto,
        );
        let previous = measured(Vec2::splat(88.0), Vec2::splat(150.0));
        let new = measured(Vec2::splat(100.0), Vec2::new(80.0, 150.0));
        let reset = advance(
            config,
            VisibleBars {
                horizontal: true,
                vertical: true,
            },
            Convergence::Stable(previous),
            new,
        );
        assert_eq!(reset.bars, VisibleBars::default());
        assert_eq!(reset.state, Convergence::Solving);
        assert!(reset.redraw);
        let single = advance(config, reset.bars, reset.state, new);
        assert_eq!(
            single.bars,
            VisibleBars {
                horizontal: false,
                vertical: true
            }
        );
        let stable = advance(config, single.bars, single.state, new);
        assert_eq!(stable.state, Convergence::Stable(new));
        let none = measured(Vec2::splat(100.0), Vec2::splat(80.0));
        let reset_again = advance(config, stable.bars, stable.state, none);
        assert_eq!(reset_again.bars, VisibleBars::default());
        assert_eq!(
            advance(config, reset_again.bars, reset_again.state, none).state,
            Convergence::Stable(none)
        );
    }

    #[test]
    fn configuration_applies_grid_geometry_and_visibility() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app.world_mut().spawn_scene(bsn! {
            Node {
                display: Display::Grid,
                grid_template_columns: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
                grid_template_rows: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
            }
            ScrollAreaConfig { axis: ScrollAxis::Both }
            Children [
                Node ScrollArea WidgetryScrollAreaViewport Children [Node WidgetryScrollAreaContent]--
                Node HorizontalScrollbar--
                Node VerticalScrollbar
            ]
        }).unwrap().id();
        app.update();
        let children = app.world().get::<Children>(root).unwrap();
        let viewport = children
            .iter()
            .find(|&entity| {
                app.world()
                    .get::<WidgetryScrollAreaViewport>(entity)
                    .is_some()
            })
            .unwrap();
        let horizontal = children
            .iter()
            .find(|&entity| app.world().get::<HorizontalScrollbar>(entity).is_some())
            .unwrap();
        let vertical = children
            .iter()
            .find(|&entity| app.world().get::<VerticalScrollbar>(entity).is_some())
            .unwrap();
        let root_node = app.world().get::<Node>(root).unwrap();
        assert_eq!(root_node.display, Display::Grid);
        assert_eq!(
            root_node.grid_template_columns,
            vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)]
        );
        assert_eq!(
            root_node.grid_template_rows,
            vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)]
        );
        let viewport_node = app.world().get::<Node>(viewport).unwrap();
        assert_eq!(viewport_node.grid_column, GridPlacement::start(1));
        assert_eq!(viewport_node.grid_row, GridPlacement::start(1));
        assert_eq!(viewport_node.overflow, ScrollAxis::Both.overflow());
        assert_eq!(viewport_node.scrollbar_width, 0.0);
        let horizontal_node = app.world().get::<Node>(horizontal).unwrap();
        assert_eq!(horizontal_node.grid_column, GridPlacement::start(1));
        assert_eq!(horizontal_node.grid_row, GridPlacement::start(2));
        assert_eq!(horizontal_node.height, px(12));
        assert_eq!(horizontal_node.display, Display::None);
        let vertical_node = app.world().get::<Node>(vertical).unwrap();
        assert_eq!(vertical_node.grid_column, GridPlacement::start(2));
        assert_eq!(vertical_node.grid_row, GridPlacement::start_span(1, 2));
        assert_eq!(vertical_node.width, px(12));
        assert_eq!(vertical_node.display, Display::None);
        assert_eq!(ScrollAreaConfig::default().scrollbar_thickness, 12.0);
    }

    #[test]
    fn scheduled_solver_updates_nodes_and_requests_redraw() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app.world_mut().spawn_scene(bsn! {
            Node
            ScrollAreaConfig { axis: ScrollAxis::Both, scrollbar_thickness: 16.0 }
            Children [
                Node ScrollArea WidgetryScrollAreaViewport Children [Node WidgetryScrollAreaContent]--
                Node HorizontalScrollbar--
                Node VerticalScrollbar
            ]
        }).unwrap().id();
        app.update();
        let children = app.world().get::<Children>(root).unwrap();
        let viewport = children
            .iter()
            .find(|&entity| {
                app.world()
                    .get::<WidgetryScrollAreaViewport>(entity)
                    .is_some()
            })
            .unwrap();
        let horizontal = children
            .iter()
            .find(|&entity| app.world().get::<HorizontalScrollbar>(entity).is_some())
            .unwrap();
        let vertical = children
            .iter()
            .find(|&entity| app.world().get::<VerticalScrollbar>(entity).is_some())
            .unwrap();
        let content = app
            .world()
            .get::<Children>(viewport)
            .unwrap()
            .iter()
            .next()
            .unwrap();
        assert_eq!(app.world().get::<Node>(horizontal).unwrap().height, px(16));
        assert_eq!(app.world().get::<Node>(vertical).unwrap().width, px(16));
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::splat(100.0),
            content_size: Vec2::new(100.0, 150.0),
            ..default()
        });
        app.world_mut().entity_mut(content).insert(ComputedNode {
            size: Vec2::new(100.0, 150.0),
            ..default()
        });
        app.update();
        assert_eq!(
            *app.world().get::<Convergence>(root).unwrap(),
            Convergence::Solving
        );
        assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
        app.update();
        assert_eq!(
            app.world().get::<Node>(vertical).unwrap().display,
            Display::Flex
        );
        assert_eq!(
            app.world().get::<Node>(horizontal).unwrap().display,
            Display::None
        );
        app.world_mut()
            .get_mut::<ComputedNode>(viewport)
            .unwrap()
            .size = Vec2::new(84.0, 100.0);
        app.update();
        assert_eq!(
            app.world().get::<Node>(horizontal).unwrap().display,
            Display::Flex
        );
        app.world_mut()
            .get_mut::<ComputedNode>(viewport)
            .unwrap()
            .size = Vec2::splat(84.0);
        app.update();
        assert!(matches!(
            app.world().get::<Convergence>(root),
            Some(Convergence::Stable(_))
        ));
        app.world_mut()
            .get_mut::<ComputedNode>(viewport)
            .unwrap()
            .content_size = Vec2::splat(80.0);
        app.update();
        assert_eq!(
            app.world().get::<Node>(horizontal).unwrap().display,
            Display::None
        );
        assert_eq!(
            app.world().get::<Node>(vertical).unwrap().display,
            Display::None
        );
        app.update();
        assert!(matches!(
            app.world().get::<Convergence>(root),
            Some(Convergence::Stable(_))
        ));
    }

    #[test]
    fn real_layout_converges_across_grid_gutter_passes() {
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        spawn_ui_camera(&mut app, UVec2::splat(200), 1.0);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                Node {
                    width: px(100), height: px(100), display: Display::Grid,
                    grid_template_columns: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
                    grid_template_rows: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
                }
                ScrollAreaConfig { axis: ScrollAxis::Both }
                Children [

                        Node
                        ScrollArea
                        WidgetryScrollAreaViewport
                        Children [
                            Node { width: px(100), height: px(150), flex_shrink: 0.0 }
                            WidgetryScrollAreaContent
                        ]
                    --
                    Node HorizontalScrollbar--
                    Node VerticalScrollbar
                ]
            })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(root).unwrap();
        let viewport = children
            .iter()
            .find(|&entity| {
                app.world()
                    .get::<WidgetryScrollAreaViewport>(entity)
                    .is_some()
            })
            .unwrap();
        let horizontal = children
            .iter()
            .find(|&entity| app.world().get::<HorizontalScrollbar>(entity).is_some())
            .unwrap();
        let vertical = children
            .iter()
            .find(|&entity| app.world().get::<VerticalScrollbar>(entity).is_some())
            .unwrap();
        app.update();
        assert_eq!(
            app.world().get::<ComputedNode>(viewport).unwrap().size(),
            Vec2::splat(100.0)
        );
        assert_eq!(
            app.world().get::<Node>(vertical).unwrap().display,
            Display::Flex
        );
        assert_eq!(
            app.world().get::<Node>(horizontal).unwrap().display,
            Display::None
        );
        app.update();
        assert_eq!(
            app.world().get::<ComputedNode>(viewport).unwrap().size(),
            Vec2::new(88.0, 100.0)
        );
        assert_eq!(
            app.world().get::<Node>(horizontal).unwrap().display,
            Display::Flex
        );
        app.update();
        assert_eq!(
            app.world().get::<ComputedNode>(viewport).unwrap().size(),
            Vec2::splat(88.0)
        );
        assert!(matches!(
            app.world().get::<Convergence>(root),
            Some(Convergence::Stable(_))
        ));

        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        app.update();
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }
}
