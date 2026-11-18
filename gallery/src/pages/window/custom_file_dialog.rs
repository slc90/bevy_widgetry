mod view;

use crate::i18n::{Language, LocalizedText, TextKey, WindowKey, tr};
use bevy::{prelude::*, text::TextEdit, ui::InteractionDisabled, ui_widgets::Activate};
use bevy_widgetry::{
    diagnostics::ErrorFrame,
    file_dialog::{
        WidgetryFileDialogAction as Action, WidgetryFileDialogChangeEvent,
        WidgetryFileDialogConfirmation as Confirmation,
        WidgetryFileDialogDirectoryState as DirectoryState, WidgetryFileDialogEntry as Entry,
        WidgetryFileDialogEntryId as EntryId, WidgetryFileDialogFilter as Filter,
        WidgetryFileDialogFilterId as FilterId, WidgetryFileDialogHeadless as Dialog,
        WidgetryFileDialogHeadlessPlugin, WidgetryFileDialogMode as Mode,
        WidgetryFileDialogNavigation as Navigation, WidgetryFileDialogResult as DialogResult,
        WidgetryFileDialogResultEvent as ResultEvent, WidgetryFileDialogSelection as Selection,
        WidgetryFileDialogSessionState as SessionState, WidgetryFileDialogSort as Sort,
        WidgetryFileDialogState as State, WidgetryFileDialogToken as Token,
    },
    path_display::display_path,
    scene::spawn_scene,
    text_field::WidgetryTextFieldHeadless as TextField,
};
use std::{ffi::OsStr, path::PathBuf};

#[derive(Component, Clone, Copy, FromTemplate)]
struct Section {
    host: Entity,
    result: Entity,
}

#[derive(Component, Clone, Copy, FromTemplate)]
struct Target {
    root: Entity,
}

#[derive(Component, Clone, Copy)]
struct Launch(Mode);

#[derive(Component, Default)]
struct LoggedFolderGeneration(Option<u64>);

#[derive(Component, Clone)]
enum Control {
    Run(Action),
    Read(Field),
    Choose { id: EntryId, token: Token },
    Confirm,
    Page(PageStep),
    SortByName,
}

#[derive(Clone, Copy)]
enum Field {
    Address,
    Search,
    Folder,
}

#[derive(Clone, Copy)]
enum PageStep {
    Previous,
    Next,
}

#[derive(Clone, Copy)]
enum FileType {
    All,
    Text,
    Image,
}

impl FileType {
    const OPTIONS: [Self; 3] = [Self::All, Self::Text, Self::Image];

    fn id(self) -> FilterId {
        match self {
            Self::All => FilterId::default(),
            Self::Text => FilterId("text".into()),
            Self::Image => FilterId("images".into()),
        }
    }

    fn label(self) -> WindowKey {
        match self {
            Self::All => WindowKey::CustomAllFiles,
            Self::Text => WindowKey::TextDocuments,
            Self::Image => WindowKey::Images,
        }
    }

    fn filter(self, language: Language) -> Filter {
        let (suffixes, default_extension) = match self {
            Self::All => (&[][..], None),
            Self::Text => (&["txt"][..], Some("txt".into())),
            Self::Image => (&["png", "jpg", "jpeg", "bmp", "webp"][..], None),
        };
        Filter {
            id: self.id(),
            label: tr(language, window_key(self.label())).into(),
            suffixes: suffixes.iter().map(|suffix| (*suffix).to_owned()).collect(),
            default_extension,
        }
    }
}

pub(super) fn install(app: &mut App) {
    if !app.is_plugin_added::<WidgetryFileDialogHeadlessPlugin>() {
        app.add_plugins(WidgetryFileDialogHeadlessPlugin);
    }
    app.add_observer(on_result)
        .add_observer(on_folder_created)
        .add_systems(
            PostUpdate,
            view::refresh
                .before(bevy_widgetry_core::ui::WidgetryUiSystems::Build)
                .run_if(any_with_component::<view::Panel>),
        );
}

