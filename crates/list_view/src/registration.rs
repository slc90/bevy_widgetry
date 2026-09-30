use crate::behavior::{
    clear_ended_presses, on_cancel, on_click, on_disabled_added, on_disabled_removed, on_drag_end,
    on_key, on_press, on_release, on_scroll, project, sync_state,
};
use crate::style::{refresh_theme, update};
use crate::view::validate_sources;
use crate::virtualization::reconcile;
use bevy::picking::{PickingSystems, pointer::PointerInput};
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy_widgetry_core::{ForegroundColorPlugin, ThemePlugin};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin;
use std::marker::PhantomData;

/// 装配与业务 T 无关的 ListView 基础设施，自动补齐 ScrollArea、theme 与 foreground propagation。
/// 应用仍需通过 WidgetryListViewAppExt 注册每一种业务 item type。
/// 真实 pointer、keyboard 与 focus 派发由应用的官方 input/picking/InputFocus plugin 提供。
pub struct WidgetryListViewPlugin;

/// 按 T 使用 Bevy plugin identity 去重 typed runtime 注册。
struct TypedListViewPlugin<T>(PhantomData<fn() -> T>);

/// 为业务 plugin 提供 generic ListView runtime 注册入口。
pub trait WidgetryListViewAppExt {
    /// 同一 T 只注册一次；多个相同 type 的 view 共享这组 typed systems。
    /// 应先安装 WidgetryListViewPlugin；调用顺序错误会先记录 ERROR 再终止。
    fn register_widgetry_list_view<T: Send + Sync + 'static>(&mut self) -> &mut Self;
}

impl Plugin for WidgetryListViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PointerInput>();
        if !app.is_plugin_added::<WidgetryScrollAreaPlugin>() {
            app.add_plugins(WidgetryScrollAreaPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
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
        app.add_systems(
            PreUpdate,
            clear_ended_presses::<T>.after(PickingSystems::Last),
        );
        // 新 row 与 renderer children 必须参与当帧 camera propagation 和文本 measurement。
        app.add_systems(
            PostUpdate,
            (sync_state::<T>, reconcile::<T>, project::<T>, update::<T>)
                .chain()
                .before(UiSystems::Prepare)
                .before(UiSystems::Propagate)
                .before(bevy::text::detect_text_needs_rerender),
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
    fn register_widgetry_list_view<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        if !self.is_plugin_added::<WidgetryListViewPlugin>() {
            widgetry_error!(
                item_type = std::any::type_name::<T>(),
                "注册 ListView item type 前必须安装 WidgetryListViewPlugin"
            );
            panic!("register_widgetry_list_view requires WidgetryListViewPlugin");
        }
        if !self.is_plugin_added::<TypedListViewPlugin<T>>() {
            self.add_plugins(TypedListViewPlugin::<T>(PhantomData));
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{LogCapture, scene_app};

    /// 同 T 重复注册保持幂等，不同 T 各自拥有独立 typed plugin，且只输出一次对应注册事实。
    #[test]
    fn typed_registration_is_deduplicated() {
        let capture = LogCapture::default();
        let mut app = scene_app();
        capture.run(|| {
            app.add_plugins(WidgetryListViewPlugin)
                .register_widgetry_list_view::<String>()
                .register_widgetry_list_view::<String>()
                .register_widgetry_list_view::<u32>();
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
