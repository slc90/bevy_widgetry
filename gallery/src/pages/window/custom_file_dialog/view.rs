use super::*;
use crate::i18n::{localized, localized_binding};
use bevy::ecs::template::EntityTemplate;
use bevy_widgetry::{
    button::WidgetryButton, scene::apply_scene, scroll_area::WidgetryScrollArea,
    text::WidgetryText, text_field::WidgetryTextField,
};
use bevy_widgetry_core::disabled::set_intrinsic_disabled;
use std::{ffi::OsString, path::Path};

const PANEL_PADDING: f32 = 12.0;
const HEADER_SIZE: f32 = 22.0;
const CONTROL_HEIGHT: f32 = 34.0;
const CONTROL_PADDING: f32 = 9.0;
const GAP: f32 = 8.0;
const INSPECTOR_WIDTH: f32 = 288.0;
const BROWSER_MIN_WIDTH: f32 = 360.0;
const TILE_COLUMNS: u16 = 3;
const PAGE_SIZE: usize = 9;
const SELECTION_PREVIEW_COUNT: usize = 3;
const OVERWRITE_HEIGHT: f32 = 64.0;

#[derive(Component, FromTemplate, Clone, Copy)]
#[require(LoggedFolderGeneration)]
pub(super) struct Panel {
    pub(super) owner: Entity,
    address: Entity,
    search: Entity,
    folder: Entity,
    pub(super) filename: Entity,
    browser_status: Entity,
    selection: Entity,
    page_label: Entity,
    entries: Entity,
    overwrite: Entity,
}

impl Panel {
    pub(super) fn input(self, field: Field) -> Entity {
        match field {
            Field::Address => self.address,
            Field::Search => self.search,
            Field::Folder => self.folder,
        }
    }
}

#[derive(Component, Clone, Default, PartialEq, Eq)]
struct Projection {
    page: usize,
    entry_token: Option<Token>,
    entry_ids: Vec<EntryId>,
    shown_address: Option<(u64, PathBuf)>,
    shown_filename: Option<ShownFilename>,
    overwrite_token: Option<Token>,
}

#[derive(Clone, PartialEq, Eq)]
struct ShownFilename {
    name: OsString,
    explicit: bool,
    directory_generation: u64,
}

impl ShownFilename {
    fn from_state(state: &State) -> Self {
        Self {
            name: state.filename_for_editing().to_owned(),
            explicit: state.explicit_filename().is_some(),
            directory_generation: state.token().generation,
        }
    }

    fn matches(&self, state: &State) -> bool {
        self.name == state.filename_for_editing()
            && self.explicit == state.explicit_filename().is_some()
            && (state.mode() != Mode::PickFile
                || self.directory_generation == state.token().generation)
    }
}

#[derive(Component)]
pub(super) struct Notice(pub(super) String);

#[derive(Component, Clone)]
struct EntryLabel {
    root: Entity,
    id: EntryId,
    name: String,
}

fn label(key: WindowKey) -> impl Scene {
    bsn! { @localized(window_key(key)) WidgetryText Pickable::IGNORE }
}

fn button_content(content: impl Scene + 'static) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @{content} }) as Box<dyn SceneList>)} }
        Node {
            min_width: px(0), height: px(CONTROL_HEIGHT), flex_shrink: 0.0,
            align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            padding: UiRect::horizontal(px(CONTROL_PADDING)),
        }
    }
}

fn button(
    root: EntityTemplate,
    name: impl Into<String>,
    key: WindowKey,
    control: Control,
) -> impl Scene {
    let name = name.into();
    bsn! {
        @button_content(label(key)) Name(name)
        Target { root: root } template(move |_| Ok(control.clone())) on(on_control)
    }
}