pub(super) fn scene() -> impl Scene {
    view::section()
}

pub(crate) fn close_embedded_file_dialogs(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, With<view::Panel>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        Dialog::apply(world, root, Action::Cancel)?;
    }
    world.flush();
    Ok(())
}

fn window_key(key: WindowKey) -> TextKey {
    TextKey::Window(key)
}

fn on_launch(event: On<Activate>, targets: Query<(&Target, &Launch)>, mut commands: Commands) {
    let Ok((target, launch)) = targets.get(event.entity) else {
        return;
    };
    let owner = target.root;
    let mode = launch.0;
    commands.queue(move |world: &mut World| open(world, owner, mode));
}

fn open(world: &mut World, owner: Entity, mode: Mode) -> Result {
    let Some(section) = world.get::<Section>(owner).copied() else {
        return Ok(());
    };
    let old = world
        .get::<Children>(section.host)
        .and_then(|children| children.first().copied());
    if let Some(old) = old {
        Dialog::apply(world, old, Action::Cancel)?;
        if let Ok(entity) = world.get_entity_mut(old) {
            entity.despawn();
        }
    }
    let language = *world.resource::<Language>();
    let mut filters = FileType::OPTIONS.map(|kind| kind.filter(language)).to_vec();
    if mode == Mode::SaveFile {
        filters.swap(0, 1);
    }
    let directory = crate::file_dialog_benchmark::fixture_directory(world);
    let root = spawn_scene(
        world,
        view::panel(owner, section.host, mode, filters, directory),
    )
    .map_err(|failure| {
        failure
            .with_context(ErrorFrame::new("gallery.custom_file_dialog.open").field("owner", owner))
            .into_bevy_error()
    })?;
    if let Some(mut result) = world.get_mut::<LocalizedText>(section.result) {
        *result = LocalizedText::new(window_key(WindowKey::ResultWaiting));
    }
    let session = world.get::<State>(root).map(|state| state.token().session);
    info!(?root, ?session, ?mode, "打开内嵌 FileDialog");
    Ok(())
}

fn on_result(
    event: On<ResultEvent>,
    sessions: Query<(&view::Panel, &State, &ChildOf)>,
    sections: Query<&Section>,
    mut results: Query<&mut LocalizedText>,
    mut commands: Commands,
) {
    let Ok((panel, state, parent)) = sessions.get(event.entity) else {
        return;
    };
    if state.token().session != event.session || state.result() != Some(&event.result) {
        return;
    }
    if let Ok(section) = sections.get(panel.owner)
        && parent.parent() == section.host
        && let Ok(mut result) = results.get_mut(section.result)
    {
        let (count, value) = super::file_dialog::summary(&event.result);
        *result = value;
        info!(
            root = ?event.entity,
            session = ?event.session,
            mode = ?state.mode(),
            count,
            cancelled = matches!(event.result, DialogResult::Cancelled),
            "内嵌 FileDialog 返回结果"
        );
    }
    commands.entity(event.entity).try_despawn();
}

fn on_folder_created(
    event: On<WidgetryFileDialogChangeEvent>,
    mut sessions: Query<(&State, &mut LoggedFolderGeneration)>,
) {
    let Ok((state, mut logged)) = sessions.get_mut(event.entity) else {
        return;
    };
    let token = state.token();
    // Committing a successful creation starts a new directory generation with a reveal path.
    if token.session == event.session
        && state.reveal_path().is_some()
        && logged.0 != Some(token.generation)
    {
        logged.0 = Some(token.generation);
        info!(root = ?event.entity, session = ?event.session, "内嵌 FileDialog 创建目录完成");
    }
}

