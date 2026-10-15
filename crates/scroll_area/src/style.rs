use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};

use crate::disabled::SuspendedScrollbar;
use crate::headless::{
    ScrollAxis, ScrollbarVisibility, WidgetryScrollAreaContent, WidgetryScrollAreaViewport,
};
use crate::layout::{
    DEFAULT_SCROLLBAR_THICKNESS, HorizontalScrollbar, ScrollAreaConfig, VerticalScrollbar,
};
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{GridPlacement, RepeatedGridTrack, ScrollPosition};
use bevy::ui_widgets::{
    ControlOrientation, ScrollArea, Scrollbar, ScrollbarDragState, ScrollbarThumb,
};

const DEFAULT_MIN_THUMB_LENGTH: f32 = 24.0;

type StyleOwner = bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryScrollArea>;

#[derive(Resource, Default)]
pub(crate) struct StyleChanges {
    roots: EntityHashSet,
    thumbs: EntityHashSet,
}

#[derive(Component, Default, PartialEq)]
pub(crate) struct ThumbInput {
    disabled: bool,
    hovered: bool,
    dragged: bool,
}

#[cfg(test)]
#[derive(Resource, Default)]
pub(crate) struct StyleWork {
    roots: usize,
    thumbs: usize,
}

#[derive(Component, Default, Clone)]
#[require(ThumbInput)]
pub(crate) struct ScrollAreaThumb;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryScrollAreaProps)]
#[require(crate::colors::ColorState)]
pub struct WidgetryScrollArea;

pub struct WidgetryScrollAreaProps {
    pub colors: crate::WidgetryScrollAreaColorOverrides,
    pub axis: ScrollAxis,
    pub scrollbar_visibility: ScrollbarVisibility,
    pub scrollbar_thickness: f32,
    pub keyboard_scroll: bool,
    pub content: Option<Box<dyn Scene>>,
    pub children: Option<Box<dyn SceneList>>,
}

impl Default for WidgetryScrollAreaProps {
    fn default() -> Self {
        Self {
            colors: default(),
            axis: ScrollAxis::default(),
            scrollbar_visibility: ScrollbarVisibility::default(),
            scrollbar_thickness: DEFAULT_SCROLLBAR_THICKNESS,
            keyboard_scroll: true,
            content: None,
            children: None,
        }
    }
}

impl WidgetryScrollArea {
    fn scene(props: WidgetryScrollAreaProps) -> impl Scene {
        let WidgetryScrollAreaProps {
            colors,
            axis,
            scrollbar_visibility,
            scrollbar_thickness,
            keyboard_scroll,
            content,
            children,
        } = props;
        let direction = if axis == ScrollAxis::Horizontal {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        };
        let horizontal = matches!(axis, ScrollAxis::Horizontal | ScrollAxis::Both).then(|| bsn! {
            HorizontalScrollbar
            template(|_| Ok(bevy::picking::cursor::EntityCursor::default()))
            Scrollbar { target: Entity::PLACEHOLDER, orientation: ControlOrientation::Horizontal, min_thumb_length: DEFAULT_MIN_THUMB_LENGTH }
            Node { grid_column: GridPlacement::start(1), grid_row: GridPlacement::start(2), height: px(scrollbar_thickness) }
            Children [ScrollAreaThumb ScrollbarThumb { border_radius: BorderRadius::all(px(6)) } Hovered(false) template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Grab)))]
        });
        let vertical = matches!(axis, ScrollAxis::Vertical | ScrollAxis::Both).then(|| bsn! {
            VerticalScrollbar
            template(|_| Ok(bevy::picking::cursor::EntityCursor::default()))
            Scrollbar { target: Entity::PLACEHOLDER, orientation: ControlOrientation::Vertical, min_thumb_length: DEFAULT_MIN_THUMB_LENGTH }
            Node { grid_column: GridPlacement::start(2), grid_row: GridPlacement::start_span(1, 2), width: px(scrollbar_thickness) }
            Children [ScrollAreaThumb ScrollbarThumb { border_radius: BorderRadius::all(px(6)) } Hovered(false) template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Grab)))]
        });
        bsn! {
            template(move |_| colors.clone().initial())
            TabIndex(-1)
            template(move |_| Ok(ScrollAreaConfig { axis, scrollbar_visibility, scrollbar_thickness, keyboard_scroll }))
            Node {
                display: Display::Grid,
                grid_template_columns: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
                grid_template_rows: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
            }
            Children [

                    WidgetryScrollAreaViewport
                    ScrollArea
                    ScrollPosition::default()
                    Node { grid_column: GridPlacement::start(1), grid_row: GridPlacement::start(1), overflow: {axis.overflow()}, scrollbar_width: 0.0, align_items: AlignItems::FlexStart }
                    Children [
                        WidgetryScrollAreaContent
                        Node { flex_direction: direction }
                        @{content}
                        Node { width: Val::Auto, height: Val::Auto, min_width: percent(100), min_height: percent(100), max_width: Val::Auto, max_height: Val::Auto, flex_shrink: 0.0, overflow: Overflow::visible() }
                        Children [{children}]
                    ]
                --
                {horizontal}--
                {vertical}
            ]
        }
    }
}

