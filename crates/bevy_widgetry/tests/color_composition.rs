//! State：父 Button、独立 CheckBox、普通内容、原生 Text 的颜色与本地/继承 Disabled。
//! Stimuli：BSN 构造、Dark→Light→Dark、父级禁用/恢复、子本地禁用与覆盖清除。
//! Invariants：Disabled 穿过独立 Widget，foreground 在独立 Widget 重建作用域。
//! Couplings：各自 state 的显式覆盖优先，最终 TextColor 同帧正确，原生 Text 保持显式色。

// 测试需要在最终输出违反 contract 时立即失败。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::button::{WidgetryButton, WidgetryButtonColorOverrides, WidgetryButtonPlugin};
use bevy_widgetry::check_box::{
    WidgetryCheckBox, WidgetryCheckBoxColorOverrides, WidgetryCheckBoxPlugin,
};
use bevy_widgetry::text::WidgetryText;
use bevy_widgetry::theme::WidgetryThemeMode;

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
            @WidgetryButton { @colors: parent }
            Children [
                Node Children [Node Children [
                    Name("parent-label") Text("parent") WidgetryText--
                    Name("native-label") Text("native") TextColor(native_color)--
                    @WidgetryCheckBox { @colors: child } Name("child")
                        Children [Name("child-label") Text("child") WidgetryText]
                ]]
            ]
        })
        .unwrap()
        .id();
    let child = named(app.world_mut(), "child");
    let parent_label = named(app.world_mut(), "parent-label");
    let child_label = named(app.world_mut(), "child-label");
    let native_label = named(app.world_mut(), "native-label");

    for mode in [
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::Light,
        WidgetryThemeMode::Dark,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
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
            mode.colors().button.normal.foreground
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
            mode.colors().check_box.unchecked.normal.foreground
        );
    }
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    WidgetryButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    WidgetryCheckBoxColorOverrides::clear_in_world(app.world_mut(), child).unwrap();
    app.update();
    let theme = WidgetryThemeMode::Dark.colors();
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
