//! State：closed/open、focus、root/viewport/content/item enabled、selection/active。
//! stimuli 为真实输入、CRUD、theme 与 lifecycle。
//! Guards：closed/disabled 输入不能选择。
//! invariant 为持久 ListView authority、单次 root 通知和独立 renderer subtree。
//! Coupling：关闭及时阻断同帧剩余 keyboard，解除禁用后保留 model metadata 并可重新打开。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::camera::visibility::VisibilitySystems;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::touch::TouchPhase;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusSystems};
use bevy::picking::PickingSystems;
use bevy::picking::events::{Click, Pointer, Scroll};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::{ComputedStackIndex, InteractionDisabled, ScrollPosition};
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverSide};
use bevy::ui_widgets::{Activate, Button, ScrollArea, ValueChange};
use bevy::window::PrimaryWindow;
use bevy_widgetry_asset::{BuiltinFont, BuiltinIcon};
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::{WidgetryAppExt, z_index};
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewItem,
    WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry_test_utils::{
    add_keyboard_dispatch, add_ui_plugins, advance_until, press_key, primary_click, primary_press,
    queue_key, scene_app, spawn_ui_camera, switch_theme,
};
use bevy_widgetry_theme::{WIDGETRY_DARK_THEME, WIDGETRY_LIGHT_THEME, WidgetryThemeMode};
use std::time::Duration;

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, Option<WidgetryListItemId>)>);

struct Fixture {
    app: App,
    source: Entity,
    root: Entity,
    field: Entity,
    popup: Entity,
    list: Entity,
    viewport: Entity,
    window: Entity,
}

fn record(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    roots: Query<(), With<WidgetryComboBox<String>>>,
    mut changes: ResMut<Changes>,
) {
    if roots.contains(event.source) {
        changes.0.push((event.source, event.value));
    }
}

fn fixture(len: usize) -> Fixture {
    let mut app = scene_app();
    app.init_resource::<UiScale>()
        .init_resource::<ButtonInput<KeyCode>>()
        .register_widgetry_combo_box::<String>()
        .unwrap()
        .init_resource::<Changes>()
        .add_observer(record);
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let mut model = WidgetryListModel::default();
    for index in 0..len {
        model.push(index.to_string()).unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source, @item_height: 24.0, @max_visible_items: 3,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
        }
    }).unwrap().id();
    let children = app.world().get::<Children>(root).unwrap();
    let (field, popup) = (children[0], children[1]);
    let list = app.world().get::<Children>(popup).unwrap()[0];
    let viewport = app
        .world()
        .get::<Children>(list)
        .unwrap()
        .iter()
        .find(|&child| app.world().get::<ScrollArea>(child).is_some())
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(200.0, 72.0),
        content_size: Vec2::new(200.0, len as f32 * 24.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    Fixture {
        app,
        source,
        root,
        field,
        popup,
        list,
        viewport,
        window,
    }
}

