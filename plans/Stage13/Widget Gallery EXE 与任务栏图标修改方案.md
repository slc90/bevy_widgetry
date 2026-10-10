# Widget Gallery：EXE 与任务栏图标修改方案

> 目标仓库：`https://github.com/slc90/bevy_widgetry`  
> 基于仓库当前 `main` 分支的 `gallery/Cargo.toml`、`gallery/src/main.rs` 和 `gallery/src/assets/icons/logo.svg` 制定。项目使用 **Bevy 0.20.0**，Windows x64。  
> 请只修改本地工作树；**不要向远程 GitHub 提交或推送**。本方案不要求改动 Bevy / Widgetry 组件库内部代码。

## 0. 实施目标与边界

1. 把资源管理器中 `widget_gallery.exe` 的图标替换成提供的 **W 字形图标**。
2. 把程序运行后 Windows **任务栏**显示的图标替换成提供的 **立体模块图标**，并设置原生窗口的小图标。
3. 保留 Widgetry 自绘标题栏现有的 `GalleryIcon::Logo` 和 `gallery/src/assets/icons/logo.svg`，**不改现有 SVG**。它是 UI 内部图标，不应与这次两处 Windows 图标混在一起。
4. 不通过 `AssetServer` 实现这两项：EXE 图标是 **PE 资源**；运行时图标通过 **winit 原生窗口 API** 设置。
5. 方案不应引入额外的运行时文件分发要求；PNG 像素通过 `include_bytes!` 编译到 EXE 中。
6. 只影响 `widget_gallery` 示例应用，不影响其它引用 Widgetry 的应用。

## 1. 已准备好的素材（请先复制）

压缩包内有：

| 文件 | 用途 | 尺寸／说明 |
|---|---|---|
| `widget_gallery_exe.ico` | 嵌入 `widget_gallery.exe` 的 PE 图标资源 | 包含 16、24、32、48、64、128、256 px |
| `widget_gallery_taskbar.ico` | 任务栏图标的 `.ico` 备用版本 / 后续 Windows 资源用途 | 同样包含七种尺寸 |
| `widget_gallery_taskbar.png` | **运行时实际读取**的任务栏与窗口图标 | 256×256、RGBA、透明圆角 |
| `widget_gallery_exe.png` | EXE 图标源 PNG / 备用 | 512×512、RGBA |
| `widget_gallery_exe.svg` | W 图标矢量源文件 | 可编辑 |
| `widget_gallery_taskbar.svg` | 模块图标矢量源文件 | 可编辑 |
| `icon_preview.png` | 视觉预览 | 不参与编译 |

以上设计是新的彩色几何图标，不等于现有 `logo.svg`（现有文件是 16×16、单色的窗口布局线稿）。

建议将 **实际参与构建的三个文件**复制到以下位置：

```text
bevy_widgetry/
├── Cargo.toml
└── gallery/
    ├── Cargo.toml
    ├── build.rs                         # 新建
    ├── assets/
    │   ├── widget_gallery_exe.ico       # EXE 构建时嵌入
    │   └── widget_gallery_taskbar.png   # 运行时 include_bytes!
    └── src/
        ├── main.rs                     # 修改
        └── assets/
            └── icons/
                └── logo.svg            # 保持原状！
```

另外的 `.ico`、`.png` 和 `.svg` 可保留在你的设计资源目录，不必全部参与 Cargo 构建。

### 为什么给任务栏也准备 ICO，但运行时代码用 PNG？

`winit::window::Icon::from_rgba` 接收 RGBA 像素、宽度和高度，**不直接接收 ICO 路径**。因此任务栏 `.ico` 是相同设计的 Windows 图标资源文件；在此实现中，运行时实际嵌入的是同图案的 `widget_gallery_taskbar.png`。

---

## 2. 修改 `gallery/Cargo.toml`