pub fn apply_owned_scroll_colors(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryScrollAreaColors,
) -> Result<(), BevyError> {
    let children = world
        .get::<Children>(root)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    for bar in children {
        let Some(scrollbar) = scrollbar(world, bar) else {
            continue;
        };
        let colors = axis_colors(colors, scrollbar.orientation);
        let disabled = world.get::<bevy::ui::InteractionDisabled>(bar).is_some();
        let track = if disabled {
            colors.track.disabled
        } else {
            colors.track.normal
        };
        set_background(world, bar, track.background);
        let thumbs = world
            .get::<Children>(bar)
            .map(|c| c.to_vec())
            .unwrap_or_default();
        for thumb in thumbs {
            if world.get::<ScrollAreaThumb>(thumb).is_some() {
                apply_thumb(world, thumb, disabled, &colors);
            }
        }
    }
    Ok(())
}

fn axis_colors(
    colors: &bevy_widgetry_theme::WidgetryScrollAreaColors,
    orientation: ControlOrientation,
) -> bevy_widgetry_theme::WidgetryScrollAxisColors {
    if orientation == ControlOrientation::Horizontal {
        colors.horizontal
    } else {
        colors.vertical
    }
}

fn scrollbar(world: &World, entity: Entity) -> Option<&Scrollbar> {
    // Disabled input keeps the original Scrollbar in SuspendedScrollbar while layout remains active.
    world
        .get::<Scrollbar>(entity)
        .or_else(|| world.get::<SuspendedScrollbar>(entity).map(|bar| &bar.0))
}

fn set_background(world: &mut World, entity: Entity, color: Color) {
    if let Some(mut background) = world.get_mut::<BackgroundColor>(entity) {
        background.set_if_neq(BackgroundColor(color));
    } else {
        world.entity_mut(entity).insert(BackgroundColor(color));
    }
}

fn apply_thumb(
    world: &mut World,
    thumb: Entity,
    bar_disabled: bool,
    colors: &bevy_widgetry_theme::WidgetryScrollAxisColors,
) {
    #[cfg(test)]
    {
        world.resource_mut::<StyleWork>().thumbs += 1;
    }
    let disabled = bar_disabled || world.get::<bevy::ui::InteractionDisabled>(thumb).is_some();
    let dragged = world
        .get::<ScrollbarDragState>(thumb)
        .is_some_and(|s| s.dragging);
    let hovered = world.get::<Hovered>(thumb).is_some_and(|h| h.0);
    let state = if disabled {
        colors.thumb.disabled
    } else if dragged {
        colors.thumb.dragged
    } else if hovered {
        colors.thumb.hovered
    } else {
        colors.thumb.normal
    };
    set_background(world, thumb, state.background);
}

