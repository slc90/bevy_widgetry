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

同层最新打开的 Modal 优先接收 Pointer / Keyboard。Window 不提供 Tab / Shift+Tab 跨控件遍历，按键继续分发到当前有效焦点控件。Modal 保存开启前的 Focus，并在关闭后恢复仍有效的目标，同时保留调用方自身的 InteractionDisabled 请求。Keyboard / IME 按实际 Native Window 路由；原始输入 Message 不会被全局读取并取消，宿主的全局快捷键仍须自行考虑 Modal 状态。

Image 背景覆盖包含 Title Bar 在内的整个窗口，并随圆角和裁剪变化。Opacity 要求 `0.0..=1.0`，缩放 Tint 的 Alpha；透明部分显示 Native Window 后方内容。Window 不限制图片分辨率，也不自动 Downscale。Title Bar 和 Body 的自定义内容配色由其组合控件负责；内建系统按钮按 Disabled → Pressed → Hovered → Normal 的优先级解析当前配色。

窗口配色按创建、Theme / Override、有效 Disabled、系统按钮 Hover / Pressed 的添加与移除和受管部件关系变化更新。静止窗口复用成功解析的 Palette，单个按钮输入只刷新该部件；MessageBox / FileDialog 托管路径保留最终 Owner 提供的配色。关系索引仅管理 Window 自己的 Title Bar、Content Host 和系统按钮，不递归业务子树或越过嵌套窗口。样式更新保留现有内容 Entity；缺失必需输出时保留待处理工作、向宿主传播 Error，持续同类错误的诊断去重，恢复后记录一次恢复信息。

窗口或 Modal 首次取得 Focus 时，仅从显式声明 `WidgetryWindowInitialFocus` 且属于当前窗口输入作用域的可见、启用候选中优先选择较小的 Priority，不要求 TabIndex。候选映射到的真实输入目标也必须有效、可见、启用且属于同一 Window Root；无有效候选时回退该 Window Root。`widgetry_window_target` 在构造期间也可查询 Root 绑定的 Native Window，无需访问内部 Root Component；`is_widgetry_window` 用于确认 Native Window 是否具有有效的 Widgetry 界面绑定。

应用显式清空 Focus 后，普通按键及快捷键 Release 交给当前 Window Scope，不自动恢复记忆中的 Editor 或聚焦新显示页面的输入框；Tab / Shift+Tab 同样不会自动选择其他控件。Modal 开关和窗口取得 Focus 的初始选择与恢复继续遵循各自的 Scope 规则。

复合控件可通过 Workspace 内部 `WidgetryInputTarget` 声明实际输入实体。Window 将显式声明初始优先级的 Root 解析为该目标；文本编辑与 IME 使用实际输入实体，键盘分发不维护 Tab Stop 或 TabGroup 顺序。

### 标题栏双击

标题栏空白拖动区域默认支持原生鼠标左键双击，在普通窗口与最大化窗口之间切换，无需额外配置。仅当 `Window.enabled_buttons.maximize` 为 `true` 时生效；隐藏最大化按钮不单独关闭该行为。第一次按下仍立即请求原生拖动，有效第二次按下执行最大化或还原，不再启动拖动。切换复用最大化按钮逻辑，圆角与按钮图标继续根据原生窗口的实际状态同步。

双击按 Windows 系统设置的时间间隔和屏幕物理坐标矩形判定，要求两次左键按下来自同一窗口、同一拖动区域，不依赖 Bevy 的点击计数。超时、超出位置范围或中间原生鼠标按下命中其他控件时重新开始判定；已匹配的两次按下不会与第三次重复配对。标题栏自定义交互控件保留自身输入行为。BRP Custom Pointer 不触发原生拖动或此双击行为。

### 标题文字绑定

在 Title Bar 内容的一个文字节点上挂载 `WidgetryWindowTitle`，将其明确绑定为本 Window 的可见标题；标记自动要求 `Text`，默认单行显示并裁剪溢出，可以与 Icon、其他 Text 或任意容器组合。标题栏保持 36 逻辑像素高，内容区可收缩并仅裁剪横向溢出，允许下拉菜单向下展开；系统按钮保持 46 逻辑像素宽，不被长标题压缩。每个 Window 最多一个标记，嵌套 Window 单独归属；正文和未标记的标题栏内容不参与更新。自定义 Title Bar 可以完全不绑定标题文字。

