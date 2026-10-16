mod configuration;
mod owners;

use crate::model::contract_error;
use crate::*;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry_core::disabled::set_intrinsic_disabled;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_scroll_area::{WidgetryScrollArea, WidgetryScrollAreaPlugin};
use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};
use bevy_widgetry_tooltip::{TooltipContentFactory, WidgetryTooltip, WidgetryTooltipPlugin};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf, Prefix};

#[cfg(test)]
thread_local! {
    static CONFIG_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Component, Clone, Debug, PartialEq)]
pub struct WidgetryFileDialogStyle {
    pub row_height: f32,
    pub overscan: usize,
    pub sidebar_width: f32,
    pub font_size: f32,
    pub spacing: f32,
    pub padding: f32,
}

#[derive(Component, Clone)]
struct Shell {
    parts: BTreeMap<PartKind, Entity>,
    displayed_path: Option<PathBuf>,
    configured: Option<WidgetryFileDialogStyle>,
}

pub(crate) fn filename_entity(world: &World, root: Entity) -> Option<Entity> {
    world
        .get::<Shell>(root)?
        .parts
        .get(&PartKind::Filename)
        .copied()
}

#[derive(Component, Clone, Copy)]
pub(crate) struct Part {
    pub(crate) root: Entity,
    pub(crate) kind: PartKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PartKind {
    Path,
    Search,
    Entries,
    Status,
    Sidebar,
    Filename,
    FilenameSummary,
    ColumnHeader,
    Folder,
    Filter,
    Sort,
    Hidden,
    System,
    FolderPanel,
    OverwritePanel,
}

#[derive(Component, Clone)]
struct Control {
    root: Entity,
    action: WidgetryFileDialogAction,
}

#[derive(Component)]
struct Notice(String);

#[derive(Component, PartialEq, Eq)]
struct RejectedStyle([u32; 5], usize);

pub(crate) fn apply_input(
    world: &mut World,
    root: Entity,
    action: WidgetryFileDialogAction,
) -> Result {
    if world.get::<InteractionDisabled>(root).is_some()
        || world
            .get::<WidgetryFileDialogState>(root)
            .is_none_or(|state| state.session_state() != WidgetryFileDialogSessionState::Open)
    {
        return Ok(());
    }
    world.entity_mut(root).remove::<Notice>();
    if action == WidgetryFileDialogAction::Confirm
        && world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| state.mode() == WidgetryFileDialogMode::PickFile)
    {
        return crate::input::confirm_filename(world, root);
    }
    match WidgetryFileDialog::apply(world, root, action) {
        Ok(_) => Ok(()),
        Err(error) => {
            world.entity_mut(root).insert(Notice(
                error
                    .to_string()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            ));
            crate::runtime::wake(world)?;
            Err(error)
        }
    }
}

pub(crate) fn filename_notice(world: &mut World, root: Entity, reason: &str) -> Result {
    world
        .entity_mut(root)
        .insert(Notice(format!("File name: {reason}")));
    crate::runtime::wake(world)
}

pub(crate) fn queue_input(commands: &mut Commands, root: Entity, action: WidgetryFileDialogAction) {
    commands.queue(move |world: &mut World| apply_input(world, root, action));
}

