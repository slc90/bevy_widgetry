# Widgetry EXE 与任务栏图标通用 API 下沉方案

## 1. 目标

将目前 `gallery` 中设置 Windows EXE 文件图标和运行时任务栏图标的实现下沉到 `crates/`，形成 Widgetry 可复用的应用级能力。

对外只提供两个正式 API：

1. `set_exe_icon(path)`：在 Cargo 构建期间将 ICO 图标嵌入 EXE。
2. `taskbar_icon!(path)`：将 PNG 图标编译进程序，并在运行时自动设置主窗口及任务栏图标。

调用方只负责提供图标路径，不需要接触 `winresource`、`winit`、图片解码或 Bevy 图标设置 system。

保留现有的构建行为、运行时行为和图标视觉效果，不引入额外资源文件分发要求。

本次属于现有能力的封装与迁移，不扩展额外的窗口图标管理功能。

## 2. 当前实现

### 2.1 EXE 文件图标

现有实现位于：

- `gallery/build.rs`
- `gallery/Cargo.toml`

通过 `winresource::WindowsResource` 在构建阶段，将 `widget_gallery_exe.ico` 嵌入 Windows PE 资源。

该功能属于 Cargo build script 阶段，与 Bevy ECS、Window 或 AssetServer 无关。

### 2.2 运行时任务栏图标

现有实现位于：

- `gallery/src/main.rs`
- `gallery/src/assets.rs`
- `gallery/src/assets/constants.rs`

实现流程：

1. 使用 `include_bytes!` 在编译期间嵌入 PNG。
2. 使用 `image` 解码 PNG，取得 RGBA 像素。
3. 使用 `winit::window::Icon::from_rgba` 创建原生图标。
4. 通过 `WINIT_WINDOWS` 取得主窗口对应的原生 winit Window。
5. 调用 `set_window_icon` 设置原生窗口图标。
6. 调用 `set_taskbar_icon` 设置 Windows 任务栏图标。

当前通过 Bevy `Update` system 执行，配合 `NonSendMarker` 与 `Local<bool>`：

- 保证操作在主线程执行。
- 原生窗口尚未就绪时继续等待。
- 设置成功后不再重复执行。

这一生命周期需要完整保留。

### 2.3 当前职责问题

这两处实现已经具有明确的通用性，却仍由 Gallery 直接维护。

其他应用若希望设置自己的图标，需要复制相同的构建脚本和运行时 system。

下沉后，Gallery 应只保留图标路径及 API 调用。

## 3. 总体架构

采用两个不同的归属。

```text
crates/
├── app_icon_build/               # 新增
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
│
├── window/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── native_icon.rs        # 新增
│
└── bevy_widgetry/
    └── src/
        └── lib.rs               # 运行时 API re-export

gallery/
├── build.rs                      # 改为调用通用 API
├── Cargo.toml
└── src/
    ├── main.rs                  # 改为调用通用 API
    ├── assets.rs
    └── assets/
        ├── constants.rs
        └── icons/
            ├── widget_gallery_exe.ico
            └── widget_gallery_taskbar.png
```

### 3.1 构建期能力：app_icon_build

新增 `crates/app_icon_build`。

Cargo package 名称：

`bevy_widgetry_app_icon_build`

职责仅限于封装 EXE 图标的构建期资源设置。

该 crate：

- 不依赖 Bevy。
- 不依赖 `crates/window`。
- 不依赖顶层 `bevy_widgetry`。
- 使用 workspace 统一管理的 `winresource`。
- 供应用通过 `[build-dependencies]` 引用。
- 不设置任何具体应用的默认图标。

不能直接将 `gallery/build.rs` 移动到 `crates/window/build.rs`。

原因是 Cargo build script 的构建产物归属于对应 package。库 crate 自身执行的构建脚本，不等价于直接为最终应用 EXE 写入图标资源。

因此必须保留调用方的 `build.rs`，将其中的实际逻辑封装为公共函数。

### 3.2 运行时能力：window

运行时任务栏图标功能下沉到现有 `crates/window`。

原因：

- 已经涉及原生 Window 操作。
- 与 `window` crate 当前职责相符。
- 可以复用现有 Windows x64 平台约束。
- 无需单独建立运行时图标 crate。
- 不需要修改现有 Widgetry 自绘标题栏、窗口生命周期或 rendering 架构。

通过顶层 facade：

`bevy_widgetry::window`

