use crate::WidgetryTreePlugin;
use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::scene::apply_scene;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::any::TypeId;
use std::sync::Arc;

/// 将业务 Component 的 owned SceneList factory type erase，不要求 T 实现 Clone。
/// factory 接收 node Entity 与只读 T；返回的内容不得长期借用 T，不负责 row/expander/style。
/// 内容可能因 virtualization 销毁，持久业务 state 应保留在 node 的 ECS Component。
pub struct WidgetryTreeRenderer<T>(Arc<dyn Fn(Entity, &T) -> Box<dyn SceneList> + Send + Sync>);

/// 按业务 Component 类型注册 Tree renderer；没有 priority matcher 或 fallback。
pub trait WidgetryTreeAppExt {
    /// 自动装配 WidgetryTreePlugin；同 T 再次注册替换其 factory 并刷新已显示内容。
    /// 每个 rendered node 必须恰好持有一种已注册 Component，否则记录 ERROR 并上抛 BevyError。
    fn register_renderer<T: Component>(
        &mut self,
        renderer: WidgetryTreeRenderer<T>,
    ) -> Result<&mut Self, BevyError>;
}

/// World-aware dispatch 留在 Tree，ListView renderer 仍只负责 row shell。
struct RegisteredRenderer {
    /// 匹配依据，不使用 priority 或 Component 插入顺序。
    component_type: TypeId,
    /// 为缺失/歧义诊断提供业务 type context。
    name: &'static str,
    /// 注册替换时推进，使同 type 的已有内容失效。
    generation: u64,
    /// 返回业务 Component 的 change tick；None 表示 node 不匹配。
    matches: fn(&World, Entity) -> Option<u32>,
    /// matched node 的业务内容 factory，不持有 mutable World。
    render: Arc<dyn Fn(&World, Entity) -> Result<Box<dyn SceneList>, BevyError> + Send + Sync>,
}

/// row 内挂载业务 renderer 的 ECS 容器，随 ListView row lifecycle 销毁。
#[derive(Component, Clone, Copy)]
#[require(ContentProjection, RenderDiagnostics)]
pub(crate) struct TreeContent {
    /// 当前容器对应的业务 node Entity。
    pub(crate) node: Entity,
}

/// 此物理 container 的 renderer 异常；随 row 销毁，不输出销毁恢复。
#[derive(Component, Default)]
struct RenderDiagnostics(FailureState);

/// 上次成功展开内容的 fingerprint，不作为业务 state。
#[derive(Component, Default)]
struct ContentProjection(Option<ContentStamp>);

/// 区分注册替换、业务 type 切换和 Component 内容修改。
#[derive(Clone, Copy, PartialEq, Eq)]
struct ContentStamp {
    /// 唯一匹配的 Component 类型。
    component_type: TypeId,
    /// factory 最近注册版本。
    generation: u64,
    /// 业务 Component 的最近 mutation tick。
    changed: u32,
}

/// App 级 Component renderer registry，独立于各 Tree model 与业务 hierarchy。
#[derive(Resource, Default)]
pub(crate) struct RendererRegistry {
    /// 一种 Component 只拥有一个 renderer。
    entries: Vec<RegisteredRenderer>,
    /// 单调注册版本，不随 type 替换回退。
    generation: u64,
}

