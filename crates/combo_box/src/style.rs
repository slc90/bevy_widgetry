use crate::{
    WidgetryComboBox,
    field::{ComboBoxDropdownIcon, ComboBoxField},
    popup::ComboBoxPopup,
};
use bevy::ecs::entity::EntityHashSet;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::{color::WidgetryStyleOwner, foreground::ResolvedForeground};
use bevy_widgetry_list_view::WidgetryListView;
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};
use std::marker::PhantomData;

#[cfg(test)]
#[derive(Resource, Default)]
pub(crate) struct OwnerWork(Vec<Entity>);

#[cfg(test)]
#[derive(Resource, Default)]
pub(crate) struct StyleWork {
    resolved: Vec<Entity>,
    applied: Vec<Entity>,
}

#[derive(Component)]
pub(crate) struct StyleInvalidation {
    dirty: bool,
    palette_dirty: bool,
    colors: Option<bevy_widgetry_theme::WidgetryComboBoxColors>,
}

impl Default for StyleInvalidation {
    fn default() -> Self {
        Self {
            dirty: true,
            palette_dirty: true,
            colors: None,
        }
    }
}

fn invalidate_structure<T: Send + Sync + 'static>(world: &mut World, entity: Entity) {
    if let Some(owner) = world.get::<OwnedPart<T>>(entity) {
        let root = owner.root;
        if let Some(mut style) = world.get_mut::<StyleInvalidation>(root) {
            style.dirty = true;
        }
    }
    let mut cursor = Some(entity);
    while let Some(entity) = cursor {
        if world.get::<WidgetryComboBox<T>>(entity).is_some() {
            if let Some(mut style) = world.get_mut::<StyleInvalidation>(entity) {
                style.dirty = true;
            }
            break;
        }
        cursor = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
}

fn invalidate_input<T: Send + Sync + 'static>(
    entity: Entity,
    parents: &Query<&ChildOf>,
    styles: &mut Query<&mut StyleInvalidation, With<WidgetryComboBox<T>>>,
) {
    for root in std::iter::once(entity).chain(parents.iter_ancestors(entity)) {
        if let Ok(mut style) = styles.get_mut(root) {
            style.dirty = true;
            break;
        }
    }
}

pub(crate) fn collect_changes<T: Send + Sync + 'static>(
    changed: Query<
        Entity,
        (
            Or<(
                With<WidgetryComboBox<T>>,
                With<ComboBoxField>,
                With<ComboBoxPopup>,
                With<ComboBoxDropdownIcon>,
            )>,
            Or<(
                Changed<Hovered>,
                Added<Pressed>,
                Added<InteractionDisabled>,
                Changed<Visibility>,
                Changed<Children>,
                Added<ComboBoxDropdownIcon>,
            )>,
        ),
    >,
    palettes: Query<
        Entity,
        (
            With<WidgetryComboBox<T>>,
            Changed<crate::colors::ColorState>,
        ),
    >,
    mut removed_owners: RemovedComponents<WidgetryStyleOwner<WidgetryComboBox<T>>>,
    parents: Query<&ChildOf>,
    mut styles: Query<&mut StyleInvalidation, With<WidgetryComboBox<T>>>,
) {
    for entity in &changed {
        invalidate_input::<T>(entity, &parents, &mut styles);
    }
    for entity in palettes.iter().chain(removed_owners.read()) {
        if let Ok(mut style) = styles.get_mut(entity) {
            style.dirty = true;
            style.palette_dirty = true;
        }
    }
}

fn style_removed<T: Send + Sync + 'static>(
    event: On<
        Remove<(
            Hovered,
            Pressed,
            InteractionDisabled,
            Children,
            BackgroundColor,
            BorderColor,
            ResolvedForeground,
        )>,
    >,
    subjects: Query<
        (),
        Or<(
            With<WidgetryComboBox<T>>,
            With<ComboBoxField>,
            With<ComboBoxPopup>,
            With<ComboBoxDropdownIcon>,
        )>,
    >,
    parents: Query<&ChildOf>,
    mut styles: Query<&mut StyleInvalidation, With<WidgetryComboBox<T>>>,
) {
    if subjects.contains(event.entity) {
        invalidate_input::<T>(event.entity, &parents, &mut styles);
    }
}

fn palette_removed<T: Send + Sync + 'static>(
    event: On<Remove<crate::colors::ColorState>>,
    mut styles: Query<&mut StyleInvalidation, With<WidgetryComboBox<T>>>,
) {
    if let Ok(mut style) = styles.get_mut(event.entity) {
        style.dirty = true;
        style.palette_dirty = true;
    }
}

#[derive(Resource)]
struct OwnerChanges<T: Send + Sync + 'static> {
    pending: EntityHashSet,
    marker: PhantomData<fn() -> T>,
}