// Bevy TabNavigation collects targets by TabIndex without filtering Display or InteractionDisabled.
// Maintain Tab stops only for FileDialog-owned controls without changing their enabled authority.
pub(crate) fn sync_tab_stop(world: &mut World, entity: Entity) {
    let mut ancestor = entity;
    let enabled = loop {
        if world.get::<InteractionDisabled>(ancestor).is_some()
            || world
                .get::<Node>(ancestor)
                .is_some_and(|node| node.display == Display::None)
        {
            break false;
        }
        let Some(parent) = world.get::<ChildOf>(ancestor) else {
            break true;
        };
        ancestor = parent.parent();
    };
    if let Some(mut index) = world.get_mut::<TabIndex>(entity) {
        index.set_if_neq(TabIndex(if enabled { 0 } else { -1 }));
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<WidgetryButtonPlugin>() {
        app.add_plugins(WidgetryButtonPlugin);
    }
    if !app.is_plugin_added::<WidgetryTextFieldPlugin>() {
        app.add_plugins(WidgetryTextFieldPlugin);
    }
    if !app.is_plugin_added::<WidgetryScrollAreaPlugin>() {
        app.add_plugins(WidgetryScrollAreaPlugin);
    }
    if !app.is_plugin_added::<WidgetryAssetPlugin>() {
        app.add_plugins(WidgetryAssetPlugin);
    }
    if !app.is_plugin_added::<WidgetryIconPlugin>() {
        app.add_plugins(WidgetryIconPlugin);
    }
    if !app.is_plugin_added::<WidgetryTooltipPlugin>() {
        app.add_plugins(WidgetryTooltipPlugin);
    }
    crate::input::install(app);
    crate::controls::install(app);
    configuration::install(app);
    owners::install(app);
    app.add_systems(PostUpdate, update_colors.in_set(WidgetryUiSystems::Colors));
    app.add_observer(refresh_colors);
    app.add_observer(on_control)
        .add_systems(
            PostUpdate,
            (reconcile, configuration::configure)
                .chain()
                .in_set(WidgetryUiSystems::Build),
        )
        .add_systems(
            PostUpdate,
            (crate::input::keyboard_jobs, crate::input::editor_updates)
                .chain()
                .after(bevy::text::EditableTextSystems)
                .before(bevy::ui::UiSystems::PostLayout),
        );
}

pub(crate) fn text_scene(label: String, font_size: f32) -> impl Scene {
    bsn! { Text(label) bevy_widgetry_core::text::WidgetryText TextFont { font_size } template(|_| Ok(Pickable::IGNORE)) }
}

fn control_scene(
    root: Entity,
    action: WidgetryFileDialogAction,
    name: &'static str,
    label: &'static str,
) -> impl Scene {
    let navigation = matches!(action, WidgetryFileDialogAction::Navigate(_));
    let icon = match &action {
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Back) => Some(BuiltinIcon::FileDialogBack),
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Forward) => Some(BuiltinIcon::FileDialogForward),
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Up) => Some(BuiltinIcon::FileDialogUp),
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Refresh) => Some(BuiltinIcon::FileDialogRefresh),
        _ => None,
    }.map(|icon| bsn_list!{@WidgetryIcon { @path: {icon.path()}, @max_size: {Some(UVec2::splat(16))} } Node {width: px(16), height: px(16)} template(|_| Ok(Pickable::IGNORE))});
    let text = (!navigation).then(|| bsn_list! { @text_scene(label.into(), 14.0) });
    let tooltip = navigation.then(|| bsn! { @WidgetryTooltip {
        @content: {TooltipContentFactory::new(move || bsn_list! { @text_scene(label.into(), 14.0) })}
    } });
    (
        bsn! {
            @WidgetryButton Name(name) TabIndex::default()
            template(move |_| {
                let mut node = accesskit::Node::new(accesskit::Role::Button);
                node.set_label(label);
                Ok(bevy::a11y::AccessibilityNode(node))
            })
            template(move |_| Ok(Control {root, action: action.clone()}))
            template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryButton>::new(root)))
            Node {height:px(30),min_width:px(if navigation {30.0} else {82.0}),flex_shrink:0.0,padding:UiRect::axes(px(8),px(4)),align_items:AlignItems::Center,justify_content:JustifyContent::Center}
            Children [{icon}-- {text}]
        },
        tooltip,
    )
}

