#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::ui_widgets::{Scrollbar, ValueChange};
use bevy_widgetry_scroll_area::{WidgetryScrollArea, WidgetryScrollAreaViewport};
use bevy_widgetry_tabs::{TabsOrientation, WidgetryTab, WidgetryTabs, WidgetryTabsPlugin};
use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};

#[derive(Resource, Default)]
struct Changes(usize);

fn fixture(
    orientation: TabsOrientation,
    count: usize,
    selected: usize,
    scale: f32,
) -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin);
    app.add_plugins(WidgetryTabsPlugin)
        .init_resource::<Changes>();
    app.add_observer(|_: On<ValueChange<usize>>, mut changes: ResMut<Changes>| changes.0 += 1);
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::new(1400, 900), scale);
    let horizontal = orientation == TabsOrientation::Horizontal;
    let items: Vec<Box<dyn Scene>> = (0..count).map(|_| {
        let header: Box<dyn Scene> = Box::new(bsn! { Node { width: {if horizontal {px(100)} else {Val::Auto}} } });
        let content: Box<dyn Scene> = Box::new(bsn! {
            @WidgetryScrollArea { @children: bsn_list! { Node { height: px(600), width: px(160), flex_shrink: 0.0 } } }
            Node { width: px(200), height: px(100) }
        });
        Box::new(bsn! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } }) as Box<dyn Scene>
    }).collect();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryTabs { @orientation: orientation, @selected_index: selected, @children: {Some(Box::new(items) as Box<dyn SceneList>)} }
        Node { position_type: PositionType::Absolute, width: {if horizontal {px(350)} else {px(500)}}, height: px(180) }
    }).unwrap().id();
    settle(&mut app);
    (app, root)
}

fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

fn descendant<T: Component>(world: &World, root: Entity) -> Entity {
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if world.get::<T>(entity).is_some() {
            return entity;
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    panic!("missing descendant {}", std::any::type_name::<T>());
}

fn bar(world: &World, root: Entity) -> Entity {
    world.get::<Children>(root).unwrap()[0]
}
fn viewport(world: &World, root: Entity) -> Entity {
    descendant::<WidgetryScrollAreaViewport>(world, bar(world, root))
}
fn bounds(world: &World, entity: Entity) -> Rect {
    Rect::from_center_size(
        world.get::<UiGlobalTransform>(entity).unwrap().translation,
        world.get::<ComputedNode>(entity).unwrap().size(),
    )
}
fn assert_visible(world: &World, root: Entity, index: usize) {
    let target = bounds(world, WidgetryTabs::header(world, root, index).unwrap());
    let view = bounds(world, viewport(world, root));
    let shared_border = (1.0
        / world
            .get::<ComputedNode>(root)
            .unwrap()
            .inverse_scale_factor)
        .ceil();
    assert!(
        target.min.x >= view.min.x - shared_border
            && target.max.x <= view.max.x + shared_border
            && target.min.y >= view.min.y - shared_border
            && target.max.y <= view.max.y + shared_border,
        "target={target:?}, viewport={view:?}"
    );
}

#[test]
fn auto_bars_only_take_space_when_overflowing_and_headers_keep_their_axis() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        for count in [3, 10] {
            let (app, root) = fixture(orientation, count, 0, 1.25);
            let world = app.world();
            let strip = bar(world, root);
            assert!(world.get::<WidgetryScrollArea>(strip).is_some());
            let scrollbar = descendant::<Scrollbar>(world, strip);
            assert_eq!(
                world.get::<Node>(scrollbar).unwrap().display != Display::None,
                count == 10
            );
            let first = bounds(world, WidgetryTabs::header(world, root, 0).unwrap());
            let last = bounds(world, WidgetryTabs::header(world, root, count - 1).unwrap());
            let panel = world
                .get::<ChildOf>(WidgetryTabs::page(world, root, 0).unwrap())
                .unwrap()
                .parent();
            if orientation == TabsOrientation::Horizontal {
                assert!((first.width() - 125.0).abs() <= 1.0);
                assert!((first.min.y - last.min.y).abs() <= 1.0);
                if count == 3 {
                    assert!((bounds(world, panel).min.y - first.max.y).abs() <= 2.0);
                }
            } else {
                assert!((first.width() - last.width()).abs() <= 1.0);
                assert!((first.min.x - last.min.x).abs() <= 1.0);
                if count == 3 {
                    assert!((bounds(world, panel).min.x - first.max.x).abs() <= 2.0);
                }
            }
        }
    }
}

#[test]
fn initial_and_programmatic_selection_locate_headers_without_touching_page_scroll() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        let (mut app, root) = fixture(orientation, 10, 9, 1.25);
        assert_visible(app.world(), root, 9);
        assert_eq!(app.world().resource::<Changes>().0, 0);
        let view = viewport(app.world(), root);
        let page = WidgetryTabs::page(app.world(), root, 9).unwrap();
        let content = descendant::<WidgetryScrollAreaViewport>(app.world(), page);
        app.world_mut()
            .get_mut::<ScrollPosition>(content)
            .unwrap()
            .0
            .y = 42.0;
        let before = app.world().get::<ScrollPosition>(view).unwrap().0;
        assert_ne!(before, Vec2::ZERO);
        assert!(!WidgetryTabs::set_selected_in_world(app.world_mut(), root, 9).unwrap());
        settle(&mut app);
        assert_eq!(app.world().get::<ScrollPosition>(view).unwrap().0, before);
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        WidgetryTabs::set_selected_in_world(app.world_mut(), root, 0).unwrap();
        settle(&mut app);
        assert_visible(app.world(), root, 0);
        assert_eq!(app.world().resource::<Changes>().0, 1);
        WidgetryTabs::set_selected_in_world(app.world_mut(), root, 9).unwrap();
        settle(&mut app);
        assert_visible(app.world(), root, 9);
        assert_eq!(
            app.world().get::<ScrollPosition>(content).unwrap().0.y,
            42.0
        );
        assert_eq!(WidgetryTabs::page(app.world(), root, 9).unwrap(), page);
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(1300);
        app.world_mut().get_mut::<Node>(root).unwrap().height = px(600);
        settle(&mut app);
        assert_eq!(
            app.world().get::<ScrollPosition>(view).unwrap().0,
            Vec2::ZERO
        );
    }
}

