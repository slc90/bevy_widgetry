# Widgetry Window 真实圆角方案（Windows）

## 目标

解决当前自定义 Window 仅在 UI 层绘制圆角、但原生窗口背景仍为矩形，从而在四角出现背景漏出的现象。

本次只处理 Windows；现有 UI 圆角和最大化时的圆角切换逻辑保持不变。

## 方案

### 1. 新增 `widgetry_window(Window) -> Window`

为 Widgetry 自定义窗口提供创建期 helper：

```rust
pub fn widgetry_window(mut window: Window) -> Window {
    window.transparent = true;
    window.decorations = false;
    window.composite_alpha_mode = CompositeAlphaMode::PreMultiplied;
    window
}
```

这三个字段无条件覆盖，作为 Widgetry Window 的创建期不变量。

原因：`Window::transparent` 在 Bevy 0.19 / winit 中不能在原生窗口创建后修改，因此必须在创建原生窗口前完成。

调用方创建主窗口和动态窗口时统一通过该 helper；不再自行记忆 `transparent` / `decorations` / `composite_alpha_mode` 的底层要求。

### 2. `target_camera` 定义为 Window 专用全窗口 UI Camera

`window(target_window, target_camera, ...)` 中的 `target_camera` 明确定义为该 Widgetry Window 的专用 UI Camera。

调用方只负责创建和销毁 Camera；Window 系统负责以下与窗口渲染直接相关的配置：

```rust
RenderTarget::Window(WindowRef::Entity(target_window))
camera.viewport = None
camera.clear_color = ClearColorConfig::Custom(Color::NONE)
```

其中：

- `RenderTarget` 由 Window 系统绑定，避免调用方重复配置或绑错窗口；
- `viewport = None` 保证该 Camera 覆盖整个目标窗口；
- `Custom(Color::NONE)` 表示真正将窗口底色清为透明，不使用 `ClearColorConfig::None`。

Window 系统不修改以下业务合成策略：

- `camera.order`
- `camera.is_active`
- 其他业务可见性 / 分层配置

业务方如需其他 viewport Camera，可自行决定其 order 和合成关系。

### 3. 在程序内部启用 DX12 DirectComposition

仅配置透明 Window 和透明 Camera 不足以完成 Windows DX12 的透明链。
当前依赖为 Bevy 0.19.1 / wgpu 29.0.4，默认 `DxgiFromHwnd` 交换链只支持 `Opaque`，
`CompositeAlphaMode::Auto` 会回退到不透明合成。相机清出的 `(0, 0, 0, 0)` 因 alpha 被忽略而显示为黑色，主窗口和独立弹窗都会受影响。

必须同时满足：

- 使用 `Dx12SwapchainKind::DxgiFromVisual`，通过 DirectComposition 呈现窗口；
- Window 使用 `CompositeAlphaMode::PreMultiplied`，让桌面合成保留 alpha。

不能只修改 Window 的合成模式：默认 HWND 交换链不支持 `PreMultiplied`，会导致表面配置失败。

Gallery 在 `gallery/src/renderer.rs` 中实现 `transparent_renderer()`：

1. 通过 wgpu 安全 API 创建 DX12 Instance，在 `Dx12BackendOptions.presentation_system` 中明确设置 `DxgiFromVisual`。
2. 请求 Adapter、Device 和 Queue，采用设备支持的能力与限制，排除实验能力；独立显卡不启用 `MAPPABLE_PRIMARY_BUFFERS`。
3. 将 GPU 资源通过 `RenderCreation::manual(...)` 交给 Bevy。
4. `main.rs` 使用 `block_on(...)` 完成初始化并配置 `RenderPlugin`；初始化错误通过 `Result` 向上返回。

渲染初始化由 Gallery 负责，Window 控件负责窗口属性与专用相机配置，不接管应用级 GPU 初始化。
Gallery 直接依赖 Workspace 统一声明的 wgpu 29.0.4，复用 Bevy 已使用的版本，不新增 Workspace crate，也不引入 `unsafe`。

最终实现不依赖 `WGPU_DX12_PRESENTATION_SYSTEM` 环境变量，`.cargo/config.toml` 中的临时配置已移除。
通过 Cargo 启动或直接双击 exe 均由程序内部选择 DirectComposition。
其他使用 Widgetry Window 的应用也需要配置支持透明的渲染后端；若仍使用 Bevy 自动初始化，可在启动前设置该环境变量为 `DxgiFromVisual`。

### 4. 保留现有 UI 圆角实现

现有 `WindowRoot`、`TitleBar`、`WindowContent` 的 `BorderRadius` 保持不变。

现有普通状态 8px、最大化状态 0px 的同步逻辑也保持不变。