pub(super) fn section() -> impl Scene {
    let launchers = [
        (Mode::PickFile, "OpenFile", WindowKey::OpenFile),
        (Mode::PickFiles, "OpenFiles", WindowKey::OpenFiles),
        (Mode::PickDirectory, "SelectFolder", WindowKey::SelectFolder),
        (
            Mode::PickDirectories,
            "SelectFolders",
            WindowKey::SelectFolders,
        ),
        (Mode::SaveFile, "SaveFile", WindowKey::SaveFile),
    ];
    bsn! {
        #Section Name("GalleryCustomFileDialogSection") Section {host:#Host, result:#Result}
        Node {width:percent(100), min_width:px(0), flex_direction:FlexDirection::Column, row_gap:px(GAP)}
        Children [
            @label(WindowKey::CustomFileDialog) TextFont {font_size:px(HEADER_SIZE)}--
            Node {flex_wrap:FlexWrap::Wrap, column_gap:px(GAP), row_gap:px(GAP)}
            @launcher_buttons(#Section, launchers)--
            #Result @label(WindowKey::Result) Name("GalleryCustomFileDialogResult")
                TextLayout {linebreak:LineBreak::AnyCharacter}--
            #Host Node {width:percent(100), flex_direction:FlexDirection::Column}
        ]
    }
}

fn launcher_buttons(
    owner: EntityTemplate,
    launches: [(Mode, &'static str, WindowKey); 5],
) -> impl Scene {
    let buttons = launches.map(|(mode, name, key)| {
        bsn! {
            @button_content(label(key)) Name({format!("GalleryCustomFileDialog{name}Launcher")})
            Target {root:owner} template(move |_| Ok(Launch(mode))) on(on_launch)
        }
    });
    bsn! { Children [{buttons.into_iter().collect::<Vec<_>>()}] }
}

pub(super) fn panel(
    owner: Entity,
    host: Entity,
    mode: Mode,
    filters: Vec<Filter>,
    directory: Option<PathBuf>,
) -> impl Scene {
    let title = match mode {
        Mode::PickFile => WindowKey::OpenFile,
        Mode::PickFiles => WindowKey::OpenFiles,
        Mode::PickDirectory => WindowKey::SelectFolder,
        Mode::PickDirectories => WindowKey::SelectFolders,
        Mode::SaveFile => WindowKey::SaveFile,
    };
    let filename_display = if edits_filename(mode) {
        Display::Flex
    } else {
        Display::None
    };
    bsn! {
        #Dialog @Dialog {@mode:mode, @filters:filters, @initial_directory:directory}
        @Dialog::prevent_reopen(|_| true)
        Name("GalleryCustomFileDialog") ChildOf(host)
        Panel {
            owner:owner, address:#Address, search:#Search, folder:#Folder, filename:#Filename,
            browser_status:#Status, selection:#Selection, page_label:#Page,
            entries:#Entries, overwrite:#Overwrite,
        }
        template(|_| Ok(Projection::default()))
        Node {width:percent(100), min_width:px(0), flex_wrap:FlexWrap::Wrap,
            padding:UiRect::all(px(PANEL_PADDING)), column_gap:px(PANEL_PADDING), row_gap:px(PANEL_PADDING)}
        Children [
            Node {width:px(INSPECTOR_WIDTH), max_width:percent(100), flex_direction:FlexDirection::Column, row_gap:px(GAP)}
            Children [
                @label(title) TextFont {font_size:px(HEADER_SIZE)}--
                #Selection Text("") WidgetryText TextLayout {linebreak:LineBreak::AnyCharacter} Name("GalleryCustomFileDialogSelection")--
                Node {display:{filename_display}, flex_direction:FlexDirection::Column, row_gap:px(GAP)}
                Children [@label(WindowKey::CustomFilename)-- #Filename @WidgetryTextField Name("GalleryCustomFileDialogFilename")]--
                Node {column_gap:px(GAP), row_gap:px(GAP), flex_wrap:FlexWrap::Wrap} Children [
                    @button(#Dialog, "GalleryCustomFileDialogConfirm", title, Control::Confirm)--
                    @button(#Dialog, "GalleryCustomFileDialogCancel", WindowKey::CustomCancel, Control::Run(Action::Cancel))--
                    @button(#Dialog, "GalleryCustomFileDialogClear", WindowKey::CustomClear, Control::Run(Action::ClearSelection))
                ]--
                #Overwrite Node {width:percent(100), flex_direction:FlexDirection::Column, row_gap:px(GAP)}--
                @label(WindowKey::CustomNewFolder)--
                Node {column_gap:px(GAP)} Children [
                    #Folder @WidgetryTextField Name("GalleryCustomFileDialogFolderName") Node {flex_grow:1.0, min_width:px(0)}--
                    @button(#Dialog, "GalleryCustomFileDialogCreateFolder", WindowKey::CustomCreate, Control::Read(Field::Folder))
                ]
            ]--
            Node {min_width:px(BROWSER_MIN_WIDTH), flex_grow:1.0, flex_basis:px(0),
                flex_direction:FlexDirection::Column, row_gap:px(GAP)}
            Children [
                Node {column_gap:px(GAP)} Children [
                    #Address @WidgetryTextField Name("GalleryCustomFileDialogAddress") Node {flex_grow:1.0, min_width:px(0)}--
                    @button(#Dialog, "GalleryCustomFileDialogGo", WindowKey::CustomGo, Control::Read(Field::Address))
                ]--
                @navigation(#Dialog)--
                Node {column_gap:px(GAP)} Children [
                    #Search @WidgetryTextField Name("GalleryCustomFileDialogSearch") Node {flex_grow:1.0, min_width:px(0)}--
                    @button(#Dialog, "GalleryCustomFileDialogApplySearch", WindowKey::CustomSearch, Control::Read(Field::Search))
                ]--
                @file_types(#Dialog)--
                #Status Text("") WidgetryText TextLayout {linebreak:LineBreak::AnyCharacter} Name("GalleryCustomFileDialogStatus")--
                #Entries Node {width:percent(100), display:Display::Grid,
                    grid_template_columns:{vec![RepeatedGridTrack::flex(TILE_COLUMNS, 1.0)]},
                    column_gap:px(GAP), row_gap:px(GAP)}--
                Node {column_gap:px(GAP), align_items:AlignItems::Center} Children [
                    @button(#Dialog, "GalleryCustomFileDialogPreviousPage", WindowKey::CustomPreviousPage, Control::Page(PageStep::Previous))--
                    #Page Text("") WidgetryText Name("GalleryCustomFileDialogPage")--
                    @button(#Dialog, "GalleryCustomFileDialogNextPage", WindowKey::CustomNextPage, Control::Page(PageStep::Next))
                ]
            ]
        ]
    }
}

fn navigation(root: EntityTemplate) -> impl Scene {
    bsn! {
        Node {column_gap:px(GAP), row_gap:px(GAP), flex_wrap:FlexWrap::Wrap} Children [
            @button(root, "GalleryCustomFileDialogBack", WindowKey::CustomBack, Control::Run(Action::Navigate(Navigation::Back)))--
            @button(root, "GalleryCustomFileDialogForward", WindowKey::CustomForward, Control::Run(Action::Navigate(Navigation::Forward)))--
            @button(root, "GalleryCustomFileDialogUp", WindowKey::CustomUp, Control::Run(Action::Navigate(Navigation::Up)))--
            @button(root, "GalleryCustomFileDialogRefresh", WindowKey::CustomRefresh, Control::Run(Action::Navigate(Navigation::Refresh)))--
            @button(root, "GalleryCustomFileDialogSort", WindowKey::CustomSort, Control::SortByName)
        ]
    }
}

fn file_types(root: EntityTemplate) -> impl Scene {
    let buttons = FileType::OPTIONS
        .into_iter()
        .map(|kind| {
            let name = match kind {
                FileType::All => "GalleryCustomFileDialogAllFiles",
                FileType::Text => "GalleryCustomFileDialogTextFiles",
                FileType::Image => "GalleryCustomFileDialogImages",
            };
            button(
                root,
                name,
                kind.label(),
                Control::Run(Action::Filter(kind.id())),
            )
        })
        .collect::<Vec<_>>();
    bsn! { Node {column_gap:px(GAP), row_gap:px(GAP), flex_wrap:FlexWrap::Wrap} Children [{buttons}] }
}

fn entry(root: Entity, index: usize, token: Token, entry: &Entry) -> impl Scene + use<> {
    let id = entry.id();
    let name = entry.name().to_string_lossy().into_owned();
    let label = EntryLabel {
        root,
        id,
        name: name.clone(),
    };
    let enter = entry.kind().is_directory().then(|| {
        button(
            root.into(),
            format!("GalleryCustomFileDialogEnter{index}"),
            WindowKey::CustomEnterFolder,
            Control::Run(Action::Activate { id, token }),
        )
    });
    bsn! {
        Node {min_width:px(0), flex_direction:FlexDirection::Column, row_gap:px(GAP)} Children [
            @button_content(bsn! {
                Text(name) WidgetryText Pickable::IGNORE TextLayout {linebreak:LineBreak::NoWrap}
                template(move |_| Ok(label.clone()))
            })
            Name({format!("GalleryCustomFileDialogEntry{index}")})
            Node {width:percent(100), overflow:Overflow::clip_x()}
            Target {root:root} template(move |_| Ok(Control::Choose {id, token})) on(on_control)--
            @{enter}
        ]
    }
}

fn overwrite(root: Entity, token: Token, path: &Path) -> impl SceneList + use<> {
    let target = LocalizedText::with_args(
        window_key(WindowKey::CustomOverwrite),
        vec![("path", display_path(path))],
    );
    bsn_list! {
        @WidgetryScrollArea {@keyboard_scroll:false, @children:bsn_list! {
            @localized_binding(target) WidgetryText TextLayout {linebreak:LineBreak::AnyCharacter}
            Name("GalleryCustomFileDialogOverwriteTarget")
        }}
        Node {width:percent(100), height:px(OVERWRITE_HEIGHT)}--
        Node {column_gap:px(GAP), row_gap:px(GAP), flex_wrap:FlexWrap::Wrap} Children [
            @button(root.into(), "GalleryCustomFileDialogReplace", WindowKey::CustomReplace, Control::Run(Action::Overwrite {token, accept:true}))--
            @button(root.into(), "GalleryCustomFileDialogKeepEditing", WindowKey::CustomKeepEditing, Control::Run(Action::Overwrite {token, accept:false}))
        ]
    }
}

pub(super) fn change_page(world: &mut World, root: Entity, step: PageStep) {
    if let Some(mut projection) = world.get_mut::<Projection>(root) {
        projection.page = match step {
            PageStep::Previous => projection.page.saturating_sub(1),
            PageStep::Next => projection.page.saturating_add(1),
        };
    }
}

pub(super) fn remember_filename(world: &mut World, root: Entity, draft: &str) {
    if let Some(state) = world.get::<State>(root)
        && state.filename_for_editing().to_string_lossy() == draft
    {
        let shown = ShownFilename::from_state(state);
        if let Some(mut projection) = world.get_mut::<Projection>(root) {
            projection.shown_filename = Some(shown);
        }
    }
}

pub(super) fn invalidate_filename(world: &mut World, root: Entity) {
    if let Some(mut projection) = world.get_mut::<Projection>(root) {
        projection.shown_filename = None;
    }
}

pub(super) fn refresh(world: &mut World) {
    let pending = world
        .query::<(Entity, &Panel, &State)>()
        .iter(world)
        .map(|(root, panel, state)| (root, *panel, state.clone()))
        .collect::<Vec<_>>();
    for (root, panel, state) in pending {
        if let Err(error) = refresh_panel(world, root, panel, &state) {
            world
                .commands()
                .queue(move |_: &mut World| -> Result { Err(error) });
        }
    }
}

fn refresh_panel(world: &mut World, root: Entity, panel: Panel, state: &State) -> Result {
    let Some(mut projection) = world.get::<Projection>(root).cloned() else {
        return Err(crate::diagnostics::error(
            "Custom FileDialog projection is missing",
        ));
    };
    let result = project(world, root, panel, state, &mut projection);
    if let Some(mut cached) = world.get_mut::<Projection>(root) {
        cached.set_if_neq(projection);
    }
    result
}

fn project(
    world: &mut World,
    root: Entity,
    panel: Panel,
    state: &State,
    projection: &mut Projection,
) -> Result {
    let language = *world.resource::<Language>();
    let token = state.token();
    let address = state.requested_path().or_else(|| state.current_path());
    let shown_address = projection
        .shown_address
        .as_ref()
        .map(|(generation, path)| (*generation, path.as_path()));
    if shown_address != address.map(|path| (token.generation, path)) {
        TextField::set_text_in_world(
            world,
            panel.address,
            address.map(display_path).unwrap_or_default(),
        )
        .map_err(|failure| failure.into_bevy_error())?;
        projection.shown_address = address.map(|path| (token.generation, path.to_owned()));
    }
    if edits_filename(state.mode()) {
        set_intrinsic_disabled(
            world,
            panel.filename,
            state.confirmation() != &Confirmation::Idle,
        )
        .map_err(|failure| failure.into_bevy_error())?;
        if !state.selection_pending()
            && projection
                .shown_filename
                .as_ref()
                .is_none_or(|shown| !shown.matches(state))
            && draft(world, panel.filename)?.is_some()
        {
            TextField::set_text_in_world(
                world,
                panel.filename,
                state.filename_for_editing().to_string_lossy(),
            )
            .map_err(|failure| failure.into_bevy_error())?;
            projection.shown_filename = Some(ShownFilename::from_state(state));
        }
    }
    let pages = state.visible().len().div_ceil(PAGE_SIZE).max(1);
    if projection
        .entry_token
        .is_some_and(|previous| !state.matches_query(previous))
    {
        projection.page = 0;
    }
    projection.page = projection.page.min(pages - 1);
    let start = projection.page * PAGE_SIZE;
    let ids = if state.projection_pending() {
        Vec::new()
    } else {
        (start..start.saturating_add(PAGE_SIZE))
            .filter_map(|index| state.visible().get(index).copied())
            .collect::<Vec<_>>()
    };
    if projection
        .entry_token
        .is_none_or(|previous| !state.matches_query(previous))
        || projection.entry_ids != ids
    {
        let rows = ids
            .iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let row = state.snapshot()?.entry(*id)?;
                Some(entry(root, index, token, row))
            })
            .collect::<Vec<_>>();
        replace_children(world, panel.entries, bsn! {Children [{rows}]})?;
        projection.entry_ids = ids;
        projection.entry_token = Some(token);
    }
    let awaiting = match state.confirmation() {
        Confirmation::AwaitingOverwrite { path, token } => Some((path, *token)),
        _ => None,
    };
    let overwrite_token = awaiting.map(|(_, token)| token);
    if projection.overwrite_token != overwrite_token {
        replace_children(
            world,
            panel.overwrite,
            bsn! {
                Children [{awaiting.map(|(path, token)| overwrite(root, token, path))}]
            },
        )?;
        projection.overwrite_token = overwrite_token;
    }
    let status = world
        .get::<Notice>(root)
        .map(|notice| notice.0.clone())
        .unwrap_or_else(|| status_text(state, language));
    set_text(world, panel.browser_status, status);
    set_text(world, panel.selection, selection_text(state, language));
    set_text(
        world,
        panel.page_label,
        format!("{} / {pages}", projection.page + 1),
    );

    let filename_draft = if edits_filename(state.mode()) {
        draft(world, panel.filename)?
    } else {
        None
    };
    let controls = world
        .query::<(Entity, &Target, &Control)>()
        .iter(world)
        .filter(|(_, target, _)| target.root == root)
        .map(|(entity, _, control)| (entity, control.clone()))
        .collect::<Vec<_>>();
    for (entity, control) in controls {
        let enabled = match control {
            Control::Run(Action::Navigate(Navigation::Back)) => state.can_go_back(),
            Control::Run(Action::Navigate(Navigation::Forward)) => state.can_go_forward(),
            Control::Run(Action::Navigate(Navigation::Up)) => state.can_go_up(),
            Control::Run(Action::Navigate(Navigation::Refresh)) => address.is_some(),
            Control::Run(Action::Activate { id, token }) => current_entry(state, id, token),
            Control::Run(Action::ClearSelection) => can_clear_selection(state),
            Control::Run(Action::Filter(ref id)) => state.preferences().filter != *id,
            Control::Read(Field::Folder) => state.can_create_folder(),
            Control::Choose { id, token } => {
                current_entry(state, id, token)
                    && state
                        .snapshot()
                        .and_then(|snapshot| snapshot.entry(id))
                        .is_some_and(|entry| {
                            state.mode().accepts(entry.kind())
                                || (state.mode() == Mode::SaveFile && entry.kind().is_file())
                        })
            }
            Control::Page(PageStep::Previous) => projection.page > 0,
            Control::Page(PageStep::Next) => projection.page + 1 < pages,
            Control::Confirm if edits_filename(state.mode()) => filename_draft
                .as_deref()
                .is_some_and(|text| state.can_confirm_filename(OsStr::new(text))),
            Control::Confirm => state.can_confirm(),
            _ => true,
        };
        set_intrinsic_disabled(world, entity, !enabled)
            .map_err(|failure| failure.into_bevy_error())?;
    }
    let labels = world
        .query::<(Entity, &EntryLabel)>()
        .iter(world)
        .filter(|(_, label)| label.root == root)
        .map(|(entity, label)| (entity, label.clone()))
        .collect::<Vec<_>>();
    for (entity, label) in labels {
        let selected = state.selected().contains(&label.id);
        set_text(
            world,
            entity,
            if selected {
                format!("✓ {}", label.name)
            } else {
                label.name
            },
        );
    }
    Ok(())
}

fn replace_children(world: &mut World, host: Entity, scene: impl Scene) -> Result {
    let mut entity = world
        .get_entity_mut(host)
        .map_err(crate::diagnostics::error)?;
    entity.despawn_children();
    apply_scene(&mut entity, scene).map_err(|failure| {
        failure
            .with_context(ErrorFrame::new("gallery.custom_file_dialog.project").field("host", host))
            .into_bevy_error()
    })
}

fn set_text(world: &mut World, entity: Entity, value: String) {
    if let Some(mut text) = world.get_mut::<Text>(entity) {
        text.set_if_neq(Text(value));
    }
}

fn status_text(state: &State, language: Language) -> String {
    if let Some(error) = state.error() {
        return visible_error(error).into();
    }
    if state.confirmation() == &Confirmation::Validating {
        return tr(language, window_key(WindowKey::CustomValidating)).into();
    }
    if state.projection_pending() {
        return tr(language, window_key(WindowKey::CustomFiltering)).into();
    }
    let key = match state.directory_state() {
        DirectoryState::Failed(error) => {
            return LocalizedText::with_args(
                window_key(WindowKey::CustomDirectoryFailed),
                vec![("error", visible_error(error).into())],
            )
            .render(language);
        }
        DirectoryState::Partial { errors, .. } => {
            return LocalizedText::with_args(
                window_key(WindowKey::CustomDirectoryPartial),
                vec![("count", errors.to_string())],
            )
            .render(language);
        }
        DirectoryState::Loading => WindowKey::CustomLoading,
        DirectoryState::Ready if state.entries().is_empty() => WindowKey::CustomEmpty,
        DirectoryState::Ready if state.visible().is_empty() => WindowKey::CustomNoMatches,
        DirectoryState::Ready => {
            let file_type = FileType::OPTIONS
                .into_iter()
                .find(|kind| kind.id() == state.preferences().filter);
            return LocalizedText::with_args(
                window_key(WindowKey::CustomEntryCount),
                vec![("count", state.visible().len().to_string())],
            )
            .with_key_arg(
                "type",
                window_key(file_type.map_or(WindowKey::CustomAllFiles, FileType::label)),
            )
            .render(language);
        }
    };
    tr(language, window_key(key)).into()
}

fn selection_text(state: &State, language: Language) -> String {
    if state.selection_pending() {
        return tr(language, window_key(WindowKey::CustomUpdatingSelection)).into();
    }
    if state.selected().is_empty() {
        if state.mode() == Mode::PickDirectory
            && let Some(path) = state.current_path()
        {
            return LocalizedText::with_args(
                window_key(WindowKey::CustomCurrentFolder),
                vec![("path", super::super::file_dialog::bounded_path(path))],
            )
            .render(language);
        }
        return tr(language, window_key(WindowKey::CustomNoSelection)).into();
    }
    let mut paths = String::new();
    if let Some(snapshot) = state.snapshot() {
        for entry in state
            .selected()
            .iter()
            .take(SELECTION_PREVIEW_COUNT)
            .filter_map(|id| snapshot.entry(*id))
        {
            paths.push('\n');
            paths.push_str(&entry.name().to_string_lossy());
        }
    }
    if state.selected().len() > SELECTION_PREVIEW_COUNT {
        paths.push_str("\n…");
    }
    LocalizedText::with_args(
        window_key(WindowKey::CustomSelectedCount),
        vec![
            ("count", state.selected().len().to_string()),
            ("paths", paths),
        ],
    )
    .render(language)
}
