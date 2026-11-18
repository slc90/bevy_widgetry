# FileDialog

FileDialog 提供文件／目录选择与保存路径确认，可以嵌入现有 UI，也可以创建独立窗口。目录读取、筛选投影和确认校验在后台执行；结果以事件交付，不阻塞应用主循环。

```rust
use bevy::prelude::*;
use bevy_widgetry::file_dialog::*;
use std::path::PathBuf;

app.add_plugins(WidgetryFileDialogPlugin);
```

## 选择模式

### 创建嵌入式选择器

```rust
let dialog = world.spawn_scene(bsn! {
    @WidgetryFileDialog {
        @mode: WidgetryFileDialogMode::PickFile,
        @initial_directory: {Some(PathBuf::from(r"C:\Data"))},
    }
    Node { width: px(900), height: px(600) }
    on(result)
})?.id();

fn result(event: On<WidgetryFileDialogResultEvent>) {
    match &event.result {
        WidgetryFileDialogResult::File(path) => info!(?path, "选择文件"),
        WidgetryFileDialogResult::SavePath(path) => info!(?path, "确认保存路径"),
        WidgetryFileDialogResult::Cancelled => info!("取消选择"),
        other => info!(?other, "选择完成"),
    }
}
```

| 模式 | 结果 |
| --- | --- |
| `PickFile` | `File(PathBuf)` |
| `PickFiles` | `Files(Arc<[PathBuf]>)` |
| `PickDirectory` | `Directory(PathBuf)` |
| `PickDirectories` | `Directories(Arc<[PathBuf]>)` |
| `SaveFile` | `SavePath(PathBuf)` |

SaveFile 只返回目标路径，不代替应用写入文件。取消使用独立的 `Cancelled`，不是空路径或失败诊断。

## 独立窗口

### 打开模态对话框

```rust
let options = WidgetryFileDialogWindow {
    parent: Some(parent_native_window),
    modality: WidgetryFileDialogModality::Modal,
    native: Window {
        title: "打开数据".into(),
        resolution: (960, 640).into(),
        ..default()
    },
};
let scene = bsn! {
    @WidgetryFileDialog {
        @mode: WidgetryFileDialogMode::PickFiles,
        @window: {Some(options)},
    }
    on(result)
};
```

`parent_native_window` 是已绑定 Widgetry 窗口界面的原生 Window Entity，不是某个内容控件根。Modal 必须有有效父窗口；NonModal 可以没有父窗口。

独立 FileDialog 管理自己创建的窗口及关闭流程。嵌入式结果后可以使用 Reopen 开始新 Session；独立窗口不通过 Reopen 原地重用，重新打开应创建新实例。

在已有 Headless 根上挂载独立窗口时，会话必须处于 Open。已决议的 Headless 会话可先执行 Reopen，再挂载窗口。

## 目录导航与搜索

### 发起导航和刷新

```rust
WidgetryFileDialog::queue(
    &mut commands, dialog,
    WidgetryFileDialogAction::Navigate(
        WidgetryFileDialogNavigation::Path(PathBuf::from(r"C:\Data\Session1")),
    ),
);
WidgetryFileDialog::queue(
    &mut commands, dialog,
    WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Refresh),
);
```

Back、Forward、Up 通过相同 Navigation 入口执行。同步场景使用 `WidgetryFileDialog::apply(world, root, action)`；返回值表示状态是否实际变化，不表示后台目录已读完。

### 搜索和排序

```rust
WidgetryFileDialog::queue(
    &mut commands, dialog,
    WidgetryFileDialogAction::Search("report".to_owned()),
);
WidgetryFileDialog::queue(
    &mut commands, dialog,
    WidgetryFileDialogAction::Sort(WidgetryFileDialogSort::ModifiedDescending),
);
```

搜索、过滤与排序可以基于已有目录缓存重建投影，不改变条目的原始路径和稳定身份。后台处理未完成时，界面继续保留当前状态及等待反馈。

默认界面没有为全部 Action 提供按钮：排序、隐藏项、系统项、Pin 与 NewFolder 等能力通过公开动作仍可使用，不能仅根据面板上是否存在按钮判断 API 是否支持。

地址栏等完整路径文字复用 [路径显示](path_display.md) 的转换：Verbatim 磁盘和 UNC 路径显示为熟悉的普通前缀。结果、文件操作和系统图标查询继续保留原始路径，显示文字不作为文件身份。

## 文件类型过滤

### 配置后缀与默认扩展名

```rust
let filter = WidgetryFileDialogFilter {
    id: WidgetryFileDialogFilterId("csv".into()),
    label: "CSV 数据".into(),
    suffixes: vec!["csv".into()],
    default_extension: Some("csv".into()),
};
let scene = bsn! {
    @WidgetryFileDialog {
        @mode: WidgetryFileDialogMode::SaveFile,
        @filters: {vec![filter]},
        @allow_different_extension: false,
    }
};
```

后缀不带前导点，不包含路径分隔或通配字符。默认扩展名必须符合该 Filter 的后缀集合；空集合表示不限制后缀。保存时是否接受不同扩展名由 `allow_different_extension` 决定。

## 选择与确认

### 操作当前选择

Active 是键盘浏览位置，Selection 才是准备确认的集合。多选模式支持 Ctrl 点击、Shift 范围和 SelectAll；选择可能需要后台准备，不能把已点亮一行当作最终确认完成。

