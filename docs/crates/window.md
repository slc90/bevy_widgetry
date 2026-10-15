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

每个 Native Window 和 Camera 只绑定一个窗口界面。Borrowed Root 回收时不销毁调用方资源；Owned Root 回收时清理自建 Window 与 Camera。同一 Parent 的多个 Modal Child 共享阻挡并在最后一个结束后释放；最内层有效 Modal 接收输入。Theme 背景跟随 Theme，Image 背景不跟随 Theme；Cover 保持纵横比并按当前布局居中裁剪。

## 边界情况与错误处理

无效 Native 目标、重复绑定、缺失 Camera 或不符合要求的窗口属性应拒绝构造，不能破坏其他已绑定 Window。Image 未加载时保持透明，不假装应用 Theme 色；输入 / IME 必须按原始 Native Window Identity 路由。Owned 生命周期收尾要允许 Render / Cursor 相关阶段正确释放资源。

## 已知限制

- Image Handle、Mode、Opacity 是构造期配置，当前不提供运行时更新 API。
- Native Window 拖动、Resize、最大化、OS Taskbar / IME 需要真实 Windows 环境验证。
- EXE ICO 由独立 `app_icon_build` 在构建时设置，不与 `taskbar_icon!` 混为一体。

