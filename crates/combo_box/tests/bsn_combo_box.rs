#![cfg(test)]

use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Selected};
use bevy::ui_widgets::{Activate, Button, ListBox, ListItem, ValueChange};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_combo_box::{
    WidgetryComboBox, WidgetryComboBoxOptionFactory, WidgetryComboBoxPlugin,
};
use bevy_widgetry_core::icon::Icon;
use bevy_widgetry_core::{DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeMode};
use bevy_widgetry_test_utils::{primary_click, primary_press, scene_app, switch_theme};
use std::time::{Duration, Instant};

/// 用嵌套内容标记验证任意 SceneList 在 Field 与列表中各有独立副本。
#[derive(Component, Clone)]
struct Content(usize);

/// 记录公开的用户值通知，以区分程序化选择与真实交互。
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize)>);

/// 收集用户通知的来源与索引。
fn record(event: On<ValueChange<usize>>, mut changes: ResMut<Changes>) {
    changes.0.push((event.source, event.value));
}

/// 完成首帧同步，后续测试只观察新操作造成的变化。
fn app_with_combo() -> (App, Entity, Entity, Entity) {
    let mut app = scene_app();
    // PointerTraversal 的可选 Window 查询也需要组件已注册，但无需创建桌面窗口。
    app.world_mut().register_component::<Window>();
    app.add_plugins(WidgetryComboBoxPlugin)
        .init_resource::<Changes>()
        .add_observer(record);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox { @options: {options()} }
        })
        .unwrap()
        .id();
    app.update();
    let field = child::<Button>(app.world(), root);
    let popup = child::<ListBox>(app.world(), root);
    (app, root, field, popup)
}

/// 返回 Field 的稳定内容容器及其派生值。
fn field_content(world: &World, field: Entity) -> (Entity, Vec<Entity>, usize) {
    let content = world.get::<Children>(field).unwrap()[0];
    let children = world.get::<Children>(content).unwrap().to_vec();
    let nested = world.get::<Children>(children[0]).unwrap()[0];
    (content, children, world.get::<Content>(nested).unwrap().0)
}

/// 验证当前唯一选择，不依赖私有 option 标记。
fn assert_selected(world: &World, popup: Entity, index: usize) {
    for (i, row) in world.get::<Children>(popup).unwrap().iter().enumerate() {
        assert_eq!(world.get::<Selected>(row).is_some(), i == index);
    }
}

/// 创建可重复展开的多实体内容，避免测试依赖固定文本布局。
fn options() -> Vec<WidgetryComboBoxOptionFactory> {
    (0..3)
        .map(|index| {
            WidgetryComboBoxOptionFactory::new(move || {
                bsn_list![
                    (Node Children [(template(move |_| Ok(Content(index))))]),
                    (Text({format!("Option {index}")})),
                ]
            })
        })
        .collect()
}

/// 从公开行为组件定位组合控件的直接子节点。
fn child<T: Component>(world: &World, root: Entity) -> Entity {
    world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|&entity| world.get::<T>(entity).is_some())
        .unwrap()
}

// BSN 必须一次展开完整层级，默认唯一选择首项，并为 Field 创建独立的任意内容副本。
#[test]
fn scene_builds_complete_hierarchy_and_arbitrary_content() {
    let mut app = scene_app();
    app.add_plugins(WidgetryComboBoxPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox { @options: {options()} }
        })
        .unwrap()
        .id();
    let world = app.world_mut();
    assert!(world.get::<WidgetryComboBox>(root).is_some());
    let field = child::<Button>(world, root);
    let popup = child::<ListBox>(world, root);
    assert_eq!(*world.get::<Visibility>(popup).unwrap(), Visibility::Hidden);
    let rows = world.get::<Children>(popup).unwrap();
    assert_eq!(rows.len(), 3);
    for (index, row) in rows.iter().enumerate() {
        assert!(world.get::<ListItem>(row).is_some());
        assert_eq!(world.get::<Selected>(row).is_some(), index == 0);
        assert_eq!(world.get::<Children>(row).unwrap().len(), 2);
    }
    let mut parents = world.query::<&ChildOf>();
    let mut contents = world.query::<(Entity, &Content)>();
    let field_values: Vec<_> = contents
        .iter(world)
        .filter(|(entity, _)| {
            parents
                .query(world)
                .iter_ancestors(*entity)
                .any(|ancestor| ancestor == field)
        })
        .map(|(_, content)| content.0)
        .collect();
    assert_eq!(field_values, vec![0]);
    app.update();
}

