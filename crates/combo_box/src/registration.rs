use crate::{WidgetryComboBoxPlugin, combo_box, field, popup};
use bevy::input_focus::InputFocusSystems;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy_widgetry_list_view::{WidgetryListViewAppExt, WidgetryListViewSystems};
use bevy_widgetry_log::widgetry_info;
use std::marker::PhantomData;

/// 以 T 的 Bevy plugin identity 去重 typed ComboBox runtime。
struct TypedComboBoxPlugin<T>(PhantomData<fn() -> T>);

/// 自动装配公共 plugin、对应 ListView type 和 ComboBox typed systems。
pub trait WidgetryComboBoxAppExt {
    /// 相同 T 幂等；不同 T 各自注册；T 无需实现 Clone 或 Default。
    fn register_widgetry_combo_box<T: Send + Sync + 'static>(&mut self) -> &mut Self;
}

impl WidgetryComboBoxAppExt for App {
    fn register_widgetry_combo_box<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        if !self.is_plugin_added::<WidgetryComboBoxPlugin>() {
            self.add_plugins(WidgetryComboBoxPlugin);
        }
        self.register_widgetry_list_view::<T>();
        if !self.is_plugin_added::<TypedComboBoxPlugin<T>>() {
            self.add_plugins(TypedComboBoxPlugin::<T>(PhantomData));
        }
        self
    }
}

impl<T: Send + Sync + 'static> Plugin for TypedComboBoxPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_observer(combo_box::handle_value_change::<T>)
            .add_observer(popup::handle_field_activate::<T>)
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
            .add_systems(Update, field::sync_icon::<T>)
            .add_systems(
                PostUpdate,
                combo_box::initialize_selection::<T>.before(WidgetryListViewSystems::SyncState),
            )
            .add_systems(
                PostUpdate,
                field::project::<T>
                    .after(WidgetryListViewSystems::SyncState)
                    .before(UiSystems::Prepare)
                    .before(UiSystems::Propagate)
                    .before(bevy::text::detect_text_needs_rerender),
            );
        widgetry_info!(
            item_type = std::any::type_name::<T>(),
            "TypedComboBoxPlugin 注册完成"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_widgetry_list_view::WidgetryListViewPlugin;
    use bevy_widgetry_test_utils::{LogCapture, scene_app};

    /// 自动补齐基础设施；同 T 重复注册只产生一份 typed plugin 和注册日志。
    #[test]
    fn typed_registration_is_idempotent() {
        let capture = LogCapture::default();
        let mut app = scene_app();
        capture.run(|| {
            app.register_widgetry_combo_box::<String>()
                .register_widgetry_combo_box::<String>()
                .register_widgetry_combo_box::<u32>();
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
