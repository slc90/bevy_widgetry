use crate::WidgetryComboBox;
use crate::{
    ComboFailure, ComboPart,
    error::{self, ComboFault, RuntimeFailure},
};
use bevy::input::keyboard::KeyboardInput;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::input_target::WidgetryInputTarget;
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListProjection, WidgetryListProjectionPolicy,
    WidgetryListView, WidgetryListViewState,
};
use bevy_widgetry_text_field::WidgetryTextFieldHeadless;
use std::sync::Arc;

pub struct WidgetryComboBoxMatcher<T>(Arc<dyn Fn(&str, &T) -> bool + Send + Sync>);

impl<T> Default for WidgetryComboBoxMatcher<T> {
    fn default() -> Self {
        Self::new(|_, _| true)
    }
}

impl<T> Clone for WidgetryComboBoxMatcher<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> WidgetryComboBoxMatcher<T> {
    pub fn new(matcher: impl Fn(&str, &T) -> bool + Send + Sync + 'static) -> Self {
        Self(Arc::new(matcher))
    }
}

#[derive(Debug)]
pub struct WidgetryComboBoxSearchState {
    pub query: String,
    pub matching_items: usize,
    pub active: Option<WidgetryListItemId>,
}

#[derive(Component, FromTemplate)]
#[component(immutable)]
#[require(SearchRuntime)]
pub struct WidgetryComboBoxSearch<T: Send + Sync + 'static> {
    pub field: Entity,
    pub matcher: WidgetryComboBoxMatcher<T>,
}

#[derive(Component, Default, Clone)]
pub(crate) struct SearchEmpty;
#[derive(Component, Default, Clone)]
pub(crate) struct SearchRuntime {
    query: Option<String>,
    actions: Vec<SearchAction>,
}

#[derive(Clone)]
struct SearchAction {
    editor: Entity,
    key: Option<KeyCode>,
    edits: Vec<TextEdit>,
}

pub(crate) fn parts<T: Send + Sync + 'static>(
    world: &World,
    root: Entity,
) -> Result<(Entity, Entity, Entity), ComboFault> {
    let binding = world
        .get::<crate::WidgetryComboBoxParts>(root)
        .ok_or(ComboFault::Part {
            entity: root,
            part: ComboPart::Bindings,
        })?;
    let search = world
        .get::<WidgetryComboBoxSearch<T>>(root)
        .ok_or(ComboFault::Part {
            entity: root,
            part: ComboPart::SearchMatcher,
        })?;
    if !crate::headless::contains(world, binding.popup, search.field)
        || crate::headless::owner(world, search.field) != Some(root)
    {
        return Err(ComboFault::Binding {
            entity: search.field,
            part: ComboPart::SearchField,
        });
    }
    Ok((binding.popup, search.field, binding.list))
}

pub(crate) fn binding<T: Send + Sync + 'static>(
    matcher: Option<WidgetryComboBoxMatcher<T>>,
    field: bevy::ecs::template::EntityTemplate,
) -> Option<impl Scene> {
    matcher.map(|matcher| bsn! { WidgetryComboBoxSearch::<T> {field:{field},matcher:{matcher}} })
}

impl<T: Send + Sync + 'static> WidgetryComboBox<T> {
    pub fn search_state(
        world: &World,
        root: Entity,
    ) -> Result<WidgetryComboBoxSearchState, ComboFailure> {
        let (list, _) = crate::combo_box::update_list::<T>(world, root)?;
        let (_, field, _) = parts::<T>(world, root).map_err(|fault| {
            error::failure(fault.into_error::<T>(root), "combo_box.search_state")
        })?;
        Ok(WidgetryComboBoxSearchState {
            query: WidgetryTextFieldHeadless::text(world, field).map_err(|error| {
                error::text(error, root, "combo_box.search").with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("combo_box.text_field.text"),
                )
            })?,
            matching_items: world
                .get::<WidgetryListProjection>(list)
                .map_or(0, WidgetryListProjection::len),
            active: world
                .get::<WidgetryListViewState>(list)
                .and_then(|state| state.active),
        })
    }
}

