//! State：root/item enabled、focused active、selected、hover/press 与 theme。
//! stimuli 为输入 state、model metadata 和 theme 更新。
//! Invariant：disabled 优先级与 foreground 继承，theme 不改变 logical identity。
//! 滚回新 row 重新投影持久 state。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed, ScrollPosition};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewItem,
    WidgetryListViewPlugin, WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::{scene_app, switch_theme};
use bevy_widgetry_theme::WidgetryThemeMode;

fn fixture() -> (App, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    for index in 0..20 {
        model.push(index.to_string()).unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source, @item_height: 32.0,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list!{Node::default() Children [Text({value.clone()}) bevy_widgetry_core::text::WidgetryText]})},
        }
    }).expect("合法 ListView Scene 应展开").id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .expect("fixture 应只有一个 viewport");
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(200.0, 96.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    (app, source, root, viewport)
}

fn row(app: &mut App, index: usize) -> Entity {
    app.world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find(|(_, item)| item.index == index)
        .expect("目标 row 应已 rendered")
        .0
}

#[test]
fn shell_and_rows_have_fixed_geometry_and_normal_colors() {
    let (mut app, _, root, _) = fixture();
    let colors = WidgetryThemeMode::Dark.colors();
    let node = app.world().get::<Node>(root).unwrap();
    assert_eq!(node.border, UiRect::all(px(1)));
    assert_eq!(node.border_radius, BorderRadius::all(px(4)));
    assert_eq!(node.overflow, Overflow::clip());
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.list_view.container.normal.border)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        colors.list_view.container.normal.background
    );
    let target = row(&mut app, 0);
    let node = app.world().get::<Node>(target).unwrap();
    assert_eq!(node.width, percent(100));
    assert_eq!(node.height, px(32));
    assert_eq!(node.min_height, px(32));
    assert_eq!(node.max_height, px(32));
    assert_eq!(node.box_sizing, BoxSizing::BorderBox);
    assert_eq!(node.border, UiRect::all(px(1)));
    assert_eq!(node.border_radius, BorderRadius::all(px(3)));
    assert_eq!(node.padding, UiRect::horizontal(px(8)));
    assert_eq!(node.margin, UiRect::ZERO);
    assert_eq!(node.flex_shrink, 0.0);
    assert_eq!(
        app.world().get::<BackgroundColor>(target).unwrap().0,
        Color::NONE
    );
    assert_eq!(
        app.world().get::<ResolvedForeground>(target).unwrap().0,
        colors.list_view.container.normal.foreground
    );
}

#[test]
fn row_interactions_and_focus_project_complete_style() {
    let (mut app, source, root, _) = fixture();
    let colors = WidgetryThemeMode::Dark.colors();
    let selected = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(0)
        .unwrap();
    let active = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1)
        .unwrap();
    WidgetryListView::<String>::set_selected(&mut app.world_mut().commands(), root, 0);
    WidgetryListView::<String>::set_active(&mut app.world_mut().commands(), root, Some(1));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .selected,
        Some(selected)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        Some(active)
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.update();
    let a = row(&mut app, 0);
    let b = row(&mut app, 1);
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.list_view.container.focused.border)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.list_view.item.selected.background
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(Color::NONE)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(b).unwrap().0,
        Color::NONE
    );
    assert_eq!(
        *app.world().get::<BorderColor>(b).unwrap(),
        BorderColor::all(colors.list_view.container.focused.border)
    );
    WidgetryListView::<String>::set_active(&mut app.world_mut().commands(), root, Some(0));
    app.world_mut().entity_mut(a).insert(Hovered(true));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.list_view.item.hovered.background
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(colors.list_view.container.focused.border)
    );
    app.world_mut().entity_mut(a).insert(Pressed);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.list_view.item.pressed.background
    );
    app.world_mut().entity_mut(a).remove::<Pressed>();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.list_view.item.hovered.background
    );
    app.world_mut().entity_mut(a).insert(Hovered(false));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.list_view.item.selected.background
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.list_view.container.normal.border)
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(Color::NONE)
    );
    WidgetryListView::<String>::clear_selection(&mut app.world_mut().commands(), root);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        Color::NONE
    );
}

