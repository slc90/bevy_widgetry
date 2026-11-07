# Cursor

## 功能概述

为各 Native Window 独立管理 OS Mouse Cursor，默认选择 System。外部主题使用公开数据把官方系统语义或 Widgetry 扩展 Hint 映射为系统光标或静态图片。显式启用默认关闭的 `kamuri_cursor` Feature 后，既有 Kamuri 动画提供方仍可使用。

## 当前设计思路

Hover 语义由 Entity / Hierarchy 提供，真正的 OS Cursor 只由有效 Mouse 身份驱动。Drag 使用有 Owner 的 `CursorLease` 临时固定样式；原生窗口 Move / Resize 期间让出 Cursor 控制权。展示主题与颜色 Theme 相互独立。

`WidgetryCursorTheme::new` 接收 `CursorSemantic` 到 `CursorStyle` 的映射。`CursorStyle::System` 直接选择系统样式，`Image(CursorImage)` 描述资产路径或 Image Handle、可选 `URect` 区域及区域内热点。路径保留 Bevy 的资产来源语法；定义不加载图片，实际语义出现时才按需加载和缓存。主题可以只定义一项，缺项按原语义回退到 System，Hint 使用既有 `system_fallback`。静态图片无需 `kamuri_cursor`，尺寸不限制为80×80；支持 RGBA8 / RGBA32Float 及可用 CPU 像素。

当前资源只读查询 `styles()` / `is_system()`，通过同步 `set_in_world` 或执行时提交的 `set(Commands, theme)` 切换；同值不标记变化，不新增业务事件。`system()` 构造空映射；开启 `kamuri_cursor` 时 `kamuri()` 构造临时内置提供方。名称和菜单属于应用。显式 `EntityCursor::Custom` 不被主题映射替换，Loading / Disabled 的既有语义优先级仍有效。

静态图片目前不支持原生光标的同 Handle 像素热更新：修改像素或同尺寸恢复后，Windows 光标可能继续显示首次缓存的位图。需要替换位图时应使用新的 Image Handle。Bevy 原生光标缓存的刷新与回收问题暂缓处理，库未引入 vendor 或依赖补丁。

`kamuri_cursor` 同时开启 Asset 的同名资源 Feature。关闭时不编译 Kamuri 专用播放器、缓存和动画唤醒，也不安装其内建 Asset 支持；通用静态加载和诊断始终可用。宿主有 AssetServer 和 Image 能力时自动确保 Atlas 支持，静态区域使用按尺寸、区域及热点复用的 Layout 身份，避免 Bevy 原生缓存忽略 hotspot 而沿用旧作用点。System 在缺少 Asset/Image 环境时仍可使用，外部图片回退对应系统语义。Facade 提供同名转发 Feature，Gallery 显式启用以保留既有演示。

## 主要功能

- 解析最近祖先 `EntityCursor`，以及 `WidgetryCursorHint` 的十一种扩展业务语义。
- 有效 Disabled 情况可展示 NotAllowed，公开 `WidgetryCursorLoading` 声明优先展示 Wait，并在遮挡、失效或生命周期变化时重新计算 Cursor。
- System Theme 使用系统样式；Kamuri 提供 34 种系统语义与 11 种扩展样式，共 45 项。
- Kamuri 的全部 45 种语义均持续循环动画，统一 80×80 RGBA 帧画布，各自定义帧数、速率与热点。
- Drag Lease 支持 Owner 失效、Cancel、Release 和失焦后的清理。

## 行为与状态契约

`WidgetryCursorLoading` 可单独挂在命中实体或其祖先上，不要求同实体另有 Cursor 声明。它表达 Style 选择的忙碌反馈，优先于祖先 Disabled 的 NotAllowed 与普通声明；移除后重新解析最近声明。该组件不禁用输入，也不修改业务 Loading 状态。默认 Button Style 及 Gallery 外部 Style 分别按自己的 Loading 维护该声明。

只有 `PointerId::Mouse` 影响 OS Cursor；Custom 与 Touch 不取得 Lease。最近有效声明优先，同一 Entity 上官方 `EntityCursor` 优先于扩展 Hint。动画进入新语义时从首帧开始，同一语义内持续显示不重启；Native 操作、失焦或离开语义时停止。未加载图集时先使用静态后备帧，再退回语义对应的 System Cursor。调用方自定义 `EntityCursor::Custom` 图像保持不变。

动画目录由 Asset 层的 `BuiltinCursorAnimation` 提供语义标识、时序、热点、图集布局和帧计算，OS 播放与 Gallery 预览共用。每种动画独立图集，按实际出现的语义懒加载并缓存，主题选择仅预载静态后备图集。后备图集的每项均为对应动画首帧，画布及热点不变。制作工具与逐种动作说明见 [素材工具说明](../../crates/asset/tools/README.md)。

每个窗口独立记录当前语义及进入时间，使用真实时间推进，不受虚拟时间暂停或单帧上限影响。只有有焦点的有效 Mouse 目标窗口及有效 Drag Lease 会保持动画唤醒；清除 Hover 不会结束仍有效的 Lease / Capture。普通页面没有有效 Mouse 位置、窗口失焦、原生窗口操作或当前语义失效时停止播放和额外重绘。资源正在加载且当前窗口确实需要该语义时，保留必要更新直至进入Loaded或Failed，避免反应式窗口停在回退图。加载完成后按动画更新；失败、已加载但CPU图像不可用、失焦或语义失效时不维持加载唤醒。静态或System回退本身不产生持续动画重绘。

## 边界情况与错误处理

主题构造拒绝空图片路径、非法 Bevy AssetPath 语法、空/反向区域和已知区域内越界热点，返回 `CursorThemeFailure` / `CursorThemeError` 并保留实际参数及路径解析原错误。图片加载后的区域/热点边界另按实际尺寸校验；加载、像素和区域故障区分记录，共享图片故障按资产复用诊断。有效加载期间请求必要更新，静态可用、失败或窗口失效后不维持无限重绘；恢复后重新使用图片。设置主题不改变 Widget 业务状态。

`WidgetryCursorHint::ReadOnly` 表达可选择、可复制但不可编辑的区域，与 Disabled 独立。System Theme 或 Kamuri 素材不可用时回退 `SystemCursorIcon::Cell`；Kamuri 使用扫描文档与闭合蓝白挂锁的循环动画，热点 `(25,56)` 固定于锁孔。Hint 只声明光标语义，应用仍负责实际编辑资格；未启用编辑的普通表格不应自动声明 ReadOnly。编辑器内部光标由编辑器自己的最近声明控制。

素材解码/加载或尺寸、像素格式、CPU数据长度校验失败会保留资产身份、路径、实际字段及原加载source，首次纳入时记录一次源头ERROR。共享图集由资源缓存按资产及原因键复用同一origin，已受管Loader集合保留全部诊断。只有真正选择静态图集或System Fallback后，才按原error_id记录一次DEBUG，注明Window和选择结果；确认图像恢复可用后INFO。Loading、运行资源尚未准备和正常主题切换保持静默，不持续制造失败或重绘循环。与 Bevy 官方 `CursorIconPlugin` 冲突时通过统一诊断保留首次位置、ID 和原始栈，按 Severity::Error 进入宿主；Window / Pointer / Lease 失效时不能残留错误的 Cursor 所有权。

## 已知限制

- 不接管 Pointer Event 派发或捕获。
- Kamuri 图片与动画要求宿主具备适当 Asset / Image 能力；Headless 环境可保留系统回退。
- OS 级光标视觉、真实 Native Move / Resize 不能只靠 Custom Pointer 模拟来证明。

