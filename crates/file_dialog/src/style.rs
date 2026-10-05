use crate::model::contract_error;
use crate::*;
use bevy::app::Propagate;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_asset::WidgetryAssetPlugin;
use bevy_widgetry_button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::icon::WidgetryIconPlugin;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_core::{ForegroundColor, ThemeMode};
use bevy_widgetry_scroll_area::{WidgetryScrollArea, WidgetryScrollAreaPlugin};
use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Component, Clone, Debug, PartialEq)]
pub struct WidgetryFileDialogStyle {
    pub row_height: f32,
    pub overscan: usize,
    pub sidebar_width: f32,
    pub font_size: f32,
    pub spacing: f32,
    pub padding: f32,
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub selected_background: Option<Color>,
    pub active_border: Option<Color>,
}

#[derive(Component, Clone)]
struct Shell {
    parts: BTreeMap<PartKind, Entity>,
    displayed_path: Option<PathBuf>,
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

pub(crate) fn queue_input(commands: &mut Commands, root: Entity, action: WidgetryFileDialogAction) {
    commands.queue(move |world: &mut World| apply_input(world, root, action));
}

// Bevy TabNavigation 根据 TabIndex 收集目标，不过滤 Display 或 InteractionDisabled。
// 这里仅维护 FileDialog 自建 controls 的 Tab stop，不修改业务 enabled authority。
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
    crate::input::install(app);
    crate::controls::install(app);
    app.add_observer(on_control)
        .add_systems(PostUpdate, reconcile.in_set(WidgetryUiSystems::Build))
        .add_systems(
            PostUpdate,
            (crate::input::keyboard_jobs, crate::input::editor_updates)
                .chain()
                .after(bevy::text::EditableTextSystems)
                .before(bevy::ui::UiSystems::Layout),
        );
}

pub(crate) fn text_scene(label: String, font_size: f32) -> impl Scene {
    bsn! { Text(label) TextFont { font_size } template(|_| Ok(Pickable::IGNORE)) }
}

fn control_scene(
    root: Entity,
    action: WidgetryFileDialogAction,
    name: &'static str,
    label: &'static str,
) -> impl Scene {
    let icon = match &action {
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Back) => Some(BuiltinIcon::FileDialogBack),
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Forward) => Some(BuiltinIcon::FileDialogForward),
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Up) => Some(BuiltinIcon::FileDialogUp),
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Refresh) => Some(BuiltinIcon::FileDialogRefresh),
        _ => None,
    }.map(|icon| bsn_list![(@WidgetryIcon { @path: {icon.path()}, @max_size: {Some(UVec2::splat(16))} } Node {width: px(16), height: px(16)} template(|_| Ok(Pickable::IGNORE)))]);
    bsn! {
        @WidgetryButton Name(name) TabIndex::default()
        template(move |_| Ok(Control {root, action: action.clone()}))
        Children [{icon}, text_scene(label.into(), 14.0)]
    }
}