pub(crate) fn collect_root_changes(
    roots: Query<
        Entity,
        (
            With<WidgetryScrollArea>,
            Or<(
                Added<WidgetryScrollArea>,
                Changed<crate::colors::ColorState>,
                Changed<Children>,
            )>,
        ),
    >,
    bars: Query<
        &ChildOf,
        (
            Or<(With<Scrollbar>, With<SuspendedScrollbar>)>,
            Or<(
                Changed<Scrollbar>,
                Changed<SuspendedScrollbar>,
                Changed<Children>,
                Changed<ChildOf>,
                Added<bevy::ui::InteractionDisabled>,
            )>,
        ),
    >,
    parents: Query<&ChildOf, Or<(With<Scrollbar>, With<SuspendedScrollbar>)>>,
    mut removed_disabled: RemovedComponents<bevy::ui::InteractionDisabled>,
    mut removed_owners: RemovedComponents<StyleOwner>,
    mut removed_colors: RemovedComponents<crate::colors::ColorState>,
    mut removed_backgrounds: RemovedComponents<BackgroundColor>,
    mut changes: ResMut<StyleChanges>,
) {
    changes.roots.extend(&roots);
    changes.roots.extend(bars.iter().map(ChildOf::parent));
    changes.roots.extend(removed_owners.read());
    changes.roots.extend(removed_colors.read());
    for entity in removed_disabled.read().chain(removed_backgrounds.read()) {
        if let Ok(parent) = parents.get(entity) {
            changes.roots.insert(parent.parent());
        }
    }
}

pub(crate) fn collect_thumb_changes(
    changed: Query<
        Entity,
        (
            With<ScrollAreaThumb>,
            Or<(
                Added<ScrollAreaThumb>,
                Changed<ChildOf>,
                Changed<Hovered>,
                Changed<ScrollbarDragState>,
                Added<bevy::ui::InteractionDisabled>,
            )>,
        ),
    >,
    mut thumbs: Query<(
        Ref<ScrollAreaThumb>,
        Option<Ref<ChildOf>>,
        Option<&Hovered>,
        Option<&ScrollbarDragState>,
        Has<bevy::ui::InteractionDisabled>,
        &mut ThumbInput,
    )>,
    mut removed_hover: RemovedComponents<Hovered>,
    mut removed_drag: RemovedComponents<ScrollbarDragState>,
    mut removed_disabled: RemovedComponents<bevy::ui::InteractionDisabled>,
    mut removed_backgrounds: RemovedComponents<BackgroundColor>,
    mut changes: ResMut<StyleChanges>,
) {
    changes.thumbs.extend(removed_backgrounds.read());
    for entity in changed
        .iter()
        .chain(removed_hover.read())
        .chain(removed_drag.read())
        .chain(removed_disabled.read())
    {
        let Ok((marker, parent, hover, drag, disabled, mut previous)) = thumbs.get_mut(entity)
        else {
            continue;
        };
        let input = ThumbInput {
            disabled,
            hovered: hover.is_some_and(|h| h.0),
            dragged: drag.is_some_and(|d| d.dragging),
        };
        // Drag position updates mark ScrollbarDragState changed without changing its color input.
        if marker.is_added() || parent.is_some_and(|p| p.is_changed()) || *previous != input {
            *previous = input;
            changes.thumbs.insert(entity);
        }
    }
}

fn standalone(world: &World, root: Entity) -> bool {
    world.get::<WidgetryScrollArea>(root).is_some() && world.get::<StyleOwner>(root).is_none()
}

fn resolved_colors(
    world: &mut World,
    root: Entity,
) -> Result<bevy_widgetry_theme::WidgetryScrollAreaColors, BevyError> {
    #[cfg(test)]
    {
        world.resource_mut::<StyleWork>().roots += 1;
    }
    let theme = &world.resource::<WidgetryThemeMode>().colors().scroll_area;
    Ok(crate::WidgetryScrollAreaColorOverrides::get(world, root)?.resolve(theme))
}