pub(crate) fn editor_scene(
    root: Entity,
    kind: PartKind,
    name: &'static str,
    value: String,
) -> impl Scene {
    bsn! {
        @WidgetryTextField Name(name) TabIndex::default()
        template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryTextField>::new(root)))
        template(move |_| Ok(Part {root, kind}))
        template(move |_| Ok(bevy_widgetry_window::WidgetryWindowInitialFocus(if kind == PartKind::Filename {0} else {i32::MAX})))
        template(move |_| {
            let mut node = accesskit::Node::new(accesskit::Role::TextInput);
            node.set_label(match kind {PartKind::Path => "Path", PartKind::Search => "Search", PartKind::Filename => "Filename", _ => "New folder name"});
            Ok(bevy::a11y::AccessibilityNode(node))
        })
        ~{EditableText::new(value)}
        Node { min_width: px(0), flex_grow: 1.0, height: px(30) }
    }
}

pub(crate) fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(_) => text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned(),
            Prefix::VerbatimUNC(_, _) => text
                .strip_prefix(r"\\?\UNC\")
                .map(|tail| format!(r"\\{tail}"))
                .unwrap_or_else(|| text.into_owned()),
            _ => text.into_owned(),
        },
        _ => text.into_owned(),
    }
}

fn shell_scene(
    root: Entity,
    state: &WidgetryFileDialogState,
    style: &WidgetryFileDialogStyle,
) -> impl Scene {
    let path = state
        .current_path()
        .or(state.requested_path())
        .map(display_path)
        .unwrap_or_default();
    let editable = matches!(
        state.mode(),
        WidgetryFileDialogMode::PickFile | WidgetryFileDialogMode::SaveFile
    );
    let filename = editable.then(|| bsn_list! {
        @editor_scene(root, PartKind::Filename, "FileDialogFilename", state.filename().to_string_lossy().into_owned())
    });
    let summary = (!editable).then(|| bsn_list! {
        Name("FileDialogFilenameSummary") template(move |_| Ok(Part {root,kind:PartKind::FilenameSummary}))
        @text_scene(String::new(),style.font_size) TextLayout::no_wrap()
        Node {flex_grow:1.0,min_width:px(0),overflow:Overflow::clip()}
    });
    let filter = (!matches!(
        state.mode(),
        WidgetryFileDialogMode::PickDirectory | WidgetryFileDialogMode::PickDirectories
    ))
    .then(|| bsn_list! { @crate::controls::file_type(root,style) });
    let label = match state.mode() {
        WidgetryFileDialogMode::SaveFile => "Save",
        WidgetryFileDialogMode::PickDirectory | WidgetryFileDialogMode::PickDirectories => "Select",
        _ => "Open",
    };
    bsn! {
        BackgroundColor::default()
        template(|_| Ok(ResolvedForeground::default()))
        Children [
            Name("FileDialogNavigation") Node { width:percent(100),flex_shrink:0.0,column_gap:px(4),align_items:AlignItems::Center } Children [
                @control_scene(root, WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Back), "FileDialogBack", "Back")--
                @control_scene(root, WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Forward), "FileDialogForward", "Forward")--
                @control_scene(root, WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Up), "FileDialogUp", "Up")--
                @control_scene(root, WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Refresh), "FileDialogRefresh", "Refresh")--
                @editor_scene(root, PartKind::Path, "FileDialogPath", path)--
                @editor_scene(root, PartKind::Search, "FileDialogSearch", state.search().into()) Node {width:percent(24),min_width:px(120),flex_grow:0.0,flex_shrink:0.0}
            ]--
            Name("FileDialogToolbar") Node {width:percent(100),flex_shrink:0.0,flex_wrap:FlexWrap::Wrap,column_gap:px(style.spacing),row_gap:px(4),align_items:AlignItems::Center} Children [
                {crate::controls::toolbar(root)}--
                Node {flex_grow:1.0,min_width:px(0)}--
                @crate::controls::options(root,style)
            ]--
            Node { flex_grow: 1.0, min_height: px(0), width: percent(100), column_gap: px(style.spacing) } Children [
                @WidgetryScrollArea { @keyboard_scroll: false, @children: bsn_list!{} } template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryScrollArea>::new(root))) Name("FileDialogSidebar") template(move |_| Ok(Part {root, kind: PartKind::Sidebar})) Node { width: px(style.sidebar_width), flex_shrink: 0.0, min_height: px(0), height: percent(100) }--
                Node {flex_direction:FlexDirection::Column,flex_grow:1.0,min_width:px(0),min_height:px(0),height:percent(100)} Children [
                    Name("FileDialogColumnHeader") template(move |_| Ok(Part {root,kind:PartKind::ColumnHeader})) @crate::view::column_header(style.font_size)--
                    @WidgetryScrollArea { @keyboard_scroll: false, @children: bsn_list!{} } template(move |_| Ok(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryScrollArea>::new(root))) Name("FileDialogEntries") template(move |_| Ok(Part {root, kind: PartKind::Entries})) TabIndex::default() bevy_widgetry_window::WidgetryWindowInitialFocus(1) Node { flex_grow: 1.0,width:percent(100), min_width: px(0), min_height: px(0) }
                ]
            ]--
            {crate::controls::panels(root)}--
            Name("FileDialogStatus") template(move |_| Ok(Part {root, kind: PartKind::Status})) Text("Loading…") bevy_widgetry_core::text::WidgetryText TextFont { font_size: {style.font_size} } TextLayout::no_wrap() Node {width:percent(100),flex_shrink:0.0,overflow:Overflow::clip()} template(|_| Ok(Pickable::IGNORE))--
            Node {width:percent(100),flex_shrink:0.0,column_gap:px(style.spacing),align_items:AlignItems::Center} Children [
                @text_scene(if matches!(state.mode(),WidgetryFileDialogMode::PickDirectory | WidgetryFileDialogMode::PickDirectories) {"Folder:".into()} else {"File name:".into()},style.font_size) Node {width:px(76),flex_shrink:0.0}--
                {filename}-- {summary}
            ]--
            {filter}--
            Node { width: percent(100),flex_shrink:0.0, justify_content: JustifyContent::FlexEnd, column_gap: px(style.spacing) } Children [
                @control_scene(root, WidgetryFileDialogAction::Confirm, "FileDialogConfirm", label)--
                @control_scene(root, WidgetryFileDialogAction::Cancel, "FileDialogCancel", "Cancel")
            ]
        ]
    }
}