pub(crate) fn editor_scene(
    root: Entity,
    kind: PartKind,
    name: &'static str,
    value: String,
) -> impl Scene {
    bsn! {
        @WidgetryTextField Name(name) TabIndex::default()
        template(move |_| Ok(Part {root, kind}))
        template(move |_| {
            let mut node = accesskit::Node::new(accesskit::Role::TextInput);
            node.set_label(match kind {PartKind::Path => "Path", PartKind::Search => "Search", PartKind::Filename => "Filename", _ => "New folder name"});
            Ok(bevy::a11y::AccessibilityNode(node))
        })
        template_value(EditableText::new(value))
        Node { min_width: px(0), flex_grow: 1.0, height: px(30) }
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
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let filename = (state.mode() == WidgetryFileDialogMode::SaveFile).then(|| {
        bsn_list![(Node {width: percent(100), column_gap: px(style.spacing), align_items: AlignItems::Center} Children [
            text_scene("Filename".into(), style.font_size),
            editor_scene(root, PartKind::Filename, "FileDialogFilename", state.filename().to_string_lossy().into_owned()),
        ])]
    });
    let label = match state.mode() {
        WidgetryFileDialogMode::SaveFile => "Save",
        WidgetryFileDialogMode::PickDirectory | WidgetryFileDialogMode::PickDirectories => "Select",
        _ => "Open",
    };
    bsn! {
        BackgroundColor::default()
        template(|_| Ok(Propagate(ForegroundColor::default())))
        Children [
            (Node { flex_direction: FlexDirection::Row, column_gap: px(style.spacing), align_items: AlignItems::Center } Children [
                control_scene(root, WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Back), "FileDialogBack", "Back"),
                control_scene(root, WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Forward), "FileDialogForward", "Forward"),
                control_scene(root, WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Up), "FileDialogUp", "Up"),
                control_scene(root, WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Refresh), "FileDialogRefresh", "Refresh"),
                {crate::controls::toolbar(root)},
            ]),
            (Node { width: percent(100), column_gap: px(style.spacing), align_items: AlignItems::Center } Children [text_scene("Path".into(), style.font_size), editor_scene(root, PartKind::Path, "FileDialogPath", path)]),
            (Node { width: percent(100), column_gap: px(style.spacing), align_items: AlignItems::Center } Children [text_scene("Search".into(), style.font_size), editor_scene(root, PartKind::Search, "FileDialogSearch", state.search().into())]),
            (Node { flex_grow: 1.0, min_height: px(0), width: percent(100), column_gap: px(style.spacing) } Children [
                (@WidgetryScrollArea { @keyboard_scroll: false, @children: bsn_list![] } Name("FileDialogSidebar") template(move |_| Ok(Part {root, kind: PartKind::Sidebar})) Node { width: px(style.sidebar_width), flex_shrink: 0.0, min_height: px(0), height: percent(100) }),
                (@WidgetryScrollArea { @keyboard_scroll: false, @children: bsn_list![] } Name("FileDialogEntries") template(move |_| Ok(Part {root, kind: PartKind::Entries})) TabIndex::default() Node { flex_grow: 1.0, min_width: px(0), min_height: px(0), height: percent(100) }),
            ]),
            {filename},
            crate::controls::options(root, style),
            {crate::controls::panels(root)},
            (Name("FileDialogStatus") template(move |_| Ok(Part {root, kind: PartKind::Status})) Text("Loading…") TextFont { font_size: {style.font_size} } template(|_| Ok(Pickable::IGNORE))),
            (Node { width: percent(100), justify_content: JustifyContent::FlexEnd, column_gap: px(style.spacing) } Children [
                control_scene(root, WidgetryFileDialogAction::Confirm, "FileDialogConfirm", label),
                control_scene(root, WidgetryFileDialogAction::Cancel, "FileDialogCancel", "Cancel"),
            ]),
        ]
    }
}

