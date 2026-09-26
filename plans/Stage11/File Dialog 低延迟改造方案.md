# File Dialog 低延迟改造方案

## 目标

针对当前 `gallery/src/pages/window/file_dialog.rs` 的实现，减少点击按钮后到 Windows 原生文件选择窗口真正出现之间的延迟。

当前实现链路为：

```text
Activate
  ↓
构造 AsyncFileDialog
  ↓
IoTaskPool::spawn(...)
  ↓
等待 Bevy worker 首次 poll task
  ↓
rfd AsyncFileDialog 首次 poll
  ↓
rfd 内部创建 Windows dialog 线程
  ↓
IFileDialog::Show()
```

这里 `IoTaskPool` 属于多余的一层异步调度。

目标改成：

```text
Activate
  ↓
构造 AsyncFileDialog
  ↓
构造 rfd Future
  ↓
在 Activate handler 中立即 poll 一次
  ↓
rfd 立即启动自己的 Windows dialog 线程
  ↓
返回 Bevy 主循环
  ↓
后续 Update 仅负责继续 poll Future 和写回 Result Text
```

这样可以让“启动文件窗口”发生在按钮激活的同一条调用链里，而不是等到 Bevy 的 IoTaskPool worker 之后才开始。

---

## 核心原则

- 继续使用 `rfd::AsyncFileDialog`
- 不使用同步 `rfd::FileDialog`
- 不使用 `IoTaskPool`
- 不使用额外 thread / channel
- 不阻塞 Bevy 主线程
- File Dialog 仍然以 Gallery 主窗口为 parent
- Result Text Entity 继续同时持有：
  - `Text`
  - File Dialog Future 状态
- 点击按钮后立即对 Future 做一次非阻塞 poll
- 后续帧再由统一 system 继续 poll
- 保持现有页面结构和 UI 不变

---

## 当前需要删除的结构

当前类似：

```rust
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};

#[derive(Component, Default)]
struct FileDialogResult {
    task: Option<Task<String>>,
}
```

改造后：

- 删除 `IoTaskPool`
- 删除 `Task<String>`
- `FileDialogResult` 不再持有 Bevy Task

---

## Result Text Entity 改为直接持有 Future

建议定义：

```rust
use std::{
    future::Future,
    pin::Pin,
};

type FileDialogFuture =
    Pin<Box<dyn Future<Output = String> + Send + 'static>>;

#[derive(Component, Default)]
struct FileDialogResult {
    future: Option<FileDialogFuture>,
}
```

语义仍然保持：

```text
Result Text Entity
├─ Text("Result: ...")
└─ FileDialogResult
   └─ future: Option<FileDialogFuture>
```

Result Text 本身仍然是唯一可见状态。

---

## Future 创建

把不同操作统一构造成一个返回 `String` 的 Future。

建议拆一个纯辅助函数：

```rust
fn dialog_future(
    dialog: AsyncFileDialog,
    operation: FileDialogDemo,
) -> FileDialogFuture {
    Box::pin(async move {
        let files = match operation {
            FileDialogDemo::OpenFile => {
                dialog.pick_file().await.map(|file| vec![file])
            }

            FileDialogDemo::OpenFiles => {
                dialog.pick_files().await
            }

            FileDialogDemo::SelectFolder => {
                dialog.pick_folder().await.map(|folder| vec![folder])
            }

            FileDialogDemo::OpenImage => {
                dialog
                    .add_filter(
                        "Images",
                        &["png", "jpg", "jpeg", "bmp", "webp"],
                    )
                    .pick_file()
                    .await
                    .map(|file| vec![file])
            }

            FileDialogDemo::SaveFile => {
                dialog
                    .set_file_name("output.txt")
                    .save_file()
                    .await
                    .map(|file| vec![file])
            }
        };

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
```

如果现有代码里更适合直接内联，也可以不拆 helper；重点是 Future 本身直接存在 `FileDialogResult` 上。

---

## 点击时立即 first-poll

这是本次改造最重要的部分。

`open_dialog()` 中：

1. 找到对应 Result Text Entity
2. 如果已有未完成 Future，直接返回
3. 在主线程中获取 Gallery 主窗口
4. `set_parent(...)`
5. 创建 File Dialog Future
6. **立即调用 `check_ready()` poll 一次**
7. 如果已经完成，直接写结果
8. 如果 Pending，则保存到 Result Text Entity

示意：

```rust
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
        warn!(
            entity = ?event.entity,
            "文件对话框示例缺少结果容器"
        );
        return;
    };

    let mut item_results = results.iter_many_mut(siblings.iter());

    let Some((mut text, mut result)) = item_results.fetch_next() else {
        warn!(
            entity = ?event.entity,
            "文件对话框示例缺少结果文本"
        );
        return;
    };

    if result.future.is_some() {
        return;
    }

    let dialog = WINIT_WINDOWS.with_borrow(|windows| {
        windows.get_window(*parent).map(|window| {
            AsyncFileDialog::new()
                .set_title(operation.title())
                .set_parent(&**window)
        })
    });

    let Some(dialog) = dialog else {
        **text = "Result: Main window unavailable".into();
        return;
    };

    let mut future = dialog_future(dialog, operation);

    // 关键：在 Activate handler 中立即第一次 poll。
    if let Some(value) = check_ready(&mut future) {
        **text = value;
        return;
    }

    **text = "Result: Waiting...".into();
    result.future = Some(future);
}
```