fn refresh<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), RuntimeFailure> {
    let (_, field, list) = parts::<T>(world, root)?;
    let query = WidgetryTextFieldHeadless::text(world, field).map_err(|error| {
        error::text(error, root, "combo_box.search").with_context(
            bevy_widgetry_core::scene::ErrorFrame::new("combo_box.text_field.text"),
        )
    })?;
    let current = world.get::<SearchRuntime>(root).ok_or(ComboFault::Part {
        entity: root,
        part: ComboPart::SearchRuntime,
    })?;
    if current.query.as_ref() != Some(&query) {
        let matcher = world
            .get::<WidgetryComboBoxSearch<T>>(root)
            .map(|search| &search.matcher)
            .ok_or(ComboFault::Part {
                entity: root,
                part: ComboPart::SearchMatcher,
            })?
            .clone();
        let captured = query.clone();
        WidgetryListView::<T>::set_projection_in_world(
            world,
            list,
            Some(WidgetryListProjectionPolicy::new(
                move |world, source, ids| {
                    if !captured.is_empty()
                        && let Some(model) = world.get::<WidgetryListModel<T>>(source)
                    {
                        ids.retain(|id| {
                            model
                                .get_by_id(*id)
                                .is_some_and(|item| (matcher.0)(&captured, item))
                        });
                    }
                },
            )),
        )
        .map_err(|error| error::list(error, root, "combo_box.search"))?;
        world
            .get_mut::<SearchRuntime>(root)
            .ok_or(ComboFault::Part {
                entity: root,
                part: ComboPart::SearchRuntime,
            })?
            .query = Some(query);
    }
    let editor = world
        .get::<WidgetryInputTarget>(field)
        .ok_or(ComboFault::Part {
            entity: field,
            part: ComboPart::InputTarget,
        })?
        .target;
    if world
        .get::<bevy_widgetry_list_view::internal::NavigationFocusOwner>(list)
        .is_none_or(|owner| owner.0 != editor)
    {
        world
            .entity_mut(list)
            .insert(bevy_widgetry_list_view::internal::NavigationFocusOwner(
                editor,
            ));
    }
    bevy_widgetry_list_view::internal::refresh_projection::<T, WidgetryListItemId>(world, list);
    let active = world
        .get::<WidgetryListViewState>(list)
        .and_then(|state| state.active);
    let source = world
        .get::<WidgetryListView<T>>(list)
        .map(WidgetryListView::source);
    let valid = active.is_some_and(|id| {
        world
            .get::<WidgetryListProjection>(list)
            .is_some_and(|projection| projection.index_of(id).is_some())
            && source
                .and_then(|source| world.get::<WidgetryListModel<T>>(source))
                .and_then(|model| {
                    model
                        .index_of(id)
                        .and_then(|index| model.is_disabled(index))
                })
                .is_some_and(|disabled| !disabled)
    });
    if !valid {
        navigate::<T>(world, root, list, KeyCode::Home)?;
    }
    Ok(())
}

pub(crate) fn open<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), RuntimeFailure> {
    let (_, field, list) = parts::<T>(world, root)?;
    if !world
        .get::<crate::headless::OpenState>(root)
        .is_some_and(|state| state.0)
    {
        return Ok(());
    }
    WidgetryTextFieldHeadless::set_text_in_world(world, field, "").map_err(|error| {
        error::text(error, root, "combo_box.search").with_context(
            bevy_widgetry_core::scene::ErrorFrame::new("combo_box.text_field.set_text_in_world"),
        )
    })?;
    refresh::<T>(world, root)?;
    // Resolve the fresh projection through the existing navigation entry point.
    bevy_widgetry_list_view::internal::set_active::<T, WidgetryListItemId>(world, list, None)
        .map_err(|error| error::list(error, root, "combo_box.search"))?;
    navigate::<T>(world, root, list, KeyCode::Home)?;
    WidgetryTextFieldHeadless::focus_in_world(world, field)
        .map_err(|error| {
            error::text(error, root, "combo_box.search").with_context(
                bevy_widgetry_core::scene::ErrorFrame::new("combo_box.focus_search")
                    .field("root", root)
                    .field("field", field),
            )
        })
        .map_err(RuntimeFailure::from)
}

