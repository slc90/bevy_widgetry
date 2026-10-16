use crate::{
    WidgetryMessageBox,
    scene::{MessageBoxAction, MessageBoxBody},
};
use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::{
    color::WidgetryStyleOwner, diagnostics::FailureState, foreground::ResolvedForeground,
};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};
use bevy_widgetry_window::internal::WindowRoot;

#[cfg(test)]
thread_local! {
    static OWNER_WORK: std::cell::RefCell<Vec<Entity>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[derive(Resource, Default)]
struct OwnerChanges {
    pending: EntityHashSet,
    failures: EntityHashMap<FailureState>,
}

#[derive(Component)]
struct ActionOwner(Entity);

#[derive(Component)]
struct WindowOwnership;

type Subjects = Or<(
    With<WidgetryMessageBox>,
    With<MessageBoxAction>,
    With<ActionOwner>,
    With<WindowOwnership>,
)>;
type Actions = Or<(With<MessageBoxAction>, With<ActionOwner>)>;

fn queue_related(
    entity: Entity,
    subjects: &Query<(), Subjects>,
    actions: &Query<(Entity, Option<&ActionOwner>), Actions>,
    parents: &Query<&ChildOf>,
    changes: &mut OwnerChanges,
) {
    if subjects.contains(entity) {
        changes.pending.insert(entity);
    }
    for (action, old) in actions {
        if old.is_some_and(|old| old.0 == entity)
            || parents
                .iter_ancestors(action)
                .any(|ancestor| ancestor == entity)
        {
            changes.pending.insert(action);
        }
    }
}

fn owner_inserted(
    event: On<
        Insert<(
            WidgetryMessageBox,
            MessageBoxAction,
            WidgetryButton,
            WindowRoot,
            ChildOf,
        )>,
    >,
    subjects: Query<(), Subjects>,
    actions: Query<(Entity, Option<&ActionOwner>), Actions>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<OwnerChanges>,
) {
    queue_related(event.entity, &subjects, &actions, &parents, &mut changes);
}

fn owner_removed(
    event: On<
        Remove<(
            WidgetryMessageBox,
            MessageBoxAction,
            WidgetryButton,
            WindowRoot,
            ChildOf,
            WidgetryStyleOwner<WindowRoot>,
            WidgetryStyleOwner<WidgetryButton>,
        )>,
    >,
    subjects: Query<(), Subjects>,
    actions: Query<(Entity, Option<&ActionOwner>), Actions>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<OwnerChanges>,
) {
    queue_related(event.entity, &subjects, &actions, &parents, &mut changes);
}

pub(crate) fn install_owners(app: &mut App) {
    app.init_resource::<OwnerChanges>()
        .add_observer(own_window)
        .add_observer(own_action)
        .add_observer(owner_inserted)
        .add_observer(owner_removed);
    let world = app.world_mut();
    let existing = world
        .query_filtered::<Entity, Or<(With<WidgetryMessageBox>, With<MessageBoxAction>)>>()
        .iter(world)
        .collect::<Vec<_>>();
    world
        .resource_mut::<OwnerChanges>()
        .pending
        .extend(existing);
}

fn action_root(world: &World, action: Entity) -> Option<Entity> {
    let mut current = world.get::<ChildOf>(action).map(ChildOf::parent);
    while let Some(entity) = current {
        if world.get::<WidgetryMessageBox>(entity).is_some() {
            return Some(entity);
        }
        if world.get::<WindowRoot>(entity).is_some() {
            break;
        }
        current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    None
}

fn reconcile_owner(world: &mut World, entity: Entity) -> Result {
    if world.get::<WidgetryMessageBox>(entity).is_some() {
        #[cfg(test)]
        OWNER_WORK.with_borrow_mut(|work| work.push(entity));
        if world.get::<WindowRoot>(entity).is_none() {
            return Err(BevyError::error("MessageBox 缺失 WindowRoot"));
        }
        if world
            .get::<WidgetryStyleOwner<WindowRoot>>(entity)
            .is_none_or(|owner| owner.entity != entity)
        {
            world
                .entity_mut(entity)
                .insert(WidgetryStyleOwner::<WindowRoot>::new(entity));
        }
        if world.get::<WindowOwnership>(entity).is_none() {
            world.entity_mut(entity).insert(WindowOwnership);
        }
    } else if world.get::<WindowOwnership>(entity).is_some() {
        if world
            .get::<WidgetryStyleOwner<WindowRoot>>(entity)
            .is_some_and(|owner| owner.entity == entity)
        {
            world
                .entity_mut(entity)
                .remove::<WidgetryStyleOwner<WindowRoot>>();
        }
        world.entity_mut(entity).remove::<WindowOwnership>();
    }
    let root = world
        .get::<MessageBoxAction>(entity)
        .and_then(|_| action_root(world, entity));
    if let Some(root) = root {
        #[cfg(test)]
        OWNER_WORK.with_borrow_mut(|work| work.push(entity));
        if world.get::<WidgetryButton>(entity).is_none() {
            return Err(BevyError::error("MessageBox 结果按钮缺失 WidgetryButton"));
        }
        if world
            .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
            .is_none_or(|owner| owner.entity != root)
        {
            world
                .entity_mut(entity)
                .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
        }
        if world
            .get::<ActionOwner>(entity)
            .is_none_or(|owner| owner.0 != root)
        {
            world.entity_mut(entity).insert(ActionOwner(root));
        }
    } else if let Some(old) = world.get::<ActionOwner>(entity).map(|owner| owner.0) {
        if world
            .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
            .is_some_and(|owner| owner.entity == old)
        {
            world
                .entity_mut(entity)
                .remove::<WidgetryStyleOwner<WidgetryButton>>();
        }
        world.entity_mut(entity).remove::<ActionOwner>();
    }
    Ok(())
}

pub(crate) fn establish_owners(world: &mut World) -> Result {
    let pending = std::mem::take(&mut world.resource_mut::<OwnerChanges>().pending);
    let mut first_error = None;
    for entity in pending {
        if !world.entities().contains(entity) {
            world
                .resource_mut::<OwnerChanges>()
                .failures
                .remove(&entity);
            continue;
        }
        let result = reconcile_owner(world, entity);
        let mut changes = world.resource_mut::<OwnerChanges>();
        let result = if result.is_err() || changes.failures.contains_key(&entity) {
            changes.failures.entry(entity).or_default().observe(
                result,
                |error| widgetry_error!(?entity, %error, "MessageBox 样式归属更新失败"),
                || widgetry_info!(?entity, "MessageBox 样式归属更新恢复"),
            )
        } else {
            result
        };
        match result {
            Ok(()) => {
                changes.failures.remove(&entity);
            }
            Err(error) => {
                changes.pending.insert(entity);
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}
pub fn apply_owned_message_box_colors(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryMessageBoxColors,
) -> Result<(), BevyError> {
    bevy_widgetry_window::internal::apply_owned_window_colors(world, root, &colors.window)?;
    let mut stack = world
        .get::<Children>(root)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    while let Some(entity) = stack.pop() {
        let disabled = world.get::<InteractionDisabled>(entity).is_some();
        if world.get::<MessageBoxBody>(entity).is_some() {
            let state = if disabled {
                colors.body.disabled
            } else {
                colors.body.normal
            };
            world
                .get_mut::<BackgroundColor>(entity)
                .ok_or_else(|| failure(entity))?
                .set_if_neq(BackgroundColor(state.background));
            world
                .get_mut::<ResolvedForeground>(entity)
                .ok_or_else(|| failure(entity))?
                .set_if_neq(ResolvedForeground(state.foreground));
        } else if world.get::<MessageBoxAction>(entity).is_some() {
            let state = if disabled {
                colors.action_button.disabled
            } else if world.get::<Pressed>(entity).is_some() {
                colors.action_button.pressed
            } else if world.get::<Hovered>(entity).is_some_and(|h| h.0) {
                colors.action_button.hovered
            } else {
                colors.action_button.normal
            };
            bevy_widgetry_button::internal::apply_owned_button_colors(world, entity, state)?;
        }
        if let Some(c) = world.get::<Children>(entity) {
            stack.extend(c.iter());
        }
    }
    Ok(())
}
fn failure(entity: Entity) -> BevyError {
    bevy_widgetry_internal_log::widgetry_error!(?entity, "MessageBox 缺失颜色输出");
    BevyError::error("MessageBox 缺失颜色输出")
}
pub(crate) fn update_colors(world: &mut World) -> Result<(), BevyError> {
    let theme = world.resource::<WidgetryThemeMode>().colors().message_box;
    let roots = world
        .query_filtered::<Entity, (
            With<WidgetryMessageBox>,
            Without<WidgetryStyleOwner<WidgetryMessageBox>>,
        )>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let colors = crate::WidgetryMessageBoxColorOverrides::get(world, root)?.resolve(&theme);
        apply_owned_message_box_colors(world, root, &colors)?;
    }
    Ok(())
}
pub(crate) fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(update_colors);
}

pub(crate) fn own_window(event: On<Add<WidgetryMessageBox>>, mut commands: Commands) {
    commands.entity(event.entity).insert((
        WidgetryStyleOwner::<WindowRoot>::new(event.entity),
        WindowOwnership,
    ));
}
pub(crate) fn own_action(
    event: On<Add<MessageBoxAction>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryMessageBox>>,
    mut commands: Commands,
) {
    if let Some(root) = parents
        .iter_ancestors(event.entity)
        .find(|root| roots.contains(*root))
    {
        commands.entity(event.entity).insert((
            WidgetryStyleOwner::<WidgetryButton>::new(root),
            ActionOwner(root),
        ));
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{WidgetryMessageBoxButtons, WidgetryMessageBoxPlugin, widgetry_message_box};
    use bevy_widgetry_test_utils::scene_app;
    use bevy_widgetry_window::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, owned_widgetry_window,
        widgetry_window_target,
    };

    fn fixture() -> (App, Vec<Entity>) {
        let mut app = scene_app();
        app.add_plugins(WidgetryMessageBoxPlugin);
        let parent = app
            .world_mut()
            .spawn_scene(bsn! {
                @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(),
                    WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{})
            })
            .unwrap()
            .id();
        let native = widgetry_window_target(app.world(), parent).unwrap();
        let mut roots = Vec::new();
        for buttons in [
            WidgetryMessageBoxButtons::Ok,
            WidgetryMessageBoxButtons::YesNo,
            WidgetryMessageBoxButtons::YesNoCancel,
        ] {
            roots.push(
                app.world_mut()
                    .spawn_scene(bsn! {
                        @widgetry_message_box(native, "Question", buttons, Default::default(),
                            bsn_list! {@WidgetryButton Children [Text("Business button")]})
                    })
                    .unwrap()
                    .id(),
            );
        }
        for _ in 0..4 {
            app.update();
        }
        OWNER_WORK.with_borrow_mut(Vec::clear);
        (app, roots)
    }

    #[test]
    fn settled_message_boxes_do_not_repeat_owner_discovery() {
        let (mut app, _) = fixture();
        for _ in 0..3 {
            app.update();
        }
        OWNER_WORK.with_borrow(|work| assert_eq!(work.len(), 0));
    }

    fn take_work() -> Vec<Entity> {
        OWNER_WORK.with_borrow_mut(std::mem::take)
    }

    fn belongs(world: &World, entity: Entity, root: Entity) -> bool {
        let mut current = world.get::<ChildOf>(entity).map(ChildOf::parent);
        while let Some(entity) = current {
            if entity == root {
                return true;
            }
            current = world.get::<ChildOf>(entity).map(ChildOf::parent);
        }
        false
    }

    fn actions(app: &mut App, root: Entity) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<MessageBoxAction>>()
            .iter(app.world())
            .filter(|entity| belongs(app.world(), *entity, root))
            .collect()
    }

    fn body(app: &mut App, root: Entity) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<MessageBoxBody>>()
            .iter(app.world())
            .find(|entity| belongs(app.world(), *entity, root))
            .unwrap()
    }

    #[test]
    fn result_combinations_own_only_their_window_and_actions() {
        let (mut app, roots) = fixture();
        for (root, count) in roots.into_iter().zip([1, 2, 3]) {
            assert_eq!(
                app.world()
                    .get::<WidgetryStyleOwner<WindowRoot>>(root)
                    .unwrap()
                    .entity,
                root
            );
            let buttons = actions(&mut app, root);
            assert_eq!(buttons.len(), count);
            for action in buttons {
                assert_eq!(
                    app.world()
                        .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                        .unwrap()
                        .entity,
                    root
                );
            }
        }
        let business = app
            .world_mut()
            .query_filtered::<Entity, (With<WidgetryButton>, Without<MessageBoxAction>)>()
            .iter(app.world())
            .collect::<Vec<_>>();
        assert_eq!(business.len(), 3);
        for button in business {
            assert!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .is_none()
            );
        }
    }

    #[test]
    fn delayed_actions_and_moving_containers_reconcile_only_affected_buttons() {
        let (mut app, roots) = fixture();
        let action = app.world_mut().spawn_scene(bsn! {
            @WidgetryButton template(|_| Ok(MessageBoxAction(crate::WidgetryMessageBoxResult::Ok)))
        }).unwrap().id();
        app.update();
        assert!(
            !app.world()
                .resource::<OwnerChanges>()
                .pending
                .contains(&action)
        );
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                .is_none()
        );
        let body_a = body(&mut app, roots[0]);
        let body_b = body(&mut app, roots[1]);
        let container = app
            .world_mut()
            .spawn_scene(bsn! {Node ChildOf(body_a)})
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(action)
            .insert(ChildOf(container));
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                .unwrap()
                .entity,
            roots[0]
        );
        take_work();
        app.world_mut()
            .entity_mut(container)
            .insert(ChildOf(body_b));
        app.update();
        assert_eq!(take_work(), vec![action]);
        assert_eq!(app.world().get::<ActionOwner>(action).unwrap().0, roots[1]);
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                .unwrap()
                .entity,
            roots[1]
        );
        app.world_mut().entity_mut(container).remove::<ChildOf>();
        app.update();
        assert!(app.world().get::<ActionOwner>(action).is_none());
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                .is_none()
        );
        app.update();
        take_work();
        app.update();
        assert!(take_work().is_empty());
    }

    #[test]
    fn removed_owners_are_restored_and_marker_removal_releases_them() {
        let (mut app, roots) = fixture();
        let root = roots[0];
        let action = actions(&mut app, root)[0];
        app.world_mut()
            .entity_mut(action)
            .remove::<WidgetryStyleOwner<WidgetryButton>>();
        app.update();
        assert_eq!(take_work(), vec![action]);
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                .unwrap()
                .entity,
            root
        );
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryStyleOwner<WindowRoot>>();
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WindowRoot>>(root)
                .unwrap()
                .entity,
            root
        );
        take_work();
        app.world_mut()
            .entity_mut(action)
            .remove::<MessageBoxAction>();
        app.update();
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                .is_none()
        );
        assert!(app.world().get::<ActionOwner>(action).is_none());
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryMessageBox>();
        app.update();
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WindowRoot>>(root)
                .is_none()
        );
        app.world_mut().entity_mut(root).insert(WidgetryMessageBox);
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WindowRoot>>(root)
                .unwrap()
                .entity,
            root
        );
    }

    #[test]
    fn losing_the_root_marker_releases_actions_and_despawn_clears_pending_work() {
        let (mut app, roots) = fixture();
        let root = roots[2];
        let buttons = actions(&mut app, root);
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryMessageBox>();
        app.update();
        for &action in &buttons {
            assert!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                    .is_none()
            );
            assert!(app.world().get::<ActionOwner>(action).is_none());
        }
        app.world_mut().entity_mut(root).insert(WidgetryMessageBox);
        app.update();
        for &action in &buttons {
            assert_eq!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                    .unwrap()
                    .entity,
                root
            );
        }
        app.world_mut().despawn(root);
        for _ in 0..2 {
            app.update();
        }
        let changes = app.world().resource::<OwnerChanges>();
        assert!(
            changes
                .pending
                .iter()
                .all(|entity| app.world().entities().contains(*entity))
        );
        assert!(changes.failures.is_empty());
        take_work();
        app.update();
        assert!(take_work().is_empty());
    }

    #[test]
    fn installing_message_box_plugin_after_construction_bootstraps_once() {
        let mut app = scene_app();
        app.add_plugins((
            bevy_widgetry_window::WidgetryWindowPlugin,
            bevy_widgetry_button::WidgetryButtonPlugin,
        ));
        let parent = app
            .world_mut()
            .spawn_scene(bsn! {
                @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(),
                    WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{})
            })
            .unwrap()
            .id();
        let native = widgetry_window_target(app.world(), parent).unwrap();
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @widgetry_message_box(native, "Question", WidgetryMessageBoxButtons::YesNo,
                    Default::default(), bsn_list!{})
            })
            .unwrap()
            .id();
        app.update();
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WindowRoot>>(root)
                .is_none()
        );
        app.add_plugins(WidgetryMessageBoxPlugin);
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WindowRoot>>(root)
                .unwrap()
                .entity,
            root
        );
        for action in actions(&mut app, root) {
            assert_eq!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                    .unwrap()
                    .entity,
                root
            );
        }
        app.update();
        take_work();
        app.update();
        assert!(take_work().is_empty());
    }

    #[test]
    fn missing_managed_button_retries_and_reports_one_fault_until_recovery() {
        use bevy::ecs::schedule::SingleThreadedExecutor;
        use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
        let logs = LogCapture::default();
        let (mut app, roots) = logs.run(fixture);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let action = actions(&mut app, roots[0])[0];
        app.world_mut()
            .entity_mut(action)
            .remove::<WidgetryButton>();
        let errors = ErrorCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        let failures = errors.take();
        assert_eq!(failures.len(), 2);
        assert!(
            failures
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
                .count(),
            1
        );
        app.world_mut()
            .entity_mut(action)
            .apply_scene(bsn! {@WidgetryButton})
            .unwrap();
        logs.run(|| errors.run(|| app.update()));
        assert!(errors.take().is_empty());
        assert!(app.world().resource::<OwnerChanges>().failures.is_empty());
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record
                    .fields
                    .get("message")
                    .is_some_and(|message| message.contains("样式归属更新恢复")))
                .count(),
            1
        );
    }
}
