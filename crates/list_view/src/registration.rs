use crate::behavior::{
    install_disabled_press_cleanup, install_pointer_cleanup, on_cancel, on_click,
    on_disabled_added, on_disabled_removed, on_drag_end, on_key, on_press, on_release, on_scroll,
    project, sync_state,
};
use crate::source::DataSource;
use crate::style::{collect_changes, refresh_theme, update};
use crate::view::validate_sources;
use crate::virtualization::reconcile;
use crate::{WidgetryListDataSource, WidgetryListItemId, WidgetryListKey, WidgetryListModelSource};
use bevy::picking::pointer::PointerInput;
use bevy::prelude::*;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin;
use bevy_widgetry_theme::WidgetryThemePlugin;
use std::marker::PhantomData;

pub struct WidgetryListViewPlugin;

struct TypedListViewPlugin<T, K>(PhantomData<fn() -> (T, K)>);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetryListViewSystems {
    Projection,
    SyncState,
    Reconcile,
}

pub trait WidgetryListViewAppExt {
    fn register_widgetry_list_view<T: Send + Sync + 'static>(
        &mut self,
    ) -> Result<&mut Self, BevyError>;
    fn register_widgetry_list_source<
        T: Send + Sync + 'static,
        K: WidgetryListKey,
        D: WidgetryListDataSource<T, K>,
    >(
        &mut self,
    ) -> Result<&mut Self, BevyError>;
}

impl Plugin for WidgetryListViewPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        app.add_message::<PointerInput>();
        #[cfg(test)]
        app.init_resource::<crate::style::StyleWork>();

        crate::style::install(app);
        if !app.is_plugin_added::<WidgetryScrollAreaPlugin>() {
            app.add_plugins(WidgetryScrollAreaPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }
        widgetry_info!("WidgetryListViewPlugin 注册完成");
    }
}

impl<T: Send + Sync + 'static, K: WidgetryListKey> Plugin for TypedListViewPlugin<T, K> {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, validate_sources::<T, K>);
        install_disabled_press_cleanup::<K>(app);
        crate::style::install_typed::<K>(app);
        install_pointer_cleanup::<T, K>(app);
        crate::drag::install::<T, K>(app);
        // Rows or renderer children created after propagation miss camera information or text measurement in that frame.
        // Complete construction during UI Build so propagation can consume it.
        app.add_systems(
            PostUpdate,
            (
                crate::projection::refresh::<T, K>.in_set(WidgetryListViewSystems::Projection),
                sync_state::<T, K>.in_set(WidgetryListViewSystems::SyncState),
                reconcile::<T, K>.in_set(WidgetryListViewSystems::Reconcile),
                project::<T, K>,
            )
                .chain()
                .in_set(WidgetryUiSystems::Build),
        );
        app.add_systems(
            PostUpdate,
            collect_changes::<T, K>.in_set(WidgetryUiSystems::StyleOwners),
        );
        app.add_systems(PostUpdate, update::<T, K>.in_set(WidgetryUiSystems::Colors));
        app.add_observer(refresh_theme::<T, K>);
        app.add_observer(on_click::<T, K>);
        app.add_observer(on_key::<T, K>);
        app.add_observer(on_scroll::<T, K>);
        app.add_observer(on_press::<T, K>)
            .add_observer(on_release::<T, K>)
            .add_observer(on_cancel::<T, K>)
            .add_observer(on_drag_end::<T, K>)
            .add_observer(on_disabled_added::<T, K>)
            .add_observer(on_disabled_removed::<T, K>);
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
        self.register_widgetry_list_source::<T, WidgetryListItemId, WidgetryListModelSource<T>>()?;
        Ok(self)
    }

    fn register_widgetry_list_source<
        T: Send + Sync + 'static,
        K: WidgetryListKey,
        D: WidgetryListDataSource<T, K>,
    >(
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
        if let Some(existing) = self.world().get_resource::<DataSource<T, K>>() {
            if existing.adapter != std::any::TypeId::of::<D>() {
                widgetry_error!("ListView 数据源适配器重复且不匹配");
                return Err(BevyError::error(
                    "ListView data source adapter already registered",
                ));
            }
        } else {
            self.insert_resource(DataSource::<T, K>::new::<D>());
        }
        if !self.is_plugin_added::<TypedListViewPlugin<T, K>>() {
            self.add_plugins(TypedListViewPlugin::<T, K>(PhantomData));
        }
        Ok(self)
    }
}

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
        assert!(app.is_plugin_added::<TypedListViewPlugin<String, WidgetryListItemId>>());
        assert!(app.is_plugin_added::<TypedListViewPlugin<u32, WidgetryListItemId>>());
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
