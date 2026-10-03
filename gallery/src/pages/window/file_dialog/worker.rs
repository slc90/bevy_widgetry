use super::{FileDialog, FileDialogDemo, FileDialogFuture};
use bevy::prelude::*;
#[cfg(target_os = "windows")]
use std::{future::poll_fn, sync::mpsc, task::Poll, thread};

#[derive(Component, Default)]
pub(super) struct FileDialogWorker {
    #[cfg(target_os = "windows")]
    sender: Option<mpsc::Sender<FileDialogRequest>>,
}

#[cfg(target_os = "windows")]
struct FileDialogRequest {
    dialog: FileDialog,
    operation: FileDialogDemo,
    result: mpsc::Sender<String>,
    wake: Box<dyn FnOnce() + Send>,
}

#[cfg(target_os = "windows")]
fn run(requests: mpsc::Receiver<FileDialogRequest>) {
    for request in requests {
        // AsyncFileDialog 每次新建 thread，会重复触发 IME thread 初始化。
        // 在同一 thread 调用同步 API，让 rfd 管理每次调用的 STA COM lifecycle，保留 IME。
        let files = match request.operation {
            FileDialogDemo::OpenFile => request.dialog.pick_file().map(|file| vec![file]),
            FileDialogDemo::OpenFiles => request.dialog.pick_files(),
            FileDialogDemo::SelectFolder => request.dialog.pick_folder().map(|file| vec![file]),
            FileDialogDemo::OpenImage => request
                .dialog
                .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp"])
                .pick_file()
                .map(|file| vec![file]),
            FileDialogDemo::SaveFile => request
                .dialog
                .set_file_name("output.txt")
                .save_file()
                .map(|file| vec![file]),
        };
        let value = super::format_result(request.operation, files);
        if request.result.send(value).is_err() {
            // 页面被销毁后不再有结果 Text，仍须释放当前 dialog 并继续正常结束 worker。
            info!(operation = ?request.operation, "文件对话框结果对应的页面已关闭");
        }
        (request.wake)();
    }
}

impl FileDialogWorker {
    pub(super) fn start(
        &mut self,
        dialog: FileDialog,
        operation: FileDialogDemo,
        _wake: impl FnOnce() + Send + 'static,
    ) -> Result<FileDialogFuture> {
        #[cfg(not(target_os = "windows"))]
        {
            Ok(super::dialog_future(dialog, operation))
        }
        #[cfg(target_os = "windows")]
        {
            if self.sender.is_none() {
                let (sender, requests) = mpsc::channel();
                thread::Builder::new()
                    .name("gallery-file-dialog".into())
                    .spawn(move || run(requests))
                    .map_err(|error| {
                        error!(%error, "文件对话框无法启动 worker");
                        BevyError::error(error)
                    })?;
                self.sender = Some(sender);
            }
            let Some(sender) = &self.sender else {
                error!("文件对话框 worker 缺少请求通道");
                return Err(BevyError::error("文件对话框 worker 缺少请求通道"));
            };
            let (result, receiver) = mpsc::channel();
            if let Err(error) = sender.send(FileDialogRequest {
                dialog,
                operation,
                result,
                wake: Box::new(_wake),
            }) {
                self.sender = None;
                error!(%error, "文件对话框 worker 已停止");
                return Err(BevyError::error("文件对话框 worker 已停止"));
            }
            Ok(Box::pin(poll_fn(move |_| match receiver.try_recv() {
                Ok(value) => Poll::Ready(Ok(value)),
                Err(mpsc::TryRecvError::Empty) => Poll::Pending,
                Err(mpsc::TryRecvError::Disconnected) => {
                    error!("文件对话框 worker 未返回结果便停止");
                    Poll::Ready(Err(BevyError::error("文件对话框 worker 未返回结果便停止")))
                }
            })))
        }
    }
}