fn real_ui_app() -> App {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.register_widgetry_combo_box::<String>()
        .unwrap()
        .init_resource::<Changes>()
        .add_observer(record)
        .configure_sets(
            PostUpdate,
            (
                VisibilitySystems::VisibilityPropagate,
                bevy::ui::UiSystems::Stack,
            )
                .before(bevy::ui::UiSystems::Propagate),
        );
    add_keyboard_dispatch(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(600), 1.0);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    let warm = app.world_mut().spawn_scene(bsn! {
        @WidgetryIcon { @path: {BuiltinIcon::ChevronDown.path()}, @max_size: {Some(UVec2::splat(16))} }
    }).unwrap().id();
    advance_until(
        &mut app,
        Duration::from_secs(10),
        "ComboBox 字体和 renderer icon 前置资源",
        |world| {
            world.resource::<Assets<Font>>().contains(&font)
                && world.get::<Children>(warm).is_some_and(|children| {
                    world.get::<ImageNode>(children[0]).is_some_and(|image| {
                        world.resource::<Assets<Image>>().contains(&image.image)
                    })
                })
        },
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
    app
}

fn assert_business_content(app: &App, parent: Entity, value: &str) {
    let world = app.world();
    let wrapper = world.get::<Children>(parent).unwrap()[0];
    let children = world.get::<Children>(wrapper).unwrap();
    let (text, icon) = (children[0], children[1]);
    let image = world.get::<Children>(icon).unwrap()[0];
    assert_eq!(world.get::<Text>(text).unwrap().0, value);
    assert!(
        !world
            .get::<bevy::text::TextLayoutInfo>(text)
            .unwrap()
            .glyphs
            .is_empty()
    );
    let node = world.get::<ImageNode>(image).unwrap();
    assert!(world.resource::<Assets<Image>>().contains(&node.image));
    assert_eq!(
        world
            .resource::<Assets<Image>>()
            .get(&node.image)
            .unwrap()
            .size(),
        UVec2::splat(16)
    );
    assert_eq!(node.color, world.get::<TextColor>(text).unwrap().0);
    for entity in [wrapper, text, icon, image] {
        assert!(world.get::<InheritedVisibility>(entity).unwrap().get());
        assert!(
            world.get::<ComputedStackIndex>(entity).unwrap().0
                > world.get::<ComputedStackIndex>(parent).unwrap().0
        );
        assert!(
            world
                .get::<ComputedNode>(entity)
                .unwrap()
                .size()
                .min_element()
                > 0.0
        );
    }
}

#[test]
fn real_popup_layout_bounds_rows_and_preserves_list_identity_across_toggle() {
    let mut app = real_ui_app();
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let mut model = WidgetryListModel::default();
    for index in 0..20 {
        model.push(index.to_string()).unwrap();
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source, @item_height: 24.0, @max_visible_items: 3,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
        }
        Node { width: px(200), height: px(32), left: px(100), top: px(100), position_type: PositionType::Absolute }
    }).unwrap().id();
    let children = app.world().get::<Children>(root).unwrap();
    let (field, popup) = (children[0], children[1]);
    let list = app.world().get::<Children>(popup).unwrap()[0];
    let viewport = app
        .world()
        .get::<Children>(list)
        .unwrap()
        .iter()
        .find(|child| app.world().get::<ScrollArea>(*child).is_some())
        .unwrap();
    assert_eq!(
        app.world().get::<ComputedNode>(viewport).unwrap().size(),
        Vec2::ZERO
    );
    app.update();
    assert!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .next()
            .is_none()
    );
    app.world_mut().trigger(primary_press(field));
    app.world_mut().flush();
    app.world_mut().trigger(primary_click(field));
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    assert_eq!(
        app.world().get::<ComputedNode>(popup).unwrap().size().y,
        74.0
    );
    assert_eq!(
        app.world().get::<ComputedNode>(viewport).unwrap().size().y,
        72.0
    );
    let rendered = app
        .world_mut()
        .query::<&WidgetryListViewItem>()
        .iter(app.world())
        .map(|item| item.index)
        .collect::<Vec<_>>();
    assert_eq!(rendered.len(), 3);
    for index in 0..3 {
        assert!(rendered.contains(&index));
        let target = row(&mut app, list, index);
        assert_eq!(
            app.world().get::<ComputedNode>(target).unwrap().size().y,
            24.0
        );
        assert!(
            app.world()
                .get::<InheritedVisibility>(target)
                .unwrap()
                .get()
        );
    }
    let state = *app.world().get::<WidgetryListViewState>(list).unwrap();
    press_key(&mut app, window, KeyCode::Escape);
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    app.world_mut().trigger(primary_press(field));
    app.world_mut().flush();
    app.world_mut().trigger(primary_click(field));
    app.update();
    assert_eq!(app.world().get::<Children>(popup).unwrap()[0], list);
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(list).unwrap(),
        state
    );
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn popup_layout_tracks_model_length_without_recreating_list() {
    let Fixture {
        mut app,
        source,
        popup,
        list,
        ..
    } = fixture(2);
    let node = app.world().get::<Node>(list).unwrap();
    assert_eq!(node.width, percent(100));
    assert_eq!(node.height, percent(100));
    assert_eq!(node.border, UiRect::ZERO);
    assert_eq!(app.world().get::<TabIndex>(list).unwrap().0, -1);
    assert_eq!(app.world().get::<Node>(popup).unwrap().height, px(50));
    for value in ["2", "3", "4"] {
        app.world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap()
            .push(value.to_owned())
            .unwrap();
        app.update();
        assert_eq!(app.world().get::<Node>(popup).unwrap().height, px(74));
    }
    app.update();
    assert_eq!(app.world().get::<Node>(popup).unwrap().height, px(74));
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .clear();
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .push(String::from("new"))
        .unwrap();
    app.update();
    assert_eq!(app.world().get::<Node>(popup).unwrap().height, px(26));
    assert_eq!(app.world().get::<Children>(popup).unwrap()[0], list);
}

