use crate::behavior::sync_models;
use crate::renderer::{RendererRegistry, render_content};
use crate::{WidgetryTreeVisibleItem, view};
use bevy::input_focus::InputFocusSystems;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_button::WidgetryButtonPlugin;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_list_view::{
    WidgetryListViewAppExt, WidgetryListViewPlugin, WidgetryListViewSystems,
};
use bevy_widgetry_log::widgetry_info;

pub struct WidgetryTreePlugin;

impl Plugin for WidgetryTreePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<WidgetryListViewPlugin>() {
            app.add_plugins(WidgetryListViewPlugin);
        }
        if !app.is_plugin_added::<WidgetryButtonPlugin>() {
            app.add_plugins(WidgetryButtonPlugin);
        }
        if !app.is_plugin_added::<WidgetryIconPlugin>() {
            app.add_plugins(WidgetryIconPlugin);
        }
        if !app.is_plugin_added::<WidgetryAssetPlugin>() {
            app.add_plugins(WidgetryAssetPlugin);
        }
        if let Err(error) = app.register_widgetry_list_view::<WidgetryTreeVisibleItem>() {
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(error) });
            return;
        }
        app.init_resource::<RendererRegistry>();
        app.add_systems(
            PostUpdate,
            render_content
                .after(WidgetryListViewSystems::Reconcile)
                .in_set(WidgetryUiSystems::Build),
        );
        app.add_systems(
            PreUpdate,
            (view::validate_sources, view::validate_hierarchy)
                .chain()
                .before(PickingSystems::ProcessInput)
                .before(InputFocusSystems::Dispatch),
        );
        app.add_systems(
            PostUpdate,
            view::project_selection
                .after(sync_models)
                .before(WidgetryListViewSystems::SyncState)
                .in_set(WidgetryUiSystems::Build),
        );
        app.add_systems(
            PostUpdate,
            view::validate_hierarchy
                .after(WidgetryListViewSystems::Reconcile)
                .in_set(WidgetryUiSystems::Build),
        );
        app.add_observer(view::on_selection);
        app.add_systems(
            PostUpdate,
            sync_models
                .in_set(WidgetryUiSystems::Build)
                .before(WidgetryListViewSystems::SyncState),
        );
        widgetry_info!("WidgetryTreePlugin 注册完成");
    }
}