fn on_control(
    event: On<Activate>,
    controls: Query<(&Target, &Control), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok((target, control)) = controls.get(event.entity) else {
        return;
    };
    let root = target.root;
    let button = event.entity;
    let control = control.clone();
    commands.queue(move |world: &mut World| -> Result {
        if world.get_entity(button).is_err() || world.get::<InteractionDisabled>(button).is_some() {
            return Ok(());
        }
        if let Err(error) = operate(world, root, control) {
            if let Ok(mut session) = world.get_entity_mut(root) {
                session.insert(view::Notice(visible_error(&error.to_string()).into()));
            }
            return Err(error);
        }
        Ok(())
    });
}

fn operate(world: &mut World, root: Entity, control: Control) -> Result {
    let Some(state) = world.get::<State>(root) else {
        return Ok(());
    };
    if state.session_state() != SessionState::Open {
        return Ok(());
    }
    let Some(panel) = world.get::<view::Panel>(root).copied() else {
        return Ok(());
    };
    let mode = state.mode();
    let session = state.token().session;
    let action = match control {
        Control::Run(Action::Activate { id, token }) if !current_entry(state, id, token) => {
            return Ok(());
        }
        Control::Run(Action::ClearSelection) if !can_clear_selection(state) => return Ok(()),
        Control::Run(Action::Overwrite { token, .. }) if !matches!(state.confirmation(), Confirmation::AwaitingOverwrite { token: pending, .. } if *pending == token) =>
        {
            return Ok(());
        }
        Control::Run(action) => action,
        Control::Choose { id, token } => {
            if !current_entry(state, id, token) {
                return Ok(());
            }
            let Some(entry) = state.snapshot().and_then(|snapshot| snapshot.entry(id)) else {
                return Ok(());
            };
            match mode {
                Mode::SaveFile if entry.kind().is_file() => {
                    Action::Filename(entry.name().to_owned())
                }
                mode if mode.accepts(entry.kind()) => Action::Select {
                    id,
                    token,
                    operation: if mode.is_multiple() {
                        Selection::Toggle
                    } else {
                        Selection::Replace
                    },
                },
                _ => return Ok(()),
            }
        }
        Control::Read(field) => {
            let Some(text) = draft(world, panel.input(field))? else {
                return Ok(());
            };
            match field {
                Field::Address if text.is_empty() => return Ok(()),
                Field::Address => Action::Navigate(Navigation::Path(
                    state
                        .current_path()
                        .filter(|path| display_path(path) == text)
                        .map_or_else(|| PathBuf::from(text), ToOwned::to_owned),
                )),
                Field::Search => Action::Search(text),
                Field::Folder if !state.can_create_folder() || text.is_empty() => return Ok(()),
                Field::Folder => Action::NewFolder(text.into()),
            }
        }
        Control::Confirm => {
            if edits_filename(mode) {
                let Some(text) = draft(world, panel.filename)? else {
                    return Ok(());
                };
                if !state.can_confirm_filename(OsStr::new(&text)) {
                    return Ok(());
                }
                if text != state.filename_for_editing().to_string_lossy() {
                    Dialog::apply(world, root, Action::Filename(text.as_str().into()))?;
                    view::remember_filename(world, root, &text);
                }
            } else if !state.can_confirm() {
                return Ok(());
            }
            Action::Confirm
        }
        Control::Page(step) => {
            view::change_page(world, root, step);
            return Ok(());
        }
        Control::SortByName => Action::Sort(match state.preferences().sort {
            Sort::NameAscending => Sort::NameDescending,
            _ => Sort::NameAscending,
        }),
    };
    let reset_filename = match &action {
        Action::Select { .. } | Action::ClearSelection => mode == Mode::PickFile,
        Action::Filename(_) => true,
        _ => false,
    };
    world.entity_mut(root).remove::<view::Notice>();
    if let Some(action) = action_name(&action) {
        info!(?root, ?session, action, "内嵌 FileDialog 提交操作请求");
    }
    Dialog::apply(world, root, action)?;
    if reset_filename {
        view::invalidate_filename(world, root);
    }
    Ok(())
}