#[test]
fn popup_and_field_keep_geometry_and_current_theme() {
    let Fixture {
        mut app,
        root,
        field,
        popup,
        list,
        ..
    } = fixture(2);
    assert!(app.world().get::<WidgetryComboBox<String>>(root).is_some());
    assert!(app.world().get::<Button>(field).is_some());
    assert!(app.world().get::<WidgetryListView<String>>(list).is_some());
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    let node = app.world().get::<Node>(field).unwrap();
    assert_eq!(node.height, px(36));
    assert_eq!(node.width, percent(100));
    assert_eq!(node.padding, UiRect::axes(px(10), px(0)));
    let icon = app.world().get::<Children>(field).unwrap()[1];
    assert!(app.world().get::<WidgetryIcon>(icon).is_some());
    let popover = app.world().get::<Popover>(popup).unwrap();
    assert_eq!(popover.window_margin, 8.0);
    assert_eq!(popover.positions.len(), 2);
    for (position, side) in popover
        .positions
        .iter()
        .zip([PopoverSide::Bottom, PopoverSide::Top])
    {
        assert_eq!(position.side, side);
        assert_eq!(position.align, PopoverAlign::Start);
        assert_eq!(position.gap, 0.0);
    }
    assert_eq!(
        app.world().get::<GlobalZIndex>(popup).unwrap().0,
        z_index::POPUP
    );
    assert_eq!(app.world().get::<Node>(popup).unwrap().width, percent(100));
    assert_eq!(
        app.world().get::<BackgroundColor>(popup).unwrap().0,
        WIDGETRY_DARK_THEME.combo_box.popup.normal.background
    );
    app.world_mut().trigger(Activate { entity: field });
    switch_theme(&mut app, WidgetryThemeMode::Light);
    assert_eq!(
        app.world().get::<BackgroundColor>(popup).unwrap().0,
        WIDGETRY_LIGHT_THEME.combo_box.popup.normal.background
    );
    assert_eq!(
        *app.world().get::<BorderColor>(popup).unwrap(),
        BorderColor::all(WIDGETRY_LIGHT_THEME.combo_box.popup.normal.border)
    );
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    app.world_mut().trigger(Activate { entity: field });
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().get::<Children>(popup).unwrap()[0], list);
    let source = app
        .world()
        .get::<WidgetryComboBox<String>>(root)
        .unwrap()
        .source();
    let other = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox::<String> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_, _: &String| bsn_list![])},
            }
            InteractionDisabled
        })
        .unwrap()
        .id();
    app.update();
    let children = app.world().get::<Children>(other).unwrap();
    let (other_field, other_popup) = (children[0], children[1]);
    let other_list = app.world().get::<Children>(other_popup).unwrap()[0];
    assert!(
        app.world()
            .get::<InteractionDisabled>(other_field)
            .is_some()
    );
    assert!(app.world().get::<InteractionDisabled>(other_list).is_some());
    assert_eq!(
        app.world().get::<BackgroundColor>(other_field).unwrap().0,
        WIDGETRY_LIGHT_THEME.button.disabled.background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(other_popup).unwrap().0,
        WIDGETRY_LIGHT_THEME.combo_box.popup.normal.background
    );
    app.world_mut().trigger(Activate {
        entity: other_field,
    });
    assert_eq!(
        *app.world().get::<Visibility>(other_popup).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn shared_model_crud_and_user_selection_are_independent() {
    let Fixture {
        mut app,
        source,
        root,
        field,
        list,
        viewport,
        ..
    } = fixture(2);
    let other = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source, @item_height: 24.0, @max_visible_items: 3,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
        }
    }).unwrap().id();
    let other_popup = app.world().get::<Children>(other).unwrap()[1];
    let other_list = app.world().get::<Children>(other_popup).unwrap()[0];
    let other_viewport = app
        .world()
        .get::<Children>(other_list)
        .unwrap()
        .iter()
        .find(|&child| app.world().get::<ScrollArea>(child).is_some())
        .unwrap();
    let geometry = *app.world().get::<ComputedNode>(viewport).unwrap();
    app.world_mut().entity_mut(other_viewport).insert(geometry);
    app.update();
    let first_id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(0)
        .unwrap();
    let target_id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1)
        .unwrap();
    let target = row(&mut app, list, 1);
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().trigger(primary_click(target));
    app.world_mut().flush();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(target_id)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(other_list)
            .unwrap()
            .selected,
        Some(first_id)
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(target_id))]
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .insert(0, String::from("new"))
        .unwrap();
    app.update();
    for view in [list, other_list] {
        let first = row(&mut app, view, 0);
        let text = app.world().get::<Children>(first).unwrap()[0];
        assert_eq!(app.world().get::<Text>(text).unwrap().0, "new");
    }
    assert!(
        app.world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap()
            .move_item(2, 0)
    );
    app.update();
    for view in [list, other_list] {
        let first = row(&mut app, view, 0);
        assert_eq!(
            app.world().get::<WidgetryListViewItem>(first).unwrap().id,
            target_id
        );
    }
    assert_eq!(
        app.world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap()
            .remove(0),
        Some(String::from("1"))
    );
    app.update();
    for view in [list, other_list] {
        let first = row(&mut app, view, 0);
        let text = app.world().get::<Children>(first).unwrap()[0];
        assert_eq!(app.world().get::<Text>(text).unwrap().0, "new");
    }
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        None
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(other_list)
            .unwrap()
            .selected,
        Some(first_id)
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(target_id))]
    );
}

