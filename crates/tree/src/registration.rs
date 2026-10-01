use crate::behavior::sync_models;
use bevy::prelude::*;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_list_view::WidgetryListViewSystems;
use bevy_widgetry_log::widgetry_info;

/// 装配 headless Tree projection；应用负责维护 hierarchy 与处理 lazy children request。
pub struct WidgetryTreePlugin;

impl Plugin for WidgetryTreePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        app.add_systems(
            PostUpdate,
            sync_models
                .in_set(WidgetryUiSystems::Build)
                .before(WidgetryListViewSystems::SyncState),
        );
        widgetry_info!("WidgetryTreePlugin 注册完成");
    }
}