当前 `gallery` crate 包名为 `widget_gallery`。保留原有依赖，在现有 `[dependencies]` 增加：

```toml
winit = "0.30"
image = { version = "0.25", default-features = false, features = ["png"] }
```

增加构建依赖：

```toml
[build-dependencies]
winresource = "0.1"
```

可在 `[package]` 内显式加入 `build = "build.rs"`，不过 Cargo 通常能自动识别 crate 根目录下的 `build.rs`。

请 Codex 尽量让 `winit` / `image` 版本与现有 Bevy 的依赖解析保持一致，不要改根 `Cargo.toml` 的 Bevy 版本。

---

## 3. 新建 `gallery/build.rs`：把 EXE 图标嵌入 PE

```rust
fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=assets/widget_gallery_exe.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/widget_gallery_exe.ico")
            .compile()?;
    }

    Ok(())
}
```

说明：

- `build.rs` 在编译期运行，路径以 `gallery` crate 为根，**不是**运行目录。
- `winresource` 的 `set_icon` 使用 `widget_gallery_exe.ico`；它不会影响 `GalleryIcon::Logo`。
- 用 `CARGO_CFG_TARGET_OS` 判断**目标平台**，不要用 build.rs 所在宿主系统的 `#[cfg(windows)]` 代替。
- Windows MSVC toolchain 通常需要 Windows SDK 的 `rc.exe`，如果环境缺失，应按错误补全构建工具，而不是删除资源步骤。

---

## 4. 修改 `gallery/src/main.rs`：设置任务栏和原生窗口图标

### 4.1 导入类型

与现有 imports 合并，避免重复：

```rust
use bevy::ecs::system::{Local, NonSendMarker};
use bevy::winit::WINIT_WINDOWS;
use winit::platform::windows::WindowExtWindows;
use winit::window::Icon;
```

现有文件已经导入 `bevy::prelude::*` 和 `PrimaryWindow`，无需重复导入。

### 4.2 新增系统

建议使用以下 **只应用一次、未准备好则重试** 的系统，而不是假设 `Startup` 时 winit 窗口一定存在：

```rust
fn set_primary_window_icons(
    _main_thread: NonSendMarker,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut applied: Local<bool>,
) -> Result {
    if *applied {
        return Ok(());
    }

    let primary_entity = primary_window.single()?;

    let ready = WINIT_WINDOWS.with_borrow(|windows| -> Result<bool> {
        let Some(native_window) = windows.get_window(primary_entity) else {
            return Ok(false);
        };

        // 通过 Rust 编译期嵌入，不经过 Bevy AssetServer。
        let rgba = image::load_from_memory(include_bytes!(
            "../assets/widget_gallery_taskbar.png"
        ))
        .map_err(BevyError::error)?
        .into_rgba8();

        let (width, height) = rgba.dimensions();
        let icon = Icon::from_rgba(rgba.into_raw(), width, height)
            .map_err(BevyError::error)?;

        // Windows ICON_SMALL：原生窗口图标
        native_window.set_window_icon(Some(icon.clone()));
        // Windows ICON_BIG：任务栏图标
        native_window.set_taskbar_icon(Some(icon));

        Ok(true)
    })?;

    *applied = ready;
    Ok(())
}
```

设计说明：

- `NonSendMarker` 要求系统在主线程执行；Win32/winit 的部分窗口方法不允许随意从工作线程调用。
- `WINIT_WINDOWS` 是 **Bevy 0.20** 的线程局部桥接入口；不要照搬旧 Bevy 教程中的 `NonSend<WinitWindows>` 代码。
- `get_window(primary_entity)` 返回 `None` 时，下一个 `Update` 再试；首次成功后 `Local<bool>` 阻止重复解码和重复调用。
- `set_window_icon` 与 `set_taskbar_icon` 分别设置窗口小图标和 Windows 任务栏的大图标，使用同一张 **模块图案 PNG**。
- Rust 源码里的 `include_bytes!("../assets/...")` 是相对 `gallery/src/main.rs` 文件的路径，不是当前工作目录。
- 当前应用目标是 Windows x64，所以使用 Windows 平台扩展 trait 是合适的；如未来支持跨平台需以 `#[cfg(target_os = "windows")]` 封装相关代码。