#[test]
fn arbitrary_renderer_builds_independent_field_and_row_subtrees() {
    let mut app = real_ui_app();
    let mut model = WidgetryListModel::default();
    model.push(String::from("0")).unwrap();
    model.push(String::from("1")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source, @item_height: 24.0,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| {
                bsn_list![(Node Children [{bsn_list![
                    (Text({value.clone()})),
                    (@WidgetryIcon { @path: {BuiltinIcon::ChevronDown.path()}, @max_size: {Some(UVec2::new(16, 16))} }),
                ]}])]
            })},
        }
        Node { width: px(200), height: px(32) }
    }).unwrap().id();
    let field = app.world().get::<Children>(root).unwrap()[0];
    let popup = app.world().get::<Children>(root).unwrap()[1];
    let list = app.world().get::<Children>(popup).unwrap()[0];
    app.update();
    // 首次真实 layout 之前 viewport 尺寸尚未有效。
    // 先完成 measurement 再 bootstrap rows，避免把未生成内容当作 renderer 失败。
    app.update();
    app.world_mut().trigger(primary_press(field));
    app.world_mut().flush();
    app.world_mut().trigger(primary_click(field));
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    let content = app.world().get::<Children>(field).unwrap()[0];
    let first_row = row(&mut app, list, 0);
    let old_field_wrapper = app.world().get::<Children>(content).unwrap()[0];
    let old_row_wrapper = app.world().get::<Children>(first_row).unwrap()[0];
    assert_ne!(old_field_wrapper, old_row_wrapper);
    for parent in [content, first_row] {
        assert_business_content(&app, parent, "0");
    }

    let old_field_children = app
        .world()
        .get::<Children>(old_field_wrapper)
        .unwrap()
        .iter()
        .collect::<Vec<_>>();
    let old_row_children = app
        .world()
        .get::<Children>(old_row_wrapper)
        .unwrap()
        .iter()
        .collect::<Vec<_>>();
    for children in [&old_field_children, &old_row_children] {
        assert_eq!(children.len(), 2);
        assert_eq!(app.world().get::<Text>(children[0]).unwrap().0, "0");
        assert!(app.world().get::<WidgetryIcon>(children[1]).is_some());
    }
    assert!(
        old_field_children
            .iter()
            .all(|entity| !old_row_children.contains(entity))
    );
    let old_images: Vec<_> = [&old_field_children, &old_row_children]
        .into_iter()
        .map(|children| app.world().get::<Children>(children[1]).unwrap()[0])
        .collect();
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(0)
        .unwrap()
        .unwrap() = String::from("updated");
    app.update();
    for entity in [old_field_wrapper, old_row_wrapper]
        .into_iter()
        .chain(old_field_children)
        .chain(old_row_children)
        .chain(old_images)
    {
        assert!(app.world().get_entity(entity).is_err());
    }
    assert_eq!(app.world().get::<Children>(field).unwrap()[0], content);
    let current_row = row(&mut app, list, 0);
    for parent in [content, current_row] {
        let wrapper = app.world().get::<Children>(parent).unwrap()[0];
        let text = app.world().get::<Children>(wrapper).unwrap()[0];
        assert_eq!(app.world().get::<Text>(text).unwrap().0, "updated");
    }
    for parent in [content, current_row] {
        assert_business_content(&app, parent, "updated");
    }
    let wrappers: Vec<_> = [content, current_row]
        .into_iter()
        .map(|parent| app.world().get::<Children>(parent).unwrap()[0])
        .collect();
    let old_colors: Vec<_> = wrappers
        .iter()
        .map(|wrapper| {
            let text = app.world().get::<Children>(*wrapper).unwrap()[0];
            app.world().get::<TextColor>(text).unwrap().0
        })
        .collect();
    switch_theme(&mut app, WidgetryThemeMode::Light);
    app.update();
    for (position, parent) in [content, current_row].into_iter().enumerate() {
        assert_eq!(
            app.world().get::<Children>(parent).unwrap()[0],
            wrappers[position]
        );
        let text = app.world().get::<Children>(wrappers[position]).unwrap()[0];
        assert_ne!(
            app.world().get::<TextColor>(text).unwrap().0,
            old_colors[position]
        );
        assert_business_content(&app, parent, "updated");
    }
}

