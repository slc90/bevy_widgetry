#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#![cfg(all(feature = "button", feature = "check_box"))]

use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy_widgetry::button::{WidgetryButton, WidgetryButtonColorOverrides, WidgetryButtonPlugin};
use bevy_widgetry::check_box::{
    WidgetryCheckBox, WidgetryCheckBoxColorOverrides, WidgetryCheckBoxPlugin,
};
use bevy_widgetry::text::WidgetryText;
use bevy_widgetry::theme::WidgetryTheme;

#[test]
fn custom_complete_palette_refreshes_existing_widgets_and_new_content_without_resetting_state() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins((WidgetryButtonPlugin, WidgetryCheckBoxPlugin));
    let fixed_border = Color::srgb(0.4, 0.2, 0.1);
    let mut local = WidgetryButtonColorOverrides::default();
    local.normal.border = Some(fixed_border);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryButton { @colors: local, }
            Children [Name("custom-theme-label") Text("kept") WidgetryText]
        })
        .unwrap()
        .id();
    let check = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryCheckBox {} Checked
        })
        .unwrap()
        .id();
    app.update();
    let mut custom = bevy_widgetry::theme::WIDGETRY_DARK_THEME;
    custom.button.normal.background = Color::srgb(0.2, 0.3, 0.4);
    custom.button.normal.foreground = Color::srgb(0.8, 0.7, 0.6);
    custom.check_box.checked.normal.background = Color::srgb(0.5, 0.6, 0.7);
    WidgetryTheme::set_in_world(app.world_mut(), custom).unwrap();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        custom.button.normal.background
    );
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(fixed_border)
    );
    assert!(app.world().get::<Checked>(check).is_some());
    app.update();
    let label = named(app.world_mut(), "custom-theme-label");
    assert_eq!(app.world().get::<Text>(label).unwrap().0, "kept");
    assert_eq!(
        app.world().get::<TextColor>(label).unwrap().0,
        custom.button.normal.foreground
    );
    let late = app
        .world_mut()
        .spawn_scene(bsn! { Name("late-theme-label") Text("late") WidgetryText })
        .unwrap()
        .id();
    app.world_mut().entity_mut(root).add_child(late);
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(late).unwrap().0,
        custom.button.normal.foreground
    );
}

fn named(world: &mut World, name: &str) -> Entity {
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, actual)| actual.as_str() == name)
        .unwrap()
        .0
}

#[test]
fn nested_widgets_keep_their_color_scope_while_disabled_crosses_it() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins((WidgetryButtonPlugin, WidgetryCheckBoxPlugin));
    let parent_color = Color::srgb_u8(121, 32, 178);
    let child_color = Color::srgb_u8(11, 116, 101);
    let native_color = Color::srgb_u8(179, 83, 9);
    let mut parent = WidgetryButtonColorOverrides::default();
    parent.disabled.foreground = Some(parent_color);
    let mut child = WidgetryCheckBoxColorOverrides::default();
    child.unchecked.disabled.foreground = Some(child_color);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryButton { @colors: parent, @children: {Some(Box::new(bsn_list! {
                Node Children [Node Children [
                    Name("parent-label") Text("parent") WidgetryText--
                    Name("native-label") Text("native") TextColor(native_color)--
                    @WidgetryCheckBox { @colors: child } Name("child")
                        Children [Name("child-label") Text("child") WidgetryText]
                ]]
            }) as Box<dyn bevy::prelude::SceneList>)}, }

        })
        .unwrap()
        .id();
    let child = named(app.world_mut(), "child");
    let parent_label = named(app.world_mut(), "parent-label");
    let child_label = named(app.world_mut(), "child-label");
    let native_label = named(app.world_mut(), "native-label");

    for mode in [
        bevy_widgetry_theme::WIDGETRY_DARK_THEME,
        bevy_widgetry_theme::WIDGETRY_LIGHT_THEME,
        bevy_widgetry_theme::WIDGETRY_DARK_THEME,
    ] {
        WidgetryTheme::set_in_world(app.world_mut(), mode).unwrap();
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        app.update();
        assert!(app.world().get::<InteractionDisabled>(child).is_some());
        assert_eq!(
            app.world().get::<TextColor>(parent_label).unwrap().0,
            parent_color
        );
        assert_eq!(
            app.world().get::<TextColor>(child_label).unwrap().0,
            child_color
        );
        assert_eq!(
            app.world().get::<TextColor>(native_label).unwrap().0,
            native_color
        );

        app.world_mut()
            .entity_mut(child)
            .insert(InteractionDisabled);
        app.world_mut()
            .entity_mut(root)
            .remove::<InteractionDisabled>();
        app.update();
        assert!(app.world().get::<InteractionDisabled>(child).is_some());
        assert_eq!(
            app.world().get::<TextColor>(parent_label).unwrap().0,
            mode.button.normal.foreground
        );
        assert_eq!(
            app.world().get::<TextColor>(child_label).unwrap().0,
            child_color
        );
        app.world_mut()
            .entity_mut(child)
            .remove::<InteractionDisabled>();
        app.update();
        assert!(app.world().get::<InteractionDisabled>(child).is_none());
        assert_eq!(
            app.world().get::<TextColor>(child_label).unwrap().0,
            mode.check_box.unchecked.normal.foreground
        );
    }
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    WidgetryButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    WidgetryCheckBoxColorOverrides::clear_in_world(app.world_mut(), child).unwrap();
    app.update();
    let theme = bevy_widgetry_theme::WIDGETRY_DARK_THEME;
    assert_eq!(
        app.world().get::<TextColor>(parent_label).unwrap().0,
        theme.button.disabled.foreground
    );
    assert_eq!(
        app.world().get::<TextColor>(child_label).unwrap().0,
        theme.check_box.unchecked.disabled.foreground
    );
    assert_eq!(
        app.world().get::<TextColor>(native_label).unwrap().0,
        native_color
    );
}