impl<T: Send + Sync + 'static> Default for OwnerChanges<T> {
    fn default() -> Self {
        Self {
            pending: default(),
            marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
enum PartKind {
    Field,
    List,
}

#[derive(Component)]
struct OwnedPart<T: Send + Sync + 'static> {
    root: Entity,
    kind: PartKind,
    marker: PhantomData<fn() -> T>,
}

impl<T: Send + Sync + 'static> OwnedPart<T> {
    fn new(root: Entity, kind: PartKind) -> Self {
        Self {
            root,
            kind,
            marker: PhantomData,
        }
    }
}

pub(crate) fn install_owners<T: Send + Sync + 'static>(app: &mut App) {
    app.init_resource::<OwnerChanges<T>>()
        .add_observer(own_field::<T>)
        .add_observer(own_list::<T>)
        .add_observer(owner_structure_added::<T>)
        .add_observer(owner_structure_removed::<T>)
        .add_observer(owner_parent_inserted::<T>)
        .add_observer(owner_parent_discarded::<T>)
        .add_observer(field_owner_discarded::<T>)
        .add_observer(list_owner_discarded::<T>)
        .add_observer(style_removed::<T>)
        .add_observer(palette_removed::<T>);
    let world = app.world_mut();
    // Registration may happen after Scenes already exist; subsequent work is driven by lifecycle signals.
    let roots = world
        .query_filtered::<Entity, With<WidgetryComboBox<T>>>()
        .iter(world)
        .collect::<EntityHashSet>();
    world
        .resource_mut::<OwnerChanges<T>>()
        .pending
        .extend(roots);
}

fn owner_structure_added<T: Send + Sync + 'static>(
    event: On<Add<(WidgetryComboBox<T>, ComboBoxPopup)>>,
    mut changes: ResMut<OwnerChanges<T>>,
) {
    changes.pending.insert(event.entity);
}

fn owner_structure_removed<T: Send + Sync + 'static>(
    event: On<
        Remove<(
            WidgetryComboBox<T>,
            ComboBoxPopup,
            ComboBoxField,
            WidgetryListView<T>,
        )>,
    >,
    mut changes: ResMut<OwnerChanges<T>>,
) {
    changes.pending.insert(event.entity);
}

fn queue_parent<T: Send + Sync + 'static>(
    entity: Entity,
    parts: &Query<
        (),
        Or<(
            With<ComboBoxField>,
            With<ComboBoxPopup>,
            With<WidgetryListView<T>>,
            With<OwnedPart<T>>,
        )>,
    >,
    changes: &mut OwnerChanges<T>,
) {
    if parts.contains(entity) {
        changes.pending.insert(entity);
    }
}

fn owner_parent_inserted<T: Send + Sync + 'static>(
    event: On<Insert<ChildOf>>,
    parts: Query<
        (),
        Or<(
            With<ComboBoxField>,
            With<ComboBoxPopup>,
            With<WidgetryListView<T>>,
            With<OwnedPart<T>>,
        )>,
    >,
    mut changes: ResMut<OwnerChanges<T>>,
) {
    queue_parent(event.entity, &parts, &mut changes);
}

fn owner_parent_discarded<T: Send + Sync + 'static>(
    event: On<Discard<ChildOf>>,
    parts: Query<
        (),
        Or<(
            With<ComboBoxField>,
            With<ComboBoxPopup>,
            With<WidgetryListView<T>>,
            With<OwnedPart<T>>,
        )>,
    >,
    mut changes: ResMut<OwnerChanges<T>>,
) {
    queue_parent(event.entity, &parts, &mut changes);
}

fn field_owner_discarded<T: Send + Sync + 'static>(
    event: On<Discard<WidgetryStyleOwner<WidgetryButton>>>,
    parts: Query<(), With<OwnedPart<T>>>,
    mut changes: ResMut<OwnerChanges<T>>,
) {
    if parts.contains(event.entity) {
        changes.pending.insert(event.entity);
    }
}

fn list_owner_discarded<T: Send + Sync + 'static>(
    event: On<Discard<WidgetryStyleOwner<WidgetryListView<T>>>>,
    parts: Query<(), With<OwnedPart<T>>>,
    mut changes: ResMut<OwnerChanges<T>>,
) {
    if parts.contains(event.entity) {
        changes.pending.insert(event.entity);
    }
}

fn part_owner<T: Send + Sync + 'static>(
    world: &World,
    entity: Entity,
    kind: PartKind,
) -> Option<Entity> {
    let parent = world.get::<ChildOf>(entity)?.parent();
    let root = match kind {
        PartKind::Field => {
            world.get::<ComboBoxField>(entity)?;
            parent
        }
        PartKind::List => {
            world.get::<WidgetryListView<T>>(entity)?;
            world.get::<ComboBoxPopup>(parent)?;
            world.get::<ChildOf>(parent)?.parent()
        }
    };
    world.get::<WidgetryComboBox<T>>(root).map(|_| root)
}

