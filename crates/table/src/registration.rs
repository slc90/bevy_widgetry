use crate::{WidgetryTableCellValue, WidgetryTableHeaderValue};
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::reflect::{GetTypeRegistration, TypeRegistry};
use bevy::ui_widgets::ScrollAreaPlugin;
use bevy_widgetry_core::ForegroundColorPlugin;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use bevy_widgetry_theme::WidgetryThemePlugin;
use std::any::Any;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct WidgetryTablePlugin;

struct TypedTablePlugin<T>(PhantomData<fn() -> T>);

pub struct WidgetryTableCellRenderer<V>(Arc<dyn Fn(&V) -> Box<dyn SceneList> + Send + Sync>);

pub struct WidgetryTableHeaderRenderer<V>(Arc<dyn Fn(&V) -> Box<dyn SceneList> + Send + Sync>);

#[derive(Resource, Default)]
pub struct WidgetryTableCellRendererRegistry(pub(crate) RendererRegistry);

#[derive(Resource, Default)]
pub struct WidgetryTableHeaderRendererRegistry(pub(crate) RendererRegistry);

#[derive(Clone)]
struct RendererData {
    generation: u64,
    render: Arc<
        dyn Fn(&(dyn Any + Send + Sync)) -> Result<Box<dyn SceneList>, BevyError> + Send + Sync,
    >,
}

#[derive(Default)]
pub(crate) struct RendererRegistry {
    types: TypeRegistry,
    generation: u64,
}

pub trait WidgetryTableAppExt {
    fn register_widgetry_table<T: Send + Sync + 'static>(&mut self) -> &mut Self;

    fn register_table_cell_renderer<V: GetTypeRegistration + Send + Sync + 'static>(
        &mut self,
        renderer: WidgetryTableCellRenderer<V>,
    ) -> Result<&mut Self, BevyError>;

    fn register_table_header_renderer<V: GetTypeRegistration + Send + Sync + 'static>(
        &mut self,
        renderer: WidgetryTableHeaderRenderer<V>,
    ) -> Result<&mut Self, BevyError>;
}

impl Plugin for WidgetryTablePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }
        if !app.is_plugin_added::<ScrollAreaPlugin>() {
            app.add_plugins(ScrollAreaPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        app.init_resource::<WidgetryTableCellRendererRegistry>()
            .init_resource::<WidgetryTableHeaderRendererRegistry>();
        app.add_message::<bevy::window::RequestRedraw>()
            .register_type::<crate::WidgetryTableBody>()
            .register_type::<crate::WidgetryTableColumnHeaders>()
            .register_type::<crate::WidgetryTableRowHeaders>()
            .register_type::<crate::WidgetryTableCorner>()
            .register_type::<crate::WidgetryTableCell>()
            .register_type::<crate::WidgetryTableColumnHeader>()
            .register_type::<crate::WidgetryTableRowHeader>()
            .register_type::<crate::WidgetryTableState>();
        crate::resize::install(app);
        app.add_systems(Last, crate::interaction::clear_pointer_focus);
        widgetry_info!("WidgetryTablePlugin 注册完成");
    }
}

impl<T: Send + Sync + 'static> Plugin for TypedTablePlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_observer(crate::interaction::on_click::<T>);
        app.add_observer(crate::interaction::on_press::<T>);
        app.add_observer(crate::interaction::on_key::<T>);
        app.add_observer(crate::interaction::on_focus::<T>);
        app.add_observer(crate::resize::on_start::<T>)
            .add_observer(crate::resize::on_drag::<T>)
            .add_observer(crate::resize::on_end::<T>)
            .add_observer(crate::resize::on_cancel::<T>)
            .add_observer(crate::resize::on_disabled_added::<T>);
        app.add_systems(
            PostUpdate,
            crate::projection::reconcile::<T>.in_set(WidgetryUiSystems::Build),
        );
        app.add_systems(
            PostUpdate,
            crate::projection::request_geometry_redraw::<T>.after(bevy::ui::UiSystems::Layout),
        );
        widgetry_info!(
            row_type = std::any::type_name::<T>(),
            "TypedTablePlugin 注册完成"
        );
    }
}

impl<V> WidgetryTableCellRenderer<V> {
    pub fn new<S: SceneList + 'static>(factory: impl Fn(&V) -> S + Send + Sync + 'static) -> Self {
        Self(Arc::new(move |value| Box::new(factory(value))))
    }
}

impl<V> WidgetryTableHeaderRenderer<V> {
    pub fn new<S: SceneList + 'static>(factory: impl Fn(&V) -> S + Send + Sync + 'static) -> Self {
        Self(Arc::new(move |value| Box::new(factory(value))))
    }
}

impl WidgetryTableCellRendererRegistry {
    pub fn render(&self, value: &WidgetryTableCellValue) -> Result<Box<dyn SceneList>, BevyError> {
        self.0
            .scene(value.as_any())
            .map(|(_, scene)| scene)
            .map_err(|error| {
                widgetry_error!(value_type = ?value.type_id(), %error, "Table Cell renderer 失败");
                error
            })
    }
}

