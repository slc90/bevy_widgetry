use crate::model::contract_error;
use crate::style::{Part, PartKind};
use crate::*;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_check_box::{WidgetryCheckBox, WidgetryCheckBoxPlugin};
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::disabled::set_intrinsic_disabled;
use bevy_widgetry_list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
use std::ffi::OsString;
use std::path::PathBuf;

struct FilterChoice(WidgetryFileDialogFilterId, String);

struct SortChoice(WidgetryFileDialogSort, &'static str);

#[derive(Component, Clone, Copy)]
struct ProjectedChoice(WidgetryListItemId);

#[derive(Component)]
struct ProjectedEditor(String);

#[derive(Component, Clone)]
struct SidebarProjection {
    locations: Vec<WidgetryFileDialogLocation>,
    pinned: Vec<PathBuf>,
    error: Option<String>,
    font: f32,
}

#[derive(Component, Clone)]
struct LocationControl {
    root: Entity,
    path: PathBuf,
}

#[derive(Component, Clone, Copy)]
struct UiControl {
    root: Entity,
    action: UiAction,
}

#[derive(Component, Clone, Copy)]
struct OverwriteDecision(WidgetryFileDialogToken);

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiAction {
    Pin,
    FolderOpen,
    FolderCreate,
    FolderCancel,
    Overwrite(bool),
}

pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<WidgetryCheckBoxPlugin>() {
        app.add_plugins(WidgetryCheckBoxPlugin);
    }
    if let Err(error) = app
        .register_widgetry_combo_box::<FilterChoice>()
        .and_then(|app| app.register_widgetry_combo_box::<SortChoice>())
    {
        let error = contract_error(&error.to_string());
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result { Err(error) });
    }
    app.add_observer(on_action)
        .add_observer(on_checked)
        .add_observer(on_filter)
        .add_observer(on_sort)
        .add_observer(on_location);
}

pub(crate) fn models(world: &mut World, root: Entity, state: &WidgetryFileDialogState) -> Result {
    let mut filters = WidgetryListModel::<FilterChoice>::default();
    for filter in state.filters() {
        filters.push(FilterChoice(filter.id.clone(), filter.label.clone()))?;
    }
    let mut sorts = WidgetryListModel::<SortChoice>::default();
    for (sort, label) in [
        (WidgetryFileDialogSort::NameAscending, "Name ↑"),
        (WidgetryFileDialogSort::NameDescending, "Name ↓"),
        (WidgetryFileDialogSort::SizeAscending, "Size ↑"),
        (WidgetryFileDialogSort::ModifiedDescending, "Modified ↓"),
    ] {
        sorts.push(SortChoice(sort, label))?;
    }
    world.entity_mut(root).insert((filters, sorts));
    Ok(())
}

fn button(root: Entity, action: UiAction, name: &'static str, label: &'static str) -> impl Scene {
    bsn! { @WidgetryButton Name(name) TabIndex::default() template(move |_| Ok(UiControl { root, action })) template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryButton>::new(root))) Children [@crate::style::text_scene(label.into(), 14.0)] }
}

pub(crate) fn toolbar(root: Entity) -> impl SceneList {
    bsn_list! {
        @button(root, UiAction::Pin, "FileDialogPin", "Pin folder")--
        @button(
            root,
            UiAction::FolderOpen,
            "FileDialogNewFolder",
            "New folder"
        )
    }
}

pub(crate) fn options(root: Entity, style: &WidgetryFileDialogStyle) -> impl Scene {
    let filter_renderer = WidgetryListViewRenderer::new(|_, choice: &FilterChoice| {
        bsn_list! {@crate::style::text_scene(choice.1.clone(), 14.0)}
    });
    let sort_renderer = WidgetryListViewRenderer::new(|_, choice: &SortChoice| {
        bsn_list! {@crate::style::text_scene(choice.1.into(), 14.0)}
    });
    bsn! { Node { width: percent(100), column_gap: px(style.spacing), align_items: AlignItems::Center } Children [
        @WidgetryComboBox::<FilterChoice> { @source: root, @item_height: 28.0, @max_visible_items: 6, @renderer: filter_renderer } template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryComboBox<FilterChoice>>::new(root))) Name("FileDialogFilter") template(move |_| Ok(Part {root, kind: PartKind::Filter})) Node { width: px(190) } TabIndex(-1)--
        @WidgetryComboBox::<SortChoice> { @source: root, @item_height: 28.0, @max_visible_items: 4, @renderer: sort_renderer } template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryComboBox<SortChoice>>::new(root))) Name("FileDialogSort") template(move |_| Ok(Part {root, kind: PartKind::Sort})) Node { width: px(140) } TabIndex(-1)--
        @WidgetryCheckBox template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryCheckBox>::new(root))) Name("FileDialogHidden") template(move |_| Ok(Part {root, kind: PartKind::Hidden})) TabIndex::default() Children [@crate::style::text_scene("Hidden".into(), 14.0)]--
        @WidgetryCheckBox template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryCheckBox>::new(root))) Name("FileDialogSystem") template(move |_| Ok(Part {root, kind: PartKind::System})) TabIndex::default() Children [@crate::style::text_scene("System".into(), 14.0)]
    ] }
}

