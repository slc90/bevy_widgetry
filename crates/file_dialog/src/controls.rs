const LOCATION_PADDING_X: f32 = 8.0;

const CONTROL_FONT_SIZE: f32 = 14.0;
const FILTER_LABEL_WIDTH: f32 = 76.0;
const FILTER_ROW_HEIGHT: f32 = 28.0;
const FILTER_MAX_VISIBLE_ITEMS: usize = 6;
const CONTROL_GAP: f32 = 8.0;
const SELECTION_SUMMARY_ITEMS: usize = 3;
const LOCATION_MIN_HEIGHT: f32 = 30.0;
const LOCATION_PADDING_Y: f32 = 4.0;

use crate::style::{Part, PartKind};
use crate::*;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::disabled::set_intrinsic_disabled;
use bevy_widgetry_list_view::{WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer};
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
use bevy_widgetry_text_field::WidgetryTextFieldHeadless;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf, Prefix};

pub(crate) struct FilterChoice(WidgetryFileDialogFilterId, String);

#[derive(Component, Clone, Copy)]
struct ProjectedChoice(WidgetryListItemId);

#[derive(Component)]
struct ProjectedEditor(String);

#[derive(Component, Clone)]
struct SidebarProjection {
    locations: Vec<WidgetryFileDialogLocation>,
    error: Option<String>,
    font: f32,
}

#[derive(Component, Clone)]
pub(crate) struct LocationControl {
    pub(crate) root: Entity,
    path: PathBuf,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct UiControl {
    pub(crate) root: Entity,
    action: UiAction,
}

#[derive(Component, Clone, Copy)]
struct OverwriteDecision(WidgetryFileDialogToken);

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiAction {
    FolderCreate,
    FolderCancel,
    Overwrite(bool),
}

pub(crate) fn install(app: &mut App) {
    if let Err(error) = app.register_widgetry_combo_box::<FilterChoice>() {
        let error = error
            .with_context(bevy_widgetry_core::scene::ErrorFrame::new(
                "file_dialog.register_controls",
            ))
            .into_bevy_error();
        app.world_mut()
            .commands()
            .queue(move |_: &mut World| -> Result { Err(error) });
    }
    app.add_observer(on_action)
        .add_observer(on_filter)
        .add_observer(on_location);
}

pub(crate) fn models(world: &mut World, root: Entity, state: &WidgetryFileDialogState) -> Result {
    let mut filters = WidgetryListModel::<FilterChoice>::default();
    for filter in state.filters() {
        let suffixes = if filter.suffixes.is_empty() {
            "*.*".into()
        } else {
            filter
                .suffixes
                .iter()
                .map(|suffix| format!("*.{suffix}"))
                .collect::<Vec<_>>()
                .join("; ")
        };
        filters
            .push(FilterChoice(
                filter.id.clone(),
                format!("{} ({suffixes})", filter.label),
            ))
            .map_err(bevy_widgetry_list_view::ListFailure::into_bevy_error)?;
    }
    world.entity_mut(root).insert(filters);
    Ok(())
}

fn button(root: Entity, action: UiAction, name: &'static str, label: &'static str) -> impl Scene {
    bsn! { @WidgetryButton { @children: {Some(Box::new(bsn_list! {@crate::style::text_scene(label.into(), CONTROL_FONT_SIZE)}) as Box<dyn bevy::prelude::SceneList>)}, } Name(name) TabIndex::default() template(move |_| Ok(UiControl { root, action })) template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryButton>::new(root)))  }
}