`Window.title` 是标题文本来源。`WidgetryWindowTitle::set_in_world(world, root, title)` 先校验 Root、Native Window / Camera 与全部显式绑定，再同步原生标题和绑定 Text；返回值表示原生标题是否实际改变，同值返回 `false`，不发出标题事件。`WidgetryWindowTitle::set(commands, root, title)` 在 Command 执行时完成同样的校验和更新。无绑定时只更新原生标题；无效 Root、失效原生绑定、缺失 Text、标记落入正文或多个标记均返回 Error，失败前不修改标题或文字。

构造、标记和层级变化，以及 Native Window 变化会触发标题投影，在 Build 阶段完成同步后交给文字布局。延后创建或挂接的标题节点使用当时最新的原生标题；直接更新官方 `Window.title` 后，绑定文字也会在该阶段同步。绑定维护只查询标记及其祖先，不遍历整个业务子树。MessageBox 和独立 FileDialog 的默认标题已使用该标记；库不依赖应用的语言或国际化类型。

收到官方 `WindowClosing` 后，标题自动同步停止，允许 Native Window 销毁到 `WindowClosed` 回收 Root 之间存在间隔；这属于正常关闭，不报告标题故障。没有关闭信号的 Native 丢失仍按失效绑定报告；显式标题 API 也继续拒绝失效 Native 目标。

## 边界情况与错误处理

无效 Native 目标、重复绑定、缺失 Camera 或不符合要求的窗口属性应拒绝构造，不能破坏其他已绑定 Window。Image 未加载时保持透明，不假装应用 Theme 色；输入 / IME 必须按原始 Native Window Identity 路由。Owned 生命周期收尾要允许 Render / Cursor 相关阶段正确释放资源。

`WidgetryWindowTitle::set_in_world` 与 `WidgetryWindowColorOverrides` 的同步查询 / 设置 / 清空返回 `WindowFailure`；排队入口显式转为 Severity::Error。`WindowError::Structure` / `WindowFault` 区分缺少必需部件、原生属性、重复 Native / Camera 绑定、相机目标、标题位置 / 歧义及托管配色拒绝。诊断保留 UI Root、Native / Camera、Owned 实际分配资源与操作阶段；非法配色继承 Core 的原始 cause、ID 和回溯，并补充配色部件 / 状态 / 字段。失败不提交标题或颜色。

初始化阶段对每个非法 Root 记录领域失败并清理该 Root，全部失败均传至宿主，外部 Native / Camera 保留。Owned Root 记录自己分配的资源身份，绑定部件失效或变化时仍只回收原分配，不依据后来被破坏的绑定猜测所有权。Scene 的下层诊断及原生 Severity / payload 继续通过 Core Scene 适配保留，部分 Patch 不承诺事务回滚。

标题和样式的持续故障按 Root / 阶段及具体结构故障保存原始诊断，歧义标题集合采用稳定实体集合身份，不受 ECS archetype 遍历顺序影响。多个窗口失败聚合全部源头，健康窗口继续更新；成功重新验证才记录对应恢复。Root 销毁只释放诊断；WindowClosing 至 WindowClosed 的正常关闭间隔不产生标题故障或虚假恢复。缺少派生状态的故障同样由独立于该状态的诊断存储承接。

`transparent_render_creation()` 和 `taskbar_icon!` 返回 `WindowFailure`。`WindowError` 区分 DX12 adapter / device 请求、PNG 解码、RGBA 图标创建和主窗口查询失败，保留 wgpu / image / winit / Bevy 的具体 source；诊断记录失败阶段、backend / adapter、图标路径 / 格式 / 尺寸等现场。启动入口负责显式报告未处理失败及原始回溯，不能依赖尚未运行的 ECS Error Handler。Gallery 在透明 renderer 初始化前安装日志订阅，窗口启动失败补充启动上下文后报告原始 ID / 栈并终止。

主窗口缺失或歧义时，图标 System 将受管错误以 Severity::Error 传给宿主；同一主窗口集合持续失效时复用原始 origin / ID，集合改变则产生新诊断。查询成功但对应 native window 尚未创建属于正常等待，不报告错误或伪恢复；只有实际调用主线程 Window / Taskbar Icon 设置后才记录一次关联恢复。图标成功应用后不再重复查询或设置，保持 `NonSendMarker` / `WINIT_WINDOWS` 的线程边界。

## 已知限制

- Image Handle、Mode、Opacity 是构造期配置，当前不提供运行时更新 API。
- Native Window 拖动、Resize、最大化、OS Taskbar / IME 需要真实 Windows 环境验证。
- EXE ICO 由独立 `app_icon_build` 在构建时设置，不与 `taskbar_icon!` 混为一体。

