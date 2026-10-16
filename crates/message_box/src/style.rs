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
    static COLOR_WORK: std::cell::RefCell<(Vec<Entity>, Vec<Entity>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

#[derive(Resource, Default)]
struct OwnerChanges {
    pending: EntityHashSet,
    failures: EntityHashMap<FailureState>,
}

#[derive(Component)]
struct PartOwner(Entity);

#[derive(Component)]
struct WindowOwnership;

type Subjects = Or<(
    With<WidgetryMessageBox>,
    With<MessageBoxAction>,
    With<MessageBoxBody>,
    With<PartOwner>,
    With<WindowOwnership>,
)>;
type Parts = Or<(
    With<MessageBoxAction>,
    With<MessageBoxBody>,
    With<PartOwner>,
)>;

fn queue_related(
    entity: Entity,
    subjects: &Query<(), Subjects>,
    parts: &Query<(Entity, Option<&PartOwner>), Parts>,
    parents: &Query<&ChildOf>,
    changes: &mut OwnerChanges,
) {
    if subjects.contains(entity) {
        changes.pending.insert(entity);
    }
    for (part, old) in parts {
        if old.is_some_and(|old| old.0 == entity)
            || parents
                .iter_ancestors(part)
                .any(|ancestor| ancestor == entity)
        {
            changes.pending.insert(part);
        }
    }
}

fn owner_inserted(
    event: On<
        Insert<(
            WidgetryMessageBox,
            MessageBoxAction,
            MessageBoxBody,
            WidgetryButton,
            WindowRoot,
            ChildOf,
        )>,
    >,
    subjects: Query<(), Subjects>,
    parts: Query<(Entity, Option<&PartOwner>), Parts>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<OwnerChanges>,
) {
    queue_related(event.entity, &subjects, &parts, &parents, &mut changes);
}

fn owner_removed(
    event: On<
        Remove<(
            WidgetryMessageBox,
            MessageBoxAction,
            MessageBoxBody,
            WidgetryButton,
            WindowRoot,
            ChildOf,
            WidgetryStyleOwner<WindowRoot>,
            WidgetryStyleOwner<WidgetryButton>,
        )>,
    >,
    subjects: Query<(), Subjects>,
    parts: Query<(Entity, Option<&PartOwner>), Parts>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<OwnerChanges>,
) {
    queue_related(event.entity, &subjects, &parts, &parents, &mut changes);
}

pub(crate) fn install_owners(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (establish_owners, collect_colors)
            .chain()
            .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::StyleOwners),
    )
    .init_resource::<OwnerChanges>()
    .add_observer(own_window)
    .add_observer(own_action)
    .add_observer(owner_inserted)
    .add_observer(owner_removed)
    .add_observer(color_removed)
    .add_observer(color_part_removed);
    let world = app.world_mut();
    let existing = world
        .query_filtered::<Entity, Or<(
            With<WidgetryMessageBox>,
            With<MessageBoxAction>,
            With<MessageBoxBody>,
        )>>()
        .iter(world)
        .collect::<Vec<_>>();
    world
        .resource_mut::<OwnerChanges>()
        .pending
        .extend(existing);
}

fn part_root(world: &World, part: Entity) -> Option<Entity> {
    let mut current = world.get::<ChildOf>(part).map(ChildOf::parent);
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
    let action = world.get::<MessageBoxAction>(entity).is_some();
    let root = (action || world.get::<MessageBoxBody>(entity).is_some())
        .then(|| part_root(world, entity))
        .flatten();
    if let Some(root) = root {
        #[cfg(test)]
        if action {
            OWNER_WORK.with_borrow_mut(|work| work.push(entity));
        }
        if action && world.get::<WidgetryButton>(entity).is_none() {
            return Err(BevyError::error("MessageBox 结果按钮缺失 WidgetryButton"));
        }
        if action
            && world
                .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                .is_none_or(|owner| owner.entity != root)
        {
            world
                .entity_mut(entity)
                .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
        }
        if world
            .get::<PartOwner>(entity)
            .is_none_or(|owner| owner.0 != root)
        {
            world.entity_mut(entity).insert(PartOwner(root));
        }
    } else if let Some(old) = world.get::<PartOwner>(entity).map(|owner| owner.0) {
        if world
            .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
            .is_some_and(|owner| owner.entity == old)
        {
            world
                .entity_mut(entity)
                .remove::<WidgetryStyleOwner<WidgetryButton>>();
        }
        world.entity_mut(entity).remove::<PartOwner>();
    }
    Ok(())
}