本次修改只补齐底层透明链：

```text
透明 native Window
    ↓
透明全窗口 UI Camera
    ↓
现有 WindowRoot 圆角绘制
    ↓
圆角外区域保留 alpha = 0
    ↓
DirectComposition 交换链 + PreMultiplied 桌面合成
    ↓
圆角外显示窗口后方内容
```

如果后续发现子节点自身越过圆角区域绘制，则单独作为 UI clipping 问题处理，不与本次方案混在一起。

## 绑定校验

`initialize_windows()` 在现有窗口 / Camera / 重复绑定校验之外，再检查：

```rust
window.transparent == true
window.decorations == false
window.composite_alpha_mode == CompositeAlphaMode::PreMultiplied
```

若任一不满足，则视为无效 Widgetry Window 绑定：记录错误并清理新建的 Window UI 根，与现有无效绑定处理一致。

这样可以避免调用方绕过 `widgetry_window(...)` 后得到“看起来能运行、但圆角悄悄漏底色”的退化结果。

`initialize_windows()` 不再运行期补写 `window.decorations = false`；创建期属性全部由 `widgetry_window(...)` 统一准备。

绑定校验不负责检测或切换 GPU 交换链；调用方仍须满足上述渲染初始化要求。

## 直接受影响位置

- `crates/window`
  - 新增并公开 `widgetry_window(...)`；
  - 更新 `window(...)` rustdoc，明确 target_camera 的专用 UI Camera 契约；
  - `initialize_windows()` 增加 Window 创建期属性校验；
  - 初始化成功后配置 target_camera 的 RenderTarget、viewport、透明 clear；
  - 删除运行期设置 decorations 的旧逻辑。

- `gallery/src/main.rs`
  - Primary Window 使用 `widgetry_window(...)`；
  - 主 UI Camera 只创建 `Camera2d`，不再自行指定 RenderTarget；
  - 调用 `renderer::transparent_renderer()`，使用手动初始化的渲染资源。

- `gallery/src/renderer.rs`
  - 新增程序内部的 DX12 DirectComposition 初始化。

- 根 `Cargo.toml`、`gallery/Cargo.toml` 和 `Cargo.lock`
  - 统一声明并为 Gallery 添加直接 wgpu 依赖，版本与 Bevy 当前使用的版本一致。

- `docs/architecture.md`
  - 同步窗口创建期约束、专用相机职责与 Gallery 渲染初始化职责。

- `gallery/src/pages/window.rs`
  - Demo Window 使用 `widgetry_window(...)`；
  - Demo Camera 不再自行指定 RenderTarget；生命周期管理保持原样。

- `bevy_widgetry` facade
  - 现有 `pub use bevy_widgetry_window::*` 可直接导出新增 helper，无需额外 facade 设计。

## 验证

至少覆盖以下行为：

1. `widgetry_window(...)` 无条件将 `transparent` 设为 `true`、`decorations` 设为 `false`、`composite_alpha_mode` 设为 `PreMultiplied`，保留调用方的标题和尺寸。
2. 合法绑定后，target_camera：
   - 指向正确的 target_window；
   - `viewport == None`；
   - `clear_color == ClearColorConfig::Custom(Color::NONE)`。
3. Window 的透明、装饰或合成模式不满足约束时，绑定失败并递归清理新 UI 树，保持原生窗口及相机配置不变；合成模式拒绝 `Auto`、`Opaque`、`Inherit` 和 `PostMultiplied`。
4. 配置 Camera 时不修改 `order` 和 `is_active`。
5. 现有无效 Window、无效 Camera、重复 WindowRoot、关闭窗口清理等测试继续通过。
6. Gallery 主窗口和 Demo Window 实际运行时，四角显示桌面内容而不是相机 / 窗口底色；最大化后仍按现有逻辑取消圆角。
7. 不设置 `WGPU_DX12_PRESENTATION_SYSTEM`，直接运行编译后的 exe，窗口和渲染初始化正常，不出现不支持 alpha 合成模式的错误。

### 当前验证记录

- Window crate 的 13 项测试通过，包括创建期属性、相机配置、无效绑定、重复绑定与关闭清理。
- Workspace 构建与格式检查通过；程序内部初始化修改后，Gallery 构建及编译检查通过。
- Clippy 未发现本次新增代码的警告；Gallery 仍有原有 `Icon::new(&asset_server, ...)` 的 `needless_borrow` 警告，按任务范围未修改。
- 启用 DirectComposition 和预乘 alpha 后，用户已确认视觉效果正常。
- 改为程序内部初始化后，已在清除环境变量的进程中直接启动新 exe，窗口创建和渲染初始化正常；该轮未单独记录主窗口、弹窗及最大化切换的完整目视验收。
