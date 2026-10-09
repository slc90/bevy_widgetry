use bevy::prelude::*;
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ResolvedForeground(pub Color);

impl Default for ResolvedForeground {
    fn default() -> Self {
        Self(Color::NONE)
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct InheritedForeground {
    pub source: bevy::prelude::Entity,
    pub color: Color,
    pub disabled: bool,
}

#[derive(Clone, Copy, Default, PartialEq)]
struct NodeContext {
    foreground: Option<InheritedForeground>,
    disabled: bool,
    ui_text: bool,
}
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct ContentContext {
    pub(crate) disabled: bool,
    pub(crate) ui_text: bool,
}
#[derive(Resource, Default)]
struct ForegroundRuntime {
    context: bevy::ecs::entity::EntityHashMap<NodeContext>,
    dirty: bevy::ecs::entity::EntityHashSet,
    #[cfg(test)]
    visits: usize,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<ForegroundRuntime>()
        .add_observer(
            |event: On<
                Remove,
                (
                    ResolvedForeground,
                    ChildOf,
                    crate::text::WidgetryText,
                    crate::icon::WidgetryIcon,
                    bevy::ui::InteractionDisabled,
                    Text,
                    Text2d,
                ),
            >,
             mut runtime: ResMut<ForegroundRuntime>| {
                runtime.dirty.insert(event.entity);
            },
        )
        .add_observer(
            |event: On<Despawn>, mut runtime: ResMut<ForegroundRuntime>| {
                runtime.context.remove(&event.entity);
                runtime.dirty.remove(&event.entity);
            },
        )
        .add_systems(
            PostUpdate,
            resolve_foreground.in_set(crate::ui::WidgetryUiSystems::Foreground),
        );
}

fn resolve_foreground(world: &mut bevy::prelude::World) {
    use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
    use bevy::prelude::*;
    let mut dirty = EntityHashSet::default();
    let mut query = world.query_filtered::<Entity, Or<(
        Changed<ResolvedForeground>,
        Changed<crate::disabled::WidgetryEffectiveDisabled>,
        Changed<ChildOf>,
        Added<crate::text::WidgetryText>,
        Added<crate::icon::WidgetryIcon>,
        Added<Node>,
        Added<bevy::ui::InteractionDisabled>,
        Added<Text>,
        Added<Text2d>,
        Added<TextSpan>,
    )>>();
    dirty.extend(query.iter(world));
    world.resource_scope(|world, mut runtime: Mut<ForegroundRuntime>| {
        #[cfg(test)]
        {
            runtime.visits = 0;
        }
        dirty.extend(std::mem::take(&mut runtime.dirty));
        let mut dirty_descendants = EntityHashSet::default();
        for &entity in &dirty {
            let mut cursor = world.get::<ChildOf>(entity).map(ChildOf::parent);
            while let Some(parent) = cursor {
                if !dirty_descendants.insert(parent) {
                    break;
                }
                cursor = world.get::<ChildOf>(parent).map(ChildOf::parent);
            }
        }
        let mut covered = EntityHashMap::<bool>::default();
        let mut roots = Vec::new();
        for &entity in &dirty {
            let mut path = Vec::new();
            let mut cursor = world.get::<ChildOf>(entity).map(ChildOf::parent);
            let mut inherited_dirty = false;
            while let Some(parent) = cursor {
                if dirty.contains(&parent) {
                    inherited_dirty = true;
                    break;
                }
                if let Some(&value) = covered.get(&parent) {
                    inherited_dirty = value;
                    break;
                }
                path.push(parent);
                cursor = world.get::<ChildOf>(parent).map(ChildOf::parent);
            }
            for parent in path {
                covered.insert(parent, inherited_dirty);
            }
            if !inherited_dirty {
                roots.push(entity);
            }
        }
        let mut stack = Vec::new();
        for entity in roots {
            let context = world
                .get::<ChildOf>(entity)
                .and_then(|parent| runtime.context.get(&parent.parent()).copied());
            stack.push((entity, context.unwrap_or_default()));
        }
        while let Some((entity, parent_context)) = stack.pop() {
            if !world.entities().contains(entity) {
                continue;
            }
            #[cfg(test)]
            {
                runtime.visits += 1;
            }
            let disabled = world
                .get::<crate::disabled::WidgetryEffectiveDisabled>(entity)
                .map(|d| d.is_disabled())
                .unwrap_or(
                    parent_context.disabled
                        || world.get::<bevy::ui::InteractionDisabled>(entity).is_some(),
                );
            let ui_text = if world.get::<Text>(entity).is_some() {
                true
            } else if world.get::<Text2d>(entity).is_some() {
                false
            } else {
                parent_context.ui_text
            };
            let context = world
                .get::<ResolvedForeground>(entity)
                .map(|c| InheritedForeground {
                    source: entity,
                    color: c.0,
                    disabled,
                })
                .or(parent_context.foreground);
            let node_context = NodeContext {
                foreground: context,
                disabled,
                ui_text,
            };
            let unchanged = runtime.context.get(&entity) == Some(&node_context);
            runtime.context.insert(entity, node_context);
            let content = world.get::<crate::text::WidgetryText>(entity).is_some()
                || world.get::<crate::icon::WidgetryIcon>(entity).is_some();
            let previous = world.get::<InheritedForeground>(entity).copied();
            let content_context = ContentContext { disabled, ui_text };
            if content && world.get::<ContentContext>(entity) != Some(&content_context) {
                world.entity_mut(entity).insert(content_context);
            } else if !content && world.get::<ContentContext>(entity).is_some() {
                world.entity_mut(entity).remove::<ContentContext>();
            }
            if content && previous != context {
                match context {
                    Some(c) => {
                        world.entity_mut(entity).insert(c);
                    }
                    None => {
                        world.entity_mut(entity).remove::<InheritedForeground>();
                    }
                }
            } else if !content && previous.is_some() {
                world.entity_mut(entity).remove::<InheritedForeground>();
            }
            if unchanged && !dirty_descendants.contains(&entity) {
                continue;
            }
            if let Some(children) = world.get::<Children>(entity) {
                stack.extend(children.iter().map(|child| {
                    (
                        child,
                        NodeContext {
                            foreground: context,
                            disabled,
                            ui_text,
                        },
                    )
                }));
            }
        }
    });
}

// 测试通过断言保护遍历工作量，仅在本 module 允许测试所需的 panic lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    //! State：100/1,000/10,000 物化节点的浅树、深树、稳定帧与祖先颜色变化。
    //! Invariants：稳定帧不遍历 hierarchy，祖先变化各访问一次，局部变化不扫描无关 sibling。
    use super::*;

    #[test]
    fn affected_tree_visits_are_linear_and_idle_visits_are_zero() {
        for count in [100, 1000, 10000] {
            for deep in [false, true] {
                let mut app = App::new();
                app.add_plugins(crate::ui::WidgetryUiPlugin);
                let root = app
                    .world_mut()
                    .spawn((Node::default(), ResolvedForeground(Color::WHITE)))
                    .id();
                let mut leaf = root;
                for _ in 1..count {
                    let parent = if deep { leaf } else { root };
                    leaf = app
                        .world_mut()
                        .spawn((Node::default(), ChildOf(parent)))
                        .id();
                }
                app.update();
                assert_eq!(app.world().resource::<ForegroundRuntime>().visits, count);
                app.update();
                assert_eq!(app.world().resource::<ForegroundRuntime>().visits, 0);
                app.world_mut()
                    .entity_mut(root)
                    .insert(ResolvedForeground(Color::BLACK));
                app.update();
                assert_eq!(app.world().resource::<ForegroundRuntime>().visits, count);
                app.world_mut()
                    .entity_mut(leaf)
                    .insert(ResolvedForeground(Color::NONE));
                app.update();
                assert_eq!(app.world().resource::<ForegroundRuntime>().visits, 1);
                println!(
                    "foreground-work nodes={count} deep={deep} ancestor_visits={count} local_visits=1 idle_visits=0"
                );
            }
        }
    }
}