fn establish_owners(world: &mut World) -> Result {
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
#[derive(Component)]
pub(crate) struct MessageBoxStyle {
    colors: Option<bevy_widgetry_theme::WidgetryMessageBoxColors>,
    palette: bool,
    full: bool,
    dirty: EntityHashSet,
    parts: EntityHashSet,
    failure: FailureState,
}

impl Default for MessageBoxStyle {
    fn default() -> Self {
        Self {
            colors: None,
            palette: true,
            full: true,
            dirty: default(),
            parts: default(),
            failure: default(),
        }
    }
}

fn collect_colors(
    inputs: Query<
        (Entity, &PartOwner),
        Or<(
            Changed<PartOwner>,
            Changed<Hovered>,
            Added<Pressed>,
            Added<InteractionDisabled>,
            Added<BackgroundColor>,
            Added<BorderColor>,
            Added<ResolvedForeground>,
        )>,
    >,
    palettes: Query<
        Entity,
        Or<(
            Changed<crate::colors::ColorState>,
            Added<WidgetryMessageBox>,
            Changed<WidgetryStyleOwner<WindowRoot>>,
        )>,
    >,
    owners: Query<Entity, Changed<WidgetryStyleOwner<WidgetryMessageBox>>>,
    mut removed_owners: RemovedComponents<WidgetryStyleOwner<WidgetryMessageBox>>,
    theme: Res<WidgetryThemeMode>,
    mut styles: Query<&mut MessageBoxStyle>,
) {
    if theme.is_changed() {
        for mut style in &mut styles {
            style.palette = true;
        }
    }
    for root in &palettes {
        if let Ok(mut style) = styles.get_mut(root) {
            style.palette = true;
        }
    }
    for root in owners.iter().chain(removed_owners.read()) {
        if let Ok(mut style) = styles.get_mut(root) {
            style.colors = None;
            style.palette = true;
            style.full = true;
        }
    }
    for (entity, owner) in &inputs {
        if let Ok(mut style) = styles.get_mut(owner.0) {
            style.parts.insert(entity);
            style.dirty.insert(entity);
        }
    }
}

fn color_removed(
    event: On<
        Remove<(
            Hovered,
            Pressed,
            InteractionDisabled,
            BackgroundColor,
            BorderColor,
            ResolvedForeground,
        )>,
    >,
    parts: Query<&PartOwner>,
    mut styles: Query<&mut MessageBoxStyle>,
) {
    if let Ok(owner) = parts.get(event.entity)
        && let Ok(mut style) = styles.get_mut(owner.0)
    {
        style.dirty.insert(event.entity);
    }
}

fn apply_part(
    world: &mut World,
    entity: Entity,
    colors: &bevy_widgetry_theme::WidgetryMessageBoxColors,
) -> Result {
    #[cfg(test)]
    COLOR_WORK.with_borrow_mut(|work| work.1.push(entity));
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
    Ok(())
}

pub fn apply_owned_message_box_colors(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryMessageBoxColors,
) -> Result {
    let Some(mut style) = world.get_mut::<MessageBoxStyle>(root) else {
        widgetry_error!(?root, "MessageBox 缺失配色派生状态");
        return Err(failure(root));
    };
    if style.colors != Some(*colors) {
        style.colors = Some(*colors);
        style.full = true;
    }
    bevy_widgetry_window::internal::apply_owned_window_colors(world, root, &colors.window)?;
    apply_pending_colors(world, root)
}

fn apply_pending_colors(world: &mut World, root: Entity) -> Result {
    let Some(style) = world.get::<MessageBoxStyle>(root) else {
        return Err(failure(root));
    };
    let Some(colors) = style.colors else {
        return Ok(());
    };
    if !style.full && style.dirty.is_empty() {
        return Ok(());
    }
    let targets = if style.full {
        style.parts.iter().copied().collect::<Vec<_>>()
    } else {
        style.dirty.iter().copied().collect()
    };
    let mut first_error = None;
    for entity in targets {
        if world
            .get::<PartOwner>(entity)
            .is_none_or(|owner| owner.0 != root)
        {
            continue;
        }
        if let Err(error) = apply_part(world, entity, &colors)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    if let Some(mut style) = world.get_mut::<MessageBoxStyle>(root) {
        let result = style.failure.observe(
            first_error.map_or(Ok(()), Err),
            |error| widgetry_error!(?root, %error, "MessageBox 配色更新失败"),
            || widgetry_info!(?root, "MessageBox 配色更新恢复"),
        );
        if result.is_ok() {
            style.full = false;
            style.dirty.clear();
        }
        return result;
    }
    Ok(())
}

fn failure(entity: Entity) -> BevyError {
    BevyError::error(format!("MessageBox {entity:?} 缺失颜色输出或派生状态"))
}

pub(crate) fn update_colors(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, With<WidgetryMessageBox>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut first_error = None;
    for root in roots {
        let standalone = world
            .get::<WidgetryStyleOwner<WidgetryMessageBox>>(root)
            .is_none();
        let palette = standalone
            && world
                .get::<MessageBoxStyle>(root)
                .is_some_and(|s| s.palette || s.colors.is_none());
        let result = if palette {
            #[cfg(test)]
            COLOR_WORK.with_borrow_mut(|work| work.0.push(root));
            let theme = world.resource::<WidgetryThemeMode>().colors().message_box;
            match crate::WidgetryMessageBoxColorOverrides::get(world, root) {
                Ok(overrides) => {
                    let colors = overrides.resolve(&theme);
                    if let Some(mut style) = world.get_mut::<MessageBoxStyle>(root) {
                        style.palette = false;
                    }
                    apply_owned_message_box_colors(world, root, &colors)
                }
                Err(error) => Err(error),
            }
        } else {
            apply_pending_colors(world, root)
        };
        if let Err(error) = result
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result {
        for mut style in world.query::<&mut MessageBoxStyle>().iter_mut(world) {
            style.palette = true;
        }
        update_colors(world)
    });
}

fn color_part_removed(
    event: On<Discard<PartOwner>>,
    parts: Query<&PartOwner>,
    mut styles: Query<&mut MessageBoxStyle>,
) {
    if let Ok(owner) = parts.get(event.entity)
        && let Ok(mut style) = styles.get_mut(owner.0)
    {
        style.parts.remove(&event.entity);
        style.dirty.remove(&event.entity);
    }
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
            PartOwner(root),
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

    #[test]
    fn settled_colors_do_no_composition_work_and_hover_only_updates_one_action() {
        let (mut app, roots) = fixture();
        COLOR_WORK.with_borrow_mut(|work| *work = default());
        app.update();
        COLOR_WORK.with_borrow(|work| assert_eq!(*work, (vec![], vec![])));
        let action = actions(&mut app, roots[1])[0];
        app.world_mut().entity_mut(action).insert(Hovered(true));
        app.update();
        COLOR_WORK.with_borrow_mut(|work| {
            assert!(work.0.is_empty());
            assert_eq!(work.1, vec![action]);
            *work = default();
        });
        assert_eq!(
            app.world().get::<BackgroundColor>(action).unwrap().0,
            app.world()
                .resource::<WidgetryThemeMode>()
                .colors()
                .message_box
                .action_button
                .hovered
                .background
        );
        app.world_mut().entity_mut(action).remove::<Hovered>();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(action).unwrap().0,
            app.world()
                .resource::<WidgetryThemeMode>()
                .colors()
                .message_box
                .action_button
                .normal
                .background
        );
        COLOR_WORK.with_borrow(|work| assert_eq!(work.1, vec![action]));
    }

    #[test]
    fn owned_palette_updates_late_parts_and_input_without_local_resolution() {
        let (mut app, roots) = fixture();
        let root = roots[2];
        app.world_mut()
            .entity_mut(root)
            .insert(WidgetryStyleOwner::<WidgetryMessageBox>::new(roots[0]));
        app.update();
        let mut colors = app
            .world()
            .resource::<WidgetryThemeMode>()
            .colors()
            .file_dialog
            .confirmation;
        colors.body.normal.background = Color::srgb(0.2, 0.3, 0.4);
        colors.action_button.hovered.background = Color::srgb(0.6, 0.3, 0.4);
        apply_owned_message_box_colors(app.world_mut(), root, &colors).unwrap();
        let host = body(&mut app, root);
        let late = app
            .world_mut()
            .spawn_scene(bsn! {
                MessageBoxBody Node BackgroundColor ResolvedForeground ChildOf(host)
            })
            .unwrap()
            .id();
        let action = actions(&mut app, root)[0];
        COLOR_WORK.with_borrow_mut(|work| *work = default());
        app.world_mut().entity_mut(action).insert(Hovered(true));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(late).unwrap().0,
            colors.body.normal.background
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(action).unwrap().0,
            colors.action_button.hovered.background
        );
        COLOR_WORK.with_borrow(|work| assert!(work.0.is_empty()));
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryStyleOwner<WidgetryMessageBox>>();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(action).unwrap().0,
            app.world()
                .resource::<WidgetryThemeMode>()
                .colors()
                .message_box
                .action_button
                .hovered
                .background
        );
    }

    #[test]
    fn restored_message_box_marker_reapplies_the_window_palette() {
        let (mut app, roots) = fixture();
        let root = roots[2];
        let border = Color::srgb(0.2, 0.4, 0.6);
        let mut colors = crate::WidgetryMessageBoxColorOverrides::default();
        colors.window.frame.normal.border = Some(border);
        crate::WidgetryMessageBoxColorOverrides::set_in_world(app.world_mut(), root, colors)
            .unwrap();
        app.update();
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(border)
        );
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryMessageBox>();
        app.update();
        assert_ne!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(border)
        );
        app.world_mut().entity_mut(root).insert(WidgetryMessageBox);
        app.update();
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(border)
        );
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
        assert_eq!(app.world().get::<PartOwner>(action).unwrap().0, roots[1]);
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(action)
                .unwrap()
                .entity,
            roots[1]
        );
        app.world_mut().entity_mut(container).remove::<ChildOf>();
        app.update();
        assert!(app.world().get::<PartOwner>(action).is_none());
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
        assert!(app.world().get::<PartOwner>(action).is_none());
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
            assert!(app.world().get::<PartOwner>(action).is_none());
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
