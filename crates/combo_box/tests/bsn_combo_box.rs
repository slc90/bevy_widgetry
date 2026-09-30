#![cfg(test)]

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, Button};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::{DARK_THEME, LIGHT_THEME, ThemeMode};
use bevy_widgetry_list_view::{WidgetryListModel, WidgetryListViewRenderer};
use bevy_widgetry_test_utils::{primary_click, primary_press, scene_app, switch_theme};
use std::time::{Duration, Instant};

/// 使用新 model contract 构造保留的 Button、disabled 与 icon regression 测试。
fn app_with_combo() -> (App, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.world_mut().register_component::<Window>();
    app.init_resource::<bevy::picking::hover::HoverMap>()
        .init_resource::<UiScale>();
    app.register_widgetry_combo_box::<u32>();
    let mut model = WidgetryListModel::default();
    model.push(0u32);
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(combo(source)).unwrap().id();
    app.update();
    let field = child::<Button>(app.world(), root);
    let popup = app.world().get::<Children>(root).unwrap()[1];
    (app, root, field, popup)
}

/// 仅提供本测试的 BSN 配置，不保留旧 option factory contract。
fn combo(source: Entity) -> impl Scene {
    bsn! { @WidgetryComboBox::<u32> { @source: source, @renderer: {WidgetryListViewRenderer::new(|_, _: &u32| bsn_list![])} } }
}

/// 按公开 component 找到直接 child。
fn child<T: Component>(world: &World, root: Entity) -> Entity {
    world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|&child| world.get::<T>(child).is_some())
        .unwrap()
}

// Popup 展开不再触发 active 配色，Field 完全使用 Button 的 hover、pressed 与 theme 配色。
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

// 两个 Widget 使用真实 Button click 路径时，一次 click 关闭旧 Popup 并打开新 Popup；外部 click 再关闭。
#[test]
fn clicking_another_combo_closes_previous_popup() {
    let (mut app, _, field, popup) = app_with_combo();
    let mut model = WidgetryListModel::default();
    model.push(0u32);
    let source = app.world_mut().spawn(model).id();
    let other = app.world_mut().spawn_scene(combo(source)).unwrap().id();
    app.update();
    let other_field = child::<Button>(app.world(), other);
    let other_popup = app.world().get::<Children>(other).unwrap()[1];
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

/// 在有截止时间的真实 asset 更新中等待 icon 生成，避免依赖固定帧数或扩大 icon 公共 API。
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

// 实际加载两种内建 SVG，验证 visibility 与 model 清空关闭的当帧 icon 同步，entity 与尺寸稳定。
#[test]
fn dropdown_icon_follows_popup_visibility() {
    let (mut app, root, field, popup) = app_with_combo();
    app.finish();
    app.cleanup();
    let icon = child::<WidgetryIcon>(app.world(), field);
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
    // WidgetryIcon 用乘色实现 foreground color 继承，SVG 必须 rasterize 为白色 premultiplied alpha mask。
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|rgba| rgba[0] == rgba[3] && rgba[1] == rgba[3] && rgba[2] == rgba[3])
    );
    // 两个参考 WidgetryIcon 保持 SVG strong handle，避免切换期间卸载后重新加载造成 cache 标识变化。
    let down_reference = app.world_mut().spawn_scene(bsn! {
        @WidgetryIcon { @path: {BuiltinIcon::ChevronDown.path()}, @max_size: {Some(UVec2::new(16, 16))} }
    }).unwrap().id();
    assert_eq!(wait_for_image(&mut app, down_reference, None), down);
    let reference = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryIcon { @path: {BuiltinIcon::ChevronUp.path()}, @max_size: {Some(UVec2::new(16, 16))} }
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
    assert_eq!(child::<WidgetryIcon>(app.world(), field), icon);
    assert_eq!(
        app.world().get::<Node>(icon).unwrap().width,
        original_node.width
    );
    assert_eq!(
        app.world().get::<Node>(icon).unwrap().height,
        original_node.height
    );
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;
    app.update();
    let source = app
        .world()
        .get::<WidgetryComboBox<u32>>(root)
        .unwrap()
        .source();
    app.world_mut()
        .get_mut::<WidgetryListModel<u32>>(source)
        .unwrap()
        .clear();
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(app.world().get::<ImageNode>(image).unwrap().image, down);
}

// 移除后同帧重加 root 的 disabled component，RemovedComponents 不得覆盖最终权威 state。
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
