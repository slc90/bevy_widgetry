use crate::{ComboError, ComboFailure, combo_box::ComboDiagnostics, error::failure};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use std::marker::PhantomData;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryComboBoxHeadlessProps)]
#[require(Node, ComboDiagnostics, crate::combo_box::ForwardedChanges, OpenState)]
pub struct WidgetryComboBoxHeadless<T: Send + Sync + 'static> {
    source: Entity,
    marker: PhantomData<fn() -> T>,
}

pub struct WidgetryComboBoxHeadlessProps {
    pub source: Entity,
}

impl Default for WidgetryComboBoxHeadlessProps {
    fn default() -> Self {
        Self {
            source: Entity::PLACEHOLDER,
        }
    }
}

#[derive(Component, FromTemplate, Clone, Copy)]
#[component(immutable)]
pub struct WidgetryComboBoxParts {
    pub trigger: Entity,
    pub popup: Entity,
    pub list: Entity,
}

#[derive(Component, Default)]
pub(crate) struct OpenState(pub(crate) bool);

struct ClosedPopup;

pub(crate) fn contains(world: &World, ancestor: Entity, mut entity: Entity) -> bool {
    loop {
        if entity == ancestor {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return false;
        };
        entity = parent.parent();
    }
}

pub(crate) fn owner(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<OpenState>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

pub(crate) fn parts(world: &World, root: Entity) -> Result<WidgetryComboBoxParts, ComboFailure> {
    let bindings = world
        .get::<WidgetryComboBoxParts>(root)
        .copied()
        .ok_or_else(|| {
            failure(
                ComboError::MissingPart {
                    root,
                    entity: root,
                    part: crate::ComboPart::Bindings,
                },
                "combo_box.bindings",
            )
        })?;
    for (entity, part) in [
        (bindings.trigger, crate::ComboPart::Field),
        (bindings.popup, crate::ComboPart::Popup),
        (bindings.list, crate::ComboPart::ListView),
    ] {
        let owner = owner(world, entity);
        if entity == root || world.get::<Node>(entity).is_none() || owner != Some(root) {
            return Err(failure(
                ComboError::InvalidBinding { root, entity, part },
                "combo_box.bindings",
            ));
        }
    }
    if bindings.trigger == bindings.popup
        || bindings.list == bindings.popup
        || !contains(world, bindings.popup, bindings.list)
        || contains(world, bindings.popup, bindings.trigger)
    {
        return Err(failure(
            ComboError::InvalidBinding {
                root,
                entity: bindings.list,
                part: crate::ComboPart::ListView,
            },
            "combo_box.bindings",
        ));
    }
    Ok(bindings)
}

pub(crate) fn close<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<bool, ComboFailure> {
    let changed = if let Some(mut state) = world.get_mut::<OpenState>(root) {
        if state.0 {
            state.0 = false;
            true
        } else {
            false
        }
    } else {
        return Ok(false);
    };
    if let Some(parts) = world.get::<WidgetryComboBoxParts>(root).copied() {
        let focused = world.get_resource::<InputFocus>().and_then(InputFocus::get);
        if owner(world, parts.popup).is_none_or(|owner| owner == root)
            && focused
                .is_some_and(|entity| entity == parts.list || contains(world, parts.popup, entity))
        {
            world.resource_mut::<InputFocus>().clear();
        }
    }
    crate::popup::project_visibility::<T>(world, root);
    if let Some(parts) = world.get::<WidgetryComboBoxParts>(root).copied() {
        let current_owner = owner(world, parts.list);
        if world.get::<Node>(parts.list).is_some()
            && current_owner.is_none_or(|owner| owner == root)
        {
            bevy_widgetry_core::disabled::set_intrinsic_disabled_for::<ClosedPopup>(
                world,
                parts.list,
                current_owner == Some(root),
            )
            .map_err(|error| crate::error::core(error, root, "combo_box.close"))?;
        }
    }
    Ok(changed)
}

fn available<T: Send + Sync + 'static>(
    world: &World,
    root: Entity,
    parts: &WidgetryComboBoxParts,
) -> bool {
    if parts.trigger == parts.popup
        || parts.list == parts.popup
        || contains(world, parts.popup, parts.trigger)
        || !contains(world, parts.popup, parts.list)
    {
        return false;
    }
    for entity in [root, parts.trigger, parts.popup] {
        if world.get::<Node>(entity).is_none() || world.get::<InteractionDisabled>(entity).is_some()
        {
            return false;
        }
    }
    if [parts.trigger, parts.popup, parts.list]
        .iter()
        .any(|&entity| owner(world, entity) != Some(root))
    {
        return false;
    }
    let Some(combo) = world.get::<WidgetryComboBoxHeadless<T>>(root) else {
        return false;
    };
    let Some(view) = world.get::<bevy_widgetry_list_view::WidgetryListView<T>>(parts.list) else {
        return false;
    };
    if combo.source() != view.source()
        || view.selection_mode() != bevy_widgetry_list_view::WidgetryListSelectionMode::Single
    {
        return false;
    }
    let searchable = world
        .get::<crate::WidgetryComboBox<T>>(root)
        .is_some_and(|combo| combo.search().is_some());
    world
        .get::<bevy_widgetry_list_view::WidgetryListModel<T>>(combo.source())
        .is_some_and(|model| !model.is_empty() || searchable)
}

