use crate::WidgetryTreePlugin;
use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::scene::apply_scene;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::any::TypeId;
use std::sync::Arc;

pub struct WidgetryTreeRenderer<T>(Arc<dyn Fn(Entity, &T) -> Box<dyn SceneList> + Send + Sync>);

pub trait WidgetryTreeAppExt {
    fn register_renderer<T: Component>(
        &mut self,
        renderer: WidgetryTreeRenderer<T>,
    ) -> Result<&mut Self, BevyError>;
}

struct RegisteredRenderer {
    component_type: TypeId,
    name: &'static str,
    generation: u64,
    matches: fn(&World, Entity) -> Option<u32>,
    render: Arc<dyn Fn(&World, Entity) -> Result<Box<dyn SceneList>, BevyError> + Send + Sync>,
}

#[derive(Component, Clone, Copy)]
#[require(ContentProjection, RenderDiagnostics)]
pub(crate) struct TreeContent {
    pub(crate) node: Entity,
}

#[derive(Component, Default)]
struct RenderDiagnostics(FailureState);

#[derive(Component, Default)]
struct ContentProjection(Option<ContentStamp>);

#[derive(Clone, Copy, PartialEq, Eq)]
struct ContentStamp {
    component_type: TypeId,
    generation: u64,
    changed: u32,
}

#[derive(Resource, Default)]
pub(crate) struct RendererRegistry {
    entries: Vec<RegisteredRenderer>,
    generation: u64,
}

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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::log::tracing::Level;
    use bevy_widgetry_test_utils::LogCapture;

    #[derive(Component)]
    struct Folder;

    #[derive(Component)]
    struct File;

    #[test]
    fn registry_lookup_uses_component_type_and_deduplicates_registration() {
        let mut world = World::new();
        let folder = world.spawn(Folder).id();
        let file = world.spawn(File).id();
        let mut registry = RendererRegistry::default();
        registry
            .register(WidgetryTreeRenderer::new(
                |_, _: &Folder| bsn_list! {Text("folder") bevy_widgetry_core::text::WidgetryText},
            ))
            .unwrap();
        registry
            .register(WidgetryTreeRenderer::new(
                |_, _: &File| bsn_list! {Text("file") bevy_widgetry_core::text::WidgetryText},
            ))
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
            .register(WidgetryTreeRenderer::new(
                |_, _: &Folder| bsn_list! {Text("updated") bevy_widgetry_core::text::WidgetryText},
            ))
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

    #[test]
    fn ambiguous_components_returns_error_and_logs() {
        let mut world = World::new();
        let node = world.spawn((Folder, File)).id();
        let mut registry = RendererRegistry::default();
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &Folder| bsn_list! {}))
            .unwrap();
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &File| bsn_list! {}))
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
            .register(WidgetryTreeRenderer::new(
                |_, _: &File| bsn_list! {Text("healthy") bevy_widgetry_core::text::WidgetryText},
            ))
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
                bsn_list! {Text("repaired") bevy_widgetry_core::text::WidgetryText}
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
