use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy_widgetry_log::widgetry_info;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetryUiSystems {
    Build,
    Materialize,
    Disabled,
    StyleOwners,
    Colors,
    Foreground,
    ContentColors,
}

pub struct WidgetryUiPlugin;

impl Plugin for WidgetryUiPlugin {
    fn build(&self, app: &mut App) {
        crate::disabled::install(app);
        crate::foreground::install(app);
        crate::text::install(app);
        if !app.is_plugin_added::<bevy_widgetry_theme::WidgetryThemePlugin>() {
            app.add_plugins(bevy_widgetry_theme::WidgetryThemePlugin);
        }
        if !app.is_plugin_added::<crate::pointer::WidgetryPointerPlugin>() {
            app.add_plugins(crate::pointer::WidgetryPointerPlugin);
        }
        app.configure_sets(
            PostUpdate,
            (
                WidgetryUiSystems::Build,
                UiSystems::Prepare,
                WidgetryUiSystems::Materialize,
                WidgetryUiSystems::Disabled,
                WidgetryUiSystems::StyleOwners,
                WidgetryUiSystems::Colors,
                UiSystems::Propagate,
                WidgetryUiSystems::ContentColors,
            )
                .chain(),
        );
        app.configure_sets(
            PostUpdate,
            WidgetryUiSystems::Foreground.in_set(UiSystems::Propagate),
        );
        app.configure_sets(
            PostUpdate,
            WidgetryUiSystems::ContentColors
                .before(bevy::text::detect_text_needs_rerender)
                .before(UiSystems::Content),
        );
        // Visibility 与 Stack 不属于 Prepare → Propagate chain。
        // 额外约束构造阶段，避免新 subtree 错过当帧 visibility/stack 消费。
        app.configure_sets(
            PostUpdate,
            WidgetryUiSystems::Materialize
                .before(VisibilitySystems::VisibilityPropagate)
                .before(UiSystems::Stack)
                .before(bevy::text::detect_text_needs_rerender),
        );
        widgetry_info!("WidgetryUiPlugin 注册完成");
    }
}