pub(crate) fn on_key<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    fields: Query<&WidgetryInputTarget>,
    roots: Query<(Entity, &WidgetryComboBoxSearch<T>)>,
    editors: Query<&EditableText>,
    mut commands: Commands,
) {
    let editor = event.original_event_target();
    if event.focused_entity != editor
        || !event.input.state.is_pressed()
        || !editors.contains(editor)
    {
        return;
    }
    let Some((root, _)) = roots.iter().find(|(_, search)| {
        fields
            .get(search.field)
            .is_ok_and(|target| target.target == editor)
    }) else {
        return;
    };
    let key = (navigation_key(event.input.key_code)
        && editors.get(editor).is_ok_and(|edit| !edit.is_composing()))
    .then_some(event.input.key_code);
    if key.is_some() {
        event.propagate(false);
    }
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if world.get::<WidgetryComboBoxSearch<T>>(root).is_none() { return Ok(()); }
        let result = (|| -> Result<(),RuntimeFailure> {
        parts::<T>(world, root)?;
        // Observer commands flush after this event's native handlers, before the next input event.
        // Capture the official edit queue here so confirmation cannot overtake or absorb later text.
        let mut edits = std::mem::take(
            &mut world.get_mut::<EditableText>(editor)
                .ok_or(ComboFault::Part{entity:editor,part:ComboPart::EditableText})?.pending_edits,
        );
        if key.is_some() {
            edits.retain(|edit| {
                !navigation_edit(edit)
                    && !(key == Some(KeyCode::Enter)
                        && matches!(edit, TextEdit::Insert(text) if matches!(text.as_str(), "\n" | "\r" | "\r\n")))
            });
        }
        let focus = world.resource::<InputFocus>().get();
        if focus != Some(editor) && !(key == Some(KeyCode::Escape) && focus.is_none()) {
            return Ok(());
        }
        world
            .get_mut::<SearchRuntime>(root)
            .ok_or(ComboFault::Part{entity:root,part:ComboPart::SearchRuntime})?
            .actions
            .push(SearchAction { editor, key, edits });
        Ok(())
        })();
        result.map_err(|failure|failure.into_bevy_error::<T>(root,"combo_box.search_key"))
    });
}

fn navigation_key(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::ArrowDown
            | KeyCode::ArrowUp
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageDown
            | KeyCode::PageUp
            | KeyCode::Enter
            | KeyCode::Escape
    )
}

fn navigation_edit(edit: &TextEdit) -> bool {
    matches!(
        edit,
        TextEdit::Up(_)
            | TextEdit::Down(_)
            | TextEdit::LineStart(_)
            | TextEdit::LineEnd(_)
            | TextEdit::HardLineStart(_)
            | TextEdit::HardLineEnd(_)
            | TextEdit::TextStart(_)
            | TextEdit::TextEnd(_)
            | TextEdit::ScrollByLines(_)
    )
}

pub(crate) fn contain_keys<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    fields: Query<&WidgetryInputTarget>,
    searches: Query<&WidgetryComboBoxSearch<T>>,
) {
    if searches
        .iter()
        .any(|search| search.field == event.focused_entity)
        && fields
            .get(event.focused_entity)
            .is_ok_and(|target| target.target == event.original_event_target())
        && navigation_key(event.input.key_code)
    {
        // Native TextInput may re-enable bubbling at the editor. Its parent has no competing editor observer.
        event.propagate(false);
    }
}