fn release_part<T: Send + Sync + 'static>(world: &mut World, entity: Entity) {
    let Some(old) = world.get::<OwnedPart<T>>(entity) else {
        return;
    };
    let (root, kind) = (old.root, old.kind);
    match kind {
        PartKind::Field => {
            if world
                .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                .is_some_and(|owner| owner.entity == root)
            {
                world
                    .entity_mut(entity)
                    .remove::<WidgetryStyleOwner<WidgetryButton>>();
            }
        }
        PartKind::List => {
            if world
                .get::<WidgetryStyleOwner<WidgetryListView<T>>>(entity)
                .is_some_and(|owner| owner.entity == root)
            {
                world
                    .entity_mut(entity)
                    .remove::<WidgetryStyleOwner<WidgetryListView<T>>>();
            }
        }
    }
    world.entity_mut(entity).remove::<OwnedPart<T>>();
}

pub(crate) fn establish_owners<T: Send + Sync + 'static>(world: &mut World) {
    // Add observers can precede BSN parent attachment; retry only the affected entities at StyleOwners.
    let mut pending = std::mem::take(&mut world.resource_mut::<OwnerChanges<T>>().pending)
        .into_iter()
        .collect::<Vec<_>>();
    let mut visited = EntityHashSet::default();
    while let Some(entity) = pending.pop() {
        if !visited.insert(entity) || !world.entities().contains(entity) {
            continue;
        }
        invalidate_structure::<T>(world, entity);
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter().filter(|child| {
                world.get::<ComboBoxField>(*child).is_some()
                    || world.get::<ComboBoxPopup>(*child).is_some()
                    || world.get::<WidgetryListView<T>>(*child).is_some()
                    || world.get::<OwnedPart<T>>(*child).is_some()
            }));
        }
        let kind = if world.get::<ComboBoxField>(entity).is_some() {
            Some(PartKind::Field)
        } else if world.get::<WidgetryListView<T>>(entity).is_some() {
            Some(PartKind::List)
        } else {
            None
        };
        let Some((kind, root)) =
            kind.and_then(|kind| part_owner::<T>(world, entity, kind).map(|root| (kind, root)))
        else {
            release_part::<T>(world, entity);
            continue;
        };
        #[cfg(test)]
        world.resource_mut::<OwnerWork>().0.push(root);
        match kind {
            PartKind::Field => {
                if world
                    .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                    .is_none_or(|owner| owner.entity != root)
                {
                    world
                        .entity_mut(entity)
                        .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
                }
            }
            PartKind::List => {
                if world
                    .get::<WidgetryStyleOwner<WidgetryListView<T>>>(entity)
                    .is_none_or(|owner| owner.entity != root)
                {
                    world
                        .entity_mut(entity)
                        .insert(WidgetryStyleOwner::<WidgetryListView<T>>::new(root));
                }
            }
        }
        if world
            .get::<OwnedPart<T>>(entity)
            .is_none_or(|owner| owner.root != root)
        {
            world
                .entity_mut(entity)
                .insert(OwnedPart::<T>::new(root, kind));
        }
    }
}
fn failure(entity: Entity) -> BevyError {
    bevy_widgetry_internal_log::widgetry_error!(?entity, "ComboBox 缺失颜色部件");
    BevyError::error("ComboBox 缺失颜色部件")
}
pub fn apply_owned_combo_colors<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryComboBoxColors,
) -> Result<(), BevyError> {
    let style = world
        .get::<StyleInvalidation>(root)
        .ok_or_else(|| failure(root))?;
    if !style.dirty && style.colors.as_ref() == Some(colors) {
        return Ok(());
    }
    // Retain invalidation on any failure so the host can repair outputs and retry.
    world
        .get_mut::<StyleInvalidation>(root)
        .ok_or_else(|| failure(root))?
        .dirty = true;
    #[cfg(test)]
    world.resource_mut::<StyleWork>().applied.push(root);
    let children = world
        .get::<Children>(root)
        .ok_or_else(|| failure(root))?
        .to_vec();
    let field = children
        .iter()
        .copied()
        .find(|e| world.get::<ComboBoxField>(*e).is_some())
        .ok_or_else(|| failure(root))?;
    let popup = children
        .iter()
        .copied()
        .find(|e| world.get::<ComboBoxPopup>(*e).is_some())
        .ok_or_else(|| failure(root))?;
    let disabled = world.get::<InteractionDisabled>(field).is_some();
    let open = world.get::<Visibility>(popup) == Some(&Visibility::Visible);
    let state = if disabled {
        colors.field.disabled
    } else if open {
        colors.field.open
    } else if world.get::<Pressed>(field).is_some() {
        colors.field.pressed
    } else if world.get::<Hovered>(field).is_some_and(|h| h.0) {
        colors.field.hovered
    } else {
        colors.field.normal
    };
    bevy_widgetry_button::internal::apply_owned_button_colors(
        world,
        field,
        bevy_widgetry_theme::WidgetryButtonStateColors {
            background: state.background,
            border: state.border,
            foreground: state.foreground,
        },
    )?;
    let icons = world
        .get::<Children>(crate::field::button_content(world, field).ok_or_else(|| failure(field))?)
        .ok_or_else(|| failure(field))?
        .to_vec();
    for icon in icons {
        if world.get::<ComboBoxDropdownIcon>(icon).is_some()
            && world.get::<ResolvedForeground>(icon)
                != Some(&ResolvedForeground(state.indicator_foreground))
        {
            world
                .entity_mut(icon)
                .insert(ResolvedForeground(state.indicator_foreground));
        }
    }
    let popup_state = if world.get::<InteractionDisabled>(popup).is_some() {
        colors.popup.disabled
    } else {
        colors.popup.normal
    };
    world
        .get_mut::<BackgroundColor>(popup)
        .ok_or_else(|| failure(popup))?
        .set_if_neq(BackgroundColor(popup_state.background));
    world
        .get_mut::<BorderColor>(popup)
        .ok_or_else(|| failure(popup))?
        .set_if_neq(BorderColor::all(popup_state.border));
    if world.get::<ResolvedForeground>(popup) != Some(&ResolvedForeground(popup_state.foreground)) {
        world
            .entity_mut(popup)
            .insert(ResolvedForeground(popup_state.foreground));
    }
    let lists = world
        .get::<Children>(popup)
        .ok_or_else(|| failure(popup))?
        .to_vec();
    let list = lists
        .into_iter()
        .find(|e| world.get::<WidgetryListView<T>>(*e).is_some())
        .ok_or_else(|| failure(popup))?;
    bevy_widgetry_list_view::internal::apply_owned_list_colors::<T>(world, list, &colors.list)?;
    let mut style = world
        .get_mut::<StyleInvalidation>(root)
        .ok_or_else(|| failure(root))?;
    style.colors = Some(*colors);
    style.palette_dirty = false;
    style.dirty = false;
    Ok(())
}
pub(crate) fn update<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let theme = world.resource::<WidgetryThemeMode>().colors().combo_box;
    let roots = world
        .query_filtered::<Entity, With<WidgetryComboBox<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure_result = None;
    for root in roots {
        let result = (|| -> Result<(), BevyError> {
            let style = world
                .get::<StyleInvalidation>(root)
                .ok_or_else(|| failure(root))?;
            if !style.dirty {
                return Ok(());
            }
            let owned = world
                .get::<WidgetryStyleOwner<WidgetryComboBox<T>>>(root)
                .is_some();
            let colors = if owned {
                // Local input uses the palette supplied by the actual composition owner.
                let Some(colors) = style.colors else {
                    return Ok(());
                };
                colors
            } else if !style.palette_dirty
                && let Some(colors) = style.colors
            {
                colors
            } else {
                #[cfg(test)]
                world.resource_mut::<StyleWork>().resolved.push(root);
                world
                    .get::<crate::colors::ColorState>(root)
                    .ok_or_else(|| failure(root))?
                    .0
                    .resolve(&theme)
            };
            apply_owned_combo_colors::<T>(world, root, &colors)
        })();
        if let Err(error) = result
            && failure_result.is_none()
        {
            failure_result = Some(error);
        }
    }
    failure_result.map_or(Ok(()), Err)
}
pub(crate) fn refresh_theme<T: Send + Sync + 'static>(
    _event: On<WidgetryThemeChanged>,
    mut commands: Commands,
) {
    commands.queue(|world: &mut World| -> Result<(), BevyError> {
        let mut styles = world.query_filtered::<&mut StyleInvalidation, (
            With<WidgetryComboBox<T>>,
            Without<WidgetryStyleOwner<WidgetryComboBox<T>>>,
        )>();
        for mut style in styles.iter_mut(world) {
            style.dirty = true;
            style.palette_dirty = true;
        }
        update::<T>(world)
    });
}

