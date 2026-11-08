#![allow(clippy::disallowed_macros)]
#![cfg(any(
    feature = "collapsible",
    feature = "slider",
    feature = "scroll_area",
    feature = "table",
    feature = "tabs",
    feature = "segmented",
    feature = "radio_group",
    feature = "text_field"
))]

use bevy::input_focus::{
    InputFocus, pointer_focus::PointerFocusPlugin, tab_navigation::TabNavigationPlugin,
};
use bevy_widgetry_test_utils::scene_app;

macro_rules! pointer_focus_plugin {
    ($name:ident, $feature:literal, $plugin:path) => {
        #[cfg(feature = $feature)]
        #[test]
        fn $name() {
            let mut app = scene_app();
            app.add_plugins($plugin);
            assert!(!app.is_plugin_added::<TabNavigationPlugin>());
            assert!(app.is_plugin_added::<PointerFocusPlugin>());
            assert!(app.world().contains_resource::<InputFocus>());
        }
    };
}

pointer_focus_plugin!(
    collapsible,
    "collapsible",
    bevy_widgetry::collapsible::WidgetryCollapsiblePlugin
);
pointer_focus_plugin!(
    slider,
    "slider",
    bevy_widgetry::slider::WidgetrySliderPlugin
);
pointer_focus_plugin!(
    scroll_area,
    "scroll_area",
    bevy_widgetry::scroll_area::WidgetryScrollAreaPlugin
);
pointer_focus_plugin!(table, "table", bevy_widgetry::table::WidgetryTablePlugin);
pointer_focus_plugin!(tabs, "tabs", bevy_widgetry::tabs::WidgetryTabsPlugin);
pointer_focus_plugin!(
    segmented,
    "segmented",
    bevy_widgetry::segmented::WidgetrySegmentedPlugin
);
pointer_focus_plugin!(
    radio_group,
    "radio_group",
    bevy_widgetry::radio_group::WidgetryRadioGroupPlugin
);
pointer_focus_plugin!(
    text_field,
    "text_field",
    bevy_widgetry::text_field::WidgetryTextFieldPlugin
);

#[cfg(all(feature = "tabs", feature = "table", feature = "segmented"))]
#[test]
fn shared_dependencies_preserve_pointer_focus_in_different_installation_orders() {
    for table_first in [false, true] {
        let mut app = scene_app();
        if table_first {
            app.add_plugins(bevy_widgetry::table::WidgetryTablePlugin);
            app.add_plugins(bevy_widgetry::tabs::WidgetryTabsPlugin);
        } else {
            app.add_plugins(bevy_widgetry::tabs::WidgetryTabsPlugin);
            app.add_plugins(bevy_widgetry::table::WidgetryTablePlugin);
        }
        app.add_plugins(bevy_widgetry::segmented::WidgetrySegmentedPlugin);
        assert!(!app.is_plugin_added::<TabNavigationPlugin>());
        assert!(app.is_plugin_added::<PointerFocusPlugin>());
        assert!(app.world().contains_resource::<InputFocus>());
    }
}
