use crate::{
    ComboError, ComboFailure, ComboPart,
    error::{ComboFault, failure, list},
};
use crate::{field, popup};
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_diagnostics::bevy::FailureState;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListSelectionChanged,
    WidgetryListSelectionState, WidgetryListView, WidgetryListViewRenderer,
};
use std::{collections::VecDeque, sync::Arc};

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryComboBoxProps<T>)]
#[require(crate::style::StyleInvalidation, ComboDiagnostics, ForwardedChanges, crate::colors::ColorState = crate::colors::ColorState::new::<T>())]
pub struct WidgetryComboBox<T: Send + Sync + 'static> {
    item_height: f32,
    max_visible_items: usize,
    renderer: WidgetryListViewRenderer<T>,
    placeholder: WidgetryComboBoxContent,
}

#[derive(Component, Default)]
pub(crate) struct ComboDiagnostics {
    pub(crate) projection: FailureState<ComboFault>,
    pub(crate) icon: FailureState<ComboFault>,
    pub(crate) search: FailureState<ComboFault>,
    pub(crate) style: FailureState<ComboFault>,
}

pub struct WidgetryComboBoxProps<T: Send + Sync + 'static> {
    pub colors: crate::WidgetryComboBoxColorOverrides,
    pub source: Entity,
    pub item_height: f32,
    pub max_visible_items: usize,
    pub renderer: WidgetryListViewRenderer<T>,
    pub placeholder: WidgetryComboBoxContent,
    pub search: Option<crate::WidgetryComboBoxMatcher<T>>,
    pub search_prompt: WidgetryComboBoxContent,
    pub no_results: WidgetryComboBoxContent,
}

#[derive(Component)]
struct ValidatedConfig;

#[derive(Component)]
pub(crate) struct PreservePopup;

#[derive(Component, Default)]
pub(crate) struct ForwardedChanges(VecDeque<ValueChange<Option<WidgetryListItemId>>>);

pub(crate) fn handle_value_change<T: Send + Sync + 'static>(
    event: On<WidgetryListSelectionChanged>,
    mut roots: Query<
        (Entity, &crate::WidgetryComboBoxParts, &mut ForwardedChanges),
        With<crate::WidgetryComboBoxHeadless<T>>,
    >,
    lists: Query<Has<PreservePopup>, With<WidgetryListView<T>>>,
    mut commands: Commands,
) {
    let Some((root, _, mut changes)) = roots
        .iter_mut()
        .find(|(_, parts, _)| parts.list == event.source)
    else {
        return;
    };
    let value = event.selected.iter().next().copied();
    let preserve = lists.get(event.source).unwrap_or(false);
    changes.0.push_back(ValueChange {
        source: root,
        value,
        is_final: true,
    });
    if !preserve && value.is_some() {
        commands.queue(move |world: &mut World| {
            crate::headless::close::<T>(world, root)
                .map(|_| ())
                .map_err(ComboFailure::into_bevy_error)
        });
    }
    commands.queue(move |world: &mut World| dispatch_value_changes(world, root));
}

// Other observers of the internal event may queue the next programmatic request first.
// Dispatch the old payload first so notification order matches selection commit order.
fn dispatch_value_changes(world: &mut World, root: Entity) {
    while let Some(event) = world
        .get_mut::<ForwardedChanges>(root)
        .and_then(|mut changes| changes.0.pop_front())
    {
        world.trigger(event);
    }
}

// Observers of the previous notification may queue model mutations.
// Queue dispatch separately so those commands run before the next request resolves a stable ID from a potentially stale index.
fn queue_pending_changes<T: Send + Sync + 'static>(commands: &mut Commands, root: Entity) {
    commands.queue(move |world: &mut World| {
        if world
            .get::<crate::WidgetryComboBoxHeadless<T>>(root)
            .is_some()
        {
            dispatch_value_changes(world, root);
        }
    });
}