pub(crate) fn reconcile<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, With<WidgetryComboBoxHeadless<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        // Closing another root can flush consumer commands that despawn this one.
        if world.get::<WidgetryComboBoxHeadless<T>>(root).is_none() {
            continue;
        }
        let valid = world
            .get::<WidgetryComboBoxParts>(root)
            .is_some_and(|parts| available::<T>(world, root, parts));
        if !valid
            || !WidgetryComboBoxHeadless::<T>::is_open(world, root)
                .map_err(ComboFailure::into_bevy_error)?
        {
            close::<T>(world, root).map_err(ComboFailure::into_bevy_error)?;
        }
    }
    Ok(())
}

pub(crate) fn disabled<T: Send + Sync + 'static>(
    event: On<Insert<bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>>,
    states: Query<&bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>,
    roots: Query<(Entity, &WidgetryComboBoxParts), With<WidgetryComboBoxHeadless<T>>>,
    mut commands: Commands,
) {
    if !states
        .get(event.entity)
        .is_ok_and(|state| state.is_disabled())
    {
        return;
    }
    for (root, parts) in &roots {
        if [root, parts.trigger, parts.popup, parts.list].contains(&event.entity) {
            commands.queue(move |world: &mut World| {
                close::<T>(world, root)
                    .map(|_| ())
                    .map_err(ComboFailure::into_bevy_error)
            });
        }
    }
}

impl<T: Send + Sync + 'static> WidgetryComboBoxHeadless<T> {
    fn scene(props: WidgetryComboBoxHeadlessProps) -> impl Scene {
        bsn! { WidgetryComboBoxHeadless::<T> {source: {props.source}, marker: PhantomData} }
    }

    pub fn source(&self) -> Entity {
        self.source
    }

    pub fn open_in_world(world: &mut World, root: Entity) -> Result<bool, ComboFailure> {
        Self::is_open(world, root)?;
        let (_, source) = crate::combo_box::update_list::<T>(world, root)?;
        let parts = parts(world, root)?;
        let searchable = world
            .get::<crate::WidgetryComboBox<T>>(root)
            .is_some_and(|combo| combo.search().is_some());
        if [root, parts.trigger, parts.popup]
            .iter()
            .any(|entity| world.get::<InteractionDisabled>(*entity).is_some())
            || (!searchable
                && world
                    .get::<bevy_widgetry_list_view::WidgetryListModel<T>>(source)
                    .is_none_or(|model| model.is_empty()))
        {
            return Ok(false);
        }
        if Self::is_open(world, root)? {
            return Ok(false);
        }
        if let Some(mut state) = world.get_mut::<OpenState>(root) {
            state.0 = true;
        }
        crate::popup::project_visibility::<T>(world, root);
        if searchable {
            world.commands().queue(move |world: &mut World| {
                crate::search::open::<T>(world, root)
                    .map_err(|error| error.into_bevy_error::<T>(root, "combo_box.open_search"))
            });
        } else if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
            focus.set(parts.list, FocusCause::Navigated);
        }
        bevy_widgetry_core::disabled::set_intrinsic_disabled_for::<ClosedPopup>(
            world, parts.list, false,
        )
        .map_err(|error| crate::error::core(error, root, "combo_box.open"))?;
        Ok(true)
    }

    pub fn close_in_world(world: &mut World, root: Entity) -> Result<bool, ComboFailure> {
        Self::is_open(world, root)?;
        close::<T>(world, root)
    }

    pub fn open(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| {
            Self::open_in_world(world, root)
                .map(|_| ())
                .map_err(ComboFailure::into_bevy_error)
        });
    }
    pub fn close(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| {
            Self::close_in_world(world, root)
                .map(|_| ())
                .map_err(ComboFailure::into_bevy_error)
        });
    }

    pub fn select_id(
        commands: &mut Commands,
        root: Entity,
        id: bevy_widgetry_list_view::WidgetryListItemId,
    ) {
        crate::WidgetryComboBox::<T>::select_id(commands, root, id);
    }
    pub fn select_index(commands: &mut Commands, root: Entity, index: usize) {
        crate::WidgetryComboBox::<T>::select_index(commands, root, index);
    }
    pub fn clear_selection(commands: &mut Commands, root: Entity) {
        crate::WidgetryComboBox::<T>::clear_selection(commands, root);
    }
    pub fn selected_id(
        world: &World,
        root: Entity,
    ) -> Result<Option<bevy_widgetry_list_view::WidgetryListItemId>, ComboFailure> {
        crate::WidgetryComboBox::<T>::selected_id(world, root)
    }

    pub fn is_open(world: &World, root: Entity) -> Result<bool, ComboFailure> {
        if world.get::<Self>(root).is_none() {
            return Err(failure(
                ComboError::InvalidRoot {
                    root,
                    item_type: std::any::type_name::<T>(),
                },
                "combo_box.is_open",
            ));
        }
        world
            .get::<OpenState>(root)
            .map(|state| state.0)
            .ok_or_else(|| {
                failure(
                    ComboError::MissingPart {
                        root,
                        entity: root,
                        part: crate::ComboPart::OpenState,
                    },
                    "combo_box.is_open",
                )
            })
    }
}
