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

窗口配色按创建、Theme / Override、有效 Disabled、系统按钮 Hover / Pressed 的添加与移除和受管部件关系变化更新。静止窗口复用成功解析的 Palette，单个按钮输入只刷新该部件；MessageBox / FileDialog 托管路径保留最终 Owner 提供的配色。关系索引仅管理 Window 自己的 Title Bar、Content Host 和系统按钮，不递归业务子树或越过嵌套窗口。样式更新保留现有内容 Entity；缺失必需输出时保留待处理工作、向宿主传播 Error，持续同类错误的诊断去重，恢复后记录一次恢复信息。

窗口或 Modal 首次取得 Focus 时，从带有 `WidgetryWindowInitialFocus` 的候选控件中优先选择较小的 Priority。`widgetry_window_target` 在构造期间也可查询 Root 绑定的 Native Window，无需访问内部 Root Component；`is_widgetry_window` 用于确认 Native Window 是否具有有效的 Widgetry 界面绑定。

### 标题文字绑定

在 Title Bar 内容的一个文字节点上挂载 `WidgetryWindowTitle`，将其明确绑定为本 Window 的可见标题；标记自动要求 `Text`，可以与 Icon、其他 Text 或任意容器组合。每个 Window 最多一个标记，嵌套 Window 单独归属；正文和未标记的标题栏内容不参与更新。自定义 Title Bar 可以完全不绑定标题文字。

`Window.title` 是标题文本来源。`WidgetryWindowTitle::set_in_world(world, root, title)` 先校验 Root、Native Window / Camera 与全部显式绑定，再同步原生标题和绑定 Text；返回值表示原生标题是否实际改变，同值返回 `false`，不发出标题事件。`WidgetryWindowTitle::set(commands, root, title)` 在 Command 执行时完成同样的校验和更新。无绑定时只更新原生标题；无效 Root、失效原生绑定、缺失 Text、标记落入正文或多个标记均返回 Error，失败前不修改标题或文字。

构造、标记和层级变化，以及 Native Window 变化会触发标题投影，在 Build 阶段完成同步后交给文字布局。延后创建或挂接的标题节点使用当时最新的原生标题；直接更新官方 `Window.title` 后，绑定文字也会在该阶段同步。绑定维护只查询标记及其祖先，不遍历整个业务子树。MessageBox 和独立 FileDialog 的默认标题已使用该标记；库不依赖应用的语言或国际化类型。

收到官方 `WindowClosing` 后，标题自动同步停止，允许 Native Window 销毁到 `WindowClosed` 回收 Root 之间存在间隔；这属于正常关闭，不报告标题故障。没有关闭信号的 Native 丢失仍按失效绑定报告；显式标题 API 也继续拒绝失效 Native 目标。

## 边界情况与错误处理

无效 Native 目标、重复绑定、缺失 Camera 或不符合要求的窗口属性应拒绝构造，不能破坏其他已绑定 Window。Image 未加载时保持透明，不假装应用 Theme 色；输入 / IME 必须按原始 Native Window Identity 路由。Owned 生命周期收尾要允许 Render / Cursor 相关阶段正确释放资源。

## 已知限制

- Image Handle、Mode、Opacity 是构造期配置，当前不提供运行时更新 API。
- Native Window 拖动、Resize、最大化、OS Taskbar / IME 需要真实 Windows 环境验证。
- EXE ICO 由独立 `app_icon_build` 在构建时设置，不与 `taskbar_icon!` 混为一体。

