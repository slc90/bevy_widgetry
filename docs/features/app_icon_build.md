# EXE 图标构建

`bevy_widgetry_app_icon_build` 在应用构建时把 ICO 嵌入可执行文件。它是 build dependency，不是运行时 Plugin，也不通过 Facade 导出。

## 图标嵌入

### 添加构建依赖

```toml
[build-dependencies]
bevy_widgetry_app_icon_build = { git = "https://github.com/slc90/bevy_widgetry" }
```

在应用包中准备 `icons/app.ico`，再创建 `build.rs`：

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    bevy_widgetry_app_icon_build::set_exe_icon("icons/app.ico")?;
    Ok(())
}
```

成功后图标位于 EXE 资源中，不要求运行时另外读取这个 ICO。构建函数会发出 `cargo:rerun-if-changed`，图标变化后由 Cargo 重新执行构建。

### 选择路径

相对路径以调用应用的 `CARGO_MANIFEST_DIR` 为基准，不以工具库所在目录为基准。也可以传入绝对路径。

```rust
let icon = std::path::Path::new("branding").join("application.ico");
bevy_widgetry_app_icon_build::set_exe_icon(icon)?;
```

路径必须能被资源编译器接受。文件缺失、无效编码或资源编译失败会返回错误，使应用构建明确失败，而不是无声生成没有目标图标的 EXE。

## 构建错误

`IconBuildFailure` 的领域类别区分环境变量、路径、文件读取和资源编译。错误保留最终路径与底层原因。构建脚本可以直接返回该失败，也可以在自己的构建报告中增加上下文。

该工具依赖 Cargo 提供的构建环境，不适合作为普通运行时 API 调用。

## 与运行时图标的区别

EXE 文件图标、窗口/任务栏图标和托盘图标是三种独立能力。这里修改的是可执行文件资源；窗口图标由 Window 能力设置，托盘图标由 Tray 创建。应用可以复用品牌素材，但一个设置不会自动替代另外两个。