fn confirm_available(state: &WidgetryFileDialogState) -> bool {
    state.session_state() == WidgetryFileDialogSessionState::Open
        && !state.projection_pending()
        && !state.selection_pending()
        && state.confirmation() == &WidgetryFileDialogConfirmation::Idle
        && state.current_path().is_some()
        && match state.mode() {
            WidgetryFileDialogMode::SaveFile => !state.filename().is_empty(),
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
        if world.get::<Node>(root).is_none() {
            world.entity_mut(root).insert(Node {
                width: percent(100),
                height: percent(100),
                ..default()
            });
        }
        bevy_widgetry_core::scene::apply_scene(
            &mut world.entity_mut(root),
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
            },
        ));
        world.write_message(bevy::window::RequestRedraw);
    }
    let colors = world.resource::<ThemeMode>().colors();
    world
        .get_mut::<BackgroundColor>(root)
        .ok_or_else(|| contract_error("FileDialog background missing"))?
        .set_if_neq(BackgroundColor(
            style.background.unwrap_or(colors.window_background),
        ));
    let foreground = ForegroundColor(style.foreground.unwrap_or(colors.foreground));
    let mut propagated = world
        .get_mut::<Propagate<ForegroundColor>>(root)
        .ok_or_else(|| contract_error("FileDialog foreground missing"))?;
    if propagated.0 != foreground {
        propagated.0 = foreground;
    }
    let mut node = world
        .get_mut::<Node>(root)
        .ok_or_else(|| contract_error("FileDialog node missing"))?;
    if node.flex_direction != FlexDirection::Column {
        node.flex_direction = FlexDirection::Column;
    }
    if node.row_gap != px(style.spacing) {
        node.row_gap = px(style.spacing);
    }
    if node.padding != UiRect::all(px(style.padding)) {
        node.padding = UiRect::all(px(style.padding));
    }
    let mut shell = world
        .get::<Shell>(root)
        .cloned()
        .ok_or_else(|| contract_error("FileDialog shell missing"))?;
    if let Some(status_entity) = shell.parts.get(&PartKind::Status) {
        let status = world
            .get::<Notice>(root)
            .map(|notice| notice.0.clone())
            .unwrap_or_else(|| status(&state));
        world
            .get_mut::<Text>(*status_entity)
            .ok_or_else(|| contract_error("FileDialog status missing"))?
            .set_if_neq(Text(status));
    }
    if state.current_path().map(ToOwned::to_owned) != shell.displayed_path {
        if let Some(path_entity) = shell.parts.get(&PartKind::Path) {
            let path = state
                .current_path()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            world
                .get_mut::<EditableText>(*path_entity)
                .ok_or_else(|| contract_error("FileDialog path editor missing"))?
                .editor_mut()
                .set_text(&path);
        }
        shell.displayed_path = state.current_path().map(ToOwned::to_owned);
    }
    world.entity_mut(root).insert(shell);
    if let Some(sidebar) = world
        .get::<Shell>(root)
        .and_then(|shell| shell.parts.get(&PartKind::Sidebar))
        .copied()
    {
        let mut node = world
            .get_mut::<Node>(sidebar)
            .ok_or_else(|| contract_error("FileDialog sidebar missing"))?;
        if node.width != px(style.sidebar_width) {
            node.width = px(style.sidebar_width);
        }
    }
    let area = world
        .get::<Shell>(root)
        .and_then(|shell| shell.parts.get(&PartKind::Entries))
        .copied()
        .ok_or_else(|| contract_error("FileDialog entry area missing"))?;
    crate::view::reconcile(world, root, area, &state, &style)?;
    crate::controls::sync(world, root, &state)?;
    let mut descendants: Vec<_> = world
        .get::<Children>(root)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    for entity in &descendants {
        if let Some(mut node) = world.get_mut::<Node>(*entity)
            && node.column_gap != px(style.spacing)
        {
            node.column_gap = px(style.spacing);
        }
    }
    while let Some(entity) = descendants.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            descendants.extend(children.iter());
        }
        if let Some(mut font) = world.get_mut::<TextFont>(entity)
            && font.font_size != bevy::text::FontSize::Px(style.font_size)
        {
            font.font_size = bevy::text::FontSize::Px(style.font_size);
        }
    }
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
            !confirm_available(&state)
        } else {
            state.session_state() != WidgetryFileDialogSessionState::Open
        };
        let disabled = disabled || world.get::<InteractionDisabled>(root).is_some();
        if disabled && world.get::<InteractionDisabled>(entity).is_none() {
            world.entity_mut(entity).insert(InteractionDisabled);
        } else if !disabled {
            world.entity_mut(entity).remove::<InteractionDisabled>();
        }
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
            background: None,
            foreground: None,
            selected_background: None,
            active_border: None,
        }
    }
}