// 实际点击嵌套选项内容后，唯一选择与 Field 同步、旧副本递归销毁并发一次root值通知。
#[test]
fn user_selection_rebuilds_field_and_closes_popup() {
    let (mut app, root, field, popup) = app_with_combo();
    let (container, old_children, _) = field_content(app.world(), field);
    let old_nested = app.world().get::<Children>(old_children[0]).unwrap()[0];
    let row = app.world().get::<Children>(popup).unwrap()[2];
    let content = app.world().get::<Children>(row).unwrap()[0];
    let nested = app.world().get::<Children>(content).unwrap()[0];
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().trigger(primary_click(nested));
    app.update();
    assert_selected(app.world(), popup, 2);
    let (new_container, _, value) = field_content(app.world(), field);
    assert_eq!(container, new_container);
    assert_eq!(value, 2);
    for old in old_children.into_iter().chain([old_nested]) {
        assert!(app.world().get_entity(old).is_err());
    }
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![(root, 2)]);
}

// 重点回归真实 ListBox 重选路径：关闭Popup但保留同一内容实体，也不产生值通知。
#[test]
fn reselect_closes_without_rebuilding_or_notifying() {
    let (mut app, _, field, popup) = app_with_combo();
    let before = field_content(app.world(), field);
    let row = app.world().get::<Children>(popup).unwrap()[0];
    let nested = app.world().get::<Children>(row).unwrap()[1];
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().trigger(primary_click(nested));
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(field_content(app.world(), field), before);
    assert_selected(app.world(), popup, 0);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

// 程序化选择允许禁用root，保持 Popup 状态；同值、越界和无效root均不重建或通知。
#[test]
fn programmatic_selection_is_silent_and_invalid_requests_are_noops() {
    let (mut app, root, field, popup) = app_with_combo();
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    WidgetryComboBox::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    assert_selected(app.world(), popup, 1);
    assert_eq!(field_content(app.world(), field).2, 1);
    let before = field_content(app.world(), field);
    let unrelated = app.world_mut().spawn_empty().id();
    for (entity, selected) in [
        (root, 1),
        (root, usize::MAX),
        (unrelated, 0),
        (Entity::PLACEHOLDER, 0),
    ] {
        WidgetryComboBox::set_selected(&mut app.world_mut().commands(), entity, selected);
    }
    app.world_mut().flush();
    app.update();
    assert_eq!(field_content(app.world(), field), before);
    assert_selected(app.world(), popup, 1);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

// root禁用添加与移除在同一帧镜像到按钮，禁止用户改值但不向选项任意内容递归传播。
#[test]
fn disabled_root_controls_interaction_and_button_mirror() {
    let (mut app, root, field, popup) = app_with_combo();
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    let row = app.world().get::<Children>(popup).unwrap()[1];
    // 镜像尚未同步时，ComboBox 行为也必须以root为准。
    app.world_mut().trigger(ValueChange {
        source: popup,
        value: row,
        is_final: true,
    });
    app.update();
    assert!(app.world().get::<WidgetryButton>(field).is_some());
    assert!(app.world().get::<InteractionDisabled>(field).is_some());
    assert!(app.world().get::<InteractionDisabled>(row).is_none());
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_disabled
    );
    app.world_mut().trigger(Activate { entity: field });
    app.world_mut().trigger(primary_click(row));
    app.update();
    assert_selected(app.world(), popup, 0);
    assert!(app.world().resource::<Changes>().0.is_empty());
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert!(app.world().get::<InteractionDisabled>(field).is_none());
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background
    );
    app.world_mut().trigger(Activate { entity: field });
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
}

// 首次以禁用root构造时，Field 在第一帧即获得禁用状态，内容副本仍正常初始化。
#[test]
fn initially_disabled_scene_initializes_field() {
    let mut app = scene_app();
    app.add_plugins(WidgetryComboBoxPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox { @options: {options()} } InteractionDisabled
        })
        .unwrap()
        .id();
    app.update();
    let field = child::<WidgetryButton>(app.world(), root);
    assert!(app.world().get::<InteractionDisabled>(field).is_some());
    assert_eq!(field_content(app.world(), field).2, 0);
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_disabled
    );
}

