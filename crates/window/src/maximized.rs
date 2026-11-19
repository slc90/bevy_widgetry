use crate::{
    title_bar::bar::TitleBar,
    window_root::{WINDOW_CORNER_RADIUS, WindowContent, WindowInitialized, WindowRoot},
};
use bevy::{ecs::system::NonSendMarker, prelude::*, winit::WINIT_WINDOWS};

/// The most recent maximized state reported by the native window.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WidgetryWindowMaximized(bool);

impl WidgetryWindowMaximized {
    pub fn is_maximized(self) -> bool {
        self.0
    }
}

pub(crate) fn native_maximized(native: Entity) -> Option<bool> {
    #[cfg(test)]
    if let Some((window, maximized)) = TEST_NATIVE_MAXIMIZED.get()
        && native == window
    {
        TEST_NATIVE_MAXIMIZED.take();
        return Some(maximized);
    }
    WINIT_WINDOWS.with_borrow(|windows| {
        windows
            .get_window(native)
            .map(|window| window.is_maximized())
    })
}

pub(crate) fn sync_maximized(
    _main_thread: NonSendMarker,
    mut roots: Query<(&WindowRoot, &mut WidgetryWindowMaximized), With<WindowInitialized>>,
) {
    for (root, mut state) in &mut roots {
        if let Some(maximized) = native_maximized(root.target_window) {
            state.set_if_neq(WidgetryWindowMaximized(maximized));
        }
    }
}

pub(crate) fn sync_corner_radii(
    roots: Query<
        (Entity, &WidgetryWindowMaximized, &Children),
        (With<WindowRoot>, Changed<WidgetryWindowMaximized>),
    >,
    title_bars: Query<(), With<TitleBar>>,
    contents: Query<(), With<WindowContent>>,
    mut nodes: Query<&mut Node>,
) {
    for (entity, state, children) in &roots {
        let radius = px(if state.is_maximized() {
            0.0
        } else {
            WINDOW_CORNER_RADIUS
        });
        if let Ok(mut node) = nodes.get_mut(entity) {
            node.border_radius = BorderRadius::all(radius);
        }
        for child in children.iter() {
            let Ok(mut node) = nodes.get_mut(child) else {
                continue;
            };
            if title_bars.contains(child) {
                node.border_radius = BorderRadius::top(radius);
            } else if contents.contains(child) {
                node.border_radius = BorderRadius::bottom(radius);
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static TEST_NATIVE_MAXIMIZED: std::cell::Cell<Option<(Entity, bool)>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        WidgetryWindowBackground, WidgetryWindowPlugin, owned_widgetry_window,
        owned_widgetry_window_with_controls, title_bar::maximize::MaximizeButton,
        widgetry_window_target,
    };
    use bevy_widgetry_asset::BuiltinIcon;
    use bevy_widgetry_core::icon::WidgetryIcon;
    use bevy_widgetry_test_utils::scene_app;

    #[test]
    fn native_state_projects_corners_and_only_the_default_maximize_icon() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let world = app.world_mut();
        let default_root = world.spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), default(), WidgetryWindowBackground::Theme,
                default(), bsn_list!{}, bsn_list!{})
        }).unwrap().id();
        let custom_root = world.spawn_scene(bsn! {
            @owned_widgetry_window_with_controls(Window::default(), default(), WidgetryWindowBackground::Theme,
                default(), bsn_list!{}, bsn_list! {
                    @WidgetryIcon { @path: {BuiltinIcon::WindowClose.path()} }
                }, bsn_list!{})
        }).unwrap().id();
        let default_native = widgetry_window_target(world, default_root).unwrap();
        let custom_native = widgetry_window_target(world, custom_root).unwrap();
        let default_button = world
            .query_filtered::<Entity, With<MaximizeButton>>()
            .single(world)
            .unwrap();
        let default_icon = world.get::<Children>(default_button).unwrap()[0];
        let custom_title = world.get::<Children>(custom_root).unwrap()[0];
        let custom_controls = world.get::<Children>(custom_title).unwrap()[2];
        let custom_icon = world.get::<Children>(custom_controls).unwrap()[0];
        app.update();

        for (root, native) in [(default_root, default_native), (custom_root, custom_native)] {
            for maximized in [true, false] {
                TEST_NATIVE_MAXIMIZED.set(Some((native, maximized)));
                app.update();
                assert!(TEST_NATIVE_MAXIMIZED.take().is_none());
                let world = app.world();
                assert_eq!(
                    world
                        .get::<WidgetryWindowMaximized>(root)
                        .unwrap()
                        .is_maximized(),
                    maximized
                );
                let radius = px(if maximized { 0.0 } else { WINDOW_CORNER_RADIUS });
                assert_eq!(
                    world.get::<Node>(root).unwrap().border_radius,
                    BorderRadius::all(radius)
                );
                let expected = if root == default_root && maximized {
                    BuiltinIcon::WindowRestore
                } else {
                    BuiltinIcon::WindowMaximize
                };
                assert_eq!(
                    world.get::<WidgetryIcon>(default_icon).unwrap().path(),
                    Some(&expected.path())
                );
                assert_eq!(
                    world.get::<WidgetryIcon>(custom_icon).unwrap().path(),
                    Some(&BuiltinIcon::WindowClose.path())
                );
            }
        }
    }
}
