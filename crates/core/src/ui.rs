use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy_widgetry_log::widgetry_info;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetryUiSystems {
    Build,
    Materialize,
}

pub struct WidgetryUiPlugin;

impl Plugin for WidgetryUiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                WidgetryUiSystems::Build,
                UiSystems::Prepare,
                WidgetryUiSystems::Materialize,
                UiSystems::Propagate,
            )
                .chain(),
        );
        // Visibility 与 Stack 不属于 Prepare → Propagate chain；额外约束构造阶段，避免新 subtree 错过当帧 visibility/stack 消费。
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