pub(crate) fn file_type(root: Entity, style: &WidgetryFileDialogStyle) -> impl Scene {
    let renderer = WidgetryListViewRenderer::new(|choice: &FilterChoice| {
        let label = choice.1.clone();
        bsn_list! { Node {width:percent(100),min_width:px(0),overflow:Overflow::clip()}
            @bevy_widgetry_tooltip::WidgetryTooltip {
                @content:{bevy_widgetry_tooltip::TooltipContentFactory::new(move || bsn_list! { @crate::style::text_scene(label.clone(),CONTROL_FONT_SIZE) })}
            }
            Children [ @crate::style::text_scene(choice.1.clone(),CONTROL_FONT_SIZE) TextLayout::no_wrap() Node {min_width:px(0),max_width:percent(100),overflow:Overflow::clip()} ]
        }
    });
    bsn! { Name("FileDialogFileTypeRow") Node {width:percent(100),flex_shrink:0.0,column_gap:px(style.spacing),align_items:AlignItems::Center} Children [
        @crate::style::text_scene("File type:".into(),style.font_size) Node {width:px(FILTER_LABEL_WIDTH),flex_shrink:0.0}--
        @WidgetryComboBox::<FilterChoice> {@source:root,@item_height:FILTER_ROW_HEIGHT,@max_visible_items:FILTER_MAX_VISIBLE_ITEMS,@renderer:renderer}
            template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryComboBox<FilterChoice>>::new(root)))
            Name("FileDialogFilter") template(move |_| Ok(Part {root,kind:PartKind::Filter})) Node {flex_grow:1.0,min_width:px(0)} TabIndex(-1)
    ] }
}