#[test]
fn manual_bar_scroll_and_same_selection_keep_the_user_offset() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        let (mut app, root) = fixture(orientation, 10, 0, 1.25);
        let view = viewport(app.world(), root);
        let position = match orientation {
            TabsOrientation::Horizontal => Vec2::new(180.0, 0.0),
            TabsOrientation::Vertical => Vec2::new(0.0, 120.0),
        };
        app.world_mut().get_mut::<ScrollPosition>(view).unwrap().0 = position;
        settle(&mut app);
        assert_eq!(app.world().get::<ScrollPosition>(view).unwrap().0, position);
        assert!(!WidgetryTabs::set_selected_in_world(app.world_mut(), root, 0).unwrap());
        settle(&mut app);
        assert_eq!(app.world().get::<ScrollPosition>(view).unwrap().0, position);
        assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
        assert_eq!(app.world().resource::<Changes>().0, 0);
        // A different item already wholly in view also preserves the offset.
        WidgetryTabs::set_selected_in_world(app.world_mut(), root, 4).unwrap();
        settle(&mut app);
        assert_eq!(app.world().get::<ScrollPosition>(view).unwrap().0, position);
        assert_visible(app.world(), root, 4);
    }
}

#[test]
fn arrow_navigation_and_click_selection_use_the_same_scroll_location_path() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        let (mut app, root) = fixture(orientation, 10, 0, 1.25);
        bevy_widgetry_test_utils::add_keyboard_dispatch(&mut app);
        let window = app
            .world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow))
            .id();
        let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
        app.world_mut()
            .resource_mut::<bevy::input_focus::InputFocus>()
            .set(header, bevy::input_focus::FocusCause::Navigated);
        let disabled = WidgetryTabs::header(app.world(), root, 1).unwrap();
        app.world_mut()
            .entity_mut(disabled)
            .insert(InteractionDisabled);
        let key = if orientation == TabsOrientation::Horizontal {
            KeyCode::ArrowRight
        } else {
            KeyCode::ArrowDown
        };
        for _ in 0..8 {
            bevy_widgetry_test_utils::press_key(&mut app, window, key);
        }
        settle(&mut app);
        assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 9);
        assert_visible(app.world(), root, 9);
        assert_eq!(app.world().resource::<Changes>().0, 8);
        let visible = WidgetryTabs::header(app.world(), root, 7).unwrap();
        app.world_mut()
            .trigger(bevy_widgetry_test_utils::primary_click(visible));
        app.world_mut().flush();
        settle(&mut app);
        assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 7);
        assert_visible(app.world(), root, 7);
        assert_eq!(app.world().resource::<Changes>().0, 9);
    }
}

#[test]
fn selection_during_resize_uses_the_clamped_viewport_position() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        let (mut app, root) = fixture(orientation, 10, 9, 1.25);
        let mut node = app.world_mut().get_mut::<Node>(root).unwrap();
        node.width = px(600);
        node.height = px(300);
        WidgetryTabs::set_selected_in_world(app.world_mut(), root, 0).unwrap();
        settle(&mut app);
        assert_visible(app.world(), root, 0);
        WidgetryTabs::set_selected_in_world(app.world_mut(), root, 9).unwrap();
        settle(&mut app);
        app.world_mut().resource_mut::<UiScale>().0 = 1.5;
        settle(&mut app);
        assert_visible(app.world(), root, 9);
    }
}

#[test]
fn wheel_scrolling_with_hover_or_focus_outline_does_not_reveal_the_old_selection() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        let (mut app, root) = fixture(orientation, 10, 0, 1.25);
        let header = WidgetryTabs::header(app.world(), root, 3).unwrap();
        app.world_mut().entity_mut(header).insert(Outline {
            width: px(2),
            offset: px(0),
            color: Color::WHITE,
        });
        app.world_mut()
            .spawn_scene(bsn! {
                ChildOf(header)
                Node { position_type: PositionType::Absolute, width: px(600), height: px(80) }
            })
            .unwrap();
        let click = bevy_widgetry_test_utils::primary_click(header);
        app.world_mut()
            .trigger(bevy::picking::events::PointerScroll {
                entity: header,
                pointer: click.pointer,
                hit: click.hit,
                unit: bevy::input::mouse::MouseScrollUnit::Pixel,
                x: if orientation == TabsOrientation::Horizontal {
                    -100.0
                } else {
                    0.0
                },
                y: if orientation == TabsOrientation::Vertical {
                    -100.0
                } else {
                    0.0
                },
                phase: bevy::input::touch::TouchPhase::Moved,
            });
        settle(&mut app);
        let position = app
            .world()
            .get::<ScrollPosition>(viewport(app.world(), root))
            .unwrap()
            .0;
        assert!((position.length() - 100.0).abs() < 0.01, "{position:?}");
        assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
        assert_eq!(app.world().resource::<Changes>().0, 0);
    }
}