导出运行时图标配置宏。

不要将该能力放入 `crates/core` 或 `crates/asset`。

## 4. 对外 API 设计

### 4.1 API 1：EXE 图标

公开函数：

```rust
pub fn set_exe_icon(
    path: impl AsRef<std::path::Path>,
) -> std::io::Result<()>
```

正式调用方式：

```rust
fn main() -> std::io::Result<()> {
    bevy_widgetry_app_icon_build::set_exe_icon(
        "src/assets/icons/widget_gallery_exe.ico",
    )
}
```

路径约定：

- 接收 `.ico` 文件路径。
- 相对路径以调用方 Cargo package 根目录为基准。
- 不依赖启动程序时的 current working directory。
- 文件不存在、无法读取或资源编译失败时，正常返回错误。
- 不使用 `unwrap`、`expect` 或 `panic!`。

内部继续使用 `winresource`，并输出对应的 `cargo:rerun-if-changed`。

确保 ICO 内容发生修改时，Cargo 会重新执行图标资源构建。

API 不负责将 PNG 转换为 ICO。调用方必须提供可用于 Windows EXE 资源的 ICO 文件。

### 4.2 API 2：运行时任务栏图标

公开宏：

```rust
bevy_widgetry::window::taskbar_icon!(
    "src/assets/icons/widget_gallery_taskbar.png"
)
```

正式调用方式：

```rust
app.add_plugins(
    bevy_widgetry::window::taskbar_icon!(
        "src/assets/icons/widget_gallery_taskbar.png"
    )?
);
```

该宏返回：

```rust
Result<impl Plugin, BevyError>
```

这里的 `impl Plugin` 用于表达公开调用契约，具体返回类型可以是内部 Plugin 类型，不要求调用方直接构造。

需要保留以下能力：

- 调用方只传入一个 PNG 路径字面量。
- 编译期嵌入 PNG 文件。
- 不要求运行时读取外部 PNG。
- 自动注册所需 Bevy system。
- 自动等待原生主窗口创建。
- 自动设置原生窗口与任务栏图标。
- 成功设置一次后停止重复执行。

#### 为什么使用宏

普通 Rust 函数无法仅凭运行时传入的字符串路径，在编译阶段自动将对应文件嵌入程序。

因此运行时图标 API 采用声明宏。

宏内部可使用类似：

```rust
include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/",
    $path
))
```

其中 `$path` 必须是字符串字面量。

`CARGO_MANIFEST_DIR` 应在调用方 crate 的编译上下文中求值，确保实际嵌入的是应用自己的图标，而不是 `crates/window` 内部的文件。

宏应通过 `$crate` 引用其实现 crate 的内部接口，避免经 facade re-export 后发生路径解析错误。

允许使用 `#[doc(hidden)]` 的内部桥接项满足 Rust 导出宏的可见性要求，但它们不属于正式支持的应用 API，也不通过 facade 单独导出。

## 5. 构建期实现

### 5.1 创建 crate

新增：

`crates/app_icon_build/Cargo.toml`

按照项目依赖与命名规则配置：

```toml
[package]
name = "bevy_widgetry_app_icon_build"
version.workspace = true
edition.workspace = true

[dependencies]
winresource.workspace = true

[lints]
workspace = true
```

在根 `Cargo.toml` 的 workspace members 中注册该 crate。

不新增重复的 `winresource` 版本声明，继续使用现有 `workspace.dependencies`。

### 5.2 实现 set_exe_icon

在 `crates/app_icon_build/src/lib.rs` 中实现。

内部要求：

1. 将输入路径解析到调用方 Cargo package。
2. 通过 `CARGO_MANIFEST_DIR` 获取调用方 package 根目录。
3. 输出 `cargo:rerun-if-changed`。
4. 调用 `winresource::WindowsResource` 设置 ICO。
5. 执行资源编译。
6. 将错误向调用方返回。

需要特别检查 `CARGO_MANIFEST_DIR` 的获取与当前工作目录是否一致，不应暗中依赖执行位置。

由于该函数在调用方 build script 中执行，其环境属于调用方构建环境。

保留 Windows target 的资源编译语义。不要使用 build helper crate 自身的编译宿主平台信息代替目标平台判断。

当前项目限定 Windows x64，优先满足现有目标；不需要额外扩展跨平台构建能力。

### 5.3 Gallery 迁移

将 `gallery/build.rs` 简化为第 4.1 节的调用形式。

