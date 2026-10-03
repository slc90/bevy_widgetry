use crate::{WidgetryComboBoxPlugin, combo_box, field, popup};
use bevy::input_focus::InputFocusSystems;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_list_view::{WidgetryListViewAppExt, WidgetryListViewSystems};
use bevy_widgetry_log::widgetry_info;
use std::marker::PhantomData;

struct TypedComboBoxPlugin<T>(PhantomData<fn() -> T>);

pub trait WidgetryComboBoxAppExt {
    fn register_widgetry_combo_box<T: Send + Sync + 'static>(
        &mut self,
    ) -> Result<&mut Self, BevyError>;
}

impl WidgetryComboBoxAppExt for App {
    fn register_widgetry_combo_box<T: Send + Sync + 'static>(
        &mut self,
    ) -> Result<&mut Self, BevyError> {
        if !self.is_plugin_added::<WidgetryComboBoxPlugin>() {
            self.add_plugins(WidgetryComboBoxPlugin);
        }
        self.register_widgetry_list_view::<T>()?;
        if !self.is_plugin_added::<TypedComboBoxPlugin<T>>() {
            self.add_plugins(TypedComboBoxPlugin::<T>(PhantomData));
        }
        Ok(self)
    }
}

impl<T: Send + Sync + 'static> Plugin for TypedComboBoxPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_observer(combo_box::handle_value_change::<T>)
            .add_observer(popup::handle_field_activate::<T>)
            .add_observer(popup::handle_row_click::<T>)
            .add_observer(popup::handle_reselection::<T>)
            .add_observer(popup::handle_escape::<T>)
            .add_observer(field::on_disabled_added::<T>)
            .add_observer(field::on_disabled_removed::<T>)
            .add_systems(
                PreUpdate,
                (
                    field::mirror_disabled_added::<T>,
                    field::mirror_disabled_removed::<T>,
                    field::initialize_disabled::<T>,
                    field::mirror_list_disabled::<T>,
                )
                    .chain()
                    .before(PickingSystems::ProcessInput)
                    .before(InputFocusSystems::Dispatch),
            )
            .add_systems(
                PreUpdate,
                popup::clear_hidden_focus::<T>
                    .after(PickingSystems::Last)
                    .before(InputFocusSystems::Dispatch),
            )
            .add_systems(
                PostUpdate,
                (
                    popup::sync_geometry::<T>,
                    field::sync_icon::<T>,
                    popup::clear_hidden_focus::<T>,
                )
                    .chain()
                    .before(WidgetryListViewSystems::SyncState),
            )
            .add_systems(
                PostUpdate,
                combo_box::initialize_selection::<T>.before(WidgetryListViewSystems::SyncState),
            )
            .add_systems(
                PostUpdate,
                field::project::<T>
                    .after(WidgetryListViewSystems::SyncState)
                    .in_set(WidgetryUiSystems::Build),
            );
        widgetry_info!(
            item_type = std::any::type_name::<T>(),
            "TypedComboBoxPlugin 注册完成"
        );
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_list_view::WidgetryListViewPlugin;
    use bevy_widgetry_test_utils::{LogCapture, scene_app};

    #[test]
    fn typed_registration_is_idempotent() {
        let capture = LogCapture::default();
        let mut app = scene_app();
        capture.run(|| {
            app.register_widgetry_combo_box::<String>()
                .unwrap()
                .register_widgetry_combo_box::<String>()
                .unwrap()
                .register_widgetry_combo_box::<u32>()
                .unwrap();
        });
        assert!(app.is_plugin_added::<WidgetryComboBoxPlugin>());
        assert!(app.is_plugin_added::<WidgetryListViewPlugin>());
        assert!(app.is_plugin_added::<TypedComboBoxPlugin<String>>());
        assert!(app.is_plugin_added::<TypedComboBoxPlugin<u32>>());
        for (message, expected) in [
            ("WidgetryComboBoxPlugin 注册完成", 1),
            ("TypedComboBoxPlugin 注册完成", 2),
            ("TypedListViewPlugin 注册完成", 2),
        ] {
            assert_eq!(
                capture
                    .records()
                    .iter()
                    .filter(|record| record
                        .fields
                        .get("message")
                        .is_some_and(|value| value == message))
                    .count(),
                expected
            );
        }
    }
}