pub(crate) fn panels(root: Entity) -> impl SceneList {
    bsn_list! {
        Name("FileDialogFolderPanel") template(move |_| Ok(Part {root, kind: PartKind::FolderPanel})) Node { display: Display::None, column_gap: px(CONTROL_GAP), width: percent(100), align_items: AlignItems::Center } Children [
            @crate::style::text_scene("New folder".into(), CONTROL_FONT_SIZE)-- @crate::style::editor_scene(root, PartKind::Folder, "FileDialogFolderName", String::new())--
            @button(root, UiAction::FolderCreate, "FileDialogFolderCreate", "Create")-- @button(root, UiAction::FolderCancel, "FileDialogFolderCancel", "Dismiss")
        ]--
        Name("FileDialogOverwritePanel") template(move |_| Ok(Part {root, kind: PartKind::OverwritePanel})) Node { display: Display::None, column_gap: px(CONTROL_GAP), width: percent(100), align_items: AlignItems::Center } Children [
            @crate::style::text_scene("Target exists. Replace?".into(), CONTROL_FONT_SIZE)-- @button(root, UiAction::Overwrite(true), "FileDialogOverwriteYes", "Replace")-- @button(root, UiAction::Overwrite(false), "FileDialogOverwriteNo", "Keep editing")
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
    crate::input::sync_filename(world, root, state)?;
    if let Some(entity) = part(world, root, PartKind::FilenameSummary) {
        let summary = selection_summary(state);
        world
            .get_mut::<Text>(entity)
            .ok_or_else(|| {
                crate::error::internal_error(FileDialogInternalFault::Missing {
                    part: "filename summary",
                })
            })?
            .set_if_neq(Text(summary));
    }
    let locations: Vec<_> = world
        .query::<(Entity, &LocationControl)>()
        .iter(world)
        .filter(|(_, control)| control.root == root)
        .map(|(entity, _)| entity)
        .collect();
    for entity in locations {
        set_intrinsic_disabled(world, entity, !interactive).map_err(|error| {
            error
                .with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("project widget disabled")
                        .field("root", root),
                )
                .into_bevy_error()
        })?;
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
                UiAction::FolderCreate => {
                    state.started_generation == Some(state.token().generation)
                        && state.folder_request.is_none()
                }
                UiAction::Overwrite(_) => matches!(
                    state.confirmation(),
                    WidgetryFileDialogConfirmation::AwaitingOverwrite { .. }
                ),
                UiAction::FolderCancel => true,
            };
        set_intrinsic_disabled(world, entity, !available).map_err(|error| {
            error
                .with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("project widget disabled")
                        .field("root", root),
                )
                .into_bevy_error()
        })?;
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
                        | PartKind::Entries
                )
        })
        .map(|(entity, part)| (entity, part.kind))
        .collect();
    for (entity, kind) in parts {
        let busy_filename = kind == PartKind::Filename
            && state.mode() == WidgetryFileDialogMode::PickFile
            && (state.selection_pending()
                || state.confirmation() != &WidgetryFileDialogConfirmation::Idle);
        set_intrinsic_disabled(world, entity, !interactive || busy_filename).map_err(|error| {
            error
                .with_context(
                    bevy_widgetry_core::scene::ErrorFrame::new("project widget disabled")
                        .field("root", root),
                )
                .into_bevy_error()
        })?;
        if kind == PartKind::Filter {
            if let Some(mut index) = world.get_mut::<TabIndex>(entity) {
                index.set_if_neq(TabIndex(-1));
            } else {
                world.entity_mut(entity).insert(TabIndex(-1));
            }
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
    if let Some(entity) = part(world, root, PartKind::OverwritePanel) {
        let display = if matches!(
            state.confirmation(),
            WidgetryFileDialogConfirmation::AwaitingOverwrite { .. }
        ) && world.get::<crate::window::Independent>(root).is_none()
        {
            Display::Flex
        } else {
            Display::None
        };
        let mut node = world.get_mut::<Node>(entity).ok_or_else(|| {
            crate::error::internal_error(FileDialogInternalFault::Missing {
                part: "overwrite panel",
            })
        })?;
        if node.display != display {
            node.display = display;
        }
        sync_panel_tabs(world, entity);
    }
    let filename = (state.mode() == WidgetryFileDialogMode::SaveFile).then(|| {
        (
            PartKind::Filename,
            state.filename().to_string_lossy().into_owned(),
        )
    });
    for (kind, text) in filename.into_iter().chain(std::iter::once((
        PartKind::Search,
        state.search().to_owned(),
    ))) {
        if let Some(entity) = part(world, root, kind) {
            let changed = world
                .get::<ProjectedEditor>(entity)
                .is_some_and(|shown| shown.0 != text);
            let edit = WidgetryTextFieldHeadless::editor(world, entity).map_err(|error| {
                error
                    .with_context(bevy_widgetry_core::scene::ErrorFrame::new(
                        "file_dialog.text_field.editor",
                    ))
                    .into_bevy_error()
            })?;
            if edit.is_composing() {
                continue;
            }
            if changed && edit.value().to_string() != text {
                WidgetryTextFieldHeadless::set_text_in_world(world, entity, &text).map_err(
                    |error| {
                        error
                            .with_context(bevy_widgetry_core::scene::ErrorFrame::new(
                                "file_dialog.text_field.set_text_in_world",
                            ))
                            .into_bevy_error()
                    },
                )?;
            }
            if changed || world.get::<ProjectedEditor>(entity).is_none() {
                world.entity_mut(entity).insert(ProjectedEditor(text));
            }
        }
    }
    let filter = part(world, root, PartKind::Filter);
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
    if let (Some(entity), Some(id)) = (filter, filter_id)
        && world.get::<ProjectedChoice>(entity).map(|choice| choice.0) != Some(id)
    {
        world.entity_mut(entity).insert(ProjectedChoice(id));
        WidgetryComboBox::<FilterChoice>::select_id(&mut world.commands(), entity, id);
    }
    Ok(())
}

fn selection_summary(state: &WidgetryFileDialogState) -> String {
    if state.selected().is_empty() && state.mode() == WidgetryFileDialogMode::PickDirectory {
        return state
            .current_path()
            .map(crate::style::display_path)
            .unwrap_or_default();
    }
    let names = state
        .selected()
        .iter()
        .take(SELECTION_SUMMARY_ITEMS)
        .filter_map(|id| state.snapshot()?.entry(*id))
        .map(|entry| entry.name().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("; ");
    if state.selected().len() == 1 {
        return names;
    }
    let kind = if state.mode() == WidgetryFileDialogMode::PickFiles {
        "files"
    } else {
        "folders"
    };
    if names.is_empty() {
        format!("0 {kind} selected")
    } else {
        format!(
            "{} {kind} selected: {names}{}",
            state.selected().len(),
            if state.selected().len() > SELECTION_SUMMARY_ITEMS {
                "; …"
            } else {
                ""
            }
        )
    }
}

fn sidebar(world: &mut World, root: Entity, state: &WidgetryFileDialogState) -> Result {
    let Some(area) = part(world, root, PartKind::Sidebar) else {
        return Ok(());
    };
    let font = world
        .get::<WidgetryFileDialogStyle>(root)
        .map(|style| style.font_size)
        .unwrap_or(CONTROL_FONT_SIZE);
    if world
        .get::<SidebarProjection>(area)
        .is_some_and(|projection| {
            projection.locations == state.locations()
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
        .ok_or_else(|| {
            crate::error::internal_error(FileDialogInternalFault::Missing {
                part: "sidebar viewport",
            })
        })?;
    let content = world
        .get::<Children>(viewport)
        .and_then(|children| {
            children
                .iter()
                .find(|entity| world.get::<WidgetryScrollAreaContent>(*entity).is_some())
        })
        .ok_or_else(|| {
            crate::error::internal_error(FileDialogInternalFault::Missing {
                part: "sidebar content",
            })
        })?;
    let previous: Vec<_> = world
        .get::<Children>(content)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    for entity in previous {
        world.despawn(entity);
    }
    let mut rows = Vec::new();
    for name in ["Home", "Desktop", "Documents", "Downloads"] {
        if let Some(place) = state.locations().iter().find(|place| place.name == name) {
            let label = place.name.clone();
            let path = place.path.clone();
            let accessible_label = label.clone();
            let scene = bsn! { @WidgetryButton { @children: {Some(Box::new(bsn_list! {@crate::style::text_scene(label, font) TextLayout::no_wrap() Node {min_width:px(0),max_width:percent(100),overflow:Overflow::clip()}}) as Box<dyn bevy::prelude::SceneList>)}, @content: {Some(Box::new(bsn! { Node { align_items: AlignItems::Center, justify_content: JustifyContent::FlexStart } }) as Box<dyn bevy::prelude::Scene>)}, } Name("FileDialogLocation") TabIndex::default() template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryButton>::new(root)))
            template(move |_| {let mut node=accesskit::Node::new(accesskit::Role::Button);node.set_label(accessible_label.clone());Ok(bevy::a11y::AccessibilityNode(node))})
            template(move |_| Ok(LocationControl {root, path: path.clone()})) Node {width: percent(100), min_height: px(LOCATION_MIN_HEIGHT), flex_shrink: 0.0,border:UiRect::ZERO,padding:UiRect::axes(px(LOCATION_PADDING_X),px(LOCATION_PADDING_Y)),justify_content:JustifyContent::FlexStart,align_items:AlignItems::Center,overflow:Overflow::clip()}
             };
            rows.push(
                bevy_widgetry_core::scene::spawn_scene(world, scene).map_err(|error| {
                    error
                        .with_context(crate::error::owner_frame(
                            world,
                            root,
                            root,
                            "file_dialog.controls.scene",
                        ))
                        .into_bevy_error()
                })?,
            );
        }
    }
    if let Some(error) = state.location_error() {
        rows.push(
            bevy_widgetry_core::scene::spawn_scene(
                world,
                crate::style::text_scene(format!("Locations unavailable: {error}"), font),
            )
            .map_err(|error| {
                error
                    .with_context(crate::error::owner_frame(
                        world,
                        root,
                        root,
                        "file_dialog.controls.scene",
                    ))
                    .into_bevy_error()
            })?,
        );
    }
    world.entity_mut(content).replace_children(&rows);
    world.entity_mut(area).insert(SidebarProjection {
        locations: state.locations().to_vec(),
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
            .ok_or_else(|| {
                crate::error::internal_error(FileDialogInternalFault::Missing {
                    part: "folder panel",
                })
            })?
            .display = Display::None;
        sync_panel_tabs(world, entity);
    }
    if let Some(area) = part(world, root, PartKind::Entries) {
        world
            .resource_mut::<InputFocus>()
            .set(area, FocusCause::Navigated);
    }
    crate::runtime::wake(world).map_err(crate::FileDialogFailure::into_bevy_error)
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
            UiAction::FolderCreate | UiAction::FolderCancel => {
                if control.action == UiAction::FolderCreate
                    && let Some(entity) = part(world, control.root, PartKind::Folder)
                {
                    let name = WidgetryTextFieldHeadless::editor(world, entity)
                        .map_err(|error| {
                            error
                                .with_context(bevy_widgetry_core::scene::ErrorFrame::new(
                                    "file_dialog.text_field.editor",
                                ))
                                .into_bevy_error()
                        })?
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
        crate::runtime::wake(world).map_err(crate::FileDialogFailure::into_bevy_error)
    });
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
fn same_location_path(left: &Path, right: &Path) -> bool {
    let mut left = left.components();
    let mut right = right.components();
    let same_prefix = match (left.next(), right.next()) {
        (Some(Component::Prefix(left)), Some(Component::Prefix(right))) => {
            match (left.kind(), right.kind()) {
                (
                    Prefix::Disk(a) | Prefix::VerbatimDisk(a),
                    Prefix::Disk(b) | Prefix::VerbatimDisk(b),
                ) => a == b,
                (
                    Prefix::UNC(a, b) | Prefix::VerbatimUNC(a, b),
                    Prefix::UNC(c, d) | Prefix::VerbatimUNC(c, d),
                ) => a == c && b == d,
                (a, b) => a == b,
            }
        }
        (a, b) => a == b,
    };
    same_prefix && left.eq(right)
}

pub(crate) fn apply_color(
    world: &mut World,
    entity: Entity,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryFileDialogColors,
) -> Result<(), BevyError> {
    if let Some(path) = world
        .get::<LocationControl>(entity)
        .map(|control| control.path.clone())
    {
        let selected = world
            .get::<WidgetryFileDialogState>(root)
            .and_then(|state| state.current_path())
            .is_some_and(|current| same_location_path(current, &path));
        let mut state = button_color(world, entity, colors.sidebar_button);
        if selected && world.get::<InteractionDisabled>(entity).is_none() {
            state.background = colors.entry.selected.background;
            state.foreground = colors.entry.selected.foreground;
        }
        if let Some(mut node) = world.get_mut::<bevy::a11y::AccessibilityNode>(entity) {
            node.0.set_selected(selected);
        }
        bevy_widgetry_button::internal::apply_owned_button_colors(world, entity, state)
            .map_err(|error| error.into_bevy_error())?;
    }
    if let Some(action) = world.get::<UiControl>(entity).map(|control| control.action) {
        let colors = match action {
            UiAction::FolderCreate => colors.folder_create_button,
            UiAction::FolderCancel => colors.folder_cancel_button,
            UiAction::Overwrite(true) => colors.overwrite_accept_button,
            UiAction::Overwrite(false) => colors.overwrite_cancel_button,
        };
        let state = button_color(world, entity, colors);
        bevy_widgetry_button::internal::apply_owned_button_colors(world, entity, state)
            .map_err(|error| error.into_bevy_error())?;
    }
    if let Some(PartKind::Filter) = world.get::<Part>(entity).map(|part| part.kind) {
        bevy_widgetry_combo_box::internal::apply_owned_combo_colors::<FilterChoice>(
            world,
            entity,
            &colors.filter,
        )
        .map_err(|error| error.into_bevy_error())?;
    }
    Ok(())
}
