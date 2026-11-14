# Widgetry 统一入口

`bevy_widgetry` 是公开 API 的 Facade。应用从这里导入控件、主题、光标和 Scene 适配，具体行为仍由对应模块的 Plugin 提供。

## 依赖与能力选择

### 使用默认控件集合

```toml
[dependencies]
bevy_widgetry = { git = "https://github.com/slc90/bevy_widgetry" }
```

默认集合包含基础输入、布局、数据视图、Canvas、Waveform 和窗口对话框控件。启用依赖不等于这些控件已经安装到 App；运行时仍需添加实际使用的插件。

### 仅启用需要的控件

```toml
[dependencies.bevy_widgetry]
git = "https://github.com/slc90/bevy_widgetry"
default-features = false
features = ["button", "text_field"]
```

控件 Feature 采用模块名，例如 `button`、`table`、`canvas`、`file_dialog`。Core、Theme、Cursor 和诊断基础入口不依赖默认控件集合。

组合控件仍会引入其实现所需的下层依赖。启用 `file_dialog` 不能让 FileDialog 脱离 Button 或列表能力独立实现；同时，下层 crate 被传递编译，也不代表 Facade 自动公开所有对应模块。应用直接导入某个模块时，应开启它自己的 Feature。

## Plugin 装配

### 添加实际使用的能力

```rust
use bevy::prelude::*;
use bevy_widgetry::{button::WidgetryButtonPlugin, text_field::WidgetryTextFieldPlugin};

app.add_plugins((WidgetryButtonPlugin, WidgetryTextFieldPlugin));
```

控件会确保自己必要的共享插件。Facade 不提供一个替应用决定所有插件、业务状态和错误策略的统一运行时。

泛型列表、表格和下拉框还需要注册对应数据类型及 Renderer，具体步骤在各功能页中说明。

## Scene 入口

### 延迟构造界面

```rust
use bevy_widgetry::{
    button::WidgetryButton,
    scene::WidgetrySceneCommandsExt,
    text::WidgetryText,
};

commands.spawn_scene_with_error_handler(bsn! {
    @WidgetryButton {
        @children: {Some(Box::new(bsn_list! {
            Text("确定") WidgetryText
        }) as Box<dyn SceneList>)}
    }
});
```

此入口把实际构造失败交给应用错误处理器。需要同步获得构造结果时，使用 `scene::spawn_scene(world, scene)`。

## 可选扩展

| Feature | 作用 |
| --- | --- |
| `light`、`pink_dream` | 编译对应颜色预设 |
| `canvas_png_export` | 同时启用 Canvas 与异步 PNG 导出 |
| `job` | 公开后台任务控制入口 |
| `app_logging` | 公开宿主终端与文件日志能力 |
| `tray` | 托盘与原生菜单 |
| `power_inhibit` | 空闲息屏与自动休眠抑制 |
| `ui_scale` | 普通窗口统一缩放 |
| `splash` | 启动画面首帧显示与主窗交接 |
| `path_display` | 公开 Windows 路径显示转换函数 |

这些扩展默认关闭。Cargo Feature 在依赖图中合并；同一个应用的其他依赖开启能力后，不能靠某一条依赖声明将其反向关闭。

## 独立 crate

`waveform_file`、`app_icon_build`、`test_utils` 和库内 `asset` 不通过 Facade 提供统一模块。分别按数据适配、构建期、测试期或库内资源职责直接依赖，避免把它们都塞进生产运行时。

使用示例可以从 [功能目录](README.md) 按能力查找；整体组织关系见 [整体设计](../design/architecture.md)。
