use crate::{WidgetryTableCellValue, WidgetryTableHeaderValue};
use bevy::prelude::*;
use bevy::reflect::{GetTypeRegistration, TypeRegistry};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::any::Any;
use std::sync::Arc;

/// 自动装配 Table 的独立 Cell/Header registry；应用提供官方 UI/input plugin。
pub struct WidgetryTablePlugin;

/// 只生成 Cell shell 的 direct children，不负责 shell style、identity 或交互。
/// factory 可重复调用，返回 owned SceneList；持久业务 state 不应依赖 Content lifecycle。
pub struct WidgetryTableCellRenderer<V>(Arc<dyn Fn(&V) -> Box<dyn SceneList> + Send + Sync>);

/// 与 Cell factory 语义独立；同一 value type 可以产生完全不同的 Header Content。
pub struct WidgetryTableHeaderRenderer<V>(Arc<dyn Fn(&V) -> Box<dyn SceneList> + Send + Sync>);

/// Resource 内持有独立 Bevy TypeRegistry，不共享 AppTypeRegistry 或 Header 注册。
#[derive(Resource, Default)]
pub struct WidgetryTableCellRendererRegistry(pub(crate) RendererRegistry);

/// Header 独立派发，不因 Cell 注册相同 type 而自动获得 factory。
#[derive(Resource, Default)]
pub struct WidgetryTableHeaderRendererRegistry(pub(crate) RendererRegistry);

/// Bevy TypeData 绑定当前 factory 和 replacement generation。
#[derive(Clone)]
struct RendererData {
    /// 成功重新注册后推进，用于 Content 刷新。
    generation: u64,
    /// 精确 type downcast，不进行格式化 fallback。
    render: Arc<
        dyn Fn(&(dyn Any + Send + Sync)) -> Result<Box<dyn SceneList>, BevyError> + Send + Sync,
    >,
}

/// 两个语义 Resource 共用内部注册算法；外部不能操作共享 registry。
#[derive(Default)]
pub(crate) struct RendererRegistry {
    /// 类型与 Widgetry TypeData 的唯一索引。
    types: TypeRegistry,
    /// 不回退的注册版本，仅用于 Content projection。
    generation: u64,
}

/// 按真实 value type 注册 renderer，自动装配 Table plugin。
pub trait WidgetryTableAppExt {
    /// V 通常 derive Reflect；同 V 再次注册替换 factory 并推进 generation。
    fn register_table_cell_renderer<V: GetTypeRegistration + Send + Sync + 'static>(
        &mut self,
        renderer: WidgetryTableCellRenderer<V>,
    ) -> Result<&mut Self, BevyError>;

    /// Header 与 Cell 必须分别注册；不要求 Row 业务 type 支持 Reflect。
    fn register_table_header_renderer<V: GetTypeRegistration + Send + Sync + 'static>(
        &mut self,
        renderer: WidgetryTableHeaderRenderer<V>,
    ) -> Result<&mut Self, BevyError>;
}

impl Plugin for WidgetryTablePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WidgetryTableCellRendererRegistry>()
            .init_resource::<WidgetryTableHeaderRendererRegistry>();
        widgetry_info!("WidgetryTablePlugin 注册完成");
    }
}

impl<V> WidgetryTableCellRenderer<V> {
    /// 接收只读业务 value，返回不借用该 value 的 SceneList。
    pub fn new<S: SceneList + 'static>(factory: impl Fn(&V) -> S + Send + Sync + 'static) -> Self {
        Self(Arc::new(move |value| Box::new(factory(value))))
    }
}

impl<V> WidgetryTableHeaderRenderer<V> {
    /// 非 String Header 同样使用 typed factory，不接管 Header shell。
    pub fn new<S: SceneList + 'static>(factory: impl Fn(&V) -> S + Send + Sync + 'static) -> Self {
        Self(Arc::new(move |value| Box::new(factory(value))))
    }
}

impl WidgetryTableCellRendererRegistry {
    /// 创建 Cell Content SceneList，由调用方通过 BSN 展开并处理 Scene 错误。
    /// 未注册 type 返回已记录 ERROR 的 BevyError（Severity::Error），不创建占位内容。
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
    /// 创建 Header Content；仅注册同类型 Cell renderer 仍然返回 missing renderer 错误。
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
    /// 插入 TypeData 或替换原 factory；generation 耗尽时原注册保持可用。
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

    /// factory 查找与 Scene 生成；日志由公开入口或后续 View owner 负责。
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

// 测试允许断言来验证注册 contract，生产代码仍禁止。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::LogCapture;

    /// generation 耗尽返回 Error 并记录 ERROR，旧 factory/generation 不被失败注册覆盖。
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
