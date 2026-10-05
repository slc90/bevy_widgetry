use bevy::{prelude::*, ui::Checked, ui_widgets::Activate, window::PrimaryWindow};
use bevy_widgetry::{
    button::WidgetryButton, check_box::WidgetryCheckBox, file_dialog::*,
    scene::WidgetrySceneCommandsExt,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(super) struct FileDialogDemoPlugin;
#[derive(Component)]
struct PageOwner;
#[derive(Component)]
struct ModalityToggle;
#[derive(Component)]
struct OrphanedDialog;
#[derive(Component, Default)]
struct DemoResult {
    active: Option<Entity>,
}
#[derive(Component)]
struct DialogRoute {
    owner: Entity,
    result: Entity,
    session: Option<WidgetryFileDialogSessionId>,
}
#[derive(Component, Clone, Copy, Debug)]
enum Demo {
    OpenFile,
    OpenFiles,
    SelectFolder,
    SelectFolders,
    OpenImage,
    SaveFile,
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("GalleryFileDialogs") template(|_| Ok(PageOwner))
        Node {width:percent(100), min_width:px(0), flex_direction:FlexDirection::Column, row_gap:px(8)}
        Children [
            Text("File Dialog"),
            (@WidgetryCheckBox Name("GalleryFileDialogModality") template(|_| Ok(ModalityToggle)) Children [Text("Modal (blocks main window)")]),
            Text("Choose files or folders. Nonmodal dialogs can stay open together."),
            (Node {width:percent(100), column_gap:px(12), align_items:AlignItems::Start} Children [demo_item(Demo::OpenFile), demo_item(Demo::OpenFiles), demo_item(Demo::SelectFolder)]),
            (Node {width:percent(100), column_gap:px(12), align_items:AlignItems::Start} Children [demo_item(Demo::SelectFolders), demo_item(Demo::OpenImage), demo_item(Demo::SaveFile)]),
        ]
    }
}

fn demo_item(operation: Demo) -> impl Scene {
    bsn! {
        Node {flex_direction:FlexDirection::Column, flex_basis:px(0), flex_grow:1.0, min_width:px(0), row_gap:px(8)}
        Children [
            (@WidgetryButton Name({format!("GalleryFileDialog{}Launcher",operation.key())}) template(move |_| Ok(operation))
                Node {height:px(40), padding:UiRect::axes(px(12),px(6)), align_self:AlignSelf::Start, align_items:AlignItems::Center}
                on(open_dialog) Children [Text({operation.title()})]),
            (Name({format!("GalleryFileDialog{}Result",operation.key())}) Text("Result: ...") TextLayout {linebreak:LineBreak::AnyCharacter}
                Node {max_width:percent(100)} template(|_| Ok(DemoResult::default()))),
        ]
    }
}

fn ancestor_with<T: Component>(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<T>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

fn open_dialog(event: On<Activate>, mut commands: Commands) {
    let launcher = event.entity;
    let activated = Instant::now();
    commands.queue(move |world: &mut World| -> Result {
        let Some(operation)=world.get::<Demo>(launcher).copied() else {return Ok(());};
        let Some(owner)=ancestor_with::<PageOwner>(world,launcher) else {return Ok(());};
        let item=world.get::<ChildOf>(launcher).map(ChildOf::parent).ok_or_else(|| BevyError::error("FileDialog launcher container missing"))?;
        let result=world.get::<Children>(item).and_then(|children| children.iter().find(|entity| world.get::<DemoResult>(*entity).is_some())).ok_or_else(|| BevyError::error("FileDialog result text missing"))?;
        if world.get::<DemoResult>(result).and_then(|state| state.active).is_some_and(|root| world.get_entity(root).is_ok()) {return Ok(());}
        let parent=world.query_filtered::<Entity,With<PrimaryWindow>>().single(world).map_err(|error| BevyError::error(format!("FileDialog primary window unavailable: {error}")))?;
        let modal=world.query_filtered::<Entity,(With<ModalityToggle>,With<Checked>)>().iter(world).any(|entity| ancestor_with::<PageOwner>(world,entity)==Some(owner));
        let window=WidgetryFileDialogWindow {parent:Some(parent),modality:if modal {WidgetryFileDialogModality::Modal} else {WidgetryFileDialogModality::NonModal},native:Window {title:operation.title().into(),..WidgetryFileDialogWindow::default().native}};
        let filters=operation.filters();
        let mode=operation.mode();
        let directory=crate::file_dialog_benchmark::fixture_directory(world);
        let sample=crate::file_dialog_benchmark::activate(world,launcher,parent,operation.key(),modal,activated)?;
        let root=world.commands().spawn_scene_with_error_handler(bsn! {
            @WidgetryFileDialog {
                @mode:{mode}, @window:{Some(window)}, @filters:{filters},
                @initial_directory:{directory}, @storage_scope:{Some(format!("gallery.file_dialog.{}",operation.key()))}
            }
            Name({format!("GalleryFileDialog{}",operation.key())})
            template(move |_| Ok(DialogRoute {owner,result,session:None}))
        }).id();
        if let Some(mut state)=world.get_mut::<DemoResult>(result) {state.active=Some(root);}
        if let Some(mut text)=world.get_mut::<Text>(result) {text.0="Result: Waiting...".into();}
        info!(?operation,?root,modal,"打开 FileDialog");
        world.commands().queue(move |world: &mut World| -> Result {
            if world.get::<PageOwner>(owner).is_none() {
                if let Ok(root)=world.get_entity_mut(root) {root.despawn();}
                return Ok(());
            }
            let Some(session)=world.get::<WidgetryFileDialogState>(root).map(|state| state.token().session) else {
                if let Some(mut text)=world.get_mut::<Text>(result) {text.0="Result: Could not open dialog".into();}
                return Ok(());
            };
            if let Some(mut route)=world.get_mut::<DialogRoute>(root) {route.session=Some(session);}
            crate::file_dialog_benchmark::scene_ready(world,sample,root,session)?;
            if matches!(operation,Demo::SaveFile) {WidgetryFileDialog::apply(world,root,WidgetryFileDialogAction::Filename("output.txt".into()))?;}
            Ok(())
        });
        Ok(())
    });
}

fn bounded_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    let mut summary: String = text.chars().take(240).collect();
    if text.chars().nth(240).is_some() {
        summary.push('…');
    }
    summary
}

