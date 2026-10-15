# Window

## 功能概述

提供自绘 Window 的 BSN 组合、Native Window 绑定、可选 Owned Lifecycle、透明多窗口渲染及 Modal 输入协调。

## 当前设计思路

区分 Borrowed 与 Owned Window：`widgetry_window` 绑定调用方已有的 Native Window / Camera；`owned_widgetry_window` 创建并管理这两类资源。Title Bar 和 Body 接收外部 SceneList，窗口外观与原生输入路由由 Window 基础设施协调。

## 主要功能

- `prepare_native_window` 配置透明、无原生 Decorations 的窗口属性。
- 自定义 Title Bar、Body、Minimize / Maximize / Restore / Close Controls、拖动及八向 Resize。
- Theme 或 Image 背景；Image 支持 Stretch、Cover 与 Opacity。
- `WidgetryModalWindow` 关联 Parent，并管理 Pointer Blocker、Keyboard / Focus Scope 与恢复。
- 提供多窗口透明渲染的 `transparent_render_creation()`。
- `taskbar_icon!` 将应用提供的 PNG 编译期嵌入，并在主窗口就绪后设置 Window / Taskbar Icon。

## 行为与状态契约

每个 Native Window 和 Camera 只绑定一个窗口界面。Borrowed Root 回收时不销毁调用方资源；Owned Root 回收时清理自建 Window 与 Camera。同一 Parent 的多个 Modal Child 共享阻挡并在最后一个结束后释放；最内层有效 Modal 接收输入。Theme 背景消费 Theme 配色，Image 背景保留指定图像，并应用当前 `image_tint` 与 Opacity；Cover 保持纵横比并按当前布局居中裁剪。

同层最新打开的 Modal 优先接收 Pointer / Keyboard，Tab 在该窗口的有效控件内循环。Modal 保存开启前的 Focus，并在关闭后恢复仍有效的目标，同时保留调用方自身的 InteractionDisabled 请求。Keyboard / IME 按实际 Native Window 路由；原始输入 Message 不会被全局读取并取消，宿主的全局快捷键仍须自行考虑 Modal 状态。

Image 背景覆盖包含 Title Bar 在内的整个窗口，并随圆角和裁剪变化。Opacity 要求 `0.0..=1.0`，缩放 Tint 的 Alpha；透明部分显示 Native Window 后方内容。Window 不限制图片分辨率，也不自动 Downscale。Title Bar 和 Body 的自定义内容配色由其组合控件负责；内建系统按钮按 Disabled → Pressed → Hovered → Normal 的优先级解析当前配色。

窗口或 Modal 首次取得 Focus 时，从带有 `WidgetryWindowInitialFocus` 的候选控件中优先选择较小的 Priority。`widgetry_window_target` 在构造期间也可查询 Root 绑定的 Native Window，无需访问内部 Root Component；`is_widgetry_window` 用于确认 Native Window 是否具有有效的 Widgetry 界面绑定。

## 边界情况与错误处理

无效 Native 目标、重复绑定、缺失 Camera 或不符合要求的窗口属性应拒绝构造，不能破坏其他已绑定 Window。Image 未加载时保持透明，不假装应用 Theme 色；输入 / IME 必须按原始 Native Window Identity 路由。Owned 生命周期收尾要允许 Render / Cursor 相关阶段正确释放资源。

## 已知限制

- Image Handle、Mode、Opacity 是构造期配置，当前不提供运行时更新 API。
- Native Window 拖动、Resize、最大化、OS Taskbar / IME 需要真实 Windows 环境验证。
- EXE ICO 由独立 `app_icon_build` 在构建时设置，不与 `taskbar_icon!` 混为一体。