// Popup 与 Option 保持原有尺寸定位，增加圆角；Field 使用 ComboBox 自身的几何覆盖。
#[test]
fn scene_preserves_geometry_and_adds_rounded_rows() {
    let (app, root, field, popup) = app_with_combo();
    let world = app.world();
    assert_eq!(world.get::<Node>(root).unwrap().width, px(200));
    let node = world.get::<Node>(field).unwrap();
    assert_eq!(node.height, px(36));
    assert_eq!(node.padding, UiRect::axes(px(10), px(0)));
    let node = world.get::<Node>(popup).unwrap();
    assert_eq!(node.position_type, PositionType::Absolute);
    assert_eq!(node.top, percent(100));
    assert_eq!(node.width, percent(100));
    assert_eq!(node.border_radius, BorderRadius::all(px(4)));
    assert_eq!(world.get::<GlobalZIndex>(popup).unwrap().0, 100);
    for row in world.get::<Children>(popup).unwrap().iter() {
        let node = world.get::<Node>(row).unwrap();
        assert_eq!(node.height, px(32));
        assert_eq!(node.border_radius, BorderRadius::all(px(4)));
        assert_eq!(node.padding, UiRect::axes(px(10), px(0)));
    }
}

// 悬停覆盖选中，root禁用覆盖二者；恢复状态与主题切换均更新背景和传播前景色。
#[test]
fn option_styles_follow_selection_hover_disabled_and_theme() {
    let (mut app, root, _, popup) = app_with_combo();
    let rows = app.world().get::<Children>(popup).unwrap().to_vec();
    assert_eq!(
        app.world().get::<BackgroundColor>(rows[0]).unwrap().0,
        DARK_THEME.item_background_selected
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(rows[1]).unwrap().0,
        DARK_THEME.popup_background
    );
    app.world_mut().entity_mut(rows[0]).insert(Hovered(true));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(rows[0]).unwrap().0,
        DARK_THEME.item_background_hovered
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    for &row in &rows {
        assert_eq!(
            app.world().get::<BackgroundColor>(row).unwrap().0,
            DARK_THEME.control_background_disabled
        );
        assert_eq!(
            app.world()
                .get::<Propagate<ForegroundColor>>(row)
                .unwrap()
                .0
                .0,
            DARK_THEME.foreground_disabled
        );
    }
    switch_theme(&mut app, ThemeMode::Light);
    for &row in &rows {
        assert_eq!(
            app.world().get::<BackgroundColor>(row).unwrap().0,
            LIGHT_THEME.control_background_disabled
        );
    }
    assert_eq!(
        app.world().get::<BackgroundColor>(popup).unwrap().0,
        LIGHT_THEME.popup_background
    );
    assert_eq!(
        *app.world().get::<BorderColor>(popup).unwrap(),
        BorderColor::all(LIGHT_THEME.popup_border)
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    WidgetryComboBox::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(rows[0]).unwrap().0,
        LIGHT_THEME.item_background_hovered
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(rows[1]).unwrap().0,
        LIGHT_THEME.item_background_selected
    );
    app.world_mut().entity_mut(rows[0]).insert(Hovered(false));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(rows[0]).unwrap().0,
        LIGHT_THEME.popup_background
    );
    let text = app.world().get::<Children>(rows[1]).unwrap()[1];
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        LIGHT_THEME.foreground
    );
}

// Popup 展开不再触发 active 配色，Field 完全使用按钮的悬停、按压与主题配色。
#[test]
fn field_uses_button_style_even_while_open() {
    let (mut app, _, field, _) = app_with_combo();
    app.world_mut().trigger(Activate { entity: field });
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background
    );
    app.world_mut().entity_mut(field).insert(Hovered(true));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_hovered
    );
    app.world_mut().trigger(primary_press(field));
    app.world_mut().flush();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_pressed
    );
    switch_theme(&mut app, ThemeMode::Light);
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        LIGHT_THEME.control_background_pressed
    );
}