pub(crate) fn panels(root: Entity) -> impl SceneList {
    bsn_list! {
        Name("FileDialogFolderPanel") template(move |_| Ok(Part {root, kind: PartKind::FolderPanel})) Node { display: Display::None, column_gap: px(8), width: percent(100), align_items: AlignItems::Center } Children [
            @crate::style::text_scene("New folder".into(), 14.0)-- @crate::style::editor_scene(root, PartKind::Folder, "FileDialogFolderName", String::new())--
            @button(root, UiAction::FolderCreate, "FileDialogFolderCreate", "Create")-- @button(root, UiAction::FolderCancel, "FileDialogFolderCancel", "Dismiss")
        ]--
        Name("FileDialogOverwritePanel") template(move |_| Ok(Part {root, kind: PartKind::OverwritePanel})) Node { display: Display::None, column_gap: px(8), width: percent(100), align_items: AlignItems::Center } Children [
            @crate::style::text_scene("Target exists. Replace?".into(), 14.0)-- @button(root, UiAction::Overwrite(true), "FileDialogOverwriteYes", "Replace")-- @button(root, UiAction::Overwrite(false), "FileDialogOverwriteNo", "Keep editing")
        ]
    }
}

fn part(world: &mut World, root: Entity, kind: PartKind) -> Option<Entity> {
    world
        .query::<(Entity, &Part)>()
        .iter(world)
        .find(|(_, part)| part.root == root && part.kind == kind)
        .map(|(entity, _)| entity)
}

