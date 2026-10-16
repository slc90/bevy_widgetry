use super::*;
use crate::controls::{FilterChoice, LocationControl, SortChoice, UiControl};
use crate::view::EntryRow;
use crate::window::{Independent, OverwriteOwner};
use bevy::ecs::entity::EntityHashSet;
use bevy_widgetry_check_box::WidgetryCheckBox;
use bevy_widgetry_combo_box::WidgetryComboBox;
use bevy_widgetry_core::color::WidgetryStyleOwner;
use bevy_widgetry_message_box::WidgetryMessageBox;
use bevy_widgetry_window::internal::WindowRoot;

#[derive(Resource, Default)]
struct OwnerChanges(EntityHashSet);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Window,
    Button,
    Part(PartKind),
    Row,
    Confirmation,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Binding {
    root: Entity,
    kind: Kind,
}

#[cfg(test)]
thread_local! {
    static OWNER_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

type Subjects = Or<(
    With<Independent>,
    With<Control>,
    With<Part>,
    With<LocationControl>,
    With<UiControl>,
    With<EntryRow>,
    With<OverwriteOwner>,
    With<Binding>,
)>;

type Owners = (
    WidgetryStyleOwner<WindowRoot>,
    WidgetryStyleOwner<WidgetryButton>,
    WidgetryStyleOwner<WidgetryTextField>,
    WidgetryStyleOwner<WidgetryScrollArea>,
    WidgetryStyleOwner<WidgetryComboBox<FilterChoice>>,
    WidgetryStyleOwner<WidgetryComboBox<SortChoice>>,
    WidgetryStyleOwner<WidgetryCheckBox>,
    WidgetryStyleOwner<WidgetryMessageBox>,
);

fn owner_inserted(
    event: On<Insert<Owners>>,
    bindings: Query<(), With<Binding>>,
    mut changes: ResMut<OwnerChanges>,
) {
    if bindings.contains(event.entity) {
        changes.0.insert(event.entity);
    }
}

fn inserted(
    event: On<
        Insert<(
            Independent,
            Control,
            Part,
            LocationControl,
            UiControl,
            EntryRow,
            OverwriteOwner,
        )>,
    >,
    mut changes: ResMut<OwnerChanges>,
) {
    changes.0.insert(event.entity);
}

fn removed(
    event: On<
        Remove<(
            Independent,
            Control,
            Part,
            LocationControl,
            UiControl,
            EntryRow,
            OverwriteOwner,
            WidgetryStyleOwner<WindowRoot>,
            WidgetryStyleOwner<WidgetryButton>,
            WidgetryStyleOwner<WidgetryTextField>,
            WidgetryStyleOwner<WidgetryScrollArea>,
            WidgetryStyleOwner<WidgetryComboBox<FilterChoice>>,
            WidgetryStyleOwner<WidgetryComboBox<SortChoice>>,
            WidgetryStyleOwner<WidgetryCheckBox>,
            WidgetryStyleOwner<WidgetryMessageBox>,
        )>,
    >,
    subjects: Query<(), Subjects>,
    mut changes: ResMut<OwnerChanges>,
) {
    if subjects.contains(event.entity) {
        changes.0.insert(event.entity);
    }
}

fn root_inserted(event: On<Insert<WidgetryFileDialog>>, mut commands: Commands) {
    let root = event.entity;
    commands.queue(move |world: &mut World| queue_root(world, root));
}

fn root_removed(event: On<Remove<WidgetryFileDialog>>, mut commands: Commands) {
    let root = event.entity;
    commands.queue(move |world: &mut World| queue_root(world, root));
}

fn queue_root(world: &mut World, root: Entity) {
    let entities = world
        .query_filtered::<Entity, Subjects>()
        .iter(world)
        .filter(|entity| {
            desired(world, *entity).is_some_and(|binding| binding.root == root)
                || world
                    .get::<Binding>(*entity)
                    .is_some_and(|binding| binding.root == root)
        })
        .collect::<Vec<_>>();
    world.resource_mut::<OwnerChanges>().0.extend(entities);
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<OwnerChanges>()
        .add_observer(inserted)
        .add_observer(removed)
        .add_observer(owner_inserted)
        .add_observer(root_inserted)
        .add_observer(root_removed)
        .add_systems(
            PostUpdate,
            establish_color_owners.in_set(WidgetryUiSystems::StyleOwners),
        );
    let world = app.world_mut();
    let entities = world
        .query_filtered::<Entity, Subjects>()
        .iter(world)
        .collect::<Vec<_>>();
    world.resource_mut::<OwnerChanges>().0.extend(entities);
}

fn desired(world: &World, entity: Entity) -> Option<Binding> {
    let (root, kind) = if world.get::<Independent>(entity).is_some() {
        (entity, Kind::Window)
    } else if let Some(control) = world.get::<Control>(entity) {
        (control.root, Kind::Button)
    } else if let Some(control) = world.get::<LocationControl>(entity) {
        (control.root, Kind::Button)
    } else if let Some(control) = world.get::<UiControl>(entity) {
        (control.root, Kind::Button)
    } else if let Some(part) = world.get::<Part>(entity) {
        (part.root, Kind::Part(part.kind))
    } else if let Some(row) = world.get::<EntryRow>(entity) {
        (row.root, Kind::Row)
    } else {
        let owner = world.get::<OverwriteOwner>(entity)?;
        (owner.root, Kind::Confirmation)
    };
    Some(Binding { root, kind })
}

fn update<T: Send + Sync + 'static>(world: &mut World, entity: Entity, root: Entity, attach: bool) {
    let previous = world
        .get::<WidgetryStyleOwner<T>>(entity)
        .map(|owner| owner.entity);
    if attach && previous != Some(root) {
        world
            .entity_mut(entity)
            .insert(WidgetryStyleOwner::<T>::new(root));
    } else if !attach && previous == Some(root) {
        world.entity_mut(entity).remove::<WidgetryStyleOwner<T>>();
    }
}

