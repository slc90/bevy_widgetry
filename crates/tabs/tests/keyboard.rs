#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input_focus::{
    FocusCause, InputFocus,
    tab_navigation::{TabGroup, TabIndex},
};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_tabs::{TabsOrientation, WidgetryTab, WidgetryTabs, WidgetryTabsPlugin};
use bevy_widgetry_test_utils::{add_keyboard_dispatch, press_key, primary_press, scene_app};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize)>);

fn fixture(orientation: TabsOrientation) -> (App, Entity, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryTabsPlugin)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Changes>()
        .add_observer(
            |event: On<ValueChange<usize>>, mut world: bevy::ecs::world::DeferredWorld| {
                assert_eq!(
                    WidgetryTabs::selected_index(&world, event.source).unwrap(),
                    event.value
                );
                for index in 0..4 {
                    let page = WidgetryTabs::page(&world, event.source, index).unwrap();
                    assert_eq!(
                        world.get::<Node>(page).unwrap().display == Display::Flex,
                        index == event.value
                    );
                }
                world
                    .resource_mut::<Changes>()
                    .0
                    .push((event.source, event.value));
            },
        );
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let items: Vec<Box<dyn Scene>> = (0..4)
        .map(|index| {
            let header: Box<dyn Scene> = Box::new(bsn! { Children [Text(format!("Tab {index}"))] });
            let content: Box<dyn Scene> = Box::new(bsn! { Node TabIndex(0) });
            Box::new(bsn! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } })
                as Box<dyn Scene>
        })
        .collect();
    let items: Box<dyn SceneList> = Box::new(items);
    let container = app.world_mut().spawn_scene(bsn! {
        Node TabGroup::default() Children [
            Node TabIndex(0)-- @WidgetryTabs { @orientation: orientation, @children: {Some(items)} }-- Node TabIndex(0)
        ]
    }).unwrap().id();
    app.update();
    let children = app.world().get::<Children>(container).unwrap().to_vec();
    (app, window, children[1], children[0], children[2])
}

#[test]
fn focused_headers_navigate_on_their_axis_skip_disabled_and_stop_at_edges() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        let (mut app, window, root, _, _) = fixture(orientation);
        let (forward, backward, other) = match orientation {
            TabsOrientation::Horizontal => {
                (KeyCode::ArrowRight, KeyCode::ArrowLeft, KeyCode::ArrowDown)
            }
            TabsOrientation::Vertical => {
                (KeyCode::ArrowDown, KeyCode::ArrowUp, KeyCode::ArrowRight)
            }
        };
        press_key(&mut app, window, forward);
        assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
        let header0 = WidgetryTabs::header(app.world(), root, 0).unwrap();
        let header1 = WidgetryTabs::header(app.world(), root, 1).unwrap();
        app.world_mut()
            .entity_mut(header1)
            .insert(InteractionDisabled);
        let content = app.world().get::<Children>(header0).unwrap()[0];
        app.world_mut().trigger(primary_press(content));
        app.world_mut().flush();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(header0));
        for (key, selected) in [
            (backward, 0),
            (forward, 2),
            (other, 2),
            (KeyCode::Home, 2),
            (KeyCode::End, 2),
            (forward, 3),
            (forward, 3),
            (backward, 2),
            (backward, 0),
        ] {
            press_key(&mut app, window, key);
            assert_eq!(
                WidgetryTabs::selected_index(app.world(), root).unwrap(),
                selected
            );
            assert_eq!(
                app.world().resource::<InputFocus>().get(),
                Some(WidgetryTabs::header(app.world(), root, selected).unwrap())
            );
        }
        assert_eq!(
            app.world().resource::<Changes>().0,
            vec![(root, 2), (root, 3), (root, 2), (root, 0)]
        );
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        press_key(&mut app, window, forward);
        assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
        assert_eq!(app.world().resource::<Changes>().0.len(), 4);
    }
}

#[test]
fn tab_navigation_skips_headers_but_visits_the_active_page() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        let (mut app, window, root, before, after) = fixture(orientation);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(before, FocusCause::Navigated);
        press_key(&mut app, window, KeyCode::Tab);
        let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
        let content = app.world().get::<Children>(page).unwrap()[0];
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(content));
        press_key(&mut app, window, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(after));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        press_key(&mut app, window, KeyCode::Tab);
        press_key(&mut app, window, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(before));
        for index in 0..4 {
            assert_eq!(
                app.world()
                    .get::<TabIndex>(WidgetryTabs::header(app.world(), root, index).unwrap())
                    .unwrap()
                    .0,
                -1
            );
        }
    }
}

#[test]
fn disabled_tail_has_no_navigation_target_and_programmatic_selection_does_not_move_focus() {
    let (mut app, window, root, before, _) = fixture(TabsOrientation::Horizontal);
    let header0 = WidgetryTabs::header(app.world(), root, 0).unwrap();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(header0, FocusCause::Navigated);
    WidgetryTabs::set_selected_in_world(app.world_mut(), root, 2).unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(header0));
    press_key(&mut app, window, KeyCode::ArrowRight);
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 1);
    let header1 = WidgetryTabs::header(app.world(), root, 1).unwrap();
    for index in 2..4 {
        let header = WidgetryTabs::header(app.world(), root, index).unwrap();
        app.world_mut()
            .entity_mut(header)
            .insert(InteractionDisabled);
    }
    let count = app.world().resource::<Changes>().0.len();
    press_key(&mut app, window, KeyCode::ArrowRight);
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 1);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(header1));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(before, FocusCause::Navigated);
    press_key(&mut app, window, KeyCode::ArrowLeft);
    assert_eq!(app.world().resource::<Changes>().0.len(), count);
}