pub(crate) fn sync(world: &mut World, root: Entity, state: &WidgetryFileDialogState) -> Result {
    sidebar(world, root, state)?;
    let interactive = state.session_state() == WidgetryFileDialogSessionState::Open;
    let locations: Vec<_> = world
        .query::<(Entity, &LocationControl)>()
        .iter(world)
        .filter(|(_, control)| control.root == root)
        .map(|(entity, _)| entity)
        .collect();
    for entity in locations {
        set_intrinsic_disabled(world, entity, !interactive);
        crate::style::sync_tab_stop(world, entity);
    }
    let controls: Vec<_> = world
        .query::<(Entity, &UiControl)>()
        .iter(world)
        .filter(|(_, control)| control.root == root)
        .map(|(entity, control)| (entity, control.action))
        .collect();
    for (entity, action) in controls {
        let available = interactive
            && match action {
                UiAction::Pin => state.current_path().is_some(),
                UiAction::FolderOpen | UiAction::FolderCreate => {
                    state.started_generation == Some(state.token().generation)
                        && state.folder_request.is_none()
                }
                UiAction::Overwrite(_) => matches!(
                    state.confirmation(),
                    WidgetryFileDialogConfirmation::AwaitingOverwrite { .. }
                ),
                UiAction::FolderCancel => true,
            };
        set_intrinsic_disabled(world, entity, !available);
        crate::style::sync_tab_stop(world, entity);
        if matches!(action, UiAction::Overwrite(_))
            && let WidgetryFileDialogConfirmation::AwaitingOverwrite { token, .. } =
                state.confirmation()
            && world
                .get::<OverwriteDecision>(entity)
                .is_none_or(|decision| decision.0 != *token)
        {
            world.entity_mut(entity).insert(OverwriteDecision(*token));
        }
    }
    let parts: Vec<_> = world
        .query::<(Entity, &Part)>()
        .iter(world)
        .filter(|(_, part)| {
            part.root == root
                && matches!(
                    part.kind,
                    PartKind::Path
                        | PartKind::Search
                        | PartKind::Filename
                        | PartKind::Folder
                        | PartKind::Filter
                        | PartKind::Sort
                        | PartKind::Hidden
                        | PartKind::System
                        | PartKind::Entries
                )
        })
        .map(|(entity, part)| (entity, part.kind))
        .collect();
    for (entity, kind) in parts {
        set_intrinsic_disabled(world, entity, !interactive);
        if matches!(kind, PartKind::Filter | PartKind::Sort) {
            world.entity_mut(entity).insert(TabIndex(-1));
            let field = world.get::<Children>(entity).and_then(|children| {
                children
                    .iter()
                    .find(|child| world.get::<WidgetryButton>(*child).is_some())
            });
            if let Some(field) = field {
                if world.get::<TabIndex>(field).is_none() {
                    world.entity_mut(field).insert(TabIndex::default());
                }
                crate::style::sync_tab_stop(world, field);
            }
        } else {
            crate::style::sync_tab_stop(world, entity);
        }
    }
    for (kind, value) in [
        (PartKind::Hidden, state.preferences().show_hidden),
        (PartKind::System, state.preferences().show_system),
    ] {
        if let Some(entity) = part(world, root, kind) {
            if value {
                if world.get::<Checked>(entity).is_none() {
                    world.entity_mut(entity).insert(Checked);
                }
            } else {
                world.entity_mut(entity).remove::<Checked>();
            }
        }
    }
    if let Some(entity) = part(world, root, PartKind::OverwritePanel) {
        world
            .get_mut::<Node>(entity)
            .ok_or_else(|| contract_error("FileDialog overwrite panel missing"))?
            .display = if matches!(
            state.confirmation(),
            WidgetryFileDialogConfirmation::AwaitingOverwrite { .. }
        ) && world.get::<crate::window::Independent>(root).is_none()
        {
            Display::Flex
        } else {
            Display::None
        };
        sync_panel_tabs(world, entity);
    }
    for (kind, text) in [
        (
            PartKind::Filename,
            state.filename().to_string_lossy().into_owned(),
        ),
        (PartKind::Search, state.search().to_owned()),
    ] {
        if let Some(entity) = part(world, root, kind) {
            let changed = world
                .get::<ProjectedEditor>(entity)
                .is_some_and(|shown| shown.0 != text);
            let mut edit = world
                .get_mut::<EditableText>(entity)
                .ok_or_else(|| contract_error("FileDialog editor missing"))?;
            if edit.is_composing() {
                continue;
            }
            if changed && edit.value().to_string() != text {
                edit.editor_mut().set_text(&text);
            }
            if changed || world.get::<ProjectedEditor>(entity).is_none() {
                world.entity_mut(entity).insert(ProjectedEditor(text));
            }
        }
    }
    let filter = part(world, root, PartKind::Filter);
    let sort = part(world, root, PartKind::Sort);
    let filter_id = world
        .get::<WidgetryListModel<FilterChoice>>(root)
        .and_then(|model| {
            (0..model.len())
                .find(|index| {
                    model
                        .get(*index)
                        .is_some_and(|choice| choice.0 == state.preferences().filter)
                })
                .and_then(|index| model.id(index))
        });
    let sort_id = world
        .get::<WidgetryListModel<SortChoice>>(root)
        .and_then(|model| {
            (0..model.len())
                .find(|index| {
                    model
                        .get(*index)
                        .is_some_and(|choice| choice.0 == state.preferences().sort)
                })
                .and_then(|index| model.id(index))
        });
    if let (Some(entity), Some(id)) = (filter, filter_id)
        && world.get::<ProjectedChoice>(entity).map(|choice| choice.0) != Some(id)
    {
        world.entity_mut(entity).insert(ProjectedChoice(id));
        WidgetryComboBox::<FilterChoice>::set_selected(&mut world.commands(), entity, id);
    }
    if let (Some(entity), Some(id)) = (sort, sort_id)
        && world.get::<ProjectedChoice>(entity).map(|choice| choice.0) != Some(id)
    {
        world.entity_mut(entity).insert(ProjectedChoice(id));
        WidgetryComboBox::<SortChoice>::set_selected(&mut world.commands(), entity, id);
    }
    Ok(())
}