#[test]
fn disabled_item_preserves_active_border_and_suppresses_background() {
    let (mut app, source, root, _) = fixture();
    let colors = WidgetryThemeMode::Dark.colors();
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(0)
        .unwrap();
    WidgetryListView::<String>::set_selected(&mut app.world_mut().commands(), root, 0);
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .selected,
        Some(id)
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(0, true);
    app.update();
    let a = row(&mut app, 0);
    let wrapper = app.world().get::<Children>(a).unwrap()[0];
    let text = app.world().get::<Children>(wrapper).unwrap()[0];
    app.world_mut()
        .entity_mut(a)
        .insert((Hovered(true), Pressed));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        Color::NONE
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(colors.list_view.item.disabled_active_border)
    );
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.list_view.container.disabled.foreground
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        Some(id)
    );
    switch_theme(&mut app, WidgetryThemeMode::Light);
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(
            WidgetryThemeMode::Light
                .colors()
                .list_view
                .item
                .disabled_active_border
        )
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        Color::NONE
    );
    switch_theme(&mut app, WidgetryThemeMode::Dark);
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(Color::NONE)
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.world_mut()
        .entity_mut(a)
        .insert(Hovered(false))
        .remove::<Pressed>();
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(0, false);
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.list_view.container.normal.foreground
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(colors.list_view.item.active_border)
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(Color::NONE)
    );
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.list_view.container.disabled.border)
    );
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.list_view.container.disabled.foreground
    );
    let b = row(&mut app, 1);
    assert_eq!(
        app.world().get::<ResolvedForeground>(b).unwrap().0,
        colors.list_view.container.disabled.foreground
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.list_view.container.focused.border)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.list_view.item.selected.background
    );
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.list_view.container.normal.foreground
    );
}

#[test]
fn theme_refresh_is_immediate_and_new_rows_use_current_mode() {
    let (mut app, source, root, viewport) = fixture();
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(0)
        .unwrap();
    WidgetryListView::<String>::set_selected(&mut app.world_mut().commands(), root, 0);
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .selected,
        Some(id)
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    let a = row(&mut app, 0);
    let b = row(&mut app, 1);
    let c = row(&mut app, 2);
    app.world_mut().entity_mut(b).insert(Hovered(true));
    app.world_mut().entity_mut(c).insert(Pressed);
    app.update();
    let geometry = app.world().get::<Node>(a).unwrap().clone();
    switch_theme(&mut app, WidgetryThemeMode::Light);
    let colors = WidgetryThemeMode::Light.colors();
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.list_view.container.focused.border)
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(colors.list_view.item.active_border)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.list_view.item.selected.background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(b).unwrap().0,
        colors.list_view.item.hovered.background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(c).unwrap().0,
        colors.list_view.item.pressed.background
    );
    assert_eq!(
        app.world().get::<ResolvedForeground>(a).unwrap().0,
        colors.list_view.container.normal.foreground
    );
    assert_eq!(*app.world().get::<Node>(a).unwrap(), geometry);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    switch_theme(&mut app, WidgetryThemeMode::Dark);
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(
            WidgetryThemeMode::Dark
                .colors()
                .list_view
                .container
                .disabled
                .border
        )
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        Color::NONE
    );
    assert_eq!(
        app.world().get::<ResolvedForeground>(a).unwrap().0,
        WidgetryThemeMode::Dark
            .colors()
            .list_view
            .container
            .disabled
            .foreground
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    switch_theme(&mut app, WidgetryThemeMode::Light);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 320.0;
    app.update();
    let new_row = row(&mut app, 10);
    assert_eq!(
        app.world().get::<ResolvedForeground>(new_row).unwrap().0,
        colors.list_view.container.normal.foreground
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(new_row).unwrap().0,
        Color::NONE
    );
    let wrapper = app.world().get::<Children>(new_row).unwrap()[0];
    let text = app.world().get::<Children>(wrapper).unwrap()[0];
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.list_view.container.normal.foreground
    );
}