#[test]
fn opening_requires_items_and_transfers_focus_to_list() {
    let Fixture {
        mut app,
        source,
        field,
        popup,
        list,
        ..
    } = fixture(0);
    let original_focus = app.world().resource::<InputFocus>().get();
    app.world_mut().trigger(Activate { entity: field });
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), original_focus);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .push(String::from("new"))
        .unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        None
    );
    app.world_mut().trigger(Activate { entity: field });
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .clear();
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

fn row(app: &mut App, list: Entity, index: usize) -> Entity {
    app.world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find_map(|(entity, item)| {
            if item.index != index {
                return None;
            }
            let mut ancestor = entity;
            while let Some(parent) = app.world().get::<ChildOf>(ancestor) {
                ancestor = parent.parent();
                if ancestor == list {
                    return Some(entity);
                }
            }
            None
        })
        .unwrap()
}

#[test]
fn row_selection_and_reselection_close_without_duplicate_notifications() {
    let Fixture {
        mut app,
        root,
        field,
        popup,
        list,
        source,
        ..
    } = fixture(3);
    let target = row(&mut app, list, 1);
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1)
        .unwrap();
    app.world_mut().trigger(Activate { entity: field });
    let target_text = app.world().get::<Children>(target).unwrap()[0];
    app.world_mut().trigger(primary_click(target_text));
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, Some(id))]);
    app.update();
    let content = app.world().get::<Children>(field).unwrap()[0];
    let field_text = app.world().get::<Children>(content).unwrap()[0];
    assert_eq!(app.world().get::<Text>(field_text).unwrap().0, "1");
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().trigger(primary_click(target));
    app.world_mut().flush();
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, Some(id))]);
    assert_eq!(app.world().get::<Children>(content).unwrap()[0], field_text);
}

#[test]
fn escape_returns_focus_without_resetting_list_state() {
    let Fixture {
        mut app,
        field,
        popup,
        list,
        window,
        ..
    } = fixture(5);
    app.world_mut().trigger(Activate { entity: field });
    press_key(&mut app, window, KeyCode::ArrowDown);
    let state = *app.world().get::<WidgetryListViewState>(list).unwrap();
    assert_ne!(state.selected, state.active);
    press_key(&mut app, window, KeyCode::Escape);
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(list).unwrap(),
        state
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn bounded_popup_inherits_virtualization_and_keyboard_selection() {
    let Fixture {
        mut app,
        source,
        root,
        field,
        popup,
        list,
        viewport,
        window,
    } = fixture(10_000);
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .count(),
        3
    );
    assert_eq!(app.world().get::<Node>(popup).unwrap().height, px(74));
    app.world_mut().trigger(Activate { entity: field });
    let initial_row = row(&mut app, list, 0);
    let pointer = primary_click(initial_row);
    app.world_mut().trigger(Pointer::new(
        pointer.pointer_id,
        pointer.pointer_location.clone(),
        Scroll {
            x: 0.0,
            y: -24.0,
            unit: MouseScrollUnit::Pixel,
            hit: pointer.hit.clone(),
            phase: TouchPhase::Moved,
        },
        initial_row,
    ));
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        24.0
    );
    press_key(&mut app, window, KeyCode::PageDown);
    assert!(app.world().get::<ScrollPosition>(viewport).unwrap().0.y > 0.0);
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .count(),
        3
    );
    press_key(&mut app, window, KeyCode::End);
    let last = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(9_999)
        .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .active,
        Some(last)
    );
    press_key(&mut app, window, KeyCode::Enter);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(last)
    );
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(last))]
    );
}