这里的 `check_ready()` 必须是非阻塞 poll。

不要 `block_on`。

---

## 为什么 first-poll 要放在点击 handler 里

如果只写：

```rust
result.future = Some(dialog_future(dialog, operation));
```

然后等 `poll_results()` 下一帧才第一次 poll，那么流程仍然会变成：

```text
点击
  ↓
保存 Future
  ↓
等待下一次 Update
  ↓
Future 首次 poll
  ↓
rfd 才开始创建 Windows dialog 线程
```

虽然比 `IoTaskPool` 少了一层，但仍会人为晚一帧。

所以应在点击 handler 里立刻 first-poll：

```text
点击
  ↓
Future 首次 poll
  ↓
rfd 立即启动 dialog thread
  ↓
handler 返回
```

这就是本方案追求的最低额外调度延迟。

---

## 后续帧轮询

`poll_results()` 保持简单：

```rust
fn poll_results(
    mut results: Query<(&mut Text, &mut FileDialogResult)>,
) {
    for (mut text, mut result) in &mut results {
        let Some(future) = result.future.as_mut() else {
            continue;
        };

        let Some(value) = check_ready(future) else {
            continue;
        };

        **text = value;
        result.future = None;
    }
}
```

也可以继续使用当前 edition 2024 风格：

```rust
fn poll_results(
    mut results: Query<(&mut Text, &mut FileDialogResult)>,
) {
    for (mut text, mut result) in &mut results {
        if let Some(future) = result.future.as_mut()
            && let Some(value) = check_ready(future)
        {
            **text = value;
            result.future = None;
        }
    }
}
```

---

## import 调整

当前：

```rust
use bevy::{
    ecs::system::NonSendMarker,
    prelude::*,
    tasks::{IoTaskPool, Task, futures::check_ready},
    ...
};
```

改为类似：

```rust
use std::{
    future::Future,
    pin::Pin,
};

use bevy::{
    ecs::system::NonSendMarker,
    prelude::*,
    tasks::futures::check_ready,
    ...
};
```

不再需要：

```rust
IoTaskPool
Task
```

---

## parent 逻辑保持不变

当前通过：

```rust
WINIT_WINDOWS.with_borrow(...)
```

获取主窗口并：

```rust
.set_parent(&**window)
```

这个方向保持。

`open_dialog()` 继续带：

```rust
_main_thread: NonSendMarker
```

以确保访问 `WINIT_WINDOWS` 和 parent 绑定发生在主线程。

不要改成自己提取 raw handle，也不要引入 unsafe。

---

## 重复点击保护保持不变

原来：

```rust
if result.task.is_some() {
    return;
}
```

改为：

```rust
if result.future.is_some() {
    return;
}
```

同一个 demo 正在等待时，不重复打开第二个 dialog。

---

## UI 不变

不要因为这次异步实现调整修改 File Dialog 页面排版。

继续保持：

```text
────────────────────────────────
File Dialog

[ Open File ] [ Open Files ] [ Select Folder ] [ Open Image ]
Result: ...    Result: ...     Result: ...       Result: ...

[ Save File ]
Result: ...
```

---

## Plugin 不变

仍然只需要：

```rust
impl Plugin for FileDialogDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, poll_results);
    }
}
```

不需要新增 executor plugin、channel resource 或 runtime。

---

## 不要做的事

这次不要：

- 使用 `IoTaskPool`
- 使用 `AsyncComputeTaskPool`
- 使用 `std::thread::spawn`
- 使用 channel
- 使用 `block_on`
- 使用同步 `FileDialog`
- 新增页面级 Resource
- 改动 Window / MessageBox 示例
- 改 File Dialog UI
- 引入 unsafe

rfd Windows backend 自己已经负责把原生阻塞 dialog 放到独立线程，因此外面不需要再人为套一层线程池。

---

## 验收重点

改完重点验证：

- [ ] 点击按钮后文件窗口启动速度比当前更快
- [ ] `IoTaskPool` 已完全移除
- [ ] Result Text Entity 直接持有 rfd Future
- [ ] 点击 handler 中立即 first-poll
- [ ] Bevy 主线程不阻塞
- [ ] Gallery 主窗口仍然作为 File Dialog parent
- [ ] File Dialog 打开时主窗口保持模态关系
- [ ] Future 完成后对应 Result Text 正确更新
- [ ] Cancel 正确显示 `Result: Cancelled`
- [ ] 同一按钮在 Future 未完成时重复点击不会创建第二个 dialog
- [ ] `cargo fmt`
- [ ] `cargo check`
- [ ] `cargo clippy` 通过

---

## 预期最终链路

改造后：

```text
Widgetry Button Activate
        ↓
open_dialog()
        ↓
获取 Result Text Entity
        ↓
获取主 winit Window
        ↓
AsyncFileDialog::new()
        ↓
set_parent(...)
        ↓
构造 Future
        ↓
check_ready() 立即 first-poll
        ↓
rfd Windows backend 启动自己的 dialog thread
        ↓
open_dialog() 返回
        ↓
Bevy 正常继续运行
        ↓
Update / poll_results()
        ↓
Future Ready
        ↓
更新同 Entity 上的 Text
```

这版的目标不是消除 Windows Shell / COM 自身创建原生 File Dialog 的时间，而是把我们自己额外引入的 Bevy task 调度延迟全部去掉。