fn confirm_available(world: &World, root: Entity, state: &WidgetryFileDialogState) -> bool {
    state.session_state() == WidgetryFileDialogSessionState::Open
        && !state.projection_pending()
        && !state.selection_pending()
        && state.confirmation() == &WidgetryFileDialogConfirmation::Idle
        && state.current_path().is_some()
        && state.started_generation == Some(state.token().generation)
        && !matches!(
            state.directory_state(),
            WidgetryFileDialogDirectoryState::Failed(_)
        )
        && match state.mode() {
            WidgetryFileDialogMode::SaveFile => !state.filename().is_empty(),
            WidgetryFileDialogMode::PickFile => crate::input::filename_available(world, root),
            WidgetryFileDialogMode::PickDirectory => true,
            _ => !state.selected().is_empty(),
        }
}

fn status(state: &WidgetryFileDialogState) -> String {
    if let Some(error) = state.error() {
        return error.into();
    }
    if state.confirmation() == &WidgetryFileDialogConfirmation::Validating {
        return "Validating…".into();
    }
    match state.directory_state() {
        WidgetryFileDialogDirectoryState::Loading => {
            format!("Loading… {} entries", state.entries().len())
        }
        WidgetryFileDialogDirectoryState::Failed(error) => {
            format!("Cannot open directory: {error}")
        }
        WidgetryFileDialogDirectoryState::Partial { errors, .. } => format!(
            "Partial: {} entries, {errors} errors. Refresh to retry.",
            state.entries().len()
        ),
        WidgetryFileDialogDirectoryState::Ready if state.entries().is_empty() => {
            "Empty directory".into()
        }
        WidgetryFileDialogDirectoryState::Ready if state.visible().is_empty() => {
            "No matching entries".into()
        }
        WidgetryFileDialogDirectoryState::Ready => format!(
            "{} entries · {} selected",
            state.visible().len(),
            state.selected().len()
        ),
    }
}