// 两个控件使用真实 Button 点击路径时，一次点击关闭旧Popup并打开新Popup；外部点击再关闭。
#[test]
fn clicking_another_combo_closes_previous_popup() {
    let (mut app, _, field, popup) = app_with_combo();
    let other = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryComboBox { @options: {options()} } })
        .unwrap()
        .id();
    app.update();
    let other_field = child::<Button>(app.world(), other);
    let other_popup = child::<ListBox>(app.world(), other);
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
    let outside = app.world_mut().spawn_empty().id();
    app.world_mut().trigger(primary_click(outside));
    assert_eq!(
        *app.world().get::<Visibility>(other_popup).unwrap(),
        Visibility::Hidden
    );
}

// 列表通知不能引用其他 ComboBox 的选项；伪造同值通知也不能冒充真实改值。
#[test]
fn foreign_options_and_duplicate_values_are_ignored() {
    let (mut app, _, field, popup) = app_with_combo();
    let other = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryComboBox { @options: {options()} } })
        .unwrap()
        .id();
    app.update();
    let other_popup = child::<ListBox>(app.world(), other);
    let foreign = app.world().get::<Children>(other_popup).unwrap()[1];
    let current = app.world().get::<Children>(popup).unwrap()[0];
    let before = field_content(app.world(), field);
    for target in [foreign, current] {
        app.world_mut().trigger(ValueChange {
            source: popup,
            value: target,
            is_final: true,
        });
        app.world_mut().flush();
    }
    app.update();
    assert_selected(app.world(), popup, 0);
    assert_eq!(field_content(app.world(), field), before);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

// 默认 props 可创建，但实际展开空选项必须遵守非空前置条件。
#[test]
#[should_panic(expected = "WidgetryComboBox requires at least one option")]
fn empty_options_are_rejected_at_scene_construction() {
    let _ = bsn! { @WidgetryComboBox };
}

/// 在有截止时间的真实资产更新中等待图标生成，避免依赖固定帧数或扩大图标公共 API。
fn wait_for_image(app: &mut App, icon: Entity, expected: Option<&Handle<Image>>) -> Handle<Image> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.update();
        let image = app.world().get::<Children>(icon).and_then(|children| {
            children.iter().find_map(|child| {
                app.world()
                    .get::<ImageNode>(child)
                    .map(|node| node.image.clone())
            })
        });
        if let Some(image) = image
            && expected.is_none_or(|expected| *expected == image)
        {
            return image;
        }
        assert!(
            Instant::now() < deadline,
            "SVG 图标未在截止时间内完成替换: icon={icon:?}, expected={expected:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

// 实际加载两种内建 SVG，验证 Popup 显隐切换使用正确图像且图标实体与尺寸稳定。
#[test]
fn dropdown_icon_follows_popup_visibility() {
    let (mut app, _, field, popup) = app_with_combo();
    app.finish();
    app.cleanup();
    let icon = child::<Icon>(app.world(), field);
    let down = wait_for_image(&mut app, icon, None);
    let pixels = app
        .world()
        .resource::<Assets<Image>>()
        .get(&down)
        .unwrap()
        .data
        .as_ref()
        .unwrap();
    assert!(pixels.as_chunks::<4>().0.iter().any(|rgba| rgba[3] > 0));
    // Icon 用乘色实现前景色继承，SVG 必须栅格化为白色预乘透明度遮罩。
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|rgba| rgba[0] == rgba[3] && rgba[1] == rgba[3] && rgba[2] == rgba[3])
    );
    // 两个参考 Icon 保持 SVG 强句柄，避免切换期间卸载后重新加载造成缓存标识变化。
    let down_reference = app.world_mut().spawn_scene(bsn! {
        @Icon { @path: {BuiltinIcon::ChevronDown.path()}, @max_size: {Some(UVec2::new(16, 16))} }
    }).unwrap().id();
    assert_eq!(wait_for_image(&mut app, down_reference, None), down);
    let reference = app
        .world_mut()
        .spawn_scene(bsn! {
            @Icon { @path: {BuiltinIcon::ChevronUp.path()}, @max_size: {Some(UVec2::new(16, 16))} }
        })
        .unwrap()
        .id();
    let up = wait_for_image(&mut app, reference, None);
    assert_ne!(down, up);
    let original_node = app.world().get::<Node>(icon).unwrap().clone();
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;
    app.update();
    let image = child::<ImageNode>(app.world(), icon);
    assert_eq!(app.world().get::<ImageNode>(image).unwrap().image, up);
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Hidden;
    app.update();
    assert_eq!(app.world().get::<ImageNode>(image).unwrap().image, down);
    assert_eq!(child::<Icon>(app.world(), field), icon);
    assert_eq!(
        app.world().get::<Node>(icon).unwrap().width,
        original_node.width
    );
    assert_eq!(
        app.world().get::<Node>(icon).unwrap().height,
        original_node.height
    );
}