#[test]
fn disabled_items_and_root_keep_listview_contract() {
    let Fixture {
        mut app,
        source,
        root,
        field,
        popup,
        list,
        window,
        ..
    } = fixture(3);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .set_disabled(1, true);
    app.update();
    let disabled_row = row(&mut app, list, 1);
    assert!(
        app.world()
            .get::<InteractionDisabled>(disabled_row)
            .is_some()
    );
    let initial = app
        .world()
        .get::<WidgetryListViewState>(list)
        .unwrap()
        .selected;
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().trigger(primary_click(disabled_row));
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        initial
    );
    let disabled_id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1)
        .unwrap();
    for key in [KeyCode::Enter, KeyCode::Space] {
        WidgetryListView::<String>::set_active(&mut app.world_mut().commands(), list, Some(1));
        app.world_mut().flush();
        assert_eq!(
            app.world()
                .get::<WidgetryListViewState>(list)
                .unwrap()
                .active,
            Some(disabled_id)
        );
        press_key(&mut app, window, key);
        assert_eq!(
            app.world()
                .get::<WidgetryListViewState>(list)
                .unwrap()
                .selected,
            initial
        );
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );
    }
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        initial
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert!(app.world().get::<InteractionDisabled>(field).is_some());
    assert!(app.world().get::<InteractionDisabled>(list).is_some());
    assert_eq!(
        app.world()
            .get::<WidgetryListModel<String>>(source)
            .unwrap()
            .is_disabled(0),
        Some(false)
    );
    let id = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1)
        .unwrap();
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, id);
    app.world_mut().flush();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(id)
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, Some(id))]);
    app.world_mut().resource_mut::<Changes>().0.clear();
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert!(app.world().get::<InteractionDisabled>(field).is_none());
    assert!(app.world().get::<InteractionDisabled>(list).is_none());
    app.world_mut().trigger(primary_press(field));
    app.world_mut().flush();
    app.world_mut().trigger(primary_click(field));
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    let disabled_row = row(&mut app, list, 1);
    assert!(
        app.world()
            .get::<InteractionDisabled>(disabled_row)
            .is_some()
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListModel<String>>(source)
            .unwrap()
            .is_disabled(1),
        Some(true)
    );
    let accepted = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(2)
        .unwrap();
    let target = row(&mut app, list, 2);
    app.world_mut().trigger(primary_click(target));
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(accepted)
    );
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(accepted))]
    );
}

#[test]
fn outside_click_and_another_field_preserve_target_focus() {
    let Fixture {
        mut app,
        source,
        field,
        popup,
        ..
    } = fixture(2);
    let other = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
        }
    }).unwrap().id();
    app.update();
    let children = app.world().get::<Children>(other).unwrap();
    let (other_field, other_popup) = (children[0], children[1]);
    let other_list = app.world().get::<Children>(other_popup).unwrap()[0];
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().trigger(primary_press(other_field));
    app.world_mut().flush();
    app.world_mut().trigger(primary_click(other_field));
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        *app.world().get::<Visibility>(other_popup).unwrap(),
        Visibility::Visible
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(other_list));
    let outside = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(outside, FocusCause::Pressed);
    app.world_mut().trigger(primary_click(outside));
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Visibility>(other_popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(outside));
}

