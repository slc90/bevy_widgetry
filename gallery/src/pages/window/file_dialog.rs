use super::WindowDemoSection;
use bevy::{
    ecs::system::NonSendMarker, platform::cell::SyncCell, prelude::*, tasks::futures::check_ready,
    ui_widgets::Activate, window::PrimaryWindow, winit::WINIT_WINDOWS,
};
use bevy_widgetry::{button::WidgetryButton, style::ThemeMode};
use rfd::AsyncFileDialog;
use std::{future::Future, pin::Pin};

/// 将五种 native 操作统一为可逐帧 poll 的文本结果。
type FileDialogFuture = Pin<Box<dyn Future<Output = String> + Send + 'static>>;

/// 将异步结果的 poll 收在 native file dialog 示例 module 中。
pub(super) struct FileDialogDemoPlugin;

/// 每个结果文本直接持有 Future，完成后文本即为唯一结果 state。
#[derive(Component, Default)]
struct FileDialogResult {
    /// 非空时忽略重复 Activate；SyncCell 以 exclusive access 满足 Component 的 Sync 要求。
    future: Option<SyncCell<FileDialogFuture>>,
}

/// button 携带操作语义，不使用页面级 ID 表或共享 state。
#[derive(Component, Clone, Copy, Debug)]
enum FileDialogDemo {
    OpenFile,
    OpenFiles,
    SelectFolder,
    OpenImage,
    SaveFile,
}

/// 四个打开类操作并排，保存操作独占下一行。
pub(super) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(WindowDemoSection))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().window_border)))
        Node { width: percent(100), min_width: px(0), border: UiRect::top(px(1)), padding: UiRect::top(px(12)), flex_direction: FlexDirection::Column, row_gap: px(8) }
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

/// 相邻的 button 与结果构成一个示例，hierarchy relationship 用于定位结果 entity。
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

/// 在 main thread 关联 native parent，并立即首次 poll 以启动 rfd 自有的 window thread。
fn open_dialog(
    event: On<Activate>,
    buttons: Query<(&FileDialogDemo, &ChildOf)>,
    children: Query<&Children>,
    mut results: Query<(&mut Text, &mut FileDialogResult)>,
    parent: Single<Entity, With<PrimaryWindow>>,
    _main_thread: NonSendMarker,
) {
    let Ok((&operation, item)) = buttons.get(event.entity) else {
        return;
    };
    let Ok(siblings) = children.get(item.parent()) else {
        error!(entity = ?event.entity, "文件对话框示例缺少结果容器");
        return;
    };
    let mut item_results = results.iter_many_mut(siblings.iter());
    let Some((mut text, mut result)) = item_results.fetch_next() else {
        error!(entity = ?event.entity, "文件对话框示例缺少结果文本");
        return;
    };
    if result.future.is_some() {
        return;
    }
    info!(operation = ?operation, "发起文件对话框操作");
    let dialog = WINIT_WINDOWS.with_borrow(|windows| {
        windows.get_window(*parent).map(|window| {
            AsyncFileDialog::new()
                .set_title(operation.title())
                .set_parent(&**window)
        })
    });
    let Some(dialog) = dialog else {
        warn!(operation = ?operation, "文件对话框无法取得主窗口");
        **text = "Result: Main window unavailable".into();
        return;
    };
    let mut future = dialog_future(dialog, operation);
    // 首次 poll 放在 Activate 调用链中，避免等待 worker 调度或下一帧才启动 window。
    if let Some(value) = check_ready(&mut future) {
        **text = value;
        return;
    }
    **text = "Result: Waiting...".into();
    result.future = Some(SyncCell::new(future));
}

/// 构造 native 操作及结果转换的 Future，首次 poll 时才启动 dialog。
fn dialog_future(dialog: AsyncFileDialog, operation: FileDialogDemo) -> FileDialogFuture {
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
        info!(operation = ?operation, cancelled = files.is_none(), paths = ?files.as_ref().map(|files| files.iter().map(|file| file.path()).collect::<Vec<_>>()), "文件对话框操作完成");
        match files {
            Some(files) => format!(
                "Result: {}",
                files
                    .iter()
                    .map(|file| file.path().display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            None => "Result: Cancelled".into(),
        }
    })
}

/// 每帧只尝试一次 non-blocking 检查，完成后释放 Future 并更新对应文本。
fn poll_results(mut results: Query<(&mut Text, &mut FileDialogResult)>) {
    for (mut text, mut result) in &mut results {
        if let Some(future) = result.future.as_mut()
            && let Some(value) = check_ready(future.get())
        {
            **text = value;
            result.future = None;
        }
    }
}

impl FileDialogDemo {
    /// button 与系统 dialog 共用名称，保持入口和 dialog 语义一致。
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
        app.add_systems(Update, poll_results);
    }
}