// 同帧多次排队选择只展示最终 Selected，程序化选择不会关闭已展开的Popup。
#[test]
fn queued_selections_keep_popup_open_and_display_final_value() {
    let (mut app, root, field, popup) = app_with_combo();
    app.world_mut().trigger(Activate { entity: field });
    for index in [1, 2, 0, 2] {
        WidgetryComboBox::set_selected(&mut app.world_mut().commands(), root, index);
    }
    app.world_mut().flush();
    app.update();
    assert_selected(app.world(), popup, 2);
    assert_eq!(field_content(app.world(), field).2, 2);
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

// 移除后同帧重加root禁用，RemovedComponents 不得覆盖最终权威状态。
#[test]
fn disabling_again_in_same_frame_preserves_mirror() {
    let (mut app, root, field, _) = app_with_combo();
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>()
        .insert(InteractionDisabled);
    app.update();
    assert!(app.world().get::<InteractionDisabled>(field).is_some());
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_disabled
    );
}

// 直接改变真实 Selected 也会更新 Field；不依赖 ValueChange 或 set_selected 调用路径。
#[test]
fn field_is_derived_from_selected_components() {
    let (mut app, _, field, popup) = app_with_combo();
    let rows = app.world().get::<Children>(popup).unwrap().to_vec();
    app.world_mut().entity_mut(rows[0]).remove::<Selected>();
    app.world_mut().entity_mut(rows[2]).insert(Selected);
    app.update();
    assert_eq!(field_content(app.world(), field).2, 2);
    assert!(app.world().resource::<Changes>().0.is_empty());
}

// 使用已有 Light 主题创建新控件时，不需要额外 ThemeChanged 通知即可获得正确初始颜色。
#[test]
fn scene_uses_current_theme_on_first_update() {
    let mut app = scene_app();
    app.insert_resource(ThemeMode::Light)
        .add_plugins(WidgetryComboBoxPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox { @options: {options()} }
        })
        .unwrap()
        .id();
    app.update();
    let field = child::<Button>(app.world(), root);
    let popup = child::<ListBox>(app.world(), root);
    let row = app.world().get::<Children>(popup).unwrap()[0];
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        LIGHT_THEME.control_background
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(row).unwrap().0,
        LIGHT_THEME.item_background_selected
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(popup).unwrap().0,
        LIGHT_THEME.popup_background
    );
}

// 已在 Popup 加载的选项图标被选入 Field 后，应在同一帧生成图像，不依赖下一次鼠标事件。
#[test]
fn selected_field_icon_materializes_in_one_update() {
    let mut app = scene_app();
    app.add_plugins(WidgetryComboBoxPlugin);
    let options = [BuiltinIcon::WindowClose, BuiltinIcon::WindowRestore]
        .into_iter()
        .map(|icon| {
            WidgetryComboBoxOptionFactory::new(move || bsn_list![(@Icon { @path: {icon.path()} })])
        })
        .collect::<Vec<_>>();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox { @options: {options} }
        })
        .unwrap()
        .id();
    app.update();
    let popup = child::<ListBox>(app.world(), root);
    let row = app.world().get::<Children>(popup).unwrap()[1];
    let reference = child::<Icon>(app.world(), row);
    let expected = wait_for_image(&mut app, reference, None);
    WidgetryComboBox::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    let field = child::<Button>(app.world(), root);
    let content = app.world().get::<Children>(field).unwrap()[0];
    let icon = child::<Icon>(app.world(), content);
    let image = child::<ImageNode>(app.world(), icon);
    assert_eq!(app.world().get::<ImageNode>(image).unwrap().image, expected);
}