fn reconcile(world: &mut World) {
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<WidgetryFileDialog>>()
        .iter(world)
        .collect();
    for root in roots {
        if let Err(error) = reconcile_root(world, root) {
            world
                .commands()
                .queue(move |_: &mut World| -> Result { Err(error) });
        }
    }
}

fn reconcile_root(world: &mut World, root: Entity) -> Result {
    let host = crate::window::content_host(world, root);
    let style = world
        .get::<WidgetryFileDialogStyle>(root)
        .cloned()
        .unwrap_or_default();
    if ![style.row_height, style.sidebar_width, style.font_size]
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
        || ![style.spacing, style.padding]
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        || style.overscan > 8
    {
        let rejected = RejectedStyle(
            [
                style.row_height.to_bits(),
                style.sidebar_width.to_bits(),
                style.font_size.to_bits(),
                style.spacing.to_bits(),
                style.padding.to_bits(),
            ],
            style.overscan,
        );
        if world.get::<RejectedStyle>(root) == Some(&rejected) {
            return Ok(());
        }
        world.entity_mut(root).insert(rejected);
        return Err(contract_error("invalid FileDialog style dimensions"));
    }
    world.entity_mut(root).remove::<RejectedStyle>();
    let state = world
        .get::<WidgetryFileDialogState>(root)
        .cloned()
        .ok_or_else(|| contract_error("FileDialog state missing"))?;
    if world.get::<Shell>(root).is_none() {
        crate::controls::models(world, root, &state)?;
        if world.get::<Node>(host).is_none() {
            world.entity_mut(host).insert(Node {
                width: percent(100),
                height: percent(100),
                ..default()
            });
        }
        bevy_widgetry_core::scene::apply_scene(
            &mut world.entity_mut(host),
            shell_scene(root, &state, &style),
        )
        .map_err(|error| contract_error(&error.to_string()))?;
        let parts = world
            .query::<(Entity, &Part)>()
            .iter(world)
            .filter(|(_, part)| part.root == root)
            .map(|(entity, part)| (part.kind, entity))
            .collect();
        world.entity_mut(root).insert((
            style.clone(),
            Shell {
                parts,
                displayed_path: None,
                configured: None,
            },
        ));
        world.write_message(bevy::window::RequestRedraw);
    }
    let (status_entity, path_entity, path_changed) = {
        let shell = world
            .get::<Shell>(root)
            .ok_or_else(|| contract_error("FileDialog shell missing"))?;
        (
            shell.parts.get(&PartKind::Status).copied(),
            shell.parts.get(&PartKind::Path).copied(),
            state.current_path() != shell.displayed_path.as_deref(),
        )
    };
    if let Some(status_entity) = status_entity {
        let status = world
            .get::<Notice>(root)
            .map(|notice| notice.0.clone())
            .unwrap_or_else(|| status(&state));
        world
            .get_mut::<Text>(status_entity)
            .ok_or_else(|| contract_error("FileDialog status missing"))?
            .set_if_neq(Text(status));
    }
    if path_changed {
        if let Some(path_entity) = path_entity {
            let path = state.current_path().map(display_path).unwrap_or_default();
            let mut edit = world
                .get_mut::<EditableText>(path_entity)
                .ok_or_else(|| contract_error("FileDialog path editor missing"))?;
            if edit.value().to_string() != path {
                edit.editor_mut().set_text(&path);
            }
        }
        world
            .get_mut::<Shell>(root)
            .ok_or_else(|| contract_error("FileDialog shell missing"))?
            .displayed_path = state.current_path().map(ToOwned::to_owned);
    }
    let area = world
        .get::<Shell>(root)
        .and_then(|shell| shell.parts.get(&PartKind::Entries))
        .copied()
        .ok_or_else(|| contract_error("FileDialog entry area missing"))?;
    crate::view::reconcile(world, root, area, &state, &style)?;
    if let Some(header) = world
        .get::<Shell>(root)
        .and_then(|shell| shell.parts.get(&PartKind::ColumnHeader))
        .copied()
        && let Some(width) = world
            .get::<Children>(area)
            .and_then(|children| {
                children.iter().find(|entity| {
                    world
                        .get::<bevy_widgetry_scroll_area::WidgetryScrollAreaViewport>(*entity)
                        .is_some()
                })
            })
            .and_then(|viewport| world.get::<ComputedNode>(viewport))
            .map(|node| node.size().x * node.inverse_scale_factor())
            .filter(|width| *width > 0.0)
    {
        let mut node = world
            .get_mut::<Node>(header)
            .ok_or_else(|| contract_error("FileDialog column header missing"))?;
        if node.width != px(width) {
            node.width = px(width);
            world.write_message(bevy::window::RequestRedraw);
        }
    }
    crate::controls::sync(world, root, &state)?;
    let controls: Vec<_> = world
        .query::<(Entity, &Control)>()
        .iter(world)
        .filter(|(_, control)| control.root == root)
        .map(|(entity, control)| (entity, control.action.clone()))
        .collect();
    for (entity, action) in controls {
        let disabled = if action == WidgetryFileDialogAction::Cancel {
            state.session_state() != WidgetryFileDialogSessionState::Open
        } else if action == WidgetryFileDialogAction::Confirm {
            !confirm_available(world, root, &state)
        } else {
            state.session_state() != WidgetryFileDialogSessionState::Open
        };
        set_intrinsic_disabled(world, entity, disabled);
        sync_tab_stop(world, entity);
    }
    Ok(())
}

