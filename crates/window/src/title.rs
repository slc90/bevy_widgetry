use crate::{
    WindowFailure, WindowFault,
    runtime_error::{self, Phase},
};
use bevy::prelude::*;
use bevy_widgetry_core::ui::WidgetryUiSystems;

use crate::{title_bar::bar::TitleBarContent, window_root::WindowRoot};

#[derive(Component, Default, FromTemplate)]
#[require(Text, TextLayout = TextLayout::no_wrap(), Node = title_text_node())]
pub struct WidgetryWindowTitle;

fn title_text_node() -> Node {
    Node {
        min_width: px(0),
        max_width: percent(100),
        overflow: Overflow::clip(),
        ..default()
    }
}

impl WidgetryWindowTitle {
    pub fn set_in_world(
        world: &mut World,
        root: Entity,
        title: impl Into<String>,
    ) -> Result<bool, WindowFailure> {
        let (native, text) = target(world, root)
            .map_err(|fault| runtime_error::failure(world, root, fault, "window.title.set"))?;
        let title = title.into();
        let changed = world
            .get::<Window>(native)
            .is_some_and(|window| window.title != title);
        if changed && let Some(mut window) = world.get_mut::<Window>(native) {
            window.title.clone_from(&title);
        }
        if let Some(text) = text
            && let Some(mut text) = world.get_mut::<Text>(text)
        {
            text.set_if_neq(Text(title));
        }
        Ok(changed)
    }

    pub fn set(commands: &mut Commands, root: Entity, title: impl Into<String>) {
        let title = title.into();
        commands.queue(move |world: &mut World| -> Result {
            Self::set_in_world(world, root, title).map_err(WindowFailure::into_bevy_error)?;
            Ok(())
        });
    }
}

#[derive(Component, Default)]
pub(crate) struct TitleState {
    closing: bool,
}

#[derive(Resource)]
struct PendingTitles(bool);

impl Default for PendingTitles {
    fn default() -> Self {
        Self(true)
    }
}

fn target(world: &mut World, root: Entity) -> Result<(Entity, Option<Entity>), WindowFault> {
    let (native, _) = crate::window_root::validate_binding(world, root, false)?;
    let candidates = world
        .query_filtered::<Entity, With<WidgetryWindowTitle>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut bound = None;
    let mut duplicates = Vec::new();
    for candidate in candidates {
        let mut current = Some(candidate);
        let mut title_content = false;
        while let Some(entity) = current {
            if world.get::<WindowRoot>(entity).is_some() {
                if entity == root {
                    if !title_content {
                        return Err(WindowFault::TitleOutsideBar { title: candidate });
                    }
                    if world.get::<Text>(candidate).is_none() {
                        return Err(WindowFault::MissingPart {
                            entity: candidate,
                            component: "Text",
                        });
                    }
                    if let Some(first) = bound.replace(candidate) {
                        if duplicates.is_empty() {
                            duplicates.push(first);
                        }
                        duplicates.push(candidate);
                    }
                }
                break;
            }
            title_content |= world.get::<TitleBarContent>(entity).is_some();
            current = world.get::<ChildOf>(entity).map(ChildOf::parent);
        }
    }
    if !duplicates.is_empty() {
        duplicates.sort_unstable();
        return Err(WindowFault::AmbiguousTitle { titles: duplicates });
    }
    Ok((native, bound))
}

fn invalidate<T: Component>(_event: On<Insert<T>>, mut pending: ResMut<PendingTitles>) {
    pending.0 = true;
}

fn removed<T: Component>(_event: On<Remove<T>>, mut pending: ResMut<PendingTitles>) {
    pending.0 = true;
}