### 4.3 注册系统

当前 `main()` 末尾有：

```rust
.add_systems(Startup, setup);
```

保留原有 `Startup`，另加：

```rust
.add_systems(Startup, setup)
.add_systems(Update, set_primary_window_icons);
```

请注意系统调用链和分号位置。不要拆掉原有的 WindowPlugin、RenderPlugin、自绘窗口逻辑或 BRP 插件。

### 4.4 对其它窗口的影响

以上代码**只设置 Gallery 主窗口**。示例的独立 Window、MessageBox、FileDialog 不应被这次任务无意改动；若未来要求所有窗口采用同一原生图标，可针对窗口创建事件扩展，但不要把它加入本次最小改动范围。

---

## 5. 先检查，再编译，再人工验证

在项目根目录执行（Windows PowerShell）：

```powershell
cargo fmt --all -- --check
cargo check -p widget_gallery
cargo build -p widget_gallery --release
```

其中 `cargo fmt --all -- --check` 如果报告新增文件不符合格式，先运行 `cargo fmt --all` 再检查；构建时如更新 `Cargo.lock`，应由 Codex 确认属于新加依赖导致的预期更改。

**人工验收：**

1. 检查 `target/release/widget_gallery.exe` 文件图标，应为 **W 图案**。
2. 启动该 EXE，任务栏应为 **立体模块图案**，与 EXE 文件图标**不同**。
3. Gallery 自绘标题栏左上角仍为原来的 `logo.svg`，和这两个 Windows 图标互不影响。
4. Light / Dark 主题切换、窗口关闭/最大化/最小化、BRP、Widgetry 控件不应回归。
5. 将 EXE 复制到一个**不包含 PNG 素材**的测试目录，仍应能显示图标（因为使用编译期嵌入）。注意程序其它既有文件依赖不属于本次更改范围。
6. 资源管理器/任务栏没更新时：退出进程，解除已有任务栏固定项并重新启动；必要时用不同的 EXE 名称排除 Windows 图标缓存。不要因为缓存现象就改动 Widgetry 绘制逻辑。

## 6. Codex 执行要求（可直接作为任务约束）

- 读取项目当前文件后再应用上述修改，尊重当前文件的实际组织结构。
- 只修改 `gallery` crate 中与 Windows 图标直接相关的文件；**不要改** `crates/window`、`crates/asset`、`gallery/src/assets/icons/logo.svg` 或根 Bevy 依赖配置。
- 保留现有日志、BRP 和自绘窗口设置。
- 按项目的 Rust lints 编写，不使用 `unwrap()`、`expect()`、`unsafe`。
- 不引入 `AssetServer` / `Handle<Image>`，也不要求在运行目录分发 PNG。
- 不在 GitHub 上修改、提交或推送；直接修改本地文件，完成后报告改动文件和验证结果。
- 如所用实际 Bevy / winit API 与上述示例略有变化，以**本地锁定依赖的实际 API**为准修正，并在最终结果中说明差异；不要为了示例而升级 Bevy。

## 7. 参考 API

- [winresource 官方文档](https://docs.rs/winresource/latest/winresource/)
- [winit 0.30 Windows 平台扩展](https://docs.rs/winit/0.30.13/winit/platform/windows/trait.WindowExtWindows.html)
- [项目 Gallery 入口](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs)
- [项目 Gallery Cargo.toml](https://github.com/slc90/bevy_widgetry/blob/main/gallery/Cargo.toml)

> 本文是**拟实施方案**，图标文件已生成；未在用户本地 Windows 工程中执行编译或运行测试。执行者须验证结果。