```rust
WidgetryFileDialog::queue(&mut commands, dialog, WidgetryFileDialogAction::SelectAll);
WidgetryFileDialog::queue(&mut commands, dialog, WidgetryFileDialogAction::Confirm);
```

Confirm 会等待必要的 Selection、Projection 和 Validation。Cancel 不需要等待后台 I/O：

```rust
WidgetryFileDialog::queue(&mut commands, dialog, WidgetryFileDialogAction::Cancel);
```

每个 Session 最多交付一次结果。`WidgetryFileDialogChangeEvent` 表示已提交状态变化；`ResultEvent` 表示该 Session 已决议，两者不是同一个时机。

### 设置保存文件名

```rust
WidgetryFileDialog::queue(
    &mut commands, dialog,
    WidgetryFileDialogAction::Filename("result.csv".into()),
);
```

`Filename` 支持 SaveFile 和 PickFile。PickFile 的文件名草稿在 `Confirm` 时解析为当前可见文件的 EntryId，找不到时会在 `state.error()` 报告原因。修改草稿或重新选择条目会清除该提示。选择行时可以直接提交 `Select` 和 `Confirm`，也可以使用 `Activate`。界面可以用有损文字显示原生文件名，提交选择时仍使用原始 EntryId。已选项的原生名字优先匹配，包括提交其有损显示文字的情况。

目标已存在时进入覆盖确认。覆盖回答携带原候选 Token，过期回答不会授权新路径。独立窗口复用 MessageBox 展示确认；自定义承接方式也必须保留候选身份。

## 偏好与持久化

### 在同一业务范围复用偏好

```rust
bsn! {
    @WidgetryFileDialog {
        @storage_scope: {Some("import-measurement".to_owned())},
        @mode: WidgetryFileDialogMode::PickFile,
    }
}
```

Scope 决定哪些实例共享目录偏好等状态，不等于 Session ID。应用配置 `WidgetryFileDialogPersistence` 后才能执行持久化；内存共享并不自动意味着已写入磁盘。

需要主动安排刷新时调用：

```rust
WidgetryFileDialog::flush_preferences(world)
    .map_err(FileDialogOperationFailure::into_bevy_error)?;
```

该入口安排刷新请求，磁盘完成仍由运行时状态判断。不要把一次 API 成功当作进程退出前所有写盘已经完成。

## 文件图标与内容布局

默认行先显示文件／文件夹占位图标，再在后台取得系统关联图标。普通扩展名可共享缓存，具有专属图标的路径分别处理；图标不可用时保留占位，不阻止选择和确认。

滚动复用行会立即绑定新 EntryId，并恢复对应占位；异步图标只更新所属缓存，再投影到当前行，不直接写回发起请求时的旧 Entity。

`WidgetryFileDialogStyle` 与颜色覆盖控制布局和状态外观。修改样式不重置 Session、目录、文本光标或选择。默认文件区域独立滚动，窄空间下操作区可以换行。

默认界面也通过公开 Headless 查询和动作处理文件名、文件夹、条目选择与确认。渲染和输入法组合态由默认控件管理，单文件名称匹配、保存扩展名规则及文件系统验证由会话处理。

Gallery 的 Window 页在默认示例下提供“内嵌文件选择器”。可以选择单文件、多文件、单目录、多目录或保存模式，在分页网格中浏览目录并使用搜索和文件类型筛选。会话将选中的路径或取消结果显示在模式入口下方。离开 Window 页会回收内嵌界面及其会话。

## 无默认界面运行

只安装 `WidgetryFileDialogHeadlessPlugin`，即可用 BSN 创建独立会话。会话根持有 `WidgetryFileDialogState`，默认窗口、列表和编辑框由上层界面决定。

```rust
app.add_plugins(WidgetryFileDialogHeadlessPlugin);

let dialog = world.spawn_scene(bsn! {
    @WidgetryFileDialogHeadless {
        @mode: WidgetryFileDialogMode::PickFile,
        @initial_directory: {Some(PathBuf::from(r"C:\Data"))},
    }
    on(result)
})?.id();

// Once the directory is available, submit the edited filename and confirm.
WidgetryFileDialogHeadless::queue(
    &mut commands,
    dialog,
    WidgetryFileDialogAction::Filename("report.csv".into()),
);
WidgetryFileDialogHeadless::queue(&mut commands, dialog, WidgetryFileDialogAction::Confirm);
```

- 查询 `directory_state()`、`snapshot()`、`visible()` 和 `selected()` 绘制列表。使用 `state.token()` 或当时用于显示行的快照 Token 提交选择，避免对旧目录或旧投影操作。
- 用 `can_go_back()`、`can_go_forward()`、`can_go_up()` 和 `can_confirm()` 驱动按钮。可提交确认表示会话已就绪且具备必要输入，提交后仍需等待文件名匹配和后台校验。用 `projection_pending()`、`selection_pending()`、`folder_request()` 和 `confirmation()` 显示各阶段的处理状态。
- `ChangeEvent` 在会话状态提交后发送，`ResultEvent` 每个 Session 最多发送一次。覆盖确认由调用方显示，再以 `AwaitingOverwrite` 中的 Token 提交 `Overwrite`。使用 `Reopen` 在已决议的根实体上开启新会话，或销毁根实体结束其生命周期。

`WidgetryFileDialogBackend` 和 `WidgetryFileDialogFileSystem` 可替换文件系统访问。手动模式可以用 `WidgetryFileDialogHeadless::deliver` 提交回复。回复必须携带相应的 Session 和请求 Token，Runtime 会丢弃过期或目标已销毁的回复。