fn apply_edits(
    world: &mut World,
    root: Entity,
    editor: Entity,
    edits: Vec<TextEdit>,
) -> Result<(), RuntimeFailure> {
    let has_paste = world
        .get::<EditableText>(editor)
        .is_some_and(|edit| edit.pending_paste.is_some());
    if edits.is_empty() && !has_paste {
        return Ok(());
    }
    let field = bevy_widgetry_text_field::WidgetryTextFieldEditor::owner(world, editor)
        .map_err(|error| error::text(error, root, "combo_box.search_editor"))?;
    world
        .get_mut::<EditableText>(editor)
        .ok_or(ComboFault::Part {
            entity: editor,
            part: ComboPart::EditableText,
        })?
        .pending_edits
        .extend(edits);
    WidgetryTextFieldHeadless::apply_pending_edits_in_world(world, field)
        .map_err(|error| RuntimeFailure::from(error::text(error, root, "combo_box.search_edits")))
}

fn navigate<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    list: Entity,
    key: KeyCode,
) -> Result<(), RuntimeFailure> {
    // set_active refreshes the current projection without changing Selection.
    let active = world
        .get::<WidgetryListViewState>(list)
        .and_then(|state| state.active);
    bevy_widgetry_list_view::internal::refresh_projection::<T, WidgetryListItemId>(world, list);
    let projection = world
        .get::<WidgetryListProjection>(list)
        .ok_or(ComboFault::Part {
            entity: list,
            part: ComboPart::ListProjection,
        })?;
    let source = world
        .get::<WidgetryListView<T>>(list)
        .ok_or(ComboFault::Part {
            entity: list,
            part: ComboPart::ListView,
        })?
        .source();
    let model = world
        .get::<WidgetryListModel<T>>(source)
        .ok_or(ComboFault::Source { source })?;
    let enabled = projection
        .ordered_ids()
        .iter()
        .enumerate()
        .filter_map(|(index, id)| {
            model
                .index_of(*id)
                .filter(|raw| model.is_disabled(*raw) == Some(false))
                .map(|_| (index, *id))
        })
        .collect::<Vec<_>>();
    let position = active.and_then(|id| enabled.iter().position(|(_, candidate)| *candidate == id));
    let last = enabled.len().saturating_sub(1);
    let position = match key {
        KeyCode::Home => 0,
        KeyCode::End => last,
        KeyCode::ArrowUp | KeyCode::PageUp => {
            position.map_or(last, |index| index.checked_sub(1).unwrap_or(last))
        }
        _ => position.map_or(0, |index| if index == last { 0 } else { index + 1 }),
    };
    bevy_widgetry_list_view::internal::set_active::<T, WidgetryListItemId>(
        world,
        list,
        enabled.get(position).map(|(index, _)| *index),
    )
    .map_err(|error| RuntimeFailure::from(error::list(error, root, "combo_box.search")))
}

