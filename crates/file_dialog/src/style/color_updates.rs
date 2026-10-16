use super::owners::Binding;
use super::*;
use crate::controls::LocationControl;
use crate::view::EntryRow;
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::ui::Pressed;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Resource, Default)]
struct Changes {
    inputs: EntityHashSet,
    palettes: EntityHashSet,
    states: EntityHashSet,
}

#[derive(Component, Default)]
struct DialogColors {
    colors: Option<bevy_widgetry_theme::WidgetryFileDialogColors>,
    parts: EntityHashSet,
    dirty: EntityHashSet,
    selected: Arc<BTreeSet<WidgetryFileDialogEntryId>>,
    active: Option<WidgetryFileDialogEntryId>,
    path: Option<PathBuf>,
}

#[derive(Component)]
struct RowColorBinding(EntryRow);

fn row_binding(mut row: EntryRow) -> EntryRow {
    row.token.edit_revision = 0;
    row.token.selection_revision = 0;
    row
}

#[cfg(test)]
thread_local! {
    pub(super) static COLOR_WORK: std::cell::RefCell<Vec<Entity>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn collect(
    inputs: Query<
        Entity,
        Or<(
            Changed<Binding>,
            Changed<Control>,
            Changed<Part>,
            Changed<LocationControl>,
            Changed<crate::controls::UiControl>,
            Changed<Hovered>,
            Changed<bevy::ui_widgets::ScrollbarDragState>,
            Added<Pressed>,
            Added<InteractionDisabled>,
            Changed<bevy::ui::Checked>,
            Changed<Children>,
            Changed<Visibility>,
            Or<(
                Added<BackgroundColor>,
                Added<BorderColor>,
                Added<ResolvedForeground>,
                Added<TextColor>,
            )>,
        )>,
    >,
    rows: Query<(Entity, &EntryRow, Option<&RowColorBinding>), Changed<EntryRow>>,
    palettes: Query<
        Entity,
        Or<(
            Added<WidgetryFileDialog>,
            Changed<crate::colors::ColorState>,
            Added<Shell>,
        )>,
    >,
    states: Query<Entity, Changed<WidgetryFileDialogState>>,
    focus: Res<InputFocus>,
    mut previous_focus: Local<Option<Entity>>,
    theme: Res<bevy_widgetry_theme::WidgetryThemeMode>,
    roots: Query<Entity, With<WidgetryFileDialog>>,
    mut changes: ResMut<Changes>,
) {
    changes.inputs.extend(&inputs);
    for (entity, row, binding) in &rows {
        if binding.is_none_or(|binding| binding.0 != row_binding(*row)) {
            changes.inputs.insert(entity);
        }
    }
    changes.palettes.extend(&palettes);
    changes.states.extend(&states);
    if theme.is_changed() {
        changes.palettes.extend(&roots);
    }
    if *previous_focus != focus.get() {
        changes.inputs.extend(*previous_focus);
        changes.inputs.extend(focus.get());
        *previous_focus = focus.get();
    }
}

fn removed(
    event: On<
        Remove<(
            Hovered,
            Pressed,
            InteractionDisabled,
            bevy::ui::Checked,
            bevy::ui_widgets::ScrollbarDragState,
            BackgroundColor,
            BorderColor,
            ResolvedForeground,
            TextColor,
        )>,
    >,
    mut changes: ResMut<Changes>,
) {
    changes.inputs.insert(event.entity);
}

fn unbound(
    event: On<Discard<Binding>>,
    bindings: Query<&Binding>,
    mut styles: Query<&mut DialogColors>,
) {
    if let Ok(binding) = bindings.get(event.entity)
        && let Ok(mut style) = styles.get_mut(binding.root)
    {
        style.parts.remove(&event.entity);
        style.dirty.remove(&event.entity);
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Changes>()
        .add_observer(removed)
        .add_observer(unbound)
        .add_systems(
            PostUpdate,
            collect
                .in_set(WidgetryUiSystems::Colors)
                .before(update_colors),
        );
}

pub(super) fn invalidate_palettes(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryFileDialog>>()
        .iter(world)
        .collect::<Vec<_>>();
    world.resource_mut::<Changes>().palettes.extend(roots);
}

fn target(world: &World, entity: Entity) -> Option<(Entity, Entity)> {
    let mut current = Some(entity);
    while let Some(entity) = current {
        if let Some(binding) = world.get::<Binding>(entity) {
            return Some((binding.root, entity));
        }
        if world.get::<WidgetryFileDialog>(entity).is_some() {
            return Some((entity, entity));
        }
        current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    None
}

pub(super) fn update(world: &mut World) -> Result {
    let changes = std::mem::take(&mut *world.resource_mut::<Changes>());
    let mut affected = changes.palettes.clone();
    affected.extend(&changes.states);
    for entity in changes.inputs {
        let Some((root, part)) = target(world, entity) else {
            continue;
        };
        if world.get::<WidgetryFileDialog>(root).is_none() {
            continue;
        }
        if world.get::<DialogColors>(root).is_none() {
            world.entity_mut(root).insert(DialogColors::default());
        }
        let mut style = world
            .get_mut::<DialogColors>(root)
            .ok_or_else(|| contract_error("FileDialog color state missing"))?;
        style.parts.insert(part);
        style.dirty.insert(part);
        affected.insert(root);
    }
    let mut first_error = None;
    for root in affected {
        if world.get::<WidgetryFileDialog>(root).is_none() {
            continue;
        }
        if world.get::<DialogColors>(root).is_none() {
            world.entity_mut(root).insert(DialogColors::default());
        }
        let result = apply_pending(world, root, changes.palettes.contains(&root));
        if let Err(error) = result {
            world.resource_mut::<Changes>().palettes.insert(root);
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn apply_pending(world: &mut World, root: Entity, palette: bool) -> Result {
    let resolve = palette
        || world
            .get::<DialogColors>(root)
            .is_none_or(|style| style.colors.is_none());
    if resolve {
        let theme = world
            .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
            .colors()
            .file_dialog;
        let colors = crate::WidgetryFileDialogColorOverrides::get(world, root)?.resolve(&theme);
        let mut style = world
            .get_mut::<DialogColors>(root)
            .ok_or_else(|| contract_error("FileDialog color state missing"))?;
        if style.colors != Some(colors) {
            style.colors = Some(colors);
            let parts = style.parts.clone();
            style.dirty.extend(parts);
            style.dirty.insert(root);
        }
    }
    let state = world.get::<WidgetryFileDialogState>(root).cloned();
    if let Some(state) = state {
        let style = world
            .get::<DialogColors>(root)
            .ok_or_else(|| contract_error("FileDialog color state missing"))?;
        let selection_changed = !Arc::ptr_eq(&style.selected, &state.selected);
        let active_changed = style.active != state.active();
        let path_changed = style.path.as_deref() != state.current_path();
        if selection_changed || active_changed || path_changed {
            let dirty = style
                .parts
                .iter()
                .copied()
                .filter(|entity| {
                    if let Some(row) = world.get::<EntryRow>(*entity) {
                        return (selection_changed
                            && style.selected.contains(&row.id)
                                != state.selected().contains(&row.id))
                            || (active_changed
                                && (style.active == Some(row.id)
                                    || state.active() == Some(row.id)));
                    }
                    path_changed && world.get::<LocationControl>(*entity).is_some()
                })
                .collect::<Vec<_>>();
            let mut style = world
                .get_mut::<DialogColors>(root)
                .ok_or_else(|| contract_error("FileDialog color state missing"))?;
            style.dirty.extend(dirty);
            style.selected = state.selected.clone();
            style.active = state.active();
            style.path = state.current_path.clone();
        }
    }
    let style = world
        .get::<DialogColors>(root)
        .ok_or_else(|| contract_error("FileDialog color state missing"))?;
    let colors = style
        .colors
        .ok_or_else(|| contract_error("FileDialog palette missing"))?;
    let targets = style.dirty.iter().copied().collect::<Vec<_>>();
    let mut first_error = None;
    for entity in targets {
        if target(world, entity) != Some((root, entity)) {
            continue;
        }
        #[cfg(test)]
        COLOR_WORK.with_borrow_mut(|work| work.push(entity));
        match apply_part(world, entity, root, &colors) {
            Ok(()) => {
                world
                    .get_mut::<DialogColors>(root)
                    .ok_or_else(|| contract_error("FileDialog color state missing"))?
                    .dirty
                    .remove(&entity);
            }
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn apply_part(
    world: &mut World,
    entity: Entity,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryFileDialogColors,
) -> Result {
    if entity == root {
        if world.get::<crate::window::Independent>(root).is_some() {
            bevy_widgetry_window::internal::apply_owned_window_colors(world, root, &colors.window)?;
        }
        let host = crate::window::content_host(world, root);
        if world.get::<Shell>(root).is_some() {
            let body = if world.get::<InteractionDisabled>(host).is_some() {
                colors.body.disabled
            } else {
                colors.body.normal
            };
            world
                .get_mut::<BackgroundColor>(host)
                .ok_or_else(|| contract_error("FileDialog background missing"))?
                .set_if_neq(BackgroundColor(body.background));
            world
                .get_mut::<ResolvedForeground>(host)
                .ok_or_else(|| contract_error("FileDialog foreground missing"))?
                .set_if_neq(ResolvedForeground(body.foreground));
        }
    }
    if let Some(action) = world
        .get::<Control>(entity)
        .map(|control| control.action.clone())
    {
        let button = match action {
            WidgetryFileDialogAction::Confirm => colors.accept_button,
            WidgetryFileDialogAction::Cancel => colors.cancel_button,
            _ => colors.toolbar_button,
        };
        let state = button_state(world, entity, &button);
        bevy_widgetry_button::internal::apply_owned_button_colors(world, entity, state)?;
    }
    if let Some(kind) = world.get::<Part>(entity).map(|part| part.kind) {
        let field = match kind {
            PartKind::Path => Some(colors.path_field),
            PartKind::Search => Some(colors.search_field),
            PartKind::Filename => Some(colors.filename_field),
            PartKind::Folder => Some(colors.folder_name_field),
            _ => None,
        };
        if let Some(field) = field {
            bevy_widgetry_text_field::internal::apply_owned_text_field_colors(
                world, entity, &field,
            )?;
        }
        let scroll = match kind {
            PartKind::Sidebar => Some(colors.sidebar_scroll),
            PartKind::Entries => Some(colors.entries_scroll),
            _ => None,
        };
        if let Some(scroll) = scroll {
            bevy_widgetry_scroll_area::internal::apply_owned_scroll_colors(world, entity, &scroll)?;
        }
        if kind == PartKind::Status {
            let state = if world.get::<InteractionDisabled>(entity).is_some() {
                colors.status.disabled
            } else {
                colors.status.normal
            };
            if world.get::<ResolvedForeground>(entity)
                != Some(&ResolvedForeground(state.foreground))
            {
                world
                    .entity_mut(entity)
                    .insert(ResolvedForeground(state.foreground));
            }
        }
    }
    crate::controls::apply_color(world, entity, root, colors)?;
    crate::view::update_entry_color(world, entity, root, colors)?;
    if let Some(row) = world.get::<EntryRow>(entity).copied() {
        world
            .entity_mut(entity)
            .insert(RowColorBinding(row_binding(row)));
    }
    if world.get::<crate::window::OverwriteOwner>(entity).is_some() {
        bevy_widgetry_message_box::internal::apply_owned_message_box_colors(
            world,
            entity,
            &colors.confirmation,
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn settled_dialogs_do_no_color_work() {
        let (mut app, _) = super::super::tests::fixture();
        COLOR_WORK.with_borrow_mut(Vec::clear);
        for _ in 0..3 {
            app.update();
        }
        COLOR_WORK.with_borrow(|work| assert!(work.is_empty(), "{work:?}"));
    }

    #[test]
    fn local_hover_and_removal_only_repaint_the_control() {
        let (mut app, root) = super::super::tests::fixture();
        let control = app
            .world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find(|(_, c)| c.root == root && c.action == WidgetryFileDialogAction::Cancel)
            .unwrap()
            .0;
        COLOR_WORK.with_borrow_mut(Vec::clear);
        app.add_systems(
            PostUpdate,
            (move |mut commands: Commands| {
                commands.entity(control).insert(Hovered(true));
            })
            .before(collect)
            .run_if(bevy::ecs::schedule::common_conditions::run_once),
        );
        app.update();
        COLOR_WORK.with_borrow_mut(|work| {
            assert_eq!(*work, vec![control]);
            work.clear();
        });
        let colors = app
            .world()
            .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
            .colors()
            .file_dialog;
        assert_eq!(
            app.world().get::<BackgroundColor>(control).unwrap().0,
            colors.cancel_button.hovered.background
        );
        app.world_mut().entity_mut(control).remove::<Hovered>();
        app.update();
        COLOR_WORK.with_borrow(|work| assert_eq!(*work, vec![control]));
        assert_eq!(
            app.world().get::<BackgroundColor>(control).unwrap().0,
            colors.cancel_button.normal.background
        );
    }
    #[test]
    fn selection_only_repaints_changed_rows_and_rebinding_initializes_the_new_identity() {
        let (mut app, root) = super::super::tests::fixture();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let token = state.token();
        let path = PathBuf::from("C:/fixture");
        let data = (0..3)
            .map(|index| {
                let name = format!("file_{index}.txt");
                WidgetryFileDialogEntryData {
                    path: path.join(&name),
                    name: name.into(),
                    kind: WidgetryFileDialogEntryKind::File,
                    size: None,
                    modified: None,
                    hidden: Some(false),
                    system: Some(false),
                }
            })
            .collect();
        let snapshot = Arc::new(
            WidgetryFileDialogSnapshot::prepare(token, path.clone(), data, &state.query(), None)
                .unwrap(),
        );
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started { token, path },
        )
        .unwrap();
        let selection = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .projection_selection_job(snapshot.clone())
            .unwrap()
            .prepare()
            .unwrap();
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Snapshot {
                snapshot,
                state: WidgetryFileDialogDirectoryState::Ready,
                selection,
            },
        )
        .unwrap();
        for _ in 0..4 {
            app.update();
        }
        let rows = app
            .world_mut()
            .query::<(Entity, &EntryRow)>()
            .iter(app.world())
            .map(|(entity, row)| (entity, *row))
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 3);
        COLOR_WORK.with_borrow_mut(Vec::clear);
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Select {
                id: rows[0].1.id,
                token,
                operation: WidgetryFileDialogSelection::Replace,
            },
        )
        .unwrap();
        let job = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selection_job()
            .unwrap();
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Selection(job.prepare().unwrap()),
        )
        .unwrap();
        app.update();
        COLOR_WORK.with_borrow_mut(|work| {
            let row_work = work
                .iter()
                .filter(|entity| app.world().get::<EntryRow>(**entity).is_some())
                .copied()
                .collect::<Vec<_>>();
            assert_eq!(row_work, vec![rows[0].0]);
            work.clear();
        });
        let colors = app
            .world()
            .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
            .colors()
            .file_dialog;
        assert_eq!(
            app.world().get::<BackgroundColor>(rows[0].0).unwrap().0,
            colors.entry.selected.background
        );
        let reused = rows[0].0;
        let binding = EntryRow {
            id: rows[1].1.id,
            ..rows[0].1
        };
        app.add_systems(
            PostUpdate,
            (move |mut commands: Commands| {
                commands.entity(reused).insert(binding);
            })
            .before(collect)
            .run_if(bevy::ecs::schedule::common_conditions::run_once),
        );
        app.update();
        COLOR_WORK.with_borrow(|work| assert!(work.contains(&rows[0].0)));
        assert_eq!(
            app.world().get::<BackgroundColor>(rows[0].0).unwrap().0,
            colors.entry.normal.background
        );
        app.update();
        COLOR_WORK.with_borrow_mut(Vec::clear);
        for _ in 0..3 {
            app.update();
        }
        COLOR_WORK.with_borrow(|work| assert!(work.is_empty(), "{work:?}"));
    }
    #[test]
    fn owned_thumb_drag_start_end_and_removal_update_without_hover_changes() {
        use bevy::ui_widgets::{ScrollbarDragState, ScrollbarThumb};
        let (mut app, root) = super::super::tests::fixture();
        let area = app.world().get::<Shell>(root).unwrap().parts[&PartKind::Entries];
        let thumb = app
            .world_mut()
            .query_filtered::<Entity, With<ScrollbarThumb>>()
            .iter(app.world())
            .find(|entity| target(app.world(), *entity) == Some((root, area)))
            .unwrap();
        let mut colors = WidgetryFileDialogColorOverrides::default();
        colors.entries_scroll.vertical.thumb.hovered.background = Some(Color::srgb(0.1, 0.2, 0.3));
        colors.entries_scroll.vertical.thumb.dragged.background = Some(Color::srgb(0.6, 0.2, 0.3));
        WidgetryFileDialogColorOverrides::set_in_world(app.world_mut(), root, colors).unwrap();
        app.world_mut()
            .entity_mut(thumb)
            .insert((Hovered(true), ScrollbarDragState::default()));
        app.world_mut().run_schedule(PostUpdate);
        let hovered = app.world().get::<BackgroundColor>(thumb).unwrap().0;
        assert_eq!(hovered, Color::srgb(0.1, 0.2, 0.3));
        COLOR_WORK.with_borrow_mut(Vec::clear);
        app.world_mut()
            .get_mut::<ScrollbarDragState>(thumb)
            .unwrap()
            .dragging = true;
        app.world_mut().run_schedule(PostUpdate);
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            Color::srgb(0.6, 0.2, 0.3)
        );
        COLOR_WORK.with_borrow(|work| assert_eq!(*work, vec![area]));
        app.world_mut()
            .get_mut::<ScrollbarDragState>(thumb)
            .unwrap()
            .dragging = false;
        app.world_mut().run_schedule(PostUpdate);
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            hovered
        );
        app.world_mut()
            .get_mut::<ScrollbarDragState>(thumb)
            .unwrap()
            .dragging = true;
        app.world_mut().run_schedule(PostUpdate);
        app.world_mut()
            .entity_mut(thumb)
            .remove::<ScrollbarDragState>();
        app.world_mut().run_schedule(PostUpdate);
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            hovered
        );
    }
}
