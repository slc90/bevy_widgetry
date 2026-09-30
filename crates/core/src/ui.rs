//! Workspace 内部的动态 UI 构造阶段，不由 facade 导出。

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy_widgetry_log::widgetry_info;

/// 跨 Widget 共享的 PostUpdate 构造阶段；deferred Commands 在后续阶段前应用。
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetryUiSystems {
    /// 同步 model 和重建 renderer subtree，供 UI Prepare 初始化 camera 信息。
    Build,
    /// 等待 UI Prepare 完成后生成已有 tree 的 asset 内容，供当帧 propagation 和 layout 使用；新 UI root 应放入 Build。
    Materialize,
}

/// 由使用构造阶段的库 plugin 自动安装，不要求应用手动装配。
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
        // Visibility 与 Stack 不属于 Prepare → Propagate chain，必须单独约束所有构造阶段。
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