迁移完成后，该文件不应再直接出现：

- `winresource::WindowsResource`
- `.set_icon(...)`
- `.compile()`

所有这些细节应位于 `app_icon_build` 内部。

## 6. 运行时实现

### 6.1 新增模块

创建：

`crates/window/src/native_icon.rs`

负责：

- 编译期 PNG 字节对应的运行时初始化。
- PNG 解码与 RGBA 转换。
- 原生图标创建。
- Bevy Plugin 装配。
- 主窗口原生图标设置。
- 一次性应用状态管理。

`crates/window/src/lib.rs` 只负责声明模块及必要的 re-export。

不要在 `lib.rs` 堆放具体 system 实现。

### 6.2 PNG 解码

继续使用现有 `image` crate。

在 `crates/window/Cargo.toml` 中增加必要依赖：

```toml
image.workspace = true
winit.workspace = true
```

保留 workspace 当前锁定的依赖版本。

图标初始化时应：

1. 解码编译期嵌入的 PNG。
2. 转换为 RGBA8。
3. 获取宽度与高度。
4. 校验图标是否可以创建。
5. 返回内部图标配置或 Plugin。

解码与初始校验失败应通过 `Result` 返回给调用方。

不要将失败留到 Plugin 的 `build()` 中后再通过 panic 终止应用。

不要使用 Bevy `AssetServer`、`Handle<Image>` 或内建 Asset 注册机制。

### 6.3 Plugin 装配

运行时公开宏应将已嵌入的图片数据交给内部 Plugin。

内部 Plugin 在 `build()` 中安装图标设置 system，调用方不需要手动注册。

使用 Bevy 原有调度机制，不实现新的窗口事件循环。

若 Plugin 内部需要保存像素或图标配置，可以使用私有数据类型或 Resource。

不得为此引入不必要的公共 Component、Resource 或 Event。

### 6.4 原生窗口设置

迁移现有的 `set_primary_window_icons` 逻辑。

必须保留：

```rust
use bevy::ecs::system::NonSendMarker;
use bevy::winit::WINIT_WINDOWS;
use winit::platform::windows::WindowExtWindows;
```

实际调用仍然包括：

```rust
native_window.set_window_icon(Some(icon.clone()));
native_window.set_taskbar_icon(Some(icon));
```

两个操作都必须执行。

前者负责原生窗口图标，后者负责 Windows 任务栏图标。

不能只设置其中一个。

### 6.5 生命周期

保留当前单次设置模式：

- 在主线程执行。
- 只针对 `PrimaryWindow`。
- 原生窗口不存在时返回未完成状态，下一帧重试。
- 成功后标记为已应用。
- 不在后续帧重复解码 PNG。
- 不在后续帧重复设置图标。
- 实际错误通过项目现有 `Result` 和错误处理机制上报。

可以继续采用 `Local<bool>`，无需引入额外的全局状态系统。

原生窗口暂时未创建属于正常等待状态，不应被误判为错误。

如果主窗口 Entity 查询本身不满足唯一性约束，则应按现有错误处理规则上报，不应静默忽略。

### 6.6 明确不扩展的行为

本次不实现：

- 对所有 Window 自动设置图标。
- 不同窗口使用不同图标。
- 运行时动态切换图标。
- 根据 Light/Dark 主题切换图标。
- 图标缩放或多分辨率策略配置。
- PNG/ICO 自动互相转换。
- 托盘图标管理。

Gallery 当前只设置主窗口，因此迁移后也只设置主窗口。

即使 Gallery 创建独立窗口、MessageBox 或 FileDialog，也不应因本次修改而改变它们原有的图标设置逻辑。

## 7. Facade 导出

修改：

`crates/bevy_widgetry/src/lib.rs`

在现有 `window` module 中导出运行时宏。

公开调用路径固定为：

```rust
bevy_widgetry::window::taskbar_icon!(...)
```

Rust 声明宏可先在 `bevy_widgetry_window` 中通过 `#[macro_export]` 导出，再在 facade 的 `window` module 中使用 `pub use ... as taskbar_icon` 进行重导出。

内部实现涉及的辅助 Plugin、图片配置和原生窗口操作，不应作为独立的正式公共 API 出现在 facade 中。

EXE 图标 API 不通过 facade 导出。

原因是 EXE 图标只供 Cargo build script 使用，而 facade 是运行时依赖。