pub(crate) fn update_list<T: Send + Sync + 'static>(
    world: &World,
    root: Entity,
) -> Result<(Entity, Entity), ComboFailure> {
    let combo = world
        .get::<crate::WidgetryComboBoxHeadless<T>>(root)
        .ok_or_else(|| {
            failure(
                ComboError::InvalidRoot {
                    root,
                    item_type: std::any::type_name::<T>(),
                },
                "combo_box.update_list",
            )
        })?;
    let source = combo.source();
    if world.get::<WidgetryListModel<T>>(source).is_none() {
        return Err(failure(
            ComboError::InvalidSource {
                root,
                source,
                item_type: std::any::type_name::<T>(),
            },
            "combo_box.update_list",
        ));
    }
    let parts = crate::headless::parts(world, root)?;
    let list = parts.list;
    if world.get::<Visibility>(parts.popup).is_none()
        || world.get::<WidgetryListView<T>>(list).is_none()
    {
        return Err(failure(
            ComboError::MissingList {
                root,
                source,
                popup: Some(parts.popup),
            },
            "combo_box.update_list",
        ));
    }
    if world.get::<WidgetryListView<T>>(list).is_some_and(|view| {
        view.selection_mode() != bevy_widgetry_list_view::WidgetryListSelectionMode::Single
    }) {
        return Err(failure(
            ComboError::InvalidBinding {
                root,
                entity: list,
                part: ComboPart::Selection,
            },
            "combo_box.update_list",
        ));
    }
    let actual_source = world
        .get::<WidgetryListView<T>>(list)
        .map(WidgetryListView::source);
    if actual_source != Some(source) {
        return Err(failure(
            ComboError::ListMapping {
                root,
                source,
                list,
                actual_source,
            },
            "combo_box.update_list",
        ));
    }
    if world.get::<WidgetryListSelectionState>(list).is_none() {
        return Err(failure(
            ComboError::MissingPart {
                root,
                entity: list,
                part: ComboPart::Selection,
            },
            "combo_box.update_list",
        ));
    }
    Ok((list, source))
}

#[derive(Clone, Copy)]
enum SelectionRequest {
    Id(WidgetryListItemId),
    Index(usize),
    Clear,
}

fn queue_selection<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    list: Entity,
    request: SelectionRequest,
) {
    let already_preserved = world.get::<PreservePopup>(list).is_some();
    world.entity_mut(list).insert(PreservePopup);
    world.commands().queue(move |world: &mut World| {
        let result = match request {
            SelectionRequest::Id(id) => WidgetryListView::<T>::select_id_in_world(world, list, id),
            SelectionRequest::Index(index) => {
                WidgetryListView::<T>::select_index_in_world(world, list, index)
            }
            SelectionRequest::Clear => WidgetryListView::<T>::clear_selection_in_world(world, list),
        };
        result
            .map(|_| ())
            .map_err(|error| crate::error::list(error, root, "combo_box.select").into_bevy_error())
    });
    if !already_preserved {
        world.commands().queue(move |world: &mut World| {
            if let Ok(mut entity) = world.get_entity_mut(list) {
                entity.remove::<PreservePopup>();
            }
        });
    }
}