fn apply(world: &mut World, entity: Entity, binding: Binding, attach: bool) {
    let root = binding.root;
    match binding.kind {
        Kind::Window => update::<WindowRoot>(world, entity, root, attach),
        Kind::Button => update::<WidgetryButton>(world, entity, root, attach),
        Kind::Confirmation => update::<WidgetryMessageBox>(world, entity, root, attach),
        Kind::Part(kind) => match kind {
            PartKind::Path | PartKind::Search | PartKind::Filename | PartKind::Folder => {
                update::<WidgetryTextField>(world, entity, root, attach)
            }
            PartKind::Sidebar | PartKind::Entries => {
                update::<WidgetryScrollArea>(world, entity, root, attach)
            }
            PartKind::Filter => {
                update::<WidgetryComboBox<FilterChoice>>(world, entity, root, attach)
            }
            PartKind::Sort => update::<WidgetryComboBox<SortChoice>>(world, entity, root, attach),
            PartKind::Hidden | PartKind::System => {
                update::<WidgetryCheckBox>(world, entity, root, attach)
            }
            _ => {}
        },
        Kind::Row => {}
    }
}

pub(super) fn establish(world: &mut World) -> Result {
    let pending = std::mem::take(&mut world.resource_mut::<OwnerChanges>().0);
    for entity in pending {
        if !world.entities().contains(entity) {
            continue;
        }
        #[cfg(test)]
        OWNER_WORK.with(|work| work.set(work.get() + 1));
        let desired = desired(world, entity)
            .filter(|binding| world.get::<WidgetryFileDialog>(binding.root).is_some());
        if let Some(old) = world.get::<Binding>(entity).copied()
            && Some(old) != desired
        {
            apply(world, entity, old, false);
            world.entity_mut(entity).remove::<Binding>();
        }
        if let Some(binding) = desired {
            apply(world, entity, binding, true);
            if world.get::<Binding>(entity) != Some(&binding) {
                world.entity_mut(entity).insert(binding);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn settled_owners_do_no_work_and_delayed_parts_rebind_and_release() {
        let (mut app, first) = super::super::tests::fixture();
        for _ in 0..3 {
            app.update();
        }
        OWNER_WORK.with(|work| work.set(0));
        for _ in 0..3 {
            app.update();
        }
        OWNER_WORK.with(|work| assert_eq!(work.get(), 0));
        let second = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog })
            .unwrap()
            .id();
        app.update();
        let part = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryButton ChildOf(first)
                template(move |_| Ok(Control {root:first, action:WidgetryFileDialogAction::Cancel}))
            })
            .unwrap()
            .id();
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(part)
                .unwrap()
                .entity,
            first
        );
        app.world_mut()
            .entity_mut(part)
            .remove::<WidgetryStyleOwner<WidgetryButton>>();
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(part)
                .unwrap()
                .entity,
            first
        );
        app.world_mut().entity_mut(part).insert((
            Control {
                root: second,
                action: WidgetryFileDialogAction::Cancel,
            },
            ChildOf(second),
        ));
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(part)
                .unwrap()
                .entity,
            second
        );
        app.world_mut()
            .entity_mut(part)
            .insert(WidgetryStyleOwner::<WidgetryButton>::new(first));
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(part)
                .unwrap()
                .entity,
            second
        );
        app.world_mut()
            .entity_mut(second)
            .remove::<WidgetryFileDialog>();
        app.update();
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(part)
                .is_none()
        );
        app.world_mut()
            .entity_mut(second)
            .insert(WidgetryFileDialog);
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(part)
                .unwrap()
                .entity,
            second
        );
        app.world_mut().entity_mut(part).remove::<Control>();
        app.update();
        assert!(
            app.world()
                .get::<WidgetryStyleOwner<WidgetryButton>>(part)
                .is_none()
        );
        assert!(app.world().get::<Binding>(part).is_none());
        app.world_mut().entity_mut(first).despawn();
        app.update();
        assert!(app.world().get::<WidgetryFileDialog>(second).is_some());
    }
}