fn own_field<T: Send + Sync + 'static>(
    event: On<Add<ComboBoxField>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
    mut changes: ResMut<OwnerChanges<T>>,
) {
    changes.pending.insert(event.entity);
    if let Ok(parent) = parents.get(event.entity)
        && roots.contains(parent.parent())
    {
        commands.entity(event.entity).insert((
            WidgetryStyleOwner::<WidgetryButton>::new(parent.parent()),
            OwnedPart::<T>::new(parent.parent(), PartKind::Field),
        ));
    }
}
fn own_list<T: Send + Sync + 'static>(
    event: On<Add<WidgetryListView<T>>>,
    parents: Query<&ChildOf>,
    popups: Query<(), With<ComboBoxPopup>>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
    mut changes: ResMut<OwnerChanges<T>>,
) {
    changes.pending.insert(event.entity);
    if let Ok(parent) = parents.get(event.entity)
        && popups.contains(parent.parent())
        && let Ok(root) = parents.get(parent.parent())
        && roots.contains(root.parent())
    {
        commands.entity(event.entity).insert((
            WidgetryStyleOwner::<WidgetryListView<T>>::new(root.parent()),
            OwnedPart::<T>::new(root.parent(), PartKind::List),
        ));
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::WidgetryComboBoxAppExt;
    use bevy_widgetry_list_view::{WidgetryListModel, WidgetryListViewRenderer};
    use bevy_widgetry_test_utils::{LogCapture, scene_app};

    fn combo(source: Entity) -> impl Scene {
        bsn! {
            @WidgetryComboBox::<String> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list! { Text({value.clone()}) bevy_widgetry_core::text::WidgetryText })},
            }
        }
    }

    fn fixture(count: usize) -> (App, Entity, Vec<Entity>) {
        let mut app = scene_app();
        LogCapture::default().run(|| {
            app.register_widgetry_combo_box::<String>().unwrap();
        });
        let mut model = WidgetryListModel::default();
        model.push("one".to_owned()).unwrap();
        model.push("two".to_owned()).unwrap();
        let source = app.world_mut().spawn(model).id();
        let roots = (0..count)
            .map(|_| app.world_mut().spawn_scene(combo(source)).unwrap().id())
            .collect();
        app.update();
        (app, source, roots)
    }

    #[test]
    fn stable_combos_do_not_walk_their_hierarchies() {
        let (mut app, _, _) = fixture(2);
        app.world_mut().resource_mut::<OwnerWork>().0.clear();
        for _ in 0..3 {
            app.update();
        }
        assert!(
            app.world().resource::<OwnerWork>().0.is_empty(),
            "stable ComboBoxes must not repeat owner discovery: {:?}",
            app.world().resource::<OwnerWork>().0
        );
    }

    #[test]
    fn stable_combos_do_not_resolve_or_apply_colors() {
        let (mut app, _, _) = fixture(2);
        app.world_mut().resource_mut::<StyleWork>().resolved.clear();
        app.world_mut().resource_mut::<StyleWork>().applied.clear();
        for _ in 0..3 {
            app.update();
        }
        let work = app.world().resource::<StyleWork>();
        assert!(
            work.resolved.is_empty(),
            "stable combos resolved: {:?}",
            work.resolved
        );
        assert!(
            work.applied.is_empty(),
            "stable combos applied: {:?}",
            work.applied
        );
    }

    fn clear_style_work(app: &mut App) {
        let mut work = app.world_mut().resource_mut::<StyleWork>();
        work.resolved.clear();
        work.applied.clear();
    }

    #[test]
    fn field_inputs_only_repaint_the_affected_combo() {
        let (mut app, _, roots) = fixture(3);
        let (field, popup, _) = parts(app.world(), roots[1]);
        let colors = WidgetryThemeMode::Dark.colors().combo_box;
        clear_style_work(&mut app);
        app.world_mut().entity_mut(field).insert(Hovered(true));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            colors.field.hovered.background
        );
        assert!(app.world().resource::<StyleWork>().resolved.is_empty());
        assert_eq!(app.world().resource::<StyleWork>().applied, [roots[1]]);
        app.world_mut().entity_mut(field).insert(Pressed);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            colors.field.pressed.background
        );
        app.world_mut().entity_mut(field).remove::<Pressed>();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            colors.field.hovered.background
        );
        app.world_mut().entity_mut(field).remove::<Hovered>();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            colors.field.normal.background
        );
        app.world_mut()
            .entity_mut(popup)
            .insert(InteractionDisabled);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(popup).unwrap().0,
            colors.popup.disabled.background
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            colors.field.normal.background
        );
        app.world_mut()
            .entity_mut(popup)
            .remove::<InteractionDisabled>();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(popup).unwrap().0,
            colors.popup.normal.background
        );
        app.world_mut()
            .entity_mut(roots[1])
            .insert(InteractionDisabled);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            colors.field.disabled.background
        );
        app.world_mut()
            .entity_mut(roots[1])
            .remove::<InteractionDisabled>();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            colors.field.normal.background
        );
        assert!(app.world().resource::<StyleWork>().resolved.is_empty());
        assert!(
            app.world()
                .resource::<StyleWork>()
                .applied
                .iter()
                .all(|root| *root == roots[1])
        );
        clear_style_work(&mut app);
        app.update();
        assert!(app.world().resource::<StyleWork>().applied.is_empty());
    }

    #[test]
    fn owned_combo_reuses_owner_palette_and_restores_standalone_on_release() {
        let (mut app, _, roots) = fixture(1);
        let root = roots[0];
        let (field, _, _) = parts(app.world(), root);
        let owner = app.world_mut().spawn_empty().id();
        app.world_mut()
            .entity_mut(root)
            .insert(WidgetryStyleOwner::<WidgetryComboBox<String>>::new(owner));
        let mut palette = WidgetryThemeMode::Light.colors().combo_box;
        palette.field.hovered.background = Color::srgb(0.7, 0.2, 0.6);
        apply_owned_combo_colors::<String>(app.world_mut(), root, &palette).unwrap();
        app.update();
        clear_style_work(&mut app);
        for _ in 0..3 {
            apply_owned_combo_colors::<String>(app.world_mut(), root, &palette).unwrap();
            app.update();
        }
        assert!(app.world().resource::<StyleWork>().applied.is_empty());
        app.world_mut().entity_mut(field).insert(Hovered(true));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            palette.field.hovered.background
        );
        assert!(app.world().resource::<StyleWork>().resolved.is_empty());
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryStyleOwner<WidgetryComboBox<String>>>();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .combo_box
                .field
                .hovered
                .background
        );
        assert_eq!(app.world().resource::<StyleWork>().resolved, [root]);
    }

    #[test]
    fn palette_invalidation_is_instance_and_type_local() {
        let (mut app, _, roots) = fixture(2);
        LogCapture::default().run(|| {
            app.register_widgetry_combo_box::<u32>().unwrap();
        });
        let mut model = WidgetryListModel::default();
        model.push(42u32).unwrap();
        let source = app.world_mut().spawn(model).id();
        app.world_mut().spawn_scene(bsn! {
            @WidgetryComboBox::<u32> { @source: source, @renderer: {WidgetryListViewRenderer::new(|_, value: &u32| bsn_list! {Text({value.to_string()}) bevy_widgetry_core::text::WidgetryText})} }
        }).unwrap();
        app.update();
        clear_style_work(&mut app);
        let mut colors = crate::WidgetryComboBoxColorOverrides::default();
        colors.field.normal.background = Some(Color::srgb(0.2, 0.4, 0.6));
        crate::WidgetryComboBoxColorOverrides::set_in_world(app.world_mut(), roots[0], colors)
            .unwrap();
        app.update();
        assert_eq!(app.world().resource::<StyleWork>().resolved, [roots[0]]);
        assert_eq!(app.world().resource::<StyleWork>().applied, [roots[0]]);
        clear_style_work(&mut app);
        app.update();
        assert!(app.world().resource::<StyleWork>().applied.is_empty());
    }

    #[test]
    fn missing_color_output_propagates_and_retries_after_repair() {
        use bevy_widgetry_test_utils::ErrorCapture;
        let (mut app, _, roots) = fixture(1);
        let (field, _, _) = parts(app.world(), roots[0]);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        app.world_mut()
            .entity_mut(field)
            .remove::<BackgroundColor>();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        errors.run(|| logs.run(|| app.update()));
        assert!(errors.take().iter().any(|error| error.severity()
            == bevy::ecs::error::Severity::Error
            && error.to_string().contains("Button 缺失托管颜色输出")));
        assert!(logs.records().iter().any(|record| {
            record
                .fields
                .get("message")
                .is_some_and(|message| message == "Button 缺失托管颜色输出")
        }));
        app.world_mut()
            .entity_mut(field)
            .insert(BackgroundColor(Color::NONE));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .combo_box
                .field
                .normal
                .background
        );
        assert!(
            !app.world()
                .get::<StyleInvalidation>(roots[0])
                .unwrap()
                .dirty
        );
    }

    #[test]
    fn replacement_arrow_receives_the_current_combo_foreground() {
        let (mut app, _, roots) = fixture(1);
        let (field, _, _) = parts(app.world(), roots[0]);
        let color = Color::srgb(0.2, 0.8, 0.5);
        let mut colors = crate::WidgetryComboBoxColorOverrides::default();
        colors.field.normal.indicator_foreground = Some(color);
        crate::WidgetryComboBoxColorOverrides::set_in_world(app.world_mut(), roots[0], colors)
            .unwrap();
        app.update();
        let icon = app
            .world()
            .get::<Children>(crate::field::button_content(app.world(), field).unwrap())
            .unwrap()
            .iter()
            .find(|child| app.world().get::<ComboBoxDropdownIcon>(*child).is_some())
            .unwrap();
        app.world_mut().despawn(icon);
        let content = crate::field::button_content(app.world(), field).unwrap();
        let icon = app.world_mut().spawn_scene(bsn! {
            ComboBoxDropdownIcon ChildOf(content)
            @bevy_widgetry_core::icon::WidgetryIcon { @path: {bevy_widgetry_asset::BuiltinIcon::ChevronDown.path()} }
        }).unwrap().id();
        app.update();
        assert_eq!(
            app.world().get::<ResolvedForeground>(icon),
            Some(&ResolvedForeground(color))
        );
    }

    fn parts(world: &World, root: Entity) -> (Entity, Entity, Entity) {
        let children = world.get::<Children>(root).unwrap();
        let field = children
            .iter()
            .find(|child| world.get::<ComboBoxField>(*child).is_some())
            .unwrap();
        let popup = children
            .iter()
            .find(|child| world.get::<ComboBoxPopup>(*child).is_some())
            .unwrap();
        let list = world
            .get::<Children>(popup)
            .unwrap()
            .iter()
            .find(|child| world.get::<WidgetryListView<String>>(*child).is_some())
            .unwrap();
        (field, popup, list)
    }

    fn assert_owners(world: &World, root: Entity, field: Entity, list: Entity) {
        assert_eq!(
            world
                .get::<WidgetryStyleOwner<WidgetryButton>>(field)
                .unwrap()
                .entity,
            root
        );
        assert_eq!(
            world
                .get::<WidgetryStyleOwner<WidgetryListView<String>>>(list)
                .unwrap()
                .entity,
            root
        );
    }

    #[test]
    fn delayed_attachment_and_popup_reparenting_follow_current_structure() {
        let (mut app, _, roots) = fixture(2);
        let (field, popup, list) = parts(app.world(), roots[0]);
        app.world_mut().resource_mut::<OwnerWork>().0.clear();
        app.world_mut().entity_mut(field).remove::<ChildOf>();
        app.world_mut().entity_mut(popup).remove::<ChildOf>();
        establish_owners::<String>(app.world_mut());
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(field)
                .is_none()
        );
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryListView<String>>>(list)
                .is_none()
        );
        establish_owners::<String>(app.world_mut());
        assert!(
            app.world()
                .resource::<OwnerChanges<String>>()
                .pending
                .is_empty()
        );
        assert!(app.world().resource::<OwnerWork>().0.is_empty());
        app.world_mut().entity_mut(roots[0]).add_child(field);
        app.world_mut().entity_mut(roots[1]).add_child(popup);
        establish_owners::<String>(app.world_mut());
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(field)
                .unwrap()
                .entity,
            roots[0]
        );
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryListView<String>>>(list)
                .unwrap()
                .entity,
            roots[1]
        );
        app.world_mut().entity_mut(roots[0]).add_child(popup);
        app.update();
        assert_owners(app.world(), roots[0], field, list);
    }

    #[test]
    fn owner_removal_and_list_rebuild_only_visit_the_affected_combo() {
        let (mut app, source, roots) = fixture(3);
        let (field, popup, list) = parts(app.world(), roots[1]);
        app.world_mut().resource_mut::<OwnerWork>().0.clear();
        app.world_mut()
            .entity_mut(field)
            .remove::<WidgetryStyleOwner<WidgetryButton>>();
        app.world_mut()
            .entity_mut(list)
            .remove::<WidgetryStyleOwner<WidgetryListView<String>>>();
        app.update();
        assert_owners(app.world(), roots[1], field, list);
        assert_eq!(app.world().resource::<OwnerWork>().0, vec![roots[1]; 2]);
        app.world_mut().resource_mut::<OwnerWork>().0.clear();
        app.world_mut().despawn(list);
        let list = app.world_mut().spawn_scene(bsn! {
            ChildOf(popup)
            @WidgetryListView::<String> { @source: source, @item_height: 32.0,
                @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list! {Text({value.clone()}) bevy_widgetry_core::text::WidgetryText})},
            }
        }).unwrap().id();
        app.update();
        assert_owners(app.world(), roots[1], field, list);
        assert_eq!(app.world().resource::<OwnerWork>().0, vec![roots[1]]);
        app.world_mut().resource_mut::<OwnerWork>().0.clear();
        app.world_mut().despawn(roots[1]);
        app.update();
        app.update();
        assert!(
            app.world()
                .resource::<OwnerChanges<String>>()
                .pending
                .is_empty()
        );
        assert!(app.world().resource::<OwnerWork>().0.is_empty());
    }

    #[test]
    fn root_marker_and_part_marker_removal_release_and_restore_owners() {
        let (mut app, _, roots) = fixture(1);
        let root = roots[0];
        let (field, _, list) = parts(app.world(), root);
        let combo = app
            .world_mut()
            .entity_mut(root)
            .take::<WidgetryComboBox<String>>()
            .unwrap();
        app.update();
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(field)
                .is_none()
        );
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryListView<String>>>(list)
                .is_none()
        );
        app.world_mut().entity_mut(root).insert(combo);
        app.update();
        assert_owners(app.world(), root, field, list);
        app.world_mut().entity_mut(field).remove::<ComboBoxField>();
        establish_owners::<String>(app.world_mut());
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(field)
                .is_none()
        );
        app.world_mut().entity_mut(field).insert(ComboBoxField);
        app.update();
        assert_owners(app.world(), root, field, list);
    }

    #[test]
    fn typed_owners_remain_independent_and_incomplete_parts_do_not_stay_pending() {
        let (mut app, _, roots) = fixture(1);
        LogCapture::default().run(|| {
            app.register_widgetry_combo_box::<u32>().unwrap();
        });
        let mut model = WidgetryListModel::default();
        model.push(42u32).unwrap();
        let source = app.world_mut().spawn(model).id();
        let root = app.world_mut().spawn_scene(bsn! {
            @WidgetryComboBox::<u32> { @source: source, @renderer: {WidgetryListViewRenderer::new(|_, value: &u32| bsn_list! {Text({value.to_string()}) bevy_widgetry_core::text::WidgetryText})} }
        }).unwrap().id();
        let detached = app
            .world_mut()
            .spawn_scene(bsn! { ComboBoxField Node::default() })
            .unwrap()
            .id();
        app.update();
        let field = app.world().get::<Children>(root).unwrap()[0];
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(field)
                .unwrap()
                .entity,
            root
        );
        assert!(app.world().get::<OwnedPart<String>>(field).is_none());
        let (first_field, _, first_list) = parts(app.world(), roots[0]);
        assert_owners(app.world(), roots[0], first_field, first_list);
        assert!(app.world().get::<OwnedPart<u32>>(first_field).is_none());
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(detached)
                .is_none()
        );
        assert!(
            app.world()
                .resource::<OwnerChanges<String>>()
                .pending
                .is_empty()
        );
        assert!(
            app.world()
                .resource::<OwnerChanges<u32>>()
                .pending
                .is_empty()
        );
        app.world_mut().resource_mut::<OwnerWork>().0.clear();
        app.update();
        assert!(app.world().resource::<OwnerWork>().0.is_empty());
        app.world_mut().despawn(detached);
        app.world_mut().despawn(root);
        app.world_mut().despawn(roots[0]);
        app.update();
        assert!(
            app.world()
                .resource::<OwnerChanges<String>>()
                .pending
                .is_empty()
        );
        assert!(
            app.world()
                .resource::<OwnerChanges<u32>>()
                .pending
                .is_empty()
        );
    }

    #[test]
    fn independently_built_scenes_bind_when_their_parents_arrive() {
        let (mut app, source, roots) = fixture(1);
        let (old_field, old_popup, _) = parts(app.world(), roots[0]);
        let field = app
            .world_mut()
            .spawn_scene(crate::field::scene())
            .unwrap()
            .id();
        let popup = app.world_mut().spawn_scene(crate::popup::scene::<String>(source, 32.0,
            WidgetryListViewRenderer::new(|_, value: &String| bsn_list! {Text({value.clone()}) bevy_widgetry_core::text::WidgetryText})
        )).unwrap().id();
        let list = app.world().get::<Children>(popup).unwrap()[0];
        app.world_mut().flush();
        establish_owners::<String>(app.world_mut());
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(field)
                .is_none()
        );
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryListView<String>>>(list)
                .is_none()
        );
        app.world_mut().despawn(old_field);
        app.world_mut().despawn(old_popup);
        app.world_mut()
            .entity_mut(roots[0])
            .add_children(&[field, popup]);
        app.update();
        assert_owners(app.world(), roots[0], field, list);
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .combo_box
                .field
                .normal
                .background
        );
    }

    #[test]
    fn broken_shell_still_reaches_the_host_as_an_error() {
        use bevy_widgetry_test_utils::ErrorCapture;
        let (mut app, _, roots) = fixture(1);
        let (_, _, list) = parts(app.world(), roots[0]);
        app.set_error_handler(ErrorCapture::handler());
        app.world_mut().despawn(list);
        let logs = LogCapture::default();
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let errors = ErrorCapture::default();
        errors.run(|| logs.run(|| app.update()));
        assert!(
            errors
                .take()
                .iter()
                .any(|error| error.to_string().contains("ComboBox 缺失颜色部件")
                    && error.severity() == bevy::ecs::error::Severity::Error)
        );
        assert!(logs.records().iter().any(|record| {
            record
                .fields
                .get("message")
                .is_some_and(|message| message == "ComboBox 缺失颜色部件")
        }));
    }

    #[test]
    fn typed_plugin_installed_after_scene_construction_compensates_once() {
        use crate::WidgetryComboBoxPlugin;
        use bevy_widgetry_list_view::WidgetryListViewAppExt;
        let mut app = scene_app();
        LogCapture::default().run(|| {
            app.add_plugins(WidgetryComboBoxPlugin)
                .register_widgetry_list_view::<String>()
                .unwrap();
        });
        let mut model = WidgetryListModel::default();
        model.push("one".to_owned()).unwrap();
        let source = app.world_mut().spawn(model).id();
        let root = app.world_mut().spawn_scene(combo(source)).unwrap().id();
        let (field, _, list) = parts(app.world(), root);
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(field)
                .is_none()
        );
        LogCapture::default().run(|| {
            app.register_widgetry_combo_box::<String>().unwrap();
        });
        app.update();
        assert_owners(app.world(), root, field, list);
        app.world_mut().resource_mut::<OwnerWork>().0.clear();
        app.update();
        assert!(app.world().resource::<OwnerWork>().0.is_empty());
    }
}
