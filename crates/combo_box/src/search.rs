use crate::{WidgetryComboBox, combo_box::update_error, popup::ComboBoxPopup};
use bevy::input::keyboard::KeyboardInput;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, FontCx, LayoutCx, TextEdit};
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::input_target::WidgetryInputTarget;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListProjection, WidgetryListProjectionPolicy,
    WidgetryListView, WidgetryListViewState,
};
use bevy_widgetry_text_field::WidgetryTextField;
use std::sync::Arc;

pub struct WidgetryComboBoxMatcher<T>(Arc<dyn Fn(&str, &T) -> bool + Send + Sync>);

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

#[derive(Component, Default, Clone)]
pub(crate) struct SearchField;
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
) -> Option<(Entity, Entity, Entity)> {
    let children = world.get::<Children>(root)?;
    let popup = children
        .iter()
        .find(|&child| world.get::<ComboBoxPopup>(child).is_some())?;
    let children = world.get::<Children>(popup)?;
    let field = children
        .iter()
        .find(|&child| world.get::<SearchField>(child).is_some())?;
    let list = children
        .iter()
        .find(|&child| world.get::<WidgetryListView<T>>(child).is_some())?;
    Some((popup, field, list))
}

impl<T: Send + Sync + 'static> WidgetryComboBox<T> {
    pub fn search_state(
        world: &World,
        root: Entity,
    ) -> Result<WidgetryComboBoxSearchState, BevyError> {
        let (list, _) = crate::combo_box::update_list::<T>(world, root)?;
        let (_, field, _) = parts::<T>(world, root)
            .ok_or_else(|| update_error(root, "未启用搜索或搜索部件缺失"))?;
        Ok(WidgetryComboBoxSearchState {
            query: WidgetryTextField::text(world, field)?,
            matching_items: world
                .get::<WidgetryListProjection>(list)
                .map_or(0, WidgetryListProjection::len),
            active: world
                .get::<WidgetryListViewState>(list)
                .and_then(|state| state.active),
        })
    }
}

fn refresh<T: Send + Sync + 'static>(world: &mut World, root: Entity) -> Result<(), BevyError> {
    let (_, field, list) =
        parts::<T>(world, root).ok_or_else(|| update_error(root, "搜索部件缺失"))?;
    let query = WidgetryTextField::text(world, field)?;
    let current = world
        .get::<SearchRuntime>(field)
        .ok_or_else(|| update_error(root, "搜索 Runtime 缺失"))?;
    if current.query.as_ref() != Some(&query) {
        let matcher = world
            .get::<WidgetryComboBox<T>>(root)
            .and_then(WidgetryComboBox::search)
            .ok_or_else(|| update_error(root, "搜索 Matcher 缺失"))?
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
        )?;
        world
            .get_mut::<SearchRuntime>(field)
            .ok_or_else(|| update_error(root, "搜索 Runtime 缺失"))?
            .query = Some(query);
    }
    let editor = world
        .get::<WidgetryInputTarget>(field)
        .ok_or_else(|| update_error(root, "搜索编辑器缺失"))?
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
        navigate::<T>(world, list, KeyCode::Home)?;
    }
    Ok(())
}

pub(crate) fn open<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), BevyError> {
    let (popup, field, list) =
        parts::<T>(world, root).ok_or_else(|| update_error(root, "搜索部件缺失"))?;
    if world.get::<Visibility>(popup) != Some(&Visibility::Visible) {
        return Ok(());
    }
    WidgetryTextField::set_text_in_world(world, field, "")?;
    refresh::<T>(world, root)?;
    // Resolve the fresh projection through the existing navigation entry point.
    bevy_widgetry_list_view::internal::set_active::<T, WidgetryListItemId>(world, list, None)?;
    navigate::<T>(world, list, KeyCode::Home)?;
    WidgetryTextField::focus_in_world(world, field)
}

