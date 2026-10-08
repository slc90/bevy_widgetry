use crate::behavior::{
    install_pointer_cleanup, on_cancel, on_click, on_disabled_added, on_disabled_removed,
    on_drag_end, on_key, on_press, on_release, on_scroll, project, sync_state,
};
use crate::style::{refresh_theme, update};
use crate::view::validate_sources;
use crate::virtualization::reconcile;
use bevy::picking::pointer::PointerInput;
use bevy::prelude::*;
use bevy_widgetry_core::ForegroundColorPlugin;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin;
use bevy_widgetry_theme::WidgetryThemePlugin;
use std::marker::PhantomData;

pub struct WidgetryListViewPlugin;

struct TypedListViewPlugin<T>(PhantomData<fn() -> T>);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetryListViewSystems {
    SyncState,
    Reconcile,
}

pub trait WidgetryListViewAppExt {
    fn register_widgetry_list_view<T: Send + Sync + 'static>(
        &mut self,
    ) -> Result<&mut Self, BevyError>;
}

impl Plugin for WidgetryListViewPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        app.add_message::<PointerInput>();
        if !app.is_plugin_added::<WidgetryScrollAreaPlugin>() {
            app.add_plugins(WidgetryScrollAreaPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }
        widgetry_info!("WidgetryListViewPlugin 注册完成");
    }
}

impl<T: Send + Sync + 'static> Plugin for TypedListViewPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, validate_sources::<T>);
        install_pointer_cleanup::<T>(app);
        // 新 row 或 renderer child 若晚于 propagation 创建，会缺少当帧 camera 信息或文本 measurement。
        // 在 UI Build 阶段完成构造。
        app.add_systems(
            PostUpdate,
            (
                sync_state::<T>.in_set(WidgetryListViewSystems::SyncState),
                reconcile::<T>.in_set(WidgetryListViewSystems::Reconcile),
                project::<T>,
                update::<T>,
            )
                .chain()
                .in_set(WidgetryUiSystems::Build),
        );
        app.add_observer(refresh_theme::<T>);
        app.add_observer(on_click::<T>);
        app.add_observer(on_key::<T>);
        app.add_observer(on_scroll::<T>);
        app.add_observer(on_press::<T>)
            .add_observer(on_release::<T>)
            .add_observer(on_cancel::<T>)
            .add_observer(on_drag_end::<T>)
            .add_observer(on_disabled_added::<T>)
            .add_observer(on_disabled_removed::<T>);
        widgetry_info!(
            item_type = std::any::type_name::<T>(),
            "TypedListViewPlugin 注册完成"
        );
    }
}

impl WidgetryListViewAppExt for App {
    fn register_widgetry_list_view<T: Send + Sync + 'static>(
        &mut self,
    ) -> Result<&mut Self, BevyError> {
        if !self.is_plugin_added::<WidgetryListViewPlugin>() {
            widgetry_error!(
                item_type = std::any::type_name::<T>(),
                "注册 ListView item type 前必须安装 WidgetryListViewPlugin"
            );
            return Err(BevyError::error(
                "register_widgetry_list_view requires WidgetryListViewPlugin",
            ));
        }
        if !self.is_plugin_added::<TypedListViewPlugin<T>>() {
            self.add_plugins(TypedListViewPlugin::<T>(PhantomData));
        }
        Ok(self)
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{LogCapture, scene_app};

    #[test]
    fn typed_registration_is_deduplicated() {
        let capture = LogCapture::default();
        let mut app = scene_app();
        capture.run(|| {
            app.add_plugins(WidgetryListViewPlugin)
                .register_widgetry_list_view::<String>()
                .unwrap()
                .register_widgetry_list_view::<String>()
                .unwrap()
                .register_widgetry_list_view::<u32>()
                .unwrap();
        });
        assert!(app.is_plugin_added::<TypedListViewPlugin<String>>());
        assert!(app.is_plugin_added::<TypedListViewPlugin<u32>>());
        assert!(app.is_plugin_added::<WidgetryScrollAreaPlugin>());
        let registrations = capture
            .records()
            .into_iter()
            .filter(|record| {
                record
                    .fields
                    .get("message")
                    .is_some_and(|message| message.contains("TypedListViewPlugin 注册完成"))
            })
            .count();
        assert_eq!(registrations, 2);
    }
}