fn collect(
    windows: Query<(), Changed<Window>>,
    mut closing: MessageReader<bevy::window::WindowClosing>,
    mut roots: Query<(&WindowRoot, &mut TitleState)>,
    mut pending: ResMut<PendingTitles>,
    mut commands: Commands,
) {
    for event in closing.read() {
        for (root, mut state) in &mut roots {
            if root.target_window == event.window {
                state.closing = true;
                pending.0 = true;
            }
        }
    }
    if !pending.0 && windows.is_empty() {
        return;
    }
    pending.0 = false;
    commands.queue(refresh);
}

fn refresh(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, With<WindowRoot>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut errors = Vec::new();
    for root in roots {
        if world
            .get::<TitleState>(root)
            .is_some_and(|state| state.closing)
        {
            continue;
        }
        let result = if world.get::<TitleState>(root).is_none() {
            Err(WindowFault::MissingPart {
                entity: root,
                component: "TitleState",
            })
        } else {
            target(world, root).map(|(native, text)| {
                if let Some(text) = text {
                    let title = world
                        .get::<Window>(native)
                        .map(|window| window.title.clone());
                    if let Some(title) = title
                        && let Some(mut text) = world.get_mut::<Text>(text)
                    {
                        text.set_if_neq(Text(title));
                    }
                }
            })
        };
        if let Err(error) =
            runtime_error::observe(world, root, Phase::Title, result, "window.title.sync")
        {
            world.resource_mut::<PendingTitles>().0 = true;
            errors.push(error);
        }
    }
    runtime_error::finish(errors)
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingTitles>()
        .add_message::<bevy::window::WindowClosing>()
        .add_observer(invalidate::<WidgetryWindowTitle>)
        .add_observer(invalidate::<TitleState>)
        .add_observer(removed::<TitleState>)
        .add_observer(removed::<WidgetryWindowTitle>)
        .add_observer(invalidate::<ChildOf>)
        .add_observer(removed::<ChildOf>)
        .add_observer(invalidate::<Text>)
        .add_observer(removed::<Text>)
        .add_observer(invalidate::<WindowRoot>)
        .add_observer(removed::<WindowRoot>)
        .add_observer(removed::<Window>)
        .add_observer(invalidate::<TitleBarContent>)
        .add_observer(removed::<TitleBarContent>)
        .add_systems(
            PostUpdate,
            collect
                .in_set(WidgetryUiSystems::Build)
                .after(crate::window_root::initialize_windows),
        );
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{WindowError, prepare_native_window};
    use bevy::ecs::{error::Severity, schedule::SingleThreadedExecutor};
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};

    #[test]
    fn native_removal_invalidates_settled_title_projection() {
        for despawn in [false, true] {
            let mut app = App::new();
            install(&mut app);
            app.init_resource::<runtime_error::WindowDiagnostics>();
            app.set_error_handler(ErrorCapture::handler());
            app.edit_schedule(PostUpdate, |schedule| {
                schedule.set_executor(SingleThreadedExecutor::new());
            });
            let native = app
                .world_mut()
                .spawn(prepare_native_window(Window::default()))
                .id();
            let camera = app.world_mut().spawn(Camera2d).id();
            let root = app
                .world_mut()
                .spawn((
                    WindowRoot {
                        target_window: native,
                        maximized: false,
                    },
                    UiTargetCamera(camera),
                ))
                .id();
            app.update();
            assert!(!app.world().resource::<PendingTitles>().0);

            if despawn {
                app.world_mut().despawn(native);
            } else {
                app.world_mut().entity_mut(native).remove::<Window>();
            }
            let errors = ErrorCapture::default();
            let logs = LogCapture::default();
            logs.run(|| errors.run(|| app.update()));
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].error.severity(), Severity::Error);
            let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
            assert!(matches!(
                diagnostic.source().unwrap().downcast_ref::<WindowError>(),
                Some(WindowError::Structure {
                    fault: WindowFault::MissingPart { entity, component: "Window" }
                }) if *entity == native
            ));
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(app.world().entities().contains(root));
            assert!(app.world().entities().contains(camera));
        }
    }
}