fn action_name(action: &Action) -> Option<&'static str> {
    Some(match action {
        Action::Navigate(_) => "Navigate",
        Action::Activate { .. } => "EnterDirectory",
        Action::Search(_) => "Search",
        Action::Filter(_) => "SetFileType",
        Action::Sort(_) => "Sort",
        Action::Select { .. } => "SelectEntry",
        Action::Filename(_) => "SelectSaveFilename",
        Action::ClearSelection => "ClearSelection",
        Action::NewFolder(_) => "CreateFolder",
        Action::Confirm => "Confirm",
        Action::Cancel => "Cancel",
        Action::Overwrite { accept: true, .. } => "AcceptOverwrite",
        Action::Overwrite { accept: false, .. } => "RejectOverwrite",
        _ => return None,
    })
}

fn can_clear_selection(state: &State) -> bool {
    !state.projection_pending() && !state.selected().is_empty()
}

fn current_entry(state: &State, id: EntryId, token: Token) -> bool {
    !state.projection_pending()
        && state.matches_query(token)
        && state
            .snapshot()
            .is_some_and(|snapshot| snapshot.visible_index(id).is_some())
}

fn edits_filename(mode: Mode) -> bool {
    matches!(mode, Mode::PickFile | Mode::SaveFile)
}

fn visible_error(message: &str) -> &str {
    let line = message.lines().next().unwrap_or_default();
    line.split_once(" [error_id=")
        .map_or(line, |(reason, _)| reason)
}