因此最终正式 API 总数仍为两个：

```text
构建期：
bevy_widgetry_app_icon_build::set_exe_icon(...)

运行时：
bevy_widgetry::window::taskbar_icon!(...)
```

## 8. Gallery 迁移

### 8.1 Cargo.toml

`gallery/Cargo.toml` 增加：

```toml
[build-dependencies]
bevy_widgetry_app_icon_build = {
    path = "../crates/app_icon_build"
}
```

移除 Gallery 对 `winresource` 的直接构建依赖。

检查 `winit` 和 `image` 是否仍由 Gallery 其他代码使用。

- 如果只服务于此次图标设置，则删除对应直接依赖。
- 如果 Gallery 其他功能仍然需要，则保留。
- 不为了表面上的依赖精简而破坏其他功能。

### 8.2 main.rs

删除原有 `set_primary_window_icons` 函数。

删除仅由该函数使用的 imports。

移除：

```rust
.add_systems(Update, set_primary_window_icons)
```

在 App 初始化阶段使用新的公开宏安装 Plugin。

不要修改以下现有装配逻辑：

- `DefaultPlugins`
- `WidgetryWindowPlugin`
- `RenderPlugin`
- `transparent_render_creation`
- `BrpRuntimePlugin`
- 日志配置
- Gallery 页面与 BSN Scene
- Waveform 与 FileDialog benchmark 入口

保持现有插件顺序及功能行为，除必要的图标 Plugin 注册外不进行额外调整。

### 8.3 assets.rs

删除：

```rust
native_window_icon()
```

移除只用于该函数的 `image` 与 `winit` imports。

不修改 `GalleryAssetPlugin` 管理其他资源的逻辑。

### 8.4 assets/constants.rs

删除：

```rust
NATIVE_WINDOW_ICON
```

保留其余 Gallery 资源常量及 `asset_data!` 宏。

不要将任务栏 PNG 注册为普通 Bevy Asset。

### 8.5 图标素材

以下文件保留原位置：

```text
gallery/src/assets/icons/widget_gallery_exe.ico
gallery/src/assets/icons/widget_gallery_taskbar.png
```

不修改图标内容。

不移动到 `crates/asset`。

Gallery 使用的 `logo.svg` 也必须保持不变，它属于自绘 UI 的内部 Logo，与 Windows EXE/任务栏图标是不同功能。

## 9. 路径与资源生命周期规则

两个 API 应采用相同的路径理解方式：

**路径相对于调用方 Cargo package 根目录，而不是运行时工作目录。**

具体语义：

| 项目 | EXE ICO API | 任务栏 PNG API |
|---|---|---|
| 输入 | ICO 路径 | PNG 字符串字面量 |
| 执行阶段 | Cargo 构建期 | Bevy 运行期 |
| 文件读取 | 构建阶段 | 编译阶段 |
| 是否嵌入 EXE | 是 | 是 |
| 运行时是否需要原始文件 | 否 | 否 |
| 是否需要 Bevy AssetServer | 否 | 否 |
| 错误处理 | `Result` | `Result` + system 错误上报 |

不允许为了统一函数形式，改变当前资源嵌入语义。

不需要建立通用资源路径解析框架。

## 10. 测试要求

### 10.1 构建期 API

验证：

- 有效 ICO 路径能够正常构建 EXE。
- 无效 ICO 路径导致明确的构建失败。
- ICO 内容修改后能触发对应 build script 重新执行。
- 图标资源确实写入 Gallery EXE。
- 调用方不需要直接依赖 `winresource`。

### 10.2 运行时 API

验证：

- 有效 PNG 能正常初始化 Plugin。
- 无效 PNG 数据返回明确错误。
- 不兼容的 RGBA 图标数据能够正确报错。
- 原生窗口尚未创建时不发生错误或 panic。
- 原生窗口创建后图标正确设置。
- 成功设置后不重复执行。
- 宏通过 facade 调用时能够正确解析路径。
- PNG 可以在调用方 crate 内通过编译期嵌入。

重点验证宏经过 facade re-export 后，`$crate` 内部路径和 `CARGO_MANIFEST_DIR` 是否正确。

### 10.3 Gallery 回归测试

必须确认：