pub(crate) fn on_key<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    fields: Query<(&ChildOf, &WidgetryInputTarget), With<SearchField>>,
    popups: Query<&ChildOf, With<ComboBoxPopup>>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
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
    let Some((parent, _)) = fields.iter().find(|(_, target)| target.target == editor) else {
        return;
    };
    let Ok(parent) = popups.get(parent.parent()) else {
        return;
    };
    let root = parent.parent();
    if !roots.contains(root) {
        return;
    }
    let key = (navigation_key(event.input.key_code)
        && editors.get(editor).is_ok_and(|edit| !edit.is_composing()))
    .then_some(event.input.key_code);
    if key.is_some() {
        event.propagate(false);
    }
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        let (_, field, _) =
            parts::<T>(world, root).ok_or_else(|| update_error(root, "搜索部件缺失"))?;
        // Observer commands flush after this event's native handlers, before the next input event.
        // Capture the official edit queue here so confirmation cannot overtake or absorb later text.
        let mut edits = std::mem::take(
            &mut world.get_mut::<EditableText>(editor)
                .ok_or_else(|| update_error(root, "搜索编辑器缺失"))?.pending_edits,
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
            .get_mut::<SearchRuntime>(field)
            .ok_or_else(|| update_error(root, "搜索 Runtime 缺失"))?
            .actions
            .push(SearchAction { editor, key, edits });
        Ok(())
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

pub(crate) fn contain_keys(
    mut event: On<FocusedInput<KeyboardInput>>,
    fields: Query<&WidgetryInputTarget, With<SearchField>>,
) {
    if fields
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
) -> Result<(), BevyError> {
    let has_paste = world
        .get::<EditableText>(editor)
        .is_some_and(|edit| edit.pending_paste.is_some());
    if edits.is_empty() && !has_paste {
        return Ok(());
    }
    if world.get_resource::<bevy::clipboard::Clipboard>().is_none() {
        return Err(update_error(root, "搜索编辑缺少 Clipboard"));
    }
    // Bevy normally applies edits after Layout, too late for this frame's row construction.
    // Use the same editor driver early; the official consumer still emits TextEditChange.
    world.resource_scope(|world, mut fonts: Mut<FontCx>| {
        world.resource_scope(|world, mut layout: Mut<LayoutCx>| {
            world.resource_scope(|world, mut clipboard: Mut<bevy::clipboard::Clipboard>| {
                let mut edit = world
                    .get_mut::<EditableText>(editor)
                    .ok_or_else(|| update_error(root, "搜索编辑器缺失"))?;
                edit.pending_edits.extend(edits);
                edit.apply_pending_edits(&mut fonts, &mut layout.0, &mut clipboard, |_| true);
                Ok(())
            })
        })
    })
}

fn navigate<T: Send + Sync + 'static>(
    world: &mut World,
    list: Entity,
    key: KeyCode,
) -> Result<(), BevyError> {
    // set_active refreshes the current projection without changing Selection.
    let active = world
        .get::<WidgetryListViewState>(list)
        .and_then(|state| state.active);
    bevy_widgetry_list_view::internal::refresh_projection::<T, WidgetryListItemId>(world, list);
    let projection = world
        .get::<WidgetryListProjection>(list)
        .ok_or_else(|| update_error(list, "Projection 缺失"))?;
    let source = world
        .get::<WidgetryListView<T>>(list)
        .ok_or_else(|| update_error(list, "ListView 缺失"))?
        .source();
    let model = world
        .get::<WidgetryListModel<T>>(source)
        .ok_or_else(|| update_error(list, "Model 缺失"))?;
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
}

pub(crate) fn sync<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query::<(Entity, &WidgetryComboBox<T>)>()
        .iter(world)
        .filter_map(|(root, combo)| combo.search().is_some().then_some(root))
        .collect::<Vec<_>>();
    for root in roots {
        let (popup, field, list) =
            parts::<T>(world, root).ok_or_else(|| update_error(root, "搜索部件缺失"))?;
        let editor = world
            .get::<WidgetryInputTarget>(field)
            .ok_or_else(|| update_error(root, "搜索编辑器缺失"))?
            .target;
        let tail = std::mem::take(
            &mut world
                .get_mut::<EditableText>(editor)
                .ok_or_else(|| update_error(root, "搜索编辑器缺失"))?
                .pending_edits,
        );
        let actions = std::mem::take(
            &mut world
                .get_mut::<SearchRuntime>(field)
                .ok_or_else(|| update_error(root, "搜索 Runtime 缺失"))?
                .actions,
        );
        for action in actions {
            let editor = action.editor;
            if world.get::<Visibility>(popup) != Some(&Visibility::Visible)
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
                    world.entity_mut(popup).insert(Visibility::Hidden);
                    let field = world
                        .get::<Children>(root)
                        .and_then(|children| children.first())
                        .copied()
                        .ok_or_else(|| update_error(root, "Field 缺失"))?;
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
                                .get::<WidgetryComboBox<T>>(root)
                                .and_then(|combo| world.get::<WidgetryListModel<T>>(combo.source()))
                                .and_then(|model| {
                                    model
                                        .index_of(active)
                                        .and_then(|index| model.is_disabled(index))
                                })
                                .is_some_and(|disabled| !disabled);
                        if valid && !crate::popup::confirmation_disabled(world, list, active) {
                            WidgetryListView::<T>::select_id_in_world(world, list, active)?;
                            world.entity_mut(popup).insert(Visibility::Hidden);
                            world.resource_mut::<InputFocus>().clear();
                        }
                    }
                }
                _ => navigate::<T>(world, list, key)?,
            }
        }
        if world.get::<Visibility>(popup) == Some(&Visibility::Visible)
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
    }
    Ok(())
}
