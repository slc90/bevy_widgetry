use crate::WidgetryTreePlugin;
use crate::{
    TreeError, TreeFailure, TreeRendererIdentity, TreeViewFault,
    error::{failure, persistent, view_failure},
};
use bevy::prelude::*;
use bevy_widgetry_core::scene::{SceneFailure, attempt::apply_scene};
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::FailureState};
use std::any::TypeId;
use std::sync::Arc;

pub struct WidgetryTreeRenderer<T>(Arc<dyn Fn(Entity, &T) -> Box<dyn SceneList> + Send + Sync>);

pub trait WidgetryTreeAppExt {
    fn register_renderer<T: Component>(
        &mut self,
        renderer: WidgetryTreeRenderer<T>,
    ) -> Result<&mut Self, TreeFailure>;
}

struct RegisteredRenderer {
    component_type: TypeId,
    name: &'static str,
    generation: u64,
    matches: fn(&World, Entity) -> Option<u32>,
    render: Arc<dyn Fn(&World, Entity) -> Result<Box<dyn SceneList>, TreeViewFault> + Send + Sync>,
}

#[derive(Component, Clone, Copy)]
#[require(ContentProjection, RenderDiagnostics)]
pub(crate) struct TreeContent {
    pub(crate) node: Entity,
}

#[derive(Component, Default)]
struct RenderDiagnostics(FailureState<TreeViewFault>);

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
    let mut failures = Vec::new();
    for (entity, node, previous) in containers {
        let result = (|| -> Result<(), RenderFailure> {
            let registry = world.resource::<RendererRegistry>();
            let found = registry.lookup(world, node).map_err(|(first, second)| {
                TreeViewFault::RendererAmbiguous {
                    content: entity,
                    node,
                    first,
                    second,
                }
            })?;
            let (entry, changed) = found.ok_or(TreeViewFault::RendererMissing {
                content: entity,
                node,
            })?;
            let stamp = ContentStamp {
                component_type: entry.component_type,
                generation: entry.generation,
                changed,
            };
            if previous == Some(stamp) {
                return Ok(());
            }
            let scene = (entry.render)(world, node)?;
            let context = renderer_frame(world, entity, node)
                .field("component", entry.name)
                .field("generation", entry.generation);
            let children = world
                .get::<Children>(entity)
                .map(|children| children.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            world.entity_mut(entity).insert(ContentProjection(None));
            for child in children {
                world.despawn(child);
            }
            let before = world.increment_change_tick();
            if let Err(error) =
                apply_scene(&mut world.entity_mut(entity), bsn! {Children [{scene}]})
            {
                cleanup_attempt(world, entity, before);
                return Err(RenderFailure::Native(
                    SceneFailure::from_native(error, entity, "tree.renderer")
                        .with_context(context)
                        .into_bevy_error(),
                ));
            }
            world
                .entity_mut(entity)
                .insert(ContentProjection(Some(stamp)));
            Ok(())
        })();
        match result {
            Ok(()) => {
                if let Some(mut state) = world.get_mut::<RenderDiagnostics>(entity) {
                    state.0.recover(|| renderer_frame_values(entity, node));
                }
            }
            Err(RenderFailure::Fault(fault)) => {
                let context = renderer_frame(world, entity, node);
                let diagnostics = match world.get_mut::<RenderDiagnostics>(entity) {
                    Some(mut state) => persistent(&mut state.0, fault, "tree.renderer"),
                    None => view_failure(fault, "tree.renderer"),
                };
                failures.push(diagnostics.with_context(context).into_bevy_error());
            }
            Err(RenderFailure::Native(error)) => failures.push(error),
        }
    }
    crate::error::dispatch(world, failures)
}
fn cleanup_attempt(world: &mut World, content: Entity, before: bevy::ecs::change_detection::Tick) {
    let current = world.change_tick();
    let mut pending = world
        .get::<Children>(content)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    let mut created = Vec::new();
    while let Some(entity) = pending.pop() {
        let new = world
            .entities()
            .entity_get_spawn_or_despawn_tick(entity)
            .is_some_and(|spawned| spawned.is_newer_than(before, current));
        if new {
            if let Some(children) = world.get::<Children>(entity) {
                pending.extend(children.iter());
            }
            created.push(entity);
        } else if let Ok(mut entity) = world.get_entity_mut(entity) {
            // Templates may attach existing business subtrees before a later sibling fails.
            // Detach those subtrees before recursive UI cleanup, retaining their committed state.
            entity.remove::<ChildOf>();
        }
    }
    for entity in created {
        if world.entities().contains(entity) {
            world.despawn(entity);
        }
    }
}

