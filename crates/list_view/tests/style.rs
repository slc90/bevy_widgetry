use bevy::app::Propagate;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed, ScrollPosition};
use bevy_widgetry_core::{ForegroundColor, ThemeMode};
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewItem,
    WidgetryListViewPlugin, WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::{scene_app, switch_theme};

/// 真实 ScrollArea shell 内提供嵌套 Text，viewport 固定为三行。
fn fixture() -> (App, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>();
    let mut model = WidgetryListModel::default();
    for index in 0..20 {
        model.push(index.to_string());
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source, @item_height: 32.0,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Node::default() Children [(Text({value.clone()}))])])},
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

/// 只通过公开 row identity 观察 virtualization 的输出。
fn row(app: &mut App, index: usize) -> Entity {
    app.world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find(|(_, item)| item.index == index)
        .expect("目标 row 应已 rendered")
        .0
}

/// root shell 提供圆角裁剪与固定 border，row chrome 包含在固定高度内。
#[test]
fn shell_and_rows_have_fixed_geometry_and_normal_colors() {
    let (mut app, _, root, _) = fixture();
    let colors = ThemeMode::Dark.colors();
    let node = app.world().get::<Node>(root).unwrap();
    assert_eq!(node.border, UiRect::all(px(1)));
    assert_eq!(node.border_radius, BorderRadius::all(px(4)));
    assert_eq!(node.overflow, Overflow::clip());
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.control_border)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        Color::NONE
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
        app.world()
            .get::<Propagate<ForegroundColor>>(target)
            .unwrap()
            .0
            .0,
        colors.foreground
    );
}

/// logical selection 与 active 可分属不同 row；hover/pressed 移除后恢复其余 state 的颜色。
#[test]
fn row_interactions_and_focus_project_complete_style() {
    let (mut app, source, root, _) = fixture();
    let colors = ThemeMode::Dark.colors();
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
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryListViewState {
            selected: Some(selected),
            active: Some(active),
        });
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.update();
    let a = row(&mut app, 0);
    let b = row(&mut app, 1);
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.control_border_active)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.item_background_selected
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
        BorderColor::all(colors.control_border_active)
    );
    app.world_mut()
        .get_mut::<WidgetryListViewState>(root)
        .unwrap()
        .active = Some(selected);
    app.world_mut().entity_mut(a).insert(Hovered(true));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.item_background_hovered
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(colors.control_border_active)
    );
    app.world_mut().entity_mut(a).insert(Pressed);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.control_background_pressed
    );
    app.world_mut().entity_mut(a).remove::<Pressed>();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.item_background_hovered
    );
    app.world_mut().entity_mut(a).insert(Hovered(false));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.item_background_selected
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.control_border)
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(Color::NONE)
    );
    app.world_mut()
        .get_mut::<WidgetryListViewState>(root)
        .unwrap()
        .selected = None;
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        Color::NONE
    );
}

/// item/root disabled 都抑制 chrome，但保留 logical active；恢复后重现当前 state。
#[test]
fn effective_disabled_suppresses_active_and_propagates_foreground() {
    let (mut app, source, root, _) = fixture();
    let colors = ThemeMode::Dark.colors();
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(0)
        .unwrap();
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryListViewState {
            selected: Some(id),
            active: Some(id),
        });
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
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        Color::NONE
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(Color::NONE)
    );
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.foreground_disabled
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(root)
            .unwrap()
            .active,
        Some(id)
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(0, false);
    app.update();
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.foreground
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(colors.control_border_active)
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.control_border_disabled)
    );
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.foreground_disabled
    );
    let b = row(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(b)
            .unwrap()
            .0
            .0,
        colors.foreground_disabled
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.control_border_active)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.item_background_selected
    );
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.foreground
    );
}

/// 不推进 frame 的 theme event 刷新全部现存 state；滚入的新 row 直接使用当前 theme。
#[test]
fn theme_refresh_is_immediate_and_new_rows_use_current_mode() {
    let (mut app, source, root, viewport) = fixture();
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(0)
        .unwrap();
    app.world_mut()
        .entity_mut(root)
        .insert(WidgetryListViewState {
            selected: Some(id),
            active: Some(id),
        });
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
    switch_theme(&mut app, ThemeMode::Light);
    let colors = ThemeMode::Light.colors();
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(colors.control_border_active)
    );
    assert_eq!(
        *app.world().get::<BorderColor>(a).unwrap(),
        BorderColor::all(colors.control_border_active)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        colors.item_background_selected
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(b).unwrap().0,
        colors.item_background_hovered
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(c).unwrap().0,
        colors.control_background_pressed
    );
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(a)
            .unwrap()
            .0
            .0,
        colors.foreground
    );
    assert_eq!(*app.world().get::<Node>(a).unwrap(), geometry);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    switch_theme(&mut app, ThemeMode::Dark);
    assert_eq!(
        *app.world().get::<BorderColor>(root).unwrap(),
        BorderColor::all(ThemeMode::Dark.colors().control_border_disabled)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).unwrap().0,
        Color::NONE
    );
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(a)
            .unwrap()
            .0
            .0,
        ThemeMode::Dark.colors().foreground_disabled
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    switch_theme(&mut app, ThemeMode::Light);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 320.0;
    app.update();
    let new_row = row(&mut app, 10);
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(new_row)
            .unwrap()
            .0
            .0,
        colors.foreground
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(new_row).unwrap().0,
        Color::NONE
    );
    let wrapper = app.world().get::<Children>(new_row).unwrap()[0];
    let text = app.world().get::<Children>(wrapper).unwrap()[0];
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        colors.foreground
    );
}