#[test]
fn only_owned_enabled_primary_row_clicks_close_popup() {
    let Fixture {
        mut app,
        source,
        field,
        popup,
        list,
        ..
    } = fixture(3);
    let outer_row = row(&mut app, list, 0);
    let nested = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source, @item_height: 24.0,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list![(Text({value.clone()}))])},
        }
    }).unwrap().id();
    app.world_mut().entity_mut(outer_row).add_child(nested);
    let nested_viewport = app
        .world()
        .get::<Children>(nested)
        .unwrap()
        .iter()
        .find(|&child| app.world().get::<ScrollArea>(child).is_some())
        .unwrap();
    app.world_mut()
        .entity_mut(nested_viewport)
        .insert(ComputedNode {
            size: Vec2::new(100.0, 24.0),
            content_size: Vec2::new(100.0, 72.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
    app.update();
    let nested_row = row(&mut app, nested, 0);
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().trigger(primary_click(nested_row));
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    app.world_mut().trigger(primary_click(list));
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    let primary = primary_click(outer_row);
    let secondary = Pointer::new(
        primary.pointer_id,
        primary.pointer_location.clone(),
        Click {
            button: PointerButton::Secondary,
            hit: primary.hit.clone(),
            duration: primary.duration,
            count: primary.count,
        },
        outer_row,
    );
    app.world_mut().trigger(secondary);
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn closed_popup_cannot_select_hidden_items_from_keyboard() {
    let Fixture {
        mut app,
        root,
        source,
        field,
        popup,
        list,
        window,
        ..
    } = fixture(3);
    app.world_mut().trigger(Activate { entity: field });
    press_key(&mut app, window, KeyCode::ArrowDown);
    press_key(&mut app, window, KeyCode::Enter);
    let selected = app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .id(1)
        .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(selected)
    );
    press_key(&mut app, window, KeyCode::End);
    press_key(&mut app, window, KeyCode::Enter);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(selected)
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(selected))]
    );
    app.world_mut().trigger(Activate { entity: field });
    let outside = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
    app.world_mut().trigger(primary_click(outside));
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    press_key(&mut app, window, KeyCode::Home);
    press_key(&mut app, window, KeyCode::Space);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(selected)
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(root, Some(selected))]
    );
}

#[test]
fn closing_popup_stops_remaining_keyboard_inputs_in_same_frame() {
    for confirm in [KeyCode::Enter, KeyCode::Space] {
        let Fixture {
            mut app,
            root,
            source,
            field,
            popup,
            list,
            window,
            ..
        } = fixture(5);
        app.world_mut().trigger(Activate { entity: field });
        let selected = app
            .world()
            .get::<WidgetryListModel<String>>(source)
            .unwrap()
            .id(1)
            .unwrap();
        // 逐帧派发会让 focus 清理掩盖隐藏列表接受后续输入的问题。
        // 整批 message 只推进一次 update，保留关闭与后续按键的同帧顺序。
        for key_code in [KeyCode::ArrowDown, confirm, KeyCode::End, confirm] {
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code,
                    logical_key: Key::Unidentified(NativeKey::Unidentified),
                    state: ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            );
        }
        app.update();
        let state = app.world().get::<WidgetryListViewState>(list).unwrap();
        assert_eq!(state.selected, Some(selected));
        assert_eq!(state.active, Some(selected));
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Hidden
        );
        assert_ne!(app.world().resource::<InputFocus>().get(), Some(list));
        assert_eq!(
            app.world().resource::<Changes>().0,
            vec![(root, Some(selected))]
        );
    }
}