enum RenderFailure {
    Fault(TreeViewFault),
    Native(BevyError),
}
impl From<TreeViewFault> for RenderFailure {
    fn from(fault: TreeViewFault) -> Self {
        Self::Fault(fault)
    }
}
fn renderer_frame_values(content: Entity, node: Entity) -> ErrorFrame {
    ErrorFrame::new("tree.renderer")
        .field("content", content)
        .field("node", node)
}
fn renderer_frame(world: &World, content: Entity, node: Entity) -> ErrorFrame {
    let view = crate::view::tree_root(world, content);
    let source = view
        .and_then(|view| world.get::<crate::WidgetryTreeView>(view))
        .map(crate::WidgetryTreeView::source);
    let expanded = view
        .and_then(|view| world.get::<crate::WidgetryTreeState>(view))
        .map(|state| state.is_expanded(node));
    renderer_frame_values(content, node)
        .field("view", format_args!("{view:?}"))
        .field("source", format_args!("{source:?}"))
        .field("expanded", format_args!("{expanded:?}"))
        .field(
            "children_state",
            format_args!("{:?}", world.get::<crate::WidgetryTreeChildrenState>(node)),
        )
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
    ) -> Result<&mut Self, TreeFailure> {
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
    ) -> Result<(), TreeFailure> {
        self.generation = self.generation.checked_add(1).ok_or_else(|| {
            failure(
                TreeError::RegistryExhausted {
                    component: TreeRendererIdentity {
                        type_id: TypeId::of::<T>(),
                        name: std::any::type_name::<T>(),
                    },
                    generation: self.generation,
                },
                "tree.register_renderer",
            )
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
                    return Err(TreeViewFault::RendererComponent {
                        node: entity,
                        component: TreeRendererIdentity {
                            type_id: TypeId::of::<T>(),
                            name: std::any::type_name::<T>(),
                        },
                    });
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
    ) -> Result<Option<(&RegisteredRenderer, u32)>, (TreeRendererIdentity, TreeRendererIdentity)>
    {
        let mut matched: Option<(&RegisteredRenderer, u32)> = None;
        for entry in &self.entries {
            if let Some(tick) = (entry.matches)(world, entity) {
                if let Some((old, _)) = matched {
                    return Err((
                        TreeRendererIdentity {
                            type_id: old.component_type,
                            name: old.name,
                        },
                        TreeRendererIdentity {
                            type_id: entry.component_type,
                            name: entry.name,
                        },
                    ));
                }
                matched = Some((entry, tick));
            }
        }
        Ok(matched)
    }
}

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
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut world = World::new();
        world.init_resource::<RendererRegistry>();
        let node = world.spawn(Folder).id();
        world.spawn(TreeContent { node });
        let capture = LogCapture::default();
        let error = capture.run(|| render_content(&mut world)).unwrap_err();
        assert!(
            error
                .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                .is_some()
        );
        assert!(
            capture
                .records()
                .iter()
                .any(|record| record.level == Level::ERROR
                    && record.fields["context"].contains("RendererMissing"))
        );
    }

    #[test]
    fn ambiguous_components_returns_error_and_logs() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
                    && record.fields["context"].contains("RendererAmbiguous"))
        );
    }

    #[test]
    fn renderer_failure_logs_edges_and_preserves_other_containers() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
    #[derive(Component)]
    struct Broken(u8);
    #[derive(Component)]
    struct BusinessRecord;

    #[test]
    fn renderer_attempts_keep_all_causes_origins_native_payloads_and_business_entities() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture};
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(Update, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        app.add_systems(Update, render_content);
        let origins = Arc::new(std::sync::Mutex::new(Vec::new()));
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let captured_origins = origins.clone();
        let count = attempts.clone();
        let mut registry = RendererRegistry::default();
        registry.register(WidgetryTreeRenderer::new(move|_,broken:&Broken| {
            let policy=broken.0;let origins=captured_origins.clone();let count=count.clone();
            bsn_list! {Node--template(move|ctx| {
                ctx.entity.world_scope(|world|{world.spawn(BusinessRecord);});
                let error=match policy {
                    0|1=>{
                        let error=bevy_widgetry_core::color::validate_color(Color::linear_rgba(f32::NAN,0.,0.,1.)).unwrap_err();
                        origins.lock().unwrap().push(error.origin().clone());error.into_bevy_error()
                    }
                    2=>{let attempt=count.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
                        BevyError::error(std::io::Error::other(format!("attempt {attempt}"))).with_payload(Box::new(attempt))}
                    _=>BevyError::panic(std::io::Error::other("native panic"),Box::new("payload".to_owned())),
                };
                Err::<Node,_>(error)
            })}
        })).unwrap();
        registry
            .register(WidgetryTreeRenderer::new(|_, _: &File| bsn_list! {Node}))
            .unwrap();
        app.world_mut().insert_resource(registry);
        let mut containers = Vec::new();
        for policy in 0..4 {
            let node = app.world_mut().spawn(Broken(policy)).id();
            containers.push(app.world_mut().spawn(TreeContent { node }).id());
        }
        let healthy = app.world_mut().spawn(File).id();
        let healthy = app.world_mut().spawn(TreeContent { node: healthy }).id();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        let mut previous = None;
        logs.run(|| {
            for attempt in 0..2usize {
                errors.run(|| app.update());
                let mut failures = errors.take();
                assert_eq!(failures.len(), 4);
                let (mut managed, mut native, mut panic_policy) = (0, 0, 0);
                for error in &mut failures {
                    let severity = error.severity();
                    let payload = error.take_payload();
                    let diagnostic = error
                        .downcast_ref::<WidgetryEcsError>()
                        .unwrap()
                        .diagnostics()
                        .next()
                        .unwrap();
                    assert_eq!(
                        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                            .len(),
                        1
                    );
                    assert!(
                        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                            .is_empty()
                    );
                    let frame = diagnostic
                        .contexts()
                        .iter()
                        .find(|frame| {
                            frame.operation() == "tree.renderer"
                                && frame.fields().iter().any(|(key, _)| key == "component")
                        })
                        .unwrap();
                    for key in [
                        "view",
                        "source",
                        "node",
                        "content",
                        "expanded",
                        "children_state",
                        "component",
                        "generation",
                    ] {
                        assert!(frame.fields().iter().any(|(name, _)| name == key));
                    }
                    if severity == bevy::ecs::error::Severity::Panic {
                        assert_eq!(*payload.unwrap().downcast::<String>().unwrap(), "payload");
                        panic_policy += 1;
                    } else if let Some(payload) = payload {
                        assert_eq!(*payload.downcast::<usize>().unwrap(), attempt);
                        assert_ne!(previous.as_ref(), Some(diagnostic.origin().id()));
                        previous = Some(diagnostic.origin().id().clone());
                        let mut cause = diagnostic.source();
                        let mut found = false;
                        while let Some(error) = cause {
                            if let Some(scene) =
                                error.downcast_ref::<bevy_widgetry_core::scene::SceneError>()
                            {
                                assert_eq!(
                                    scene
                                        .native_template_error()
                                        .unwrap()
                                        .downcast_ref::<std::io::Error>()
                                        .unwrap()
                                        .to_string(),
                                    format!("attempt {attempt}")
                                );
                                found = true;
                            }
                            cause = error.source();
                        }
                        assert!(found);
                        native += 1;
                    } else {
                        assert_eq!(severity, bevy::ecs::error::Severity::Error);
                        assert!(
                            origins
                                .lock()
                                .unwrap()
                                .iter()
                                .any(|origin| Arc::ptr_eq(origin, diagnostic.origin()))
                        );
                        managed += 1;
                    }
                }
                assert_eq!((managed, native, panic_policy), (2, 1, 1));
            }
        });
        assert_eq!(
            app.world_mut()
                .query::<&BusinessRecord>()
                .iter(app.world())
                .count(),
            8
        );
        for container in containers {
            assert!(
                app.world()
                    .get::<Children>(container)
                    .is_none_or(|children| children.is_empty())
            );
        }
        assert!(
            app.world()
                .get::<Children>(healthy)
                .is_some_and(|children| !children.is_empty())
        );
    }

    #[test]
    fn structural_faults_retain_identity_until_repaired_and_reregistered_failure_is_new() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut app = bevy_widgetry_test_utils::scene_app();
        let world = app.world_mut();
        world.init_resource::<RendererRegistry>();
        let node = world.spawn(Folder).id();
        let content = world.spawn(TreeContent { node }).id();
        let first = render_content(world).unwrap_err();
        world
            .resource_mut::<RendererRegistry>()
            .register(WidgetryTreeRenderer::new(|_, _: &File| bsn_list! {}))
            .unwrap();
        let same = render_content(world).unwrap_err();
        let get_origin = |error: &BevyError| {
            error
                .downcast_ref::<crate::TreeViewFailure>()
                .unwrap()
                .diagnostics()
                .next()
                .unwrap()
                .origin()
                .clone()
        };
        assert!(Arc::ptr_eq(&get_origin(&first), &get_origin(&same)));
        let diagnostic = first
            .downcast_ref::<crate::TreeViewFailure>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        let mut cause = diagnostic.source();
        let mut found = false;
        while let Some(error) = cause {
            if let Some(TreeError::View {
                fault:
                    TreeViewFault::RendererMissing {
                        content: actual,
                        node: actual_node,
                    },
            }) = error.downcast_ref::<TreeError>()
            {
                assert_eq!((*actual, *actual_node), (content, node));
                found = true;
            }
            cause = error.source();
        }
        assert!(found);
        world
            .resource_mut::<RendererRegistry>()
            .register(WidgetryTreeRenderer::new(|_, _: &Folder| bsn_list! {}))
            .unwrap();
        render_content(world).unwrap();
        world.entity_mut(node).remove::<Folder>();
        let again = render_content(world).unwrap_err();
        assert!(!Arc::ptr_eq(&get_origin(&first), &get_origin(&again)));
    }
    #[derive(Component)]
    struct Attachment(Entity);
    #[test]
    fn failed_renderer_detaches_existing_business_entities_before_removing_new_parents() {
        let _guard = crate::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut app = bevy_widgetry_test_utils::scene_app();
        let world = app.world_mut();
        let business = world.spawn((BusinessRecord, Name::new("before"))).id();
        let node = world.spawn(Attachment(business)).id();
        let content = world.spawn(TreeContent { node }).id();
        let mut registry = RendererRegistry::default();
        registry.register(WidgetryTreeRenderer::new(|_,attachment:&Attachment| {
            let business=attachment.0;
            bsn_list! {Node template(move|ctx|{
                ctx.entity.world_scope(|world|{world.entity_mut(business).insert(Name::new("committed"));});
                ctx.entity.add_child(business);Ok(Visibility::Inherited)
            })--template(|_|Err::<Node,_>(BevyError::error(std::io::Error::other("later sibling"))))}
        })).unwrap();
        world.insert_resource(registry);
        for _ in 0..2 {
            assert!(render_content(world).is_err());
            assert!(world.entities().contains(business));
            assert_eq!(world.get::<Name>(business).unwrap().as_str(), "committed");
            assert!(world.get::<ChildOf>(business).is_none());
            assert!(
                world
                    .get::<Children>(content)
                    .is_none_or(|children| children.is_empty())
            );
        }
    }
}