fn draft(world: &World, field: Entity) -> Result<Option<String>> {
    let edit = TextField::editor(world, field).map_err(|failure| failure.into_bevy_error())?;
    let composing = edit.is_composing()
        || edit.pending_edits.iter().any(|edit| match edit {
            TextEdit::ImeSetCompose { value, .. } => !value.is_empty(),
            TextEdit::ImeCommit { .. } => true,
            _ => false,
        });
    Ok((!composing).then(|| edit.value().to_string()))
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry::{
        button::WidgetryButtonPlugin,
        file_dialog::{
            WidgetryFileDialogEntryData, WidgetryFileDialogEntryKind, WidgetryFileDialogReply,
            WidgetryFileDialogResult, WidgetryFileDialogRuntimeOptions, WidgetryFileDialogSnapshot,
        },
        scroll_area::WidgetryScrollAreaPlugin,
        text_field::WidgetryTextFieldPlugin,
    };
    use std::sync::Arc;

    fn fixture() -> (App, Entity) {
        let mut app = bevy_widgetry_test_utils::scene_app();
        bevy_widgetry_test_utils::add_ui_plugins(&mut app);
        crate::i18n::install(&mut app);
        app.add_plugins((
            WidgetryButtonPlugin,
            WidgetryScrollAreaPlugin,
            WidgetryTextFieldPlugin,
        ));
        app.insert_resource(WidgetryFileDialogRuntimeOptions {
            automatic: false,
            ..default()
        });
        install(&mut app);
        let owner = spawn_scene(app.world_mut(), scene()).unwrap();
        (app, owner)
    }

    fn active(world: &World, owner: Entity) -> Entity {
        let section = world.get::<Section>(owner).unwrap();
        world.get::<Children>(section.host).unwrap()[0]
    }

    #[test]
    fn changing_modes_disposes_of_the_old_session_and_routes_only_the_current_result() {
        let (mut app, owner) = fixture();
        let world = app.world_mut();
        open(world, owner, Mode::PickFiles).unwrap();
        let previous = active(world, owner);
        let previous_session = world.get::<State>(previous).unwrap().token().session;
        open(world, owner, Mode::SaveFile).unwrap();
        let current = active(world, owner);
        assert!(world.get_entity(previous).is_err());
        assert_eq!(world.get::<State>(current).unwrap().mode(), Mode::SaveFile);
        assert_ne!(
            world.get::<State>(current).unwrap().token().session,
            previous_session
        );
        assert_eq!(world.query::<&Window>().iter(world).count(), 0);

        world.trigger(ResultEvent {
            entity: current,
            session: previous_session,
            result: WidgetryFileDialogResult::Cancelled,
        });
        world.flush();
        assert_eq!(active(world, owner), current);
        let result = world.get::<Section>(owner).unwrap().result;
        assert_eq!(
            world
                .get::<LocalizedText>(result)
                .unwrap()
                .render(Language::English),
            "Result: Waiting..."
        );
        close_embedded_file_dialogs(world).unwrap();
        assert!(world.get_entity(current).is_err());
        assert_eq!(
            world
                .get::<LocalizedText>(result)
                .unwrap()
                .render(Language::English),
            "Result: Cancelled"
        );
    }

    fn ready_directory(world: &mut World, root: Entity) -> (EntryId, Token) {
        let state = world.get::<State>(root).unwrap();
        let token = state.token();
        let directory = PathBuf::from(r"C:\fixture");
        let snapshot = Arc::new(
            WidgetryFileDialogSnapshot::prepare(
                token,
                directory.clone(),
                vec![WidgetryFileDialogEntryData {
                    path: directory.join("first.txt"),
                    name: "first.txt".into(),
                    kind: WidgetryFileDialogEntryKind::File,
                    size: None,
                    modified: None,
                    hidden: None,
                    system: None,
                }],
                &state.query(),
                None,
            )
            .unwrap(),
        );
        Dialog::deliver(
            world,
            root,
            WidgetryFileDialogReply::Started {
                token,
                path: directory,
            },
        )
        .unwrap();
        let selection = world
            .get::<State>(root)
            .unwrap()
            .projection_selection_job(snapshot.clone())
            .unwrap()
            .prepare()
            .unwrap();
        let id = snapshot.visible()[0];
        Dialog::deliver(
            world,
            root,
            WidgetryFileDialogReply::Snapshot {
                snapshot,
                state: DirectoryState::Ready,
                selection,
            },
        )
        .unwrap();
        (id, token)
    }

    fn choose_file(world: &mut World, root: Entity, id: EntryId, token: Token) {
        operate(world, root, Control::Choose { id, token }).unwrap();
        let selection = world
            .get::<State>(root)
            .unwrap()
            .selection_job()
            .unwrap()
            .prepare()
            .unwrap();
        Dialog::deliver(world, root, WidgetryFileDialogReply::Selection(selection)).unwrap();
    }

    #[test]
    fn an_entry_control_from_before_navigation_cannot_select_in_the_next_directory() {
        let (mut app, owner) = fixture();
        let world = app.world_mut();
        open(world, owner, Mode::PickFiles).unwrap();
        let root = active(world, owner);
        let (id, token) = ready_directory(world, root);
        assert!(current_entry(world.get::<State>(root).unwrap(), id, token));

        Dialog::apply(world, root, Action::Navigate(Navigation::Refresh)).unwrap();
        let next = world.get::<State>(root).unwrap().token();
        operate(world, root, Control::Choose { id, token }).unwrap();
        let state = world.get::<State>(root).unwrap();
        assert_eq!(state.token(), next);
        assert!(state.selected().is_empty());
        assert!(!state.selection_pending());
    }

    #[test]
    fn refreshing_the_current_directory_replaces_an_unsubmitted_address() {
        let (mut app, owner) = fixture();
        let world = app.world_mut();
        open(world, owner, Mode::PickFile).unwrap();
        let root = active(world, owner);
        let address = world
            .get::<view::Panel>(root)
            .unwrap()
            .input(Field::Address);
        ready_directory(world, root);
        view::refresh(world);
        TextField::set_text_in_world(world, address, r"C:\other").unwrap();

        operate(
            world,
            root,
            Control::Run(Action::Navigate(Navigation::Refresh)),
        )
        .unwrap();
        view::refresh(world);
        assert_eq!(TextField::text(world, address).unwrap(), r"C:\fixture");
    }

    #[test]
    fn reselecting_a_file_and_leaving_a_directory_discard_the_single_file_draft() {
        let (mut app, owner) = fixture();
        let world = app.world_mut();
        open(world, owner, Mode::PickFile).unwrap();
        let root = active(world, owner);
        let filename = world.get::<view::Panel>(root).unwrap().filename;
        let (id, token) = ready_directory(world, root);
        choose_file(world, root, id, token);
        view::refresh(world);
        TextField::set_text_in_world(world, filename, "other.txt").unwrap();

        operate(world, root, Control::Choose { id, token }).unwrap();
        view::refresh(world);
        assert_eq!(TextField::text(world, filename).unwrap(), "first.txt");

        operate(world, root, Control::Run(Action::ClearSelection)).unwrap();
        let selection = world.get::<State>(root).unwrap().selection_job().unwrap();
        Dialog::deliver(
            world,
            root,
            WidgetryFileDialogReply::Selection(selection.prepare().unwrap()),
        )
        .unwrap();
        view::refresh(world);
        TextField::set_text_in_world(world, filename, "other.txt").unwrap();

        operate(world, root, Control::Run(Action::Navigate(Navigation::Up))).unwrap();
        view::refresh(world);
        assert_eq!(TextField::text(world, filename).unwrap(), "");
    }

    #[test]
    fn save_filename_is_disabled_until_the_candidate_is_rejected() {
        let (mut app, owner) = fixture();
        let world = app.world_mut();
        open(world, owner, Mode::SaveFile).unwrap();
        let root = active(world, owner);
        let filename = world.get::<view::Panel>(root).unwrap().filename;
        let editor = TextField::editor_entity(world, filename).unwrap();
        ready_directory(world, root);
        view::refresh(world);
        TextField::set_text_in_world(world, filename, "first.txt").unwrap();

        operate(world, root, Control::Confirm).unwrap();
        view::refresh(world);
        assert!(world.get::<InteractionDisabled>(editor).is_some());
        let candidate = world
            .get::<State>(root)
            .unwrap()
            .validation_job()
            .unwrap()
            .prepare()
            .unwrap();
        let token = candidate.token();
        Dialog::deliver(
            world,
            root,
            WidgetryFileDialogReply::Validated {
                candidate,
                exists: true,
            },
        )
        .unwrap();
        view::refresh(world);
        assert!(world.get::<InteractionDisabled>(editor).is_some());

        operate(
            world,
            root,
            Control::Run(Action::Overwrite {
                token,
                accept: false,
            }),
        )
        .unwrap();
        view::refresh(world);
        assert!(world.get::<InteractionDisabled>(editor).is_none());
        assert_eq!(TextField::text(world, filename).unwrap(), "first.txt");
    }

    #[test]
    fn clearing_selection_waits_for_the_current_projection() {
        let (mut app, owner) = fixture();
        let world = app.world_mut();
        open(world, owner, Mode::PickFiles).unwrap();
        let root = active(world, owner);
        let (id, token) = ready_directory(world, root);
        choose_file(world, root, id, token);
        view::refresh(world);
        let clear = world
            .query::<(Entity, &Control)>()
            .iter(world)
            .find_map(|(entity, control)| {
                matches!(control, Control::Run(Action::ClearSelection)).then_some(entity)
            })
            .unwrap();
        assert!(world.get::<InteractionDisabled>(clear).is_none());

        operate(world, root, Control::Run(Action::Search("first".into()))).unwrap();
        assert!(world.get::<State>(root).unwrap().projection_pending());
        operate(world, root, Control::Run(Action::ClearSelection)).unwrap();
        view::refresh(world);
        assert!(world.get::<InteractionDisabled>(clear).is_some());
        assert!(world.get::<State>(root).unwrap().selected().contains(&id));
    }
}