pub(crate) fn update_thumb_style(world: &mut World) -> Result<(), BevyError> {
    let (roots, thumbs) = {
        let mut changes = world.resource_mut::<StyleChanges>();
        (
            std::mem::take(&mut changes.roots),
            std::mem::take(&mut changes.thumbs),
        )
    };
    let mut failure = None;
    for root in &roots {
        if standalone(world, *root) {
            let result = resolved_colors(world, *root)
                .and_then(|colors| apply_owned_scroll_colors(world, *root, &colors));
            if let Err(error) = result
                && failure.is_none()
            {
                failure = Some(error);
            }
        }
    }
    for thumb in thumbs {
        if world.get::<ScrollAreaThumb>(thumb).is_none() {
            continue;
        }
        let Some(bar) = world.get::<ChildOf>(thumb).map(ChildOf::parent) else {
            continue;
        };
        let Some(root) = world.get::<ChildOf>(bar).map(ChildOf::parent) else {
            continue;
        };
        if roots.contains(&root) || !standalone(world, root) {
            continue;
        }
        let Some(scrollbar) = scrollbar(world, bar) else {
            continue;
        };
        let orientation = scrollbar.orientation;
        let disabled = world.get::<bevy::ui::InteractionDisabled>(bar).is_some();
        match resolved_colors(world, root) {
            Ok(colors) => apply_thumb(world, thumb, disabled, &axis_colors(&colors, orientation)),
            Err(error) if failure.is_none() => failure = Some(error),
            Err(_) => {}
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result<(), BevyError> {
        let roots = world
            .query_filtered::<Entity, (With<WidgetryScrollArea>, Without<StyleOwner>)>()
            .iter(world)
            .collect::<EntityHashSet>();
        world.resource_mut::<StyleChanges>().roots.extend(roots);
        update_thumb_style(world)
    });
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::ui::{ComputedNode, OverflowAxis};
    use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};

    fn parts(world: &World, root: Entity) -> (Entity, Entity, Vec<Entity>) {
        let children = world.get::<Children>(root).unwrap();
        let viewport = children
            .iter()
            .find(|&child| world.get::<WidgetryScrollAreaViewport>(child).is_some())
            .unwrap();
        let content = world.get::<Children>(viewport).unwrap()[0];
        let bars = children
            .iter()
            .filter(|&child| world.get::<Scrollbar>(child).is_some())
            .collect();
        (viewport, content, bars)
    }

    #[test]
    fn idle_and_single_thumb_changes_limit_color_work() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let mut thumbs = Vec::new();
        for _ in 0..2 {
            let root = app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryScrollArea { @axis: ScrollAxis::Both } })
                .unwrap()
                .id();
            let (_, _, bars) = parts(app.world(), root);
            for bar in bars {
                thumbs.push(app.world().get::<Children>(bar).unwrap()[0]);
            }
        }
        app.update();
        std::mem::take(&mut *app.world_mut().resource_mut::<StyleWork>());
        app.update();
        let work = app.world().resource::<StyleWork>();
        assert_eq!((work.roots, work.thumbs), (0, 0));
        app.world_mut().entity_mut(thumbs[0]).insert(Hovered(true));
        app.update();
        let work = std::mem::take(&mut *app.world_mut().resource_mut::<StyleWork>());
        assert_eq!((work.roots, work.thumbs), (1, 1));
        app.world_mut()
            .get_mut::<ScrollbarDragState>(thumbs[0])
            .unwrap()
            .dragging = true;
        app.update();
        std::mem::take(&mut *app.world_mut().resource_mut::<StyleWork>());
        app.world_mut()
            .get_mut::<ScrollbarDragState>(thumbs[0])
            .unwrap()
            .dragging = true;
        app.update();
        let work = app.world().resource::<StyleWork>();
        assert_eq!((work.roots, work.thumbs), (0, 0));
    }

    #[test]
    fn real_layout_and_scroll_do_not_repeat_color_work() {
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app.world_mut().spawn_scene(bsn! {
            @WidgetryScrollArea {
                @axis: ScrollAxis::Both,
                @children: bsn_list! { Node { width: px(500), height: px(500), flex_shrink: 0.0 } },
            }
            Node { width: px(120), height: px(120) }
        }).unwrap().id();
        for _ in 0..5 {
            app.update();
        }
        std::mem::take(&mut *app.world_mut().resource_mut::<StyleWork>());
        let (viewport, _, bars) = parts(app.world(), root);
        let thumb = app.world().get::<Children>(bars[0]).unwrap()[0];
        let before = *app
            .world()
            .get::<bevy::ui::UiGlobalTransform>(thumb)
            .unwrap();
        app.world_mut()
            .get_mut::<ScrollPosition>(viewport)
            .unwrap()
            .0 = Vec2::splat(100.0);
        app.update();
        assert_ne!(
            *app.world()
                .get::<bevy::ui::UiGlobalTransform>(thumb)
                .unwrap(),
            before
        );
        for _ in 0..3 {
            app.update();
        }
        let work = app.world().resource::<StyleWork>();
        assert_eq!((work.roots, work.thumbs), (0, 0));
    }

    #[test]
    fn removed_inputs_disabled_overrides_and_owner_handoff_refresh_colors() {
        use bevy::ui::InteractionDisabled;
        for axis in [
            ScrollAxis::Horizontal,
            ScrollAxis::Vertical,
            ScrollAxis::Both,
        ] {
            let mut app = scene_app();
            app.add_plugins(crate::WidgetryScrollAreaPlugin);
            let root = app
                .world_mut()
                .spawn_scene(bsn! {
                    @WidgetryScrollArea { @axis: {axis} }
                })
                .unwrap()
                .id();
            app.update();
            let (_, _, bars) = parts(app.world(), root);
            let bar = bars[0];
            let orientation = scrollbar(app.world(), bar).unwrap().orientation;
            let thumb = app.world().get::<Children>(bar).unwrap()[0];
            app.world_mut().entity_mut(thumb).insert(Hovered(true));
            app.world_mut()
                .get_mut::<ScrollbarDragState>(thumb)
                .unwrap()
                .dragging = true;
            app.update();
            app.world_mut()
                .entity_mut(thumb)
                .remove::<ScrollbarDragState>();
            app.update();
            let dark = axis_colors(&WidgetryThemeMode::Dark.colors().scroll_area, orientation);
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                dark.thumb.hovered.background
            );
            app.world_mut().entity_mut(thumb).remove::<Hovered>();
            app.update();
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                dark.thumb.normal.background
            );
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
            app.update();
            assert!(app.world().get::<SuspendedScrollbar>(bar).is_some());
            assert_eq!(
                app.world().get::<BackgroundColor>(bar).unwrap().0,
                dark.track.disabled.background
            );
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                dark.thumb.disabled.background
            );
            WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
            let light = axis_colors(&WidgetryThemeMode::Light.colors().scroll_area, orientation);
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                light.thumb.disabled.background
            );
            let mut overrides = crate::WidgetryScrollAreaColorOverrides::default();
            overrides.horizontal.thumb.disabled.background = Some(Color::BLACK);
            overrides.vertical.thumb.disabled.background = Some(Color::BLACK);
            crate::WidgetryScrollAreaColorOverrides::set_in_world(app.world_mut(), root, overrides)
                .unwrap();
            app.update();
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                Color::BLACK
            );
            crate::WidgetryScrollAreaColorOverrides::set_in_world(app.world_mut(), root, default())
                .unwrap();
            app.update();
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                light.thumb.disabled.background
            );
            app.world_mut()
                .entity_mut(root)
                .remove::<InteractionDisabled>();
            app.update();
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                light.thumb.normal.background
            );
            app.world_mut()
                .entity_mut(root)
                .insert(StyleOwner::new(root));
            let mut owned = WidgetryThemeMode::Light.colors().scroll_area;
            owned.horizontal.thumb.normal.background = Color::BLACK;
            owned.vertical.thumb.normal.background = Color::BLACK;
            owned.horizontal.thumb.hovered.background = Color::WHITE;
            owned.vertical.thumb.hovered.background = Color::WHITE;
            apply_owned_scroll_colors(app.world_mut(), root, &owned).unwrap();
            app.world_mut().entity_mut(thumb).insert(Hovered(true));
            app.update();
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                Color::BLACK
            );
            apply_owned_scroll_colors(app.world_mut(), root, &owned).unwrap();
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                Color::WHITE
            );
            app.world_mut().entity_mut(root).remove::<StyleOwner>();
            app.update();
            assert_eq!(
                app.world().get::<BackgroundColor>(thumb).unwrap().0,
                light.thumb.hovered.background
            );
        }
    }

    #[test]
    fn new_parts_and_removed_backgrounds_are_styled_without_refreshing_other_roots() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryScrollArea })
            .unwrap()
            .id();
        app.update();
        let (_, _, bars) = parts(app.world(), root);
        let bar = bars[0];
        let old = app.world().get::<Children>(bar).unwrap()[0];
        app.world_mut().despawn(old);
        let thumb = app
            .world_mut()
            .spawn_scene(bsn! {
                ScrollAreaThumb ScrollbarThumb Hovered(true)
            })
            .unwrap()
            .id();
        app.world_mut().entity_mut(bar).add_child(thumb);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .scroll_area
                .vertical
                .thumb
                .hovered
                .background
        );
        for entity in [bar, thumb] {
            app.world_mut()
                .entity_mut(entity)
                .remove::<BackgroundColor>();
            app.update();
            assert!(app.world().get::<BackgroundColor>(entity).is_some());
        }
        std::mem::take(&mut *app.world_mut().resource_mut::<StyleWork>());
        app.world_mut()
            .spawn_scene(bsn! { @WidgetryScrollArea })
            .unwrap();
        app.update();
        let work = app.world().resource::<StyleWork>();
        assert_eq!((work.roots, work.thumbs), (1, 1));
    }

    #[test]
    fn default_scene_has_expected_hierarchy() {
        let props = WidgetryScrollAreaProps::default();
        assert_eq!(props.axis, ScrollAxis::Vertical);
        assert_eq!(props.scrollbar_visibility, ScrollbarVisibility::default());
        assert_eq!(props.scrollbar_thickness, 12.0);
        assert!(props.content.is_none() && props.children.is_none());
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryScrollArea })
            .unwrap()
            .id();
        let (viewport, content, bars) = parts(app.world(), root);
        app.update();
        assert_eq!(bars.len(), 1);
        assert_eq!(
            app.world().get::<Scrollbar>(bars[0]).unwrap().target,
            viewport
        );
        assert_eq!(
            app.world().get::<Scrollbar>(bars[0]).unwrap().orientation,
            ControlOrientation::Vertical
        );
        assert_eq!(
            app.world()
                .get::<Scrollbar>(bars[0])
                .unwrap()
                .min_thumb_length,
            24.0
        );
        assert!(app.world().get::<ScrollPosition>(viewport).is_some());
        assert_eq!(app.world().get::<Children>(viewport).unwrap().len(), 1);
        assert_eq!(app.world().get::<Children>(content).unwrap().len(), 0);
        let root_node = app.world().get::<Node>(root).unwrap();
        assert_eq!(root_node.display, Display::Grid);
        assert_eq!(root_node.width, Val::Auto);
        assert_eq!(root_node.height, Val::Auto);
        assert_eq!(app.world().get::<TabIndex>(root).unwrap().0, -1);
        let viewport_node = app.world().get::<Node>(viewport).unwrap();
        assert_eq!(viewport_node.grid_column, GridPlacement::start(1));
        assert_eq!(viewport_node.grid_row, GridPlacement::start(1));
        assert_eq!(viewport_node.scrollbar_width, 0.0);
        assert_eq!(viewport_node.overflow.x, OverflowAxis::Clip);
        assert_eq!(viewport_node.overflow.y, OverflowAxis::Scroll);
        assert_eq!(
            app.world().get::<Node>(content).unwrap().flex_direction,
            FlexDirection::Column
        );
    }

    #[test]
    fn both_axes_and_content_patch_keep_sizing_invariant() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app.world_mut().spawn_scene(bsn! {
            @WidgetryScrollArea {
                @axis: ScrollAxis::Both,
                @scrollbar_thickness: 17.0,
                @content: bsn! { Node { flex_direction: FlexDirection::Row, width: px(44), min_width: px(3), padding: UiRect::all(px(5)) } },
                @children: bsn_list!{Node { width: px(200), height: px(300) }},
            }
        }).unwrap().id();
        let (viewport, content, bars) = parts(app.world(), root);
        app.update();
        assert_eq!(bars.len(), 2);
        assert!(
            bars.iter()
                .all(|&bar| app.world().get::<Scrollbar>(bar).unwrap().target == viewport)
        );
        let h = bars
            .iter()
            .copied()
            .find(|&bar| app.world().get::<HorizontalScrollbar>(bar).is_some())
            .unwrap();
        let v = bars
            .iter()
            .copied()
            .find(|&bar| app.world().get::<VerticalScrollbar>(bar).is_some())
            .unwrap();
        assert_eq!(app.world().get::<Node>(h).unwrap().height, px(17));
        assert_eq!(app.world().get::<Node>(v).unwrap().width, px(17));
        assert_eq!(
            app.world().get::<Node>(viewport).unwrap().overflow,
            Overflow::scroll()
        );
        let node = app.world().get::<Node>(content).unwrap();
        assert_eq!(node.flex_direction, FlexDirection::Row);
        assert_eq!(node.padding, UiRect::all(px(5)));
        assert_eq!(node.width, Val::Auto);
        assert_eq!(node.min_width, percent(100));
        assert_eq!(node.min_height, percent(100));
        assert_eq!(node.flex_shrink, 0.0);
        assert_eq!(node.overflow, Overflow::visible());
        assert_eq!(app.world().get::<Children>(content).unwrap().len(), 1);
    }

    #[test]
    fn thumb_style_tracks_hover_drag_and_theme() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryScrollArea })
            .unwrap()
            .id();
        let (_, _, bars) = parts(app.world(), root);
        let thumb = app.world().get::<Children>(bars[0]).unwrap()[0];
        assert!(app.world().get::<ScrollbarThumb>(thumb).is_some());
        assert!(app.world().get::<ScrollbarDragState>(thumb).is_some());
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .scroll_area
                .vertical
                .thumb
                .normal
                .background
        );
        app.world_mut().entity_mut(thumb).insert(Hovered(true));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .scroll_area
                .vertical
                .thumb
                .hovered
                .background
        );
        app.world_mut()
            .get_mut::<ScrollbarDragState>(thumb)
            .unwrap()
            .dragging = true;
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .scroll_area
                .vertical
                .thumb
                .dragged
                .background
        );
        WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light)
            .expect("theme switch succeeds");
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            WidgetryThemeMode::Light
                .colors()
                .scroll_area
                .vertical
                .thumb
                .dragged
                .background
        );
        app.world_mut()
            .get_mut::<ScrollbarDragState>(thumb)
            .unwrap()
            .dragging = false;
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            WidgetryThemeMode::Light
                .colors()
                .scroll_area
                .vertical
                .thumb
                .hovered
                .background
        );
        app.world_mut().entity_mut(thumb).insert(Hovered(false));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            WidgetryThemeMode::Light
                .colors()
                .scroll_area
                .vertical
                .thumb
                .normal
                .background
        );
    }

    #[test]
    fn unrelated_scrollbar_thumb_keeps_its_color() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let external = app
            .world_mut()
            .spawn((
                ScrollbarThumb::default(),
                Hovered(true),
                BackgroundColor(Color::srgb_u8(12, 34, 56)),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(external).unwrap().0,
            Color::srgb_u8(12, 34, 56)
        );
        WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light)
            .expect("theme switch succeeds");
        assert_eq!(
            app.world().get::<BackgroundColor>(external).unwrap().0,
            Color::srgb_u8(12, 34, 56)
        );
    }

    #[test]
    fn content_sizing_contract_holds_in_real_layout() {
        for axis in [
            ScrollAxis::Vertical,
            ScrollAxis::Horizontal,
            ScrollAxis::Both,
        ] {
            let mut app = scene_app();
            add_ui_plugins(&mut app);
            app.add_plugins(crate::WidgetryScrollAreaPlugin);
            spawn_ui_camera(&mut app, UVec2::splat(200), 1.0);
            let root = app.world_mut().spawn_scene(bsn! {
                @WidgetryScrollArea {
                    @axis: {axis},
                    @children: bsn_list!{Node { width: px(150), height: px(150), flex_shrink: 0.0 }},
                }
                Node { width: px(100), height: px(100) }
            }).unwrap().id();
            let (viewport, content, _) = parts(app.world(), root);
            app.update();
            let viewport_size = app.world().get::<ComputedNode>(viewport).unwrap().size();
            let content_size = app.world().get::<ComputedNode>(content).unwrap().size();
            assert!(
                content_size.x >= viewport_size.x,
                "{axis:?}: {content_size:?} < {viewport_size:?}"
            );
            assert!(
                content_size.y >= viewport_size.y,
                "{axis:?}: {content_size:?} < {viewport_size:?}"
            );
            match axis {
                ScrollAxis::Vertical => assert!(
                    content_size.y >= 150.0,
                    "{axis:?}: {content_size:?} / {viewport_size:?}"
                ),
                ScrollAxis::Horizontal => assert!(
                    content_size.x >= 150.0,
                    "{axis:?}: {content_size:?} / {viewport_size:?}"
                ),
                ScrollAxis::Both => assert!(
                    content_size.x >= 150.0 && content_size.y >= 150.0,
                    "{axis:?}: {content_size:?} / {viewport_size:?}"
                ),
            }
        }
    }
}