fn sidebar(world: &mut World, root: Entity, state: &WidgetryFileDialogState) -> Result {
    let Some(area) = part(world, root, PartKind::Sidebar) else {
        return Ok(());
    };
    let font = world
        .get::<WidgetryFileDialogStyle>(root)
        .map(|style| style.font_size)
        .unwrap_or(14.0);
    if world
        .get::<SidebarProjection>(area)
        .is_some_and(|projection| {
            projection.locations == state.locations()
                && projection.pinned == state.preferences().pinned
                && projection.error.as_deref() == state.location_error()
                && projection.font == font
        })
    {
        return Ok(());
    }
    let viewport = world
        .get::<Children>(area)
        .and_then(|children| {
            children
                .iter()
                .find(|entity| world.get::<WidgetryScrollAreaViewport>(*entity).is_some())
        })
        .ok_or_else(|| contract_error("FileDialog sidebar viewport missing"))?;
    let content = world
        .get::<Children>(viewport)
        .and_then(|children| {
            children
                .iter()
                .find(|entity| world.get::<WidgetryScrollAreaContent>(*entity).is_some())
        })
        .ok_or_else(|| contract_error("FileDialog sidebar content missing"))?;
    let previous: Vec<_> = world
        .get::<Children>(content)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    for entity in previous {
        world.despawn(entity);
    }
    let places = state
        .locations()
        .iter()
        .map(|place| (place.name.clone(), place.path.clone()))
        .chain(
            state
                .preferences()
                .pinned
                .iter()
                .map(|path| (path.to_string_lossy().into_owned(), path.clone())),
        );
    let mut rows = Vec::new();
    for (label, path) in places {
        let scene = bsn! { @WidgetryButton Name("FileDialogLocation") TabIndex::default() template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryButton>::new(root))) template(move |_| Ok(LocationControl {root, path: path.clone()})) Node {width: percent(100), min_height: px(28), flex_shrink: 0.0} Children [@crate::style::text_scene(label, font)] };
        rows.push(
            bevy_widgetry_core::scene::spawn_scene(world, scene)
                .map_err(|error| contract_error(&error.to_string()))?,
        );
    }
    if let Some(error) = state.location_error() {
        rows.push(
            bevy_widgetry_core::scene::spawn_scene(
                world,
                crate::style::text_scene(format!("Locations unavailable: {error}"), font),
            )
            .map_err(|error| contract_error(&error.to_string()))?,
        );
    }
    world.entity_mut(content).replace_children(&rows);
    world.entity_mut(area).insert(SidebarProjection {
        locations: state.locations().to_vec(),
        pinned: state.preferences().pinned.clone(),
        error: state.location_error().map(ToOwned::to_owned),
        font,
    });
    world.write_message(bevy::window::RequestRedraw);
    Ok(())
}

fn on_location(
    event: On<Activate>,
    controls: Query<&LocationControl, Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    if let Ok(control) = controls.get(event.entity) {
        let root = control.root;
        let path = control.path.clone();
        commands.queue(move |world: &mut World| -> Result {
            if world
                .get::<WidgetryFileDialogState>(root)
                .is_some_and(|state| state.session_state() == WidgetryFileDialogSessionState::Open)
                && world.get::<InteractionDisabled>(root).is_none()
            {
                crate::style::apply_input(
                    world,
                    root,
                    WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path(path)),
                )?;
            }
            Ok(())
        });
    }
}

