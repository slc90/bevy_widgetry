mod worker;

use bevy::{
    ecs::system::NonSendMarker,
    platform::cell::SyncCell,
    prelude::*,
    tasks::futures::check_ready,
    ui_widgets::Activate,
    window::PrimaryWindow,
    winit::{EventLoopProxyWrapper, WINIT_WINDOWS, WinitUserEvent},
};
use bevy_widgetry::button::WidgetryButton;
#[cfg(not(target_os = "windows"))]
use rfd::AsyncFileDialog as FileDialog;
#[cfg(target_os = "windows")]
use rfd::FileDialog;
use std::{future::Future, pin::Pin};
use worker::FileDialogWorker;

type FileDialogFuture = Pin<Box<dyn Future<Output = Result<String>> + Send + 'static>>;

pub(super) struct FileDialogDemoPlugin;

#[derive(Component, Default)]
struct FileDialogResult {
    future: Option<SyncCell<FileDialogFuture>>,
}

#[derive(Component, Clone, Copy, Debug)]
enum FileDialogDemo {
    OpenFile,
    OpenFiles,
    SelectFolder,
    OpenImage,
    SaveFile,
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { width: percent(100), min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text("File Dialog"),
            (Node { width: percent(100), column_gap: px(12), align_items: AlignItems::Start } Children [
                demo_item(FileDialogDemo::OpenFile),
                demo_item(FileDialogDemo::OpenFiles),
                demo_item(FileDialogDemo::SelectFolder),
                demo_item(FileDialogDemo::OpenImage),
            ]),
            (Node { width: percent(100), margin: UiRect::top(px(8)) } Children [
                demo_item(FileDialogDemo::SaveFile),
            ]),
        ]
    }
}

fn demo_item(operation: FileDialogDemo) -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, flex_basis: px(0), flex_grow: 1.0, min_width: px(0), row_gap: px(8) }
        Children [
            (@WidgetryButton
                template(move |_| Ok(operation))
                Node { height: px(40), padding: UiRect::axes(px(12), px(6)), align_self: AlignSelf::Start, align_items: AlignItems::Center }
                on(open_dialog)
                Children [Text({operation.title()})]),
            (Text("Result: ...")
                TextLayout { linebreak: LineBreak::AnyCharacter }
                Node { max_width: percent(100) }
                template(|_| Ok(FileDialogResult::default()))),
        ]
    }
}

fn open_dialog(
    event: On<Activate>,
    buttons: Query<(&FileDialogDemo, &ChildOf)>,
    children: Query<&Children>,
    mut results: Query<(&mut Text, &mut FileDialogResult)>,
    parent: Single<(Entity, &mut FileDialogWorker), With<PrimaryWindow>>,
    event_loop: Res<EventLoopProxyWrapper>,
    _main_thread: NonSendMarker,
) -> Result {
    let Ok((&operation, item)) = buttons.get(event.entity) else {
        return Ok(());
    };
    let Ok(siblings) = children.get(item.parent()) else {
        error!(entity = ?event.entity, "文件对话框示例缺少结果容器");
        return Err(BevyError::error("文件对话框示例缺少结果容器"));
    };
    // 专用 thread 顺序执行 native modal dialog，不让不同按钮排队打开过期 window。
    if results.iter().any(|(_, result)| result.future.is_some()) {
        return Ok(());
    }
    let mut item_results = results.iter_many_mut(siblings.iter());
    let Some((mut text, mut result)) = item_results.fetch_next() else {
        error!(entity = ?event.entity, "文件对话框示例缺少结果文本");
        return Err(BevyError::error("文件对话框示例缺少结果文本"));
    };
    info!(operation = ?operation, "发起文件对话框操作");
    let (parent_entity, mut worker) = parent.into_inner();
    let dialog = WINIT_WINDOWS.with_borrow(|windows| {
        windows.get_window(parent_entity).map(|window| {
            FileDialog::new()
                .set_title(operation.title())
                .set_parent(&**window)
        })
    });
    let Some(dialog) = dialog else {
        warn!(operation = ?operation, "文件对话框无法取得主窗口");
        **text = "Result: Main window unavailable".into();
        return Ok(());
    };
    let proxy = event_loop.clone();
    let mut future = worker.start(dialog, operation, move || {
        if let Err(error) = proxy.send_event(WinitUserEvent::WakeUp) {
            warn!(%error, "文件对话框完成后无法唤醒已关闭的 event loop");
        }
    })?;
    // 非 Windows 的 AsyncFileDialog 到首次 poll 才启动，Reactive App 必须立即 poll。
    if let Some(value) = check_ready(&mut future) {
        **text = value?;
        return Ok(());
    }
    **text = "Result: Waiting...".into();
    result.future = Some(SyncCell::new(future));
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn dialog_future(dialog: FileDialog, operation: FileDialogDemo) -> FileDialogFuture {
    Box::pin(async move {
        let files = match operation {
            FileDialogDemo::OpenFile => dialog.pick_file().await.map(|file| vec![file]),
            FileDialogDemo::OpenFiles => dialog.pick_files().await,
            FileDialogDemo::SelectFolder => dialog.pick_folder().await.map(|file| vec![file]),
            FileDialogDemo::OpenImage => dialog
                .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp"])
                .pick_file()
                .await
                .map(|file| vec![file]),
            FileDialogDemo::SaveFile => dialog
                .set_file_name("output.txt")
                .save_file()
                .await
                .map(|file| vec![file]),
        };
        Ok(format_result(
            operation,
            files.map(|files| {
                files
                    .into_iter()
                    .map(|file| file.path().to_owned())
                    .collect()
            }),
        ))
    })
}

fn format_result(operation: FileDialogDemo, files: Option<Vec<std::path::PathBuf>>) -> String {
    info!(operation = ?operation, cancelled = files.is_none(), paths = ?files, "文件对话框操作完成");
    match files {
        Some(files) => format!(
            "Result: {}",
            files
                .iter()
                .map(|file| file.display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        ),
        None => "Result: Cancelled".into(),
    }
}

fn poll_results(mut results: Query<(&mut Text, &mut FileDialogResult)>) -> Result {
    for (mut text, mut result) in &mut results {
        if let Some(future) = result.future.as_mut()
            && let Some(value) = check_ready(future.get())
        {
            result.future = None;
            match value {
                Ok(value) => **text = value,
                Err(error) => {
                    **text = "Result: Worker unavailable".into();
                    return Err(error);
                }
            }
        }
    }
    Ok(())
}

fn setup_worker(mut commands: Commands, parent: Single<Entity, With<PrimaryWindow>>) {
    commands.entity(*parent).insert(FileDialogWorker::default());
}

impl FileDialogDemo {
    fn title(self) -> &'static str {
        match self {
            Self::OpenFile => "Open File",
            Self::OpenFiles => "Open Files",
            Self::SelectFolder => "Select Folder",
            Self::OpenImage => "Open Image",
            Self::SaveFile => "Save File",
        }
    }
}

impl Plugin for FileDialogDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_worker)
            .add_systems(Update, poll_results);
    }
}