#[test]
fn keyboard_reselection_closes_popup_without_notification() {
    for confirm in [KeyCode::Enter, KeyCode::Space] {
        let Fixture {
            mut app,
            field,
            popup,
            list,
            window,
            ..
        } = fixture(5);
        let initial = *app.world().get::<WidgetryListViewState>(list).unwrap();
        app.world_mut().trigger(Activate { entity: field });
        for key_code in [confirm, KeyCode::End, confirm] {
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code,
                    logical_key: Key::Unidentified(NativeKey::Unidentified),
                    state: ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            );
        }
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<WidgetryListViewState>(list).unwrap(),
            initial
        );
        assert_ne!(app.world().resource::<InputFocus>().get(), Some(list));
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn local_disabled_row_click_preserves_popup_and_selection_until_recovery() {
    for index in [0, 1] {
        let Fixture {
            mut app,
            root,
            field,
            popup,
            list,
            source,
            ..
        } = fixture(3);
        let target = row(&mut app, list, index);
        let initial = *app.world().get::<WidgetryListViewState>(list).unwrap();
        app.world_mut().trigger(Activate { entity: field });
        app.world_mut()
            .entity_mut(target)
            .insert(InteractionDisabled);
        app.world_mut().flush();
        let text = app.world().get::<Children>(target).unwrap()[0];
        app.world_mut().trigger(primary_click(text));
        app.world_mut().flush();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            app.world()
                .get::<WidgetryListViewState>(list)
                .unwrap()
                .selected,
            initial.selected
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
        assert!(app.world().resource::<Changes>().0.is_empty());
        app.world_mut()
            .entity_mut(target)
            .remove::<InteractionDisabled>();
        app.world_mut().flush();
        app.world_mut().trigger(primary_click(text));
        app.world_mut().flush();
        app.update();
        let selected = app
            .world()
            .get::<WidgetryListModel<String>>(source)
            .unwrap()
            .id(index);
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            app.world()
                .get::<WidgetryListViewState>(list)
                .unwrap()
                .selected,
            selected
        );
        let expected = if initial.selected == selected {
            vec![]
        } else {
            vec![(root, selected)]
        };
        assert_eq!(app.world().resource::<Changes>().0, expected);
    }
}

#[test]
fn local_disabled_row_or_container_blocks_keyboard_reselection_until_recovery() {
    for confirm in [KeyCode::Enter, KeyCode::Space] {
        for scope in ["row", "viewport", "content"] {
            let Fixture {
                mut app,
                field,
                popup,
                list,
                viewport,
                window,
                ..
            } = fixture(3);
            let target = match scope {
                "viewport" => viewport,
                "content" => app.world().get::<Children>(viewport).unwrap()[0],
                _ => row(&mut app, list, 0),
            };
            let initial = *app.world().get::<WidgetryListViewState>(list).unwrap();
            app.world_mut().trigger(Activate { entity: field });
            app.world_mut()
                .entity_mut(target)
                .insert(InteractionDisabled);
            app.world_mut().flush();
            press_key(&mut app, window, confirm);
            assert_eq!(
                *app.world().get::<Visibility>(popup).unwrap(),
                Visibility::Visible
            );
            assert_eq!(
                *app.world().get::<WidgetryListViewState>(list).unwrap(),
                initial
            );
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
            assert!(app.world().resource::<Changes>().0.is_empty());
            app.world_mut()
                .entity_mut(target)
                .remove::<InteractionDisabled>();
            app.world_mut().flush();
            press_key(&mut app, window, confirm);
            assert_eq!(
                *app.world().get::<Visibility>(popup).unwrap(),
                Visibility::Hidden
            );
            assert_eq!(
                *app.world().get::<WidgetryListViewState>(list).unwrap(),
                initial
            );
            assert_ne!(app.world().resource::<InputFocus>().get(), Some(list));
            assert!(app.world().resource::<Changes>().0.is_empty());
        }
    }
}

#[test]
fn pointer_close_stops_keyboard_selection_in_same_frame() {
    for outside in [false, true] {
        let Fixture {
            mut app,
            field,
            popup,
            list,
            window,
            ..
        } = fixture(5);
        let initial = *app.world().get::<WidgetryListViewState>(list).unwrap();
        let target = if outside {
            app.world_mut().spawn_scene(bsn! { Node }).unwrap().id()
        } else {
            row(&mut app, list, 0)
        };
        app.world_mut().trigger(Activate { entity: field });
        // 普通 update 边界可能掩盖 picking 后的 focus 残留。
        // 在官方 picking 阶段执行 click，再让同帧 keyboard dispatch 消费剩余输入。
        app.configure_sets(
            PreUpdate,
            (PickingSystems::ProcessInput, PickingSystems::Last).chain(),
        )
        .add_systems(
            PreUpdate,
            (move |world: &mut World| {
                world.trigger(primary_click(target));
                world.flush();
            })
            .in_set(PickingSystems::Last)
            .before(InputFocusSystems::Dispatch),
        );
        for key_code in [KeyCode::End, KeyCode::Enter] {
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code,
                    logical_key: Key::Unidentified(NativeKey::Unidentified),
                    state: ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            );
        }
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<WidgetryListViewState>(list).unwrap(),
            initial
        );
        assert_ne!(app.world().resource::<InputFocus>().get(), Some(list));
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}