1. EXE 文件仍显示原有 W 图标。
2. Windows 任务栏仍显示原有模块图标。
3. EXE 图标与任务栏图标保持不同。
4. Gallery 自绘标题栏 Logo 不变。
5. 最小化、最大化、恢复、关闭行为正常。
6. Light/Dark 主题切换正常。
7. WidgetryWindow 的透明渲染正常。
8. BRP 连接与 GUI 调试正常。
9. FileDialog、MessageBox 与独立窗口行为无回归。
10. 无额外图片资源分发要求。

特别验证：将编译好的 Gallery EXE 放到不包含图标 PNG 的目录中运行，任务栏图标仍然正确显示。

此验证仅针对图标资源，不代表 Gallery 其他文件依赖也必须全部消失。

### 10.4 编译与静态检查

在 Windows x64 开发环境执行：

```powershell
cargo fmt --all -- --check

cargo check -p bevy_widgetry_app_icon_build
cargo check -p bevy_widgetry_window
cargo check -p bevy_widgetry
cargo check -p widget_gallery

cargo clippy --workspace --all-targets -- -D warnings

cargo test -p bevy_widgetry_window
cargo test -p bevy_widgetry

cargo build -p widget_gallery --release
```

还应运行新增 crate 的相关测试，以及受本次修改直接影响的其他测试。

不允许留下新的 warning。

若某项命令因外部环境或现有无关问题失败，必须报告真实原因，不得将其描述为通过。

## 11. 文档同步

更新必要的说明：

- `crates/app_icon_build/src/lib.rs` 的 crate 级文档。
- `crates/window/src/lib.rs` 的运行时图标能力说明。
- `crates/bevy_widgetry/src/lib.rs` 的 facade API 说明。
- `docs/architecture.md` 中的 workspace 组成与依赖关系。

文档应说明：

- 两种图标处于不同生命周期。
- 构建期 helper 由应用 build script 调用。
- 运行时使用宏进行编译期嵌入。
- 路径相对应用 Cargo package。
- 运行时当前仅作用于主窗口。
- 图标素材始终由应用自行提供。

不需要为此重写其他 Widget 文档。

## 12. 实施顺序

按以下顺序执行，避免 Gallery 提前失去现有图标功能。

1. 新建 `crates/app_icon_build` 并实现 `set_exe_icon`。
2. 注册 workspace 成员，检查构建期 helper 能正常编译。
3. 在 `crates/window` 实现运行时图标 Plugin 与公开宏。
4. 在 facade 的 `window` module 导出宏。
5. 为构建期与运行时 API 添加必要测试。
6. 修改 Gallery 的 `build.rs` 与 `main.rs`，切换至新 API。
7. 删除 Gallery 中不再使用的图标实现和依赖。
8. 更新架构及 API 文档。
9. 执行格式、编译、Clippy 和测试。
10. 构建 Gallery Release EXE，完成 Windows GUI 人工验收。

迁移过程中不应同时重构其他 Window 或 Gallery 逻辑。

## 13. 完成标准

本次任务完成应同时满足：

- EXE 图标实现已从 Gallery 下沉到 `crates/app_icon_build`。
- 运行时图标实现已从 Gallery 下沉到 `crates/window`。
- 调用方只需两个正式 API，参数分别为 ICO/PNG 路径。
- Gallery 不再直接处理 `winresource`、原生窗口图标或 PNG 图标解码。
- 两个图标资源都保持编译期嵌入，不引入运行时文件依赖。
- 当前 Gallery 图标效果与修改前一致。
- 没有破坏现有 Window、多窗口或其他 Widget 行为。
- Workspace 依赖、代码规则和错误处理规则得到遵守。
- 编译、测试、静态检查及 Windows GUI 验收结果明确。

## 14. 实施约束

执行代码修改时必须遵守仓库现有的：

- `AGENTS.md`
- `rules/architecture.md`
- `rules/dependencies.md`
- `rules/code.md`
- `rules/testing.md`
- `rules/documentation.md`

尤其注意：

- 不引入 `unsafe`。
- 不使用 `unwrap`、`expect` 或主动 panic。
- 不静默吞掉错误。
- 不通过扩大 visibility 解决普通实现问题。
- 不新增与目标无关的公共 API。
- 不修改 Bevy 版本。
- 不引入不必要的外部依赖。
- 不修改现有图标素材。
- 不因本次任务重构其他 Widget 或窗口 rendering。
- 不向 GitHub 远程仓库提交或推送。

**最终目标：Gallery 只负责声明使用哪两个图标，Widgetry 负责其余全部构建期与运行时实现。**