fn summary(result: &WidgetryFileDialogResult) -> (usize, String) {
    let paths: &[PathBuf] = match result {
        WidgetryFileDialogResult::Cancelled => return (0, "Result: Cancelled".into()),
        WidgetryFileDialogResult::File(path)
        | WidgetryFileDialogResult::Directory(path)
        | WidgetryFileDialogResult::SavePath(path) => std::slice::from_ref(path),
        WidgetryFileDialogResult::Files(paths) | WidgetryFileDialogResult::Directories(paths) => {
            paths
        }
    };
    let mut text = format!("Result: {} path(s)", paths.len());
    for path in paths.iter().take(8) {
        text.push('\n');
        text.push_str(&bounded_path(path));
    }
    if paths.len() > 8 {
        text.push_str("\n…");
    }
    (paths.len(), text)
}

fn on_result(
    event: On<WidgetryFileDialogResultEvent>,
    routes: Query<&DialogRoute>,
    owners: Query<(), With<PageOwner>>,
    mut results: Query<(&mut Text, &mut DemoResult)>,
) {
    let Ok(route) = routes.get(event.entity) else {
        return;
    };
    if route.session != Some(event.session) || !owners.contains(route.owner) {
        return;
    }
    let Ok((mut text, mut state)) = results.get_mut(route.result) else {
        return;
    };
    if state.active != Some(event.entity) {
        return;
    }
    let (count, value) = summary(&event.result);
    text.0 = value;
    state.active = None;
    info!(root=?event.entity,session=?event.session,count,cancelled=matches!(event.result,WidgetryFileDialogResult::Cancelled),"FileDialog 返回结果");
}

fn owner_ended(
    event: On<Despawn, PageOwner>,
    routes: Query<(Entity, &DialogRoute)>,
    mut commands: Commands,
) {
    for (root, route) in &routes {
        if route.owner == event.entity {
            commands.entity(root).try_insert(OrphanedDialog);
        }
    }
}

// BRP 等外部删除可能晚于 Winit 的窗口回收阶段。
// 立即销毁 native Window 会让后续 redraw 找不到 Window，必须在正常关闭阶段回收。
fn cleanup_ended_owners(roots: Query<Entity, With<OrphanedDialog>>, mut commands: Commands) {
    for root in &roots {
        commands.entity(root).try_despawn();
    }
}

fn route_ended(
    event: On<Despawn, DialogRoute>,
    routes: Query<&DialogRoute>,
    mut results: Query<&mut DemoResult>,
) {
    if let Ok(route) = routes.get(event.entity)
        && let Ok(mut result) = results.get_mut(route.result)
        && result.active == Some(event.entity)
    {
        result.active = None;
    }
}

impl Demo {
    fn key(self) -> &'static str {
        match self {
            Self::OpenFile => "OpenFile",
            Self::OpenFiles => "OpenFiles",
            Self::SelectFolder => "SelectFolder",
            Self::SelectFolders => "SelectFolders",
            Self::OpenImage => "OpenImage",
            Self::SaveFile => "SaveFile",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::OpenFile => "Open File",
            Self::OpenFiles => "Open Files",
            Self::SelectFolder => "Select Folder",
            Self::SelectFolders => "Select Folders",
            Self::OpenImage => "Open Image",
            Self::SaveFile => "Save File",
        }
    }
    fn mode(self) -> WidgetryFileDialogMode {
        match self {
            Self::OpenFile | Self::OpenImage => WidgetryFileDialogMode::PickFile,
            Self::OpenFiles => WidgetryFileDialogMode::PickFiles,
            Self::SelectFolder => WidgetryFileDialogMode::PickDirectory,
            Self::SelectFolders => WidgetryFileDialogMode::PickDirectories,
            Self::SaveFile => WidgetryFileDialogMode::SaveFile,
        }
    }
    fn filters(self) -> Vec<WidgetryFileDialogFilter> {
        let filter = match self {
            Self::OpenImage => Some(WidgetryFileDialogFilter {
                id: WidgetryFileDialogFilterId("images".into()),
                label: "Images".into(),
                suffixes: ["png", "jpg", "jpeg", "bmp", "webp"]
                    .map(String::from)
                    .to_vec(),
                default_extension: None,
            }),
            Self::SaveFile => Some(WidgetryFileDialogFilter {
                id: WidgetryFileDialogFilterId("text".into()),
                label: "Text documents".into(),
                suffixes: vec!["txt".into()],
                default_extension: Some("txt".into()),
            }),
            _ => None,
        };
        filter
            .into_iter()
            .chain(std::iter::once(WidgetryFileDialogFilter::default()))
            .collect()
    }
}

impl Plugin for FileDialogDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WidgetryFileDialogPlugin)
            .add_observer(on_result)
            .add_observer(owner_ended)
            .add_observer(route_ended)
            .add_systems(
                Last,
                (cleanup_ended_owners, ApplyDeferred)
                    .chain()
                    .before(bevy::window::close_when_requested),
            );
    }
}