fn on_control(
    event: On<Activate>,
    controls: Query<&Control, Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    if let Ok(control) = controls.get(event.entity) {
        let root = control.root;
        let action = control.action.clone();
        commands.queue(move |world: &mut World| apply_input(world, root, action));
    }
}

impl Default for WidgetryFileDialogStyle {
    fn default() -> Self {
        Self {
            row_height: 28.0,
            overscan: 2,
            sidebar_width: 160.0,
            font_size: 14.0,
            spacing: 8.0,
            padding: 10.0,
        }
    }
}

pub(crate) fn establish_color_owners(world: &mut World) -> Result {
    owners::establish(world)
}
fn button_state(
    world: &World,
    entity: Entity,
    colors: &bevy_widgetry_theme::WidgetryButtonColors,
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
pub(crate) fn update_colors(world: &mut World) -> Result<(), BevyError> {
    let theme = world
        .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
        .colors()
        .file_dialog;
    let roots = world
        .query_filtered::<Entity, With<WidgetryFileDialog>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let colors = crate::WidgetryFileDialogColorOverrides::get(world, root)?.resolve(&theme);
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
        let controls = world
            .query::<(Entity, &Control)>()
            .iter(world)
            .filter(|(_, c)| c.root == root)
            .map(|(e, c)| (e, c.action.clone()))
            .collect::<Vec<_>>();
        for (entity, action) in controls {
            let button = match action {
                WidgetryFileDialogAction::Confirm => colors.accept_button,
                WidgetryFileDialogAction::Cancel => colors.cancel_button,
                _ => colors.toolbar_button,
            };
            let state = button_state(world, entity, &button);
            bevy_widgetry_button::internal::apply_owned_button_colors(world, entity, state)?;
        }
        let parts = world
            .query::<(Entity, &Part)>()
            .iter(world)
            .filter(|(_, p)| p.root == root)
            .map(|(e, p)| (e, p.kind))
            .collect::<Vec<_>>();
        for (entity, kind) in parts {
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
                bevy_widgetry_scroll_area::internal::apply_owned_scroll_colors(
                    world, entity, &scroll,
                )?;
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
        crate::view::update_entry_colors(world, root, &colors)?;
        crate::controls::apply_colors(world, root, &colors)?;
        crate::window::apply_confirmation_colors(world, root, &colors)?;
    }
    Ok(())
}
pub(crate) fn refresh_colors(
    _event: On<bevy_widgetry_theme::WidgetryThemeChanged>,
    mut commands: Commands,
) {
    commands.queue(update_colors);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;

    pub(super) fn fixture() -> (App, Entity) {
        let mut app = bevy_widgetry_test_utils::text_input_app();
        bevy_widgetry_test_utils::add_ui_plugins(&mut app);
        bevy_widgetry_test_utils::spawn_ui_camera(&mut app, UVec2::new(1000, 700), 1.0);
        app.insert_resource(WidgetryFileDialogRuntimeOptions {
            automatic: false,
            ..default()
        });
        app.add_plugins(WidgetryFileDialogPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryFileDialog Node {width: px(900), height: px(650)}
            })
            .unwrap()
            .id();
        for _ in 0..4 {
            app.update();
        }
        (app, root)
    }

    #[test]
    fn settled_configuration_does_not_walk_fonts_or_rewrite_shell() {
        let (mut app, root) = fixture();
        CONFIG_WORK.with(|work| work.set(0));
        let tick = app
            .world()
            .entity(root)
            .get_ref::<Shell>()
            .unwrap()
            .last_changed();
        for _ in 0..3 {
            app.update();
        }
        CONFIG_WORK.with(|work| assert_eq!(work.get(), 0));
        assert_eq!(
            app.world()
                .entity(root)
                .get_ref::<Shell>()
                .unwrap()
                .last_changed(),
            tick
        );
    }

    #[test]
    fn rejected_configuration_retains_delayed_fonts_until_recovery() {
        let (mut app, root) = fixture();
        app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        let style = WidgetryFileDialogStyle {
            font_size: 18.0,
            ..default()
        };
        app.world_mut().entity_mut(root).insert(style.clone());
        app.update();
        app.world_mut()
            .entity_mut(root)
            .insert(WidgetryFileDialogStyle {
                row_height: 0.0,
                ..style.clone()
            });
        let text = app
            .world_mut()
            .spawn_scene(bsn! {
                Text("Delayed") TextFont {font_size: bevy::text::FontSize::Px(14.0)} ChildOf(root)
            })
            .unwrap()
            .id();
        errors.run(|| {
            app.update();
            app.update();
        });
        assert_eq!(
            app.world().get::<TextFont>(text).unwrap().font_size,
            bevy::text::FontSize::Px(14.0)
        );
        app.world_mut().entity_mut(root).insert(style);
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(text).unwrap().font_size,
            bevy::text::FontSize::Px(18.0)
        );
    }

    #[test]
    fn delayed_and_reparented_fonts_receive_current_configuration() {
        let (mut app, first) = fixture();
        let second = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryFileDialog Node {width: px(900), height: px(650)}
            })
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(second)
            .insert(WidgetryFileDialogStyle {
                font_size: 19.0,
                ..default()
            });
        app.update();
        let container = app.world_mut().spawn_scene(bsn! {
            Node ChildOf(first) Children [Text("Late text") TextFont {font_size: bevy::text::FontSize::Px(99.0)}]
        }).unwrap().id();
        let text = app.world().get::<Children>(container).unwrap()[0];
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(text).unwrap().font_size,
            bevy::text::FontSize::Px(14.0)
        );
        app.world_mut()
            .entity_mut(container)
            .insert(ChildOf(second));
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(text).unwrap().font_size,
            bevy::text::FontSize::Px(19.0)
        );
    }
}
