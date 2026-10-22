#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::expect_used)]
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy_widgetry_segmented::{
    WidgetrySegmented, WidgetrySegmentedOption, WidgetrySegmentedPlugin,
};
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_theme::WidgetryThemeMode;

#[test]
fn checked_background_moves_in_one_update_without_changing_the_selection() {
    let mut app = scene_app();
    app.add_plugins(WidgetrySegmentedPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetrySegmented Children [
            @WidgetrySegmentedOption Children [Text("Today")]--
            @WidgetrySegmentedOption Children [Text("Last 30 days")]
        ] })
        .unwrap()
        .id();
    app.update();
    let children = app.world().get::<Children>(root).unwrap().to_vec();
    let colors = WidgetryThemeMode::Dark.colors().segmented;
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        colors.container.normal.background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(children[0]).unwrap().0,
        colors.option.checked.normal.background
    );
    WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    assert!(app.world().get::<Checked>(children[1]).is_some());
    assert_eq!(
        app.world().get::<BackgroundColor>(children[0]).unwrap().0,
        colors.option.unchecked.normal.background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(children[1]).unwrap().0,
        colors.option.checked.normal.background
    );
}

fn fixture() -> (App, Entity, Vec<Entity>) {
    let mut app = scene_app();
    app.add_plugins(WidgetrySegmentedPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetrySegmented Children [
            @WidgetrySegmentedOption Children [bevy_widgetry_core::text::WidgetryText]--
            @WidgetrySegmentedOption Children [Text("Long content")]
        ] })
        .unwrap()
        .id();
    app.update();
    let children = app.world().get::<Children>(root).unwrap().to_vec();
    (app, root, children)
}

#[test]
fn focus_hover_and_disabled_keep_selection_and_local_requests() {
    use bevy::input_focus::InputFocus;
    use bevy::picking::hover::Hovered;
    use bevy::ui::InteractionDisabled;
    let (mut app, root, options) = fixture();
    let colors = WidgetryThemeMode::Dark.colors().segmented;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().entity_mut(options[1]).insert(Hovered(true));
    app.update();
    assert_eq!(
        app.world().get::<BorderColor>(root).unwrap().top,
        colors.container.focused.border
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(options[1]).unwrap().0,
        colors.option.unchecked.hovered.background
    );
    app.world_mut()
        .entity_mut(options[0])
        .insert(InteractionDisabled);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(options[0]).unwrap().0,
        colors.option.checked.disabled.background
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        colors.container.disabled.background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(options[1]).unwrap().0,
        colors.option.unchecked.disabled.background
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert!(app.world().get::<InteractionDisabled>(options[0]).is_some());
    assert!(app.world().get::<InteractionDisabled>(options[1]).is_none());
    assert!(app.world().get::<Checked>(options[0]).is_some());
    assert_eq!(
        app.world().get::<BorderColor>(root).unwrap().top,
        colors.container.normal.border
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(options[1]).unwrap().0,
        colors.option.unchecked.hovered.background
    );
}

#[test]
fn theme_group_and_local_overrides_have_fieldwise_priority() {
    use bevy_widgetry_segmented::{
        WidgetrySegmentedColorOverrides as Group, WidgetrySegmentedOptionColorOverrides as Local,
    };
    let (mut app, root, options) = fixture();
    let mut group = Group::default();
    group.option.checked.normal.background = Some(Color::srgb(0.1, 0.2, 0.3));
    group.option.checked.normal.foreground = Some(Color::srgb(0.4, 0.5, 0.6));
    Group::set_in_world(app.world_mut(), root, group.clone()).unwrap();
    let mut local = Local::default();
    local.checked.normal.background = Some(Color::WHITE);
    Local::set_in_world(app.world_mut(), options[0], local).unwrap();
    for mode in [
        WidgetryThemeMode::Light,
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            mode.colors().segmented.container.normal.background
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(options[0]).unwrap().0,
            Color::WHITE
        );
        assert_eq!(
            app.world()
                .get::<bevy_widgetry_core::foreground::ResolvedForeground>(options[0])
                .unwrap()
                .0,
            group.option.checked.normal.foreground.unwrap()
        );
        assert!(app.world().get::<Checked>(options[0]).is_some());
    }
    Local::clear_in_world(app.world_mut(), options[0]).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(options[0]).unwrap().0,
        group.option.checked.normal.background.unwrap()
    );
    Group::clear(&mut app.world_mut().commands(), root);
    app.world_mut().flush();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(options[0]).unwrap().0,
        WidgetryThemeMode::KamuriViolet
            .colors()
            .segmented
            .option
            .checked
            .normal
            .background
    );
}

#[test]
fn invalid_colors_and_managed_targets_reject_without_partial_changes() {
    use bevy_widgetry_core::color::WidgetryStyleOwner;
    use bevy_widgetry_segmented::{
        WidgetrySegmentedColorOverrides as Group, WidgetrySegmentedOptionColorOverrides as Local,
    };
    let (mut app, root, options) = fixture();
    let mut colors = Group::default();
    colors.option.checked.hovered.foreground = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
    let error = Group::set_in_world(app.world_mut(), root, colors).unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(Group::get(app.world(), root).unwrap(), &Group::default());
    assert!(Group::get(app.world(), options[0]).is_err());
    assert!(!Local::clear_in_world(app.world_mut(), options[0]).unwrap());
    app.world_mut()
        .entity_mut(options[0])
        .insert(WidgetryStyleOwner::<WidgetrySegmentedOption>::new(root));
    assert!(Local::clear_in_world(app.world_mut(), options[0]).is_err());
}

#[test]
fn natural_content_layout_and_node_patches_preserve_content_ownership() {
    use bevy_widgetry_test_utils::{add_ui_plugins, spawn_ui_camera};
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins(WidgetrySegmentedPlugin);
    spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetrySegmented { } Node { position_type: PositionType::Absolute, padding: UiRect::all(px(3)) }
        Children [
            @WidgetrySegmentedOption Children [Node { width: px(20), height: px(10) } BackgroundColor(Color::WHITE)]--
            @WidgetrySegmentedOption Children [Node { width: px(90), height: px(10) } BackgroundColor(Color::BLACK)]
        ]
    }).unwrap().id();
    app.update();
    let children = app.world().get::<Children>(root).unwrap();
    let a = app
        .world()
        .get::<ComputedNode>(children[0])
        .unwrap()
        .size()
        .x;
    let b = app
        .world()
        .get::<ComputedNode>(children[1])
        .unwrap()
        .size()
        .x;
    assert!(b > a + 60.0);
    assert_eq!(
        app.world().get::<Node>(root).unwrap().padding,
        UiRect::all(px(3))
    );
    for (option, expected) in children.iter().zip([Color::WHITE, Color::BLACK]) {
        let content = app.world().get::<Children>(option).unwrap()[0];
        assert_eq!(
            app.world().get::<BackgroundColor>(content).unwrap().0,
            expected
        );
    }
}