impl<T: Send + Sync + 'static> WidgetryComboBox<T> {
    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    pub fn max_visible_items(&self) -> usize {
        self.max_visible_items
    }

    pub fn renderer(&self) -> &WidgetryListViewRenderer<T> {
        &self.renderer
    }

    pub fn select_id(commands: &mut Commands, entity: Entity, item_id: WidgetryListItemId) {
        queue_pending_changes::<T>(commands, entity);
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (list, source) =
                update_list::<T>(world, entity).map_err(ComboFailure::into_bevy_error)?;
            if world
                .get::<WidgetryListModel<T>>(source)
                .is_none_or(|model| model.index_of(item_id).is_none())
            {
                return Err(failure(
                    ComboError::MissingItem {
                        root: entity,
                        source,
                        list,
                        item_id,
                        index: None,
                        part: ComboPart::Item,
                    },
                    "combo_box.select_id",
                )
                .into_bevy_error());
            }
            queue_selection::<T>(world, entity, list, SelectionRequest::Id(item_id));
            Ok(())
        });
    }

    pub fn select_index(commands: &mut Commands, entity: Entity, index: usize) {
        queue_pending_changes::<T>(commands, entity);
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (list, _) =
                update_list::<T>(world, entity).map_err(ComboFailure::into_bevy_error)?;
            queue_selection::<T>(world, entity, list, SelectionRequest::Index(index));
            Ok(())
        });
    }

    pub fn selected_id(
        world: &World,
        entity: Entity,
    ) -> Result<Option<WidgetryListItemId>, ComboFailure> {
        let (internal, _) = update_list::<T>(world, entity)?;
        Ok(WidgetryListView::<T>::selection(world, internal)
            .map_err(|error| list(error, entity, "combo_box.selected_id"))?
            .selected_id())
    }

    pub(crate) fn placeholder(&self) -> &WidgetryComboBoxContent {
        &self.placeholder
    }

    pub fn clear_selection(commands: &mut Commands, entity: Entity) {
        queue_pending_changes::<T>(commands, entity);
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (list, _) =
                update_list::<T>(world, entity).map_err(ComboFailure::into_bevy_error)?;
            queue_selection::<T>(world, entity, list, SelectionRequest::Clear);
            Ok(())
        });
    }

    fn scene(props: WidgetryComboBoxProps<T>) -> impl Scene {
        let height = props.item_height * props.max_visible_items as f32 + 2.0;
        let max_visible_items = props.max_visible_items;
        let source = props.source;
        let item_height = props.item_height;
        let renderer = props.renderer.clone();
        let search = props.search.is_some();
        let search_prompt = props.search_prompt;
        let no_results = props.no_results;
        let search_content = search.then_some((search_prompt, no_results));
        bsn! {
            template(move |context| props.colors.clone().initial::<T>(context.entity.id()).map_err(ComboFailure::into_bevy_error))
            @crate::WidgetryComboBoxHeadless::<T> {@source:source}
            crate::WidgetryComboBoxParts {trigger:#Trigger,popup:#Popup,list:#List}
            @crate::search::binding(props.search, #Search)
            WidgetryComboBox::<T> {
                item_height: {props.item_height},
                max_visible_items: {props.max_visible_items}, renderer: {props.renderer}, placeholder: {props.placeholder},
            }
            template(move |context| {
                if max_visible_items == 0 || !height.is_finite() {
                    return Err(failure(ComboError::Configuration {root:context.entity.id(),source,item_height,max_visible_items,viewport_height:height}, "combo_box.initialize").into_bevy_error());
                }
                Ok(ValidatedConfig)
            })
            Node { width: px(200) }
            Children [#Trigger @field::scene()-- #Popup @popup::scene::<T>(source,item_height,renderer,search_content,#List,#Search)]
        }
    }
}

impl<T: Send + Sync + 'static> Default for WidgetryComboBoxProps<T> {
    fn default() -> Self {
        Self {
            colors: default(),
            source: Entity::PLACEHOLDER,
            item_height: 32.0,
            max_visible_items: 8,
            renderer: WidgetryListViewRenderer::default(),
            placeholder: default(),
            search: None,
            search_prompt: WidgetryComboBoxContent::new(
                || bsn_list! {Text("Search") bevy_widgetry_core::text::WidgetryText},
            ),
            no_results: WidgetryComboBoxContent::new(
                || bsn_list! {Text("No results") bevy_widgetry_core::text::WidgetryText},
            ),
        }
    }
}

#[derive(Clone)]
pub struct WidgetryComboBoxContent(Arc<dyn Fn() -> Box<dyn SceneList> + Send + Sync>);

impl Default for WidgetryComboBoxContent {
    fn default() -> Self {
        Self::new(|| bsn_list! { Text("Select…") bevy_widgetry_core::text::WidgetryText })
    }
}

impl WidgetryComboBoxContent {
    pub fn new<S: SceneList + 'static>(factory: impl Fn() -> S + Send + Sync + 'static) -> Self {
        Self(Arc::new(move || Box::new(factory())))
    }
    pub(crate) fn build(&self) -> Box<dyn SceneList> {
        (self.0)()
    }
}

pub(crate) fn root_reference(reference: bevy::ecs::template::EntityTemplate) -> impl Scene {
    bevy::scene::SceneFunction(move |_, scene| {
        if let bevy::ecs::template::EntityTemplate::SceneEntityReference(reference) = reference {
            scene.entity_references.push(reference);
        }
    })
}
