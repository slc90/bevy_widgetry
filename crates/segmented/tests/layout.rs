#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_segmented::{
    WidgetrySegmented, WidgetrySegmentedOption, WidgetrySegmentedPlugin,
};
use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};
use bevy_widgetry_theme::WidgetryThemeMode;

fn fixture(equal_width: bool) -> (App, Entity, Vec<Entity>) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins(WidgetrySegmentedPlugin);
    spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetrySegmented { @equal_width: equal_width }
            Node { position_type: PositionType::Absolute, width: px(400) }
            Children [
                @WidgetrySegmentedOption Children [Node { width: px(20), height: px(10) }]--
                @WidgetrySegmentedOption Children [Node { width: px(70), height: px(10) }]--
                @WidgetrySegmentedOption Children [Node { width: px(45), height: px(10) }]--
                @WidgetrySegmentedOption Children [Node { width: px(90), height: px(10) }]
            ]
        })
        .unwrap()
        .id();
    app.update();
    let children = app.world().get::<Children>(root).unwrap().to_vec();
    (app, root, children)
}

fn widths(app: &App, children: &[Entity]) -> Vec<f32> {
    children
        .iter()
        .map(|child| app.world().get::<ComputedNode>(*child).unwrap().size().x)
        .collect()
}

fn assert_equal(widths: &[f32]) {
    assert!(
        widths.iter().all(|width| (width - widths[0]).abs() <= 1.0),
        "{widths:?}"
    );
    assert!(widths[0] > 0.0);
}

#[test]
fn equal_width_distributes_patched_container_width_independent_of_content() {
    let (mut app, root, children) = fixture(true);
    let initial = widths(&app, &children);
    assert_equal(&initial);
    for mode in [
        WidgetryThemeMode::Light,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        app.world_mut()
            .entity_mut(children[1])
            .insert(InteractionDisabled);
        app.world_mut().entity_mut(root).insert(InteractionDisabled);
        WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 2);
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        app.update();
        assert_eq!(widths(&app, &children), initial);
    }
    let extra = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetrySegmentedOption ChildOf(root) Children [Node { width: px(10), height: px(10) }]
        })
        .unwrap()
        .id();
    app.update();
    let mut children = children;
    children.push(extra);
    assert_equal(&widths(&app, &children));
    assert!(widths(&app, &children)[0] < initial[0]);
}

#[test]
fn default_widths_follow_children_and_explicit_node_patch_can_override_grid() {
    let (app, _, children) = fixture(false);
    let sizes = widths(&app, &children);
    assert!(sizes[1] > sizes[0] + 40.0, "{sizes:?}");
    let (mut app, root, children) = fixture(true);
    app.world_mut()
        .entity_mut(root)
        .get_mut::<Node>()
        .unwrap()
        .width = px(600);
    app.update();
    let sizes = widths(&app, &children);
    assert_equal(&sizes);
    assert!(sizes[0] > 140.0);
    let patched = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetrySegmented { @equal_width: true }
            Node { display: Display::Flex, position_type: PositionType::Absolute }
            Children [
                @WidgetrySegmentedOption Children [Node { width: px(20), height: px(10) }]--
                @WidgetrySegmentedOption Children [Node { width: px(90), height: px(10) }]
            ]
        })
        .unwrap()
        .id();
    app.update();
    let children = app.world().get::<Children>(patched).unwrap();
    let sizes = widths(&app, children);
    assert!(sizes[1] > sizes[0] + 60.0, "{sizes:?}");
}

#[test]
fn intrinsic_equal_width_uses_content_without_a_fixed_group_width() {
    let (mut app, root, children) = fixture(true);
    app.world_mut().get_mut::<Node>(root).unwrap().width = Val::Auto;
    app.update();
    assert_equal(&widths(&app, &children));
    assert!(widths(&app, &children)[0] >= 110.0);
}

#[test]
fn vertical_layout_preserves_content_heights_and_uses_the_shared_style() {
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy::ui::Checked;
    use bevy_widgetry_segmented::SegmentedOrientation;
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins(WidgetrySegmentedPlugin);
    spawn_ui_camera(&mut app, UVec2::new(800, 600), 1.0);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetrySegmented { @orientation: SegmentedOrientation::Vertical, @equal_width: true }
            Node { position_type: PositionType::Absolute }
            Children [
                @WidgetrySegmentedOption Children [Node { width: px(20), height: px(10) }]--
                @WidgetrySegmentedOption Children [Node { width: px(80), height: px(50) }]
            ]
        })
        .unwrap()
        .id();
    app.update();
    let children = app.world().get::<Children>(root).unwrap().to_vec();
    let first = app.world().get::<ComputedNode>(children[0]).unwrap().size();
    let second = app.world().get::<ComputedNode>(children[1]).unwrap().size();
    assert!((first.x - second.x).abs() <= 1.0);
    assert!(second.y > first.y + 20.0);
    let a = app
        .world()
        .get::<bevy::ui::UiGlobalTransform>(children[0])
        .unwrap()
        .translation;
    let b = app
        .world()
        .get::<bevy::ui::UiGlobalTransform>(children[1])
        .unwrap()
        .translation;
    assert!(b.y > a.y);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    for mode in [
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::Light,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        app.update();
        assert_eq!(
            app.world().get::<BorderColor>(root).unwrap().top,
            mode.colors().segmented.container.focused.border
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(children[0]).unwrap().0,
            mode.colors().segmented.option.checked.normal.background
        );
        assert!(app.world().get::<Checked>(children[0]).is_some());
    }
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(children[0]).unwrap().0,
        WidgetryThemeMode::KamuriViolet
            .colors()
            .segmented
            .option
            .checked
            .disabled
            .background
    );
    assert_eq!(
        app.world().get::<ComputedNode>(children[1]).unwrap().size(),
        second
    );
}