pub(crate) fn escape(world: &mut World, root: Entity) -> Result<bool> {
    if let Some(entity) = part(world, root, PartKind::FolderPanel)
        && world
            .get::<Node>(entity)
            .is_some_and(|node| node.display != Display::None)
    {
        close_folder(world, root)?;
        return Ok(true);
    }
    if let Some(state) = world.get::<WidgetryFileDialogState>(root)
        && let WidgetryFileDialogConfirmation::AwaitingOverwrite { token, .. } =
            state.confirmation()
    {
        let token = *token;
        crate::style::apply_input(
            world,
            root,
            WidgetryFileDialogAction::Overwrite {
                token,
                accept: false,
            },
        )?;
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn create_folder(world: &mut World, root: Entity, name: OsString) -> Result {
    crate::style::apply_input(world, root, WidgetryFileDialogAction::NewFolder(name))?;
    close_folder(world, root)
}

fn close_folder(world: &mut World, root: Entity) -> Result {
    if let Some(entity) = part(world, root, PartKind::FolderPanel) {
        world
            .get_mut::<Node>(entity)
            .ok_or_else(|| contract_error("FileDialog folder panel missing"))?
            .display = Display::None;
        sync_panel_tabs(world, entity);
    }
    if let Some(area) = part(world, root, PartKind::Entries) {
        world
            .resource_mut::<InputFocus>()
            .set(area, FocusCause::Navigated);
    }
    crate::runtime::wake(world)
}

fn sync_panel_tabs(world: &mut World, panel: Entity) {
    let mut pending = vec![panel];
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
        crate::style::sync_tab_stop(world, entity);
    }
}

fn on_checked(event: On<ValueChange<bool>>, parts: Query<&Part>, mut commands: Commands) {
    if let Ok(part) = parts.get(event.source) {
        let action = match part.kind {
            PartKind::Hidden => Some(WidgetryFileDialogAction::ShowHidden(event.value)),
            PartKind::System => Some(WidgetryFileDialogAction::ShowSystem(event.value)),
            _ => None,
        };
        if let Some(action) = action {
            crate::style::queue_input(&mut commands, part.root, action);
        }
    }
}

fn on_filter(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    parts: Query<&Part>,
    models: Query<&WidgetryListModel<FilterChoice>>,
    mut commands: Commands,
) {
    if let Ok(part) = parts.get(event.source)
        && part.kind == PartKind::Filter
        && let Some(choice) = event
            .value
            .and_then(|id| models.get(part.root).ok()?.get_by_id(id))
    {
        crate::style::queue_input(
            &mut commands,
            part.root,
            WidgetryFileDialogAction::Filter(choice.0.clone()),
        );
    }
}

fn on_sort(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    parts: Query<&Part>,
    models: Query<&WidgetryListModel<SortChoice>>,
    mut commands: Commands,
) {
    if let Ok(part) = parts.get(event.source)
        && part.kind == PartKind::Sort
        && let Some(choice) = event
            .value
            .and_then(|id| models.get(part.root).ok()?.get_by_id(id))
    {
        crate::style::queue_input(
            &mut commands,
            part.root,
            WidgetryFileDialogAction::Sort(choice.0),
        );
    }
}

fn on_action(
    event: On<Activate>,
    controls: Query<(&UiControl, Option<&OverwriteDecision>), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok((control, decision)) = controls.get(event.entity) else {
        return;
    };
    let control = *control;
    let decision = decision.copied();
    commands.queue(move |world: &mut World| -> Result {
        let Some(state) = world.get::<WidgetryFileDialogState>(control.root) else {
            return Ok(());
        };
        if state.session_state() != WidgetryFileDialogSessionState::Open
            || world.get::<InteractionDisabled>(control.root).is_some()
        {
            return Ok(());
        }
        match control.action {
            UiAction::Pin => {
                if let Some(path) = state.current_path().map(ToOwned::to_owned) {
                    let action = if state.preferences().pinned.contains(&path) {
                        WidgetryFileDialogAction::Unpin(path)
                    } else {
                        WidgetryFileDialogAction::Pin(path)
                    };
                    crate::style::apply_input(world, control.root, action)?;
                }
            }
            UiAction::FolderOpen => {
                if let Some(entity) = part(world, control.root, PartKind::FolderPanel) {
                    world
                        .get_mut::<Node>(entity)
                        .ok_or_else(|| contract_error("FileDialog folder panel missing"))?
                        .display = Display::Flex;
                    sync_panel_tabs(world, entity);
                }
                if let Some(entity) = part(world, control.root, PartKind::Folder) {
                    world
                        .resource_mut::<InputFocus>()
                        .set(entity, FocusCause::Navigated);
                }
            }
            UiAction::FolderCreate | UiAction::FolderCancel => {
                if control.action == UiAction::FolderCreate
                    && let Some(entity) = part(world, control.root, PartKind::Folder)
                {
                    let name = world
                        .get::<EditableText>(entity)
                        .ok_or_else(|| contract_error("FileDialog folder editor missing"))?
                        .value()
                        .to_string();
                    create_folder(world, control.root, name.into())?;
                }
                if control.action == UiAction::FolderCancel {
                    close_folder(world, control.root)?;
                }
            }
            UiAction::Overwrite(accept) => {
                if let Some(OverwriteDecision(token)) = decision {
                    crate::style::apply_input(
                        world,
                        control.root,
                        WidgetryFileDialogAction::Overwrite { token, accept },
                    )?;
                }
            }
        }
        crate::runtime::wake(world)
    });
}

pub(crate) fn establish_color_owners(world: &mut World) {
    use bevy_widgetry_core::color::WidgetryStyleOwner;
    let buttons = world
        .query_filtered::<(Entity, &LocationControl), Without<WidgetryStyleOwner<WidgetryButton>>>()
        .iter(world)
        .map(|(e, c)| (e, c.root))
        .collect::<Vec<_>>();
    let controls = world
        .query_filtered::<(Entity, &UiControl), Without<WidgetryStyleOwner<WidgetryButton>>>()
        .iter(world)
        .map(|(e, c)| (e, c.root))
        .collect::<Vec<_>>();
    for (entity, root) in buttons.into_iter().chain(controls) {
        world
            .entity_mut(entity)
            .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
    }
    let parts = world
        .query::<(Entity, &Part)>()
        .iter(world)
        .map(|(e, p)| (e, *p))
        .collect::<Vec<_>>();
    for (entity, part) in parts {
        match part.kind {
            PartKind::Filter => {
                if world
                    .get::<WidgetryStyleOwner<WidgetryComboBox<FilterChoice>>>(entity)
                    .is_none()
                {
                    world.entity_mut(entity).insert(WidgetryStyleOwner::<
                        WidgetryComboBox<FilterChoice>,
                    >::new(part.root));
                }
            }
            PartKind::Sort => {
                if world
                    .get::<WidgetryStyleOwner<WidgetryComboBox<SortChoice>>>(entity)
                    .is_none()
                {
                    world.entity_mut(entity).insert(WidgetryStyleOwner::<
                        WidgetryComboBox<SortChoice>,
                    >::new(part.root));
                }
            }
            PartKind::Hidden | PartKind::System
                if world
                    .get::<WidgetryStyleOwner<WidgetryCheckBox>>(entity)
                    .is_none() =>
            {
                world
                    .entity_mut(entity)
                    .insert(WidgetryStyleOwner::<WidgetryCheckBox>::new(part.root));
            }
            _ => {}
        }
    }
}
fn button_color(
    world: &World,
    entity: Entity,
    colors: bevy_widgetry_theme::WidgetryButtonColors,
) -> bevy_widgetry_theme::WidgetryButtonStateColors {
    if world.get::<InteractionDisabled>(entity).is_some() {
        colors.disabled
    } else if world.get::<bevy::ui::Pressed>(entity).is_some() {
        colors.pressed
    } else if world
        .get::<bevy::picking::hover::Hovered>(entity)
        .is_some_and(|h| h.0)
    {
        colors.hovered
    } else {
        colors.normal
    }
}
pub(crate) fn apply_colors(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryFileDialogColors,
) -> Result<(), BevyError> {
    let buttons = world
        .query::<(Entity, &LocationControl)>()
        .iter(world)
        .filter(|(_, c)| c.root == root)
        .map(|(e, _)| e)
        .collect::<Vec<_>>();
    for entity in buttons {
        let state = button_color(world, entity, colors.sidebar_button);
        bevy_widgetry_button::internal::apply_owned_button_colors(world, entity, state)?;
    }
    let controls = world
        .query::<(Entity, &UiControl)>()
        .iter(world)
        .filter(|(_, c)| c.root == root)
        .map(|(e, c)| (e, c.action))
        .collect::<Vec<_>>();
    for (entity, action) in controls {
        let colors = match action {
            UiAction::FolderCreate => colors.folder_create_button,
            UiAction::FolderCancel => colors.folder_cancel_button,
            UiAction::Overwrite(true) => colors.overwrite_accept_button,
            UiAction::Overwrite(false) => colors.overwrite_cancel_button,
            _ => colors.toolbar_button,
        };
        let state = button_color(world, entity, colors);
        bevy_widgetry_button::internal::apply_owned_button_colors(world, entity, state)?;
    }
    let parts = world
        .query::<(Entity, &Part)>()
        .iter(world)
        .filter(|(_, p)| p.root == root)
        .map(|(e, p)| (e, p.kind))
        .collect::<Vec<_>>();
    for (entity, kind) in parts {
        match kind {
            PartKind::Filter => bevy_widgetry_combo_box::internal::apply_owned_combo_colors::<
                FilterChoice,
            >(world, entity, &colors.filter)?,
            PartKind::Sort => bevy_widgetry_combo_box::internal::apply_owned_combo_colors::<
                SortChoice,
            >(world, entity, &colors.sort)?,
            PartKind::Hidden => bevy_widgetry_check_box::internal::apply_owned_checkbox_colors(
                world,
                entity,
                &colors.hidden_option,
            )?,
            PartKind::System => bevy_widgetry_check_box::internal::apply_owned_checkbox_colors(
                world,
                entity,
                &colors.system_option,
            )?,
            _ => {}
        }
    }
    Ok(())
}