impl WidgetryTableHeaderRendererRegistry {
    pub fn render(
        &self,
        value: &WidgetryTableHeaderValue,
    ) -> Result<Box<dyn SceneList>, BevyError> {
        self.0.scene(value.as_any()).map(|(_, scene)| scene).map_err(|error| {
            widgetry_error!(value_type = ?value.type_id(), %error, "Table Header renderer 失败");
            error
        })
    }
}

impl WidgetryTableAppExt for App {
    fn register_widgetry_table<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        if !self.is_plugin_added::<WidgetryTablePlugin>() {
            self.add_plugins(WidgetryTablePlugin);
        }
        if !self.is_plugin_added::<TypedTablePlugin<T>>() {
            self.add_plugins(TypedTablePlugin::<T>(PhantomData));
        }
        self
    }

    fn register_table_cell_renderer<V: GetTypeRegistration + Send + Sync + 'static>(
        &mut self,
        renderer: WidgetryTableCellRenderer<V>,
    ) -> Result<&mut Self, BevyError> {
        if !self.is_plugin_added::<WidgetryTablePlugin>() {
            self.add_plugins(WidgetryTablePlugin);
        }
        let Some(mut registry) = self
            .world_mut()
            .get_resource_mut::<WidgetryTableCellRendererRegistry>()
        else {
            widgetry_error!("Table Cell renderer registry 缺失");
            return Err(BevyError::error("Table Cell renderer registry missing"));
        };
        registry.0.register(renderer.0)?;
        Ok(self)
    }

    fn register_table_header_renderer<V: GetTypeRegistration + Send + Sync + 'static>(
        &mut self,
        renderer: WidgetryTableHeaderRenderer<V>,
    ) -> Result<&mut Self, BevyError> {
        if !self.is_plugin_added::<WidgetryTablePlugin>() {
            self.add_plugins(WidgetryTablePlugin);
        }
        let Some(mut registry) = self
            .world_mut()
            .get_resource_mut::<WidgetryTableHeaderRendererRegistry>()
        else {
            widgetry_error!("Table Header renderer registry 缺失");
            return Err(BevyError::error("Table Header renderer registry missing"));
        };
        registry.0.register(renderer.0)?;
        Ok(self)
    }
}

impl RendererRegistry {
    pub(crate) fn generation(&self, value_type: std::any::TypeId) -> Result<u64, BevyError> {
        self.types
            .get_type_data::<RendererData>(value_type)
            .map(|entry| entry.generation)
            .ok_or_else(|| BevyError::error("Table value has no registered renderer"))
    }

    fn register<V: GetTypeRegistration + Send + Sync + 'static>(
        &mut self,
        factory: Arc<dyn Fn(&V) -> Box<dyn SceneList> + Send + Sync>,
    ) -> Result<(), BevyError> {
        let Some(generation) = self.generation.checked_add(1) else {
            widgetry_error!("Table renderer generation 已耗尽");
            return Err(BevyError::error("Table renderer generation exhausted"));
        };
        let mut registration = V::get_type_registration();
        registration.insert(RendererData {
            generation,
            render: Arc::new(move |value| {
                value
                    .downcast_ref::<V>()
                    .map(|value| factory(value))
                    .ok_or_else(|| BevyError::error("Table renderer value type mismatch"))
            }),
        });
        self.types.register::<V>();
        let Some(current) = self.types.get_mut(registration.type_id()) else {
            widgetry_error!("Table renderer TypeRegistration 缺失");
            return Err(BevyError::error("Table renderer TypeRegistration missing"));
        };
        *current = registration;
        self.generation = generation;
        Ok(())
    }

    pub(crate) fn scene(
        &self,
        value: &(dyn Any + Send + Sync),
    ) -> Result<(u64, Box<dyn SceneList>), BevyError> {
        let entry = self
            .types
            .get_type_data::<RendererData>(value.type_id())
            .ok_or_else(|| BevyError::error("Table value has no registered renderer"))?;
        Ok((entry.generation, (entry.render)(value)?))
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::LogCapture;

    #[test]
    fn exhausted_generation_retains_old_registration() {
        let mut registry = RendererRegistry::default();
        registry
            .register::<u32>(Arc::new(|_| Box::new(bsn_list![(Text("old"))])))
            .unwrap();
        let old = registry
            .types
            .get_type_data::<RendererData>(std::any::TypeId::of::<u32>())
            .unwrap()
            .generation;
        registry.generation = u64::MAX;
        let capture = LogCapture::default();
        let error = capture
            .run(|| registry.register::<u32>(Arc::new(|_| Box::new(bsn_list![(Text("new"))]))))
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("generation exhausted"));
        assert_eq!(capture.records().len(), 1);
        assert_eq!(registry.generation, u64::MAX);
        assert_eq!(
            registry
                .types
                .get_type_data::<RendererData>(std::any::TypeId::of::<u32>())
                .unwrap()
                .generation,
            old
        );
    }
}