pub(crate) fn sync<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryComboBoxSearch<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        if world.get::<WidgetryComboBoxSearch<T>>(root).is_none()
            || world
                .get::<crate::WidgetryComboBoxHeadless<T>>(root)
                .is_none()
        {
            continue;
        }
        let result = sync_root::<T>(world, root);
        let failure = match result {
            Ok(()) => {
                if let Some(mut diagnostics) =
                    world.get_mut::<crate::combo_box::ComboDiagnostics>(root)
                {
                    diagnostics
                        .search
                        .recover(|| error::frame(root, "combo_box.search_sync"));
                }
                None
            }
            Err(RuntimeFailure::Fault(fault)) => Some(
                match world.get_mut::<crate::combo_box::ComboDiagnostics>(root) {
                    Some(mut diagnostics) => error::persistent::<T>(
                        &mut diagnostics.search,
                        fault,
                        root,
                        "combo_box.search_sync",
                    )
                    .into_bevy_error(),
                    None => WidgetryEcsError::new(error::failure(
                        fault.into_error::<T>(root),
                        "combo_box.search_sync",
                    ))
                    .into_bevy_error(),
                },
            ),
            Err(failure) => Some(failure.into_bevy_error::<T>(root, "combo_box.search_sync")),
        };
        if let Some(failure) = failure {
            world
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(failure) });
        }
    }
}
fn sync_root<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), RuntimeFailure> {
    let (_, field, list) = parts::<T>(world, root)?;
    let editor = world
        .get::<WidgetryInputTarget>(field)
        .ok_or(ComboFault::Part {
            entity: field,
            part: ComboPart::InputTarget,
        })?
        .target;
    let tail = std::mem::take(
        &mut world
            .get_mut::<EditableText>(editor)
            .ok_or(ComboFault::Part {
                entity: editor,
                part: ComboPart::EditableText,
            })?
            .pending_edits,
    );
    let actions = std::mem::take(
        &mut world
            .get_mut::<SearchRuntime>(root)
            .ok_or(ComboFault::Part {
                entity: root,
                part: ComboPart::SearchRuntime,
            })?
            .actions,
    );
    for action in actions {
        let editor = action.editor;
        if !world
            .get::<crate::headless::OpenState>(root)
            .is_some_and(|state| state.0)
            || [root, field, editor, list]
                .iter()
                .any(|entity| world.get::<InteractionDisabled>(*entity).is_some())
        {
            break;
        }
        apply_edits(world, root, editor, action.edits)?;
        let Some(key) = action.key else { continue };
        if world
            .get::<EditableText>(editor)
            .is_some_and(EditableText::is_composing)
        {
            continue;
        }
        refresh::<T>(world, root)?;
        match key {
            KeyCode::Escape => {
                crate::headless::close::<T>(world, root)?;
                let field = world
                    .get::<crate::WidgetryComboBoxParts>(root)
                    .map(|parts| parts.trigger)
                    .ok_or(ComboFault::Part {
                        entity: root,
                        part: ComboPart::Field,
                    })?;
                world
                    .resource_mut::<InputFocus>()
                    .set(field, FocusCause::Navigated);
            }
            KeyCode::Enter => {
                if let Some(active) = world
                    .get::<WidgetryListViewState>(list)
                    .and_then(|state| state.active)
                {
                    let valid = world
                        .get::<WidgetryListProjection>(list)
                        .is_some_and(|projection| projection.index_of(active).is_some())
                        && world
                            .get::<crate::WidgetryComboBoxHeadless<T>>(root)
                            .and_then(|combo| world.get::<WidgetryListModel<T>>(combo.source()))
                            .and_then(|model| {
                                model
                                    .index_of(active)
                                    .and_then(|index| model.is_disabled(index))
                            })
                            .is_some_and(|disabled| !disabled);
                    if valid && !crate::popup::confirmation_disabled(world, list, active) {
                        WidgetryListView::<T>::select_id_in_world(world, list, active)
                            .map_err(|error| error::list(error, root, "combo_box.search"))?;
                        // Closing flushes notifications whose consumers can focus elsewhere, reopen, or despawn.
                        // Finish this input batch before that flush and never resume its old actions afterward.
                        if let Some(mut edit) = world.get_mut::<EditableText>(editor) {
                            edit.pending_edits.clear();
                            edit.pending_paste = None;
                        }
                        crate::headless::close::<T>(world, root)?;
                        return Ok(());
                    }
                }
            }
            _ => navigate::<T>(world, root, list, key)?,
        }
    }
    if world
        .get::<crate::headless::OpenState>(root)
        .is_some_and(|state| state.0)
        && ![root, field, list, editor]
            .iter()
            .any(|entity| world.get::<InteractionDisabled>(*entity).is_some())
    {
        apply_edits(world, root, editor, tail)?;
    } else if let Some(mut edit) = world.get_mut::<EditableText>(editor) {
        edit.pending_edits.clear();
        edit.pending_paste = None;
    }
    refresh::<T>(world, root)?;
    Ok(())
}

impl<T: Send + Sync + 'static> WidgetryComboBoxSearch<T> {
    pub fn state(world: &World, root: Entity) -> Result<WidgetryComboBoxSearchState, ComboFailure> {
        WidgetryComboBox::<T>::search_state(world, root)
    }
}