/// ListView 完成 row shell 后，在同帧 UI 消费前展开真实 ECS renderer 内容。
pub(crate) fn render_content(world: &mut World) -> Result<(), BevyError> {
    let containers = world
        .query::<(Entity, &TreeContent, &ContentProjection)>()
        .iter(world)
        .map(|(entity, content, projection)| (entity, content.node, projection.0))
        .collect::<Vec<_>>();
    let mut failure = None;
    for (entity, node, previous) in containers {
        let result = (|| -> Result<(), BevyError> {
            let registry = world.resource::<RendererRegistry>();
            let Some((entry, changed)) = registry.lookup(world, node)? else {
                return Err(BevyError::error("Tree node has no registered renderer"));
            };
            let stamp = ContentStamp {
                component_type: entry.component_type,
                generation: entry.generation,
                changed,
            };
            if previous == Some(stamp) {
                return Ok(());
            }
            let scene = (entry.render)(world, node)?;
            let children = world
                .get::<Children>(entity)
                .map(|children| children.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            world.entity_mut(entity).insert(ContentProjection(None));
            for child in children {
                world.despawn(child);
            }
            if let Err(error) =
                apply_scene(&mut world.entity_mut(entity), bsn! { Children [{scene}] })
            {
                return Err(BevyError::error(error));
            }
            world
                .entity_mut(entity)
                .insert(ContentProjection(Some(stamp)));

            Ok(())
        })();
        let result = if let Some(mut diagnostics) = world.get_mut::<RenderDiagnostics>(entity) {
            diagnostics.0.observe(
                result,
                |error| widgetry_error!(?node, ?entity, %error, "Tree renderer 失败"),
                || widgetry_info!(?node, ?entity, "Tree renderer 恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

impl<T> WidgetryTreeRenderer<T> {
    /// 接收可重复调用的 SceneList factory，业务数据由外部 Component 驱动。
    pub fn new<S, F>(factory: F) -> Self
    where
        S: SceneList + 'static,
        F: Fn(Entity, &T) -> S + Send + Sync + 'static,
    {
        Self(Arc::new(move |entity, value| {
            Box::new(factory(entity, value))
        }))
    }
}

impl WidgetryTreeAppExt for App {
    fn register_renderer<T: Component>(
        &mut self,
        renderer: WidgetryTreeRenderer<T>,
    ) -> Result<&mut Self, BevyError> {
        if !self.is_plugin_added::<WidgetryTreePlugin>() {
            self.add_plugins(WidgetryTreePlugin);
        }
        self.init_resource::<RendererRegistry>();
        self.world_mut()
            .resource_mut::<RendererRegistry>()
            .register(renderer)?;
        Ok(self)
    }
}

impl RendererRegistry {
    /// 替换同 type，不通过重复 entry 改变 lookup 语义。
    fn register<T: Component>(
        &mut self,
        renderer: WidgetryTreeRenderer<T>,
    ) -> Result<(), BevyError> {
        self.generation = self.generation.checked_add(1).ok_or_else(|| {
            widgetry_error!("Tree renderer registry generation 已耗尽");
            BevyError::error("Tree renderer generation exhausted")
        })?;
        let entry = RegisteredRenderer {
            component_type: TypeId::of::<T>(),
            name: std::any::type_name::<T>(),
            generation: self.generation,
            matches: |world, entity| {
                world
                    .get_entity(entity)
                    .ok()?
                    .get_ref::<T>()
                    .map(|component| component.last_changed().get())
            },
            render: Arc::new(move |world, entity| {
                let Some(value) = world.get::<T>(entity) else {
                    widgetry_error!(
                        ?entity,
                        component_type = std::any::type_name::<T>(),
                        "Tree renderer dispatch 后业务 Component 缺失"
                    );
                    return Err(BevyError::error("Tree renderer component missing"));
                };
                Ok(renderer.0(entity, value))
            }),
        };
        if let Some(old) = self
            .entries
            .iter_mut()
            .find(|old| old.component_type == entry.component_type)
        {
            *old = entry;
        } else {
            self.entries.push(entry);
        }
        Ok(())
    }

    /// 缺少匹配返回 None；多个匹配是配置错误，不能静默选择某个 renderer。
    fn lookup(
        &self,
        world: &World,
        entity: Entity,
    ) -> Result<Option<(&RegisteredRenderer, u32)>, BevyError> {
        let mut matched: Option<(&RegisteredRenderer, u32)> = None;
        for entry in &self.entries {
            if let Some(tick) = (entry.matches)(world, entity) {
                if let Some((old, _)) = matched {
                    return Err(BevyError::error(format!(
                        "Ambiguous Tree renderer components: {entity:?}, {}, {}",
                        old.name, entry.name
                    )));
                }
                matched = Some((entry, tick));
            }
        }
        Ok(matched)
    }
}

// 测试 module 中的断言用于验证 contract，生产代码仍禁止。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::log::tracing::Level;
    use bevy_widgetry_test_utils::LogCapture;

    /// 两类业务 marker 用于验证 Component lookup，而非 user-defined matcher。
    #[derive(Component)]
    struct Folder;

    /// File 独立于 Folder，不能因注册顺序错误 dispatch。
    #[derive(Component)]
    struct File;

    /// registry 按实际 Component 选择 factory，同 type 替换不增加歧义 entry。
    #[test]
    fn registry_lookup_uses_component_type_and_deduplicates_registration() {
        let mut world = World::new();
        let folder = world.spawn(Folder).id();
        let file = world.spawn(File).id();
        let mut registry = RendererRegistry::default();
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &Folder| {
                bsn_list![(Text("folder"))]
            }))
            .unwrap();
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &File| {
                bsn_list![(Text("file"))]
            }))
            .unwrap();
        assert_eq!(
            registry
                .lookup(&world, folder)
                .unwrap()
                .unwrap()
                .0
                .component_type,
            TypeId::of::<Folder>()
        );
        assert_eq!(
            registry
                .lookup(&world, file)
                .unwrap()
                .unwrap()
                .0
                .component_type,
            TypeId::of::<File>()
        );
        let before = registry
            .lookup(&world, folder)
            .unwrap()
            .unwrap()
            .0
            .generation;
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &Folder| {
                bsn_list![(Text("updated"))]
            }))
            .unwrap();
        assert_eq!(registry.entries.len(), 2);
        assert!(
            registry
                .lookup(&world, folder)
                .unwrap()
                .unwrap()
                .0
                .generation
                > before
        );
        assert!(
            registry
                .lookup(&world, Entity::PLACEHOLDER)
                .unwrap()
                .is_none()
        );
    }

    /// 无 renderer 的 rendered node 记录 ERROR 并返回错误，不产生 fallback 内容。
    #[test]
    fn missing_renderer_returns_error_and_logs() {
        let mut world = World::new();
        world.init_resource::<RendererRegistry>();
        let node = world.spawn(Folder).id();
        world.spawn(TreeContent { node });
        let capture = LogCapture::default();
        assert!(capture.run(|| render_content(&mut world)).is_err());
        assert!(
            capture
                .records()
                .iter()
                .any(|record| record.level == Level::ERROR
                    && record.fields["error"].contains("no registered renderer"))
        );
    }

    /// 同 node 持有两种已注册业务 Component 属于歧义，不能按注册顺序选择。
    #[test]
    fn ambiguous_components_returns_error_and_logs() {
        let mut world = World::new();
        let node = world.spawn((Folder, File)).id();
        let mut registry = RendererRegistry::default();
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &Folder| bsn_list![]))
            .unwrap();
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &File| bsn_list![]))
            .unwrap();
        world.insert_resource(registry);
        world.spawn(TreeContent { node });
        let capture = LogCapture::default();
        assert!(capture.run(|| render_content(&mut world)).is_err());
        assert!(
            capture
                .records()
                .iter()
                .any(|record| record.level == Level::ERROR
                    && record.fields["error"].contains("Ambiguous"))
        );
    }

    /// 持续无 renderer 不刷日志或阻塞另一 container，修复后仅记录一次恢复。
    #[test]
    fn renderer_failure_logs_edges_and_preserves_other_containers() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        let world = app.world_mut();
        let missing = world.spawn(Folder).id();
        let healthy = world.spawn(File).id();
        let failed_content = world.spawn(TreeContent { node: missing }).id();
        let healthy_content = world.spawn(TreeContent { node: healthy }).id();
        let mut registry = RendererRegistry::default();
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &File| {
                bsn_list![(Text("healthy"))]
            }))
            .unwrap();
        world.insert_resource(registry);
        let capture = LogCapture::default();
        for _ in 0..2 {
            assert!(capture.run(|| render_content(world)).is_err());
        }
        assert!(world.get::<Children>(healthy_content).is_some());
        assert!(world.get::<Children>(failed_content).is_none());
        assert_eq!(
            capture
                .records()
                .iter()
                .filter(|record| record.level == Level::ERROR)
                .count(),
            1
        );
        world
            .resource_mut::<RendererRegistry>()
            .register(WidgetryTreeRenderer::new(|_, _: &Folder| {
                bsn_list![(Text("repaired"))]
            }))
            .unwrap();
        capture.run(|| render_content(world)).unwrap();
        capture.run(|| render_content(world)).unwrap();
        assert!(world.get::<Children>(failed_content).is_some());
        assert_eq!(
            capture
                .records()
                .iter()
                .filter(|record| record
                    .fields
                    .get("message")
                    .is_some_and(|message| message.contains("恢复")))
                .count(),
            1
        );
    }
}
