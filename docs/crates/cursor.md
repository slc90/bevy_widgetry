# Cursor

## 功能概述

为各 Native Window 独立管理 OS Mouse Cursor，并提供 System 与 Kamuri 两种显示主题。Widget 可以通过官方 Cursor 声明或 Widgetry 扩展 Hint 请求相应样式。

## 当前设计思路

Hover 语义由 Entity / Hierarchy 提供，真正的 OS Cursor 只由有效 Mouse 身份驱动。Drag 使用有 Owner 的 `CursorLease` 临时固定样式；原生窗口 Move / Resize 期间让出 Cursor 控制权。展示主题与颜色 Theme 相互独立。

## 主要功能

- 解析最近祖先 `EntityCursor`，以及 `WidgetryCursorHint` 的十一种扩展业务语义。
- 有效 Disabled 情况可展示 NotAllowed，Button Loading 的内部标记优先展示 Wait，并在遮挡、失效或生命周期变化时重新计算 Cursor。
- System Theme 使用系统样式；Kamuri 提供 34 种系统语义与 11 种扩展样式，共 45 项。
- Kamuri 的全部 45 种语义均持续循环动画，统一 80×80 RGBA 帧画布，各自定义帧数、速率与热点。
- Drag Lease 支持 Owner 失效、Cancel、Release 和失焦后的清理。

## 行为与状态契约

只有 `PointerId::Mouse` 影响 OS Cursor；Custom 与 Touch 不取得 Lease。最近有效声明优先，同一 Entity 上官方 `EntityCursor` 优先于扩展 Hint。动画进入新语义时从首帧开始，同一语义内持续显示不重启；Native 操作、失焦或离开语义时停止。未加载图集时先使用静态后备帧，再退回语义对应的 System Cursor。调用方自定义 `EntityCursor::Custom` 图像保持不变。

动画目录由 Asset 层的 `BuiltinCursorAnimation` 提供语义标识、时序、热点、图集布局和帧计算，OS 播放与 Gallery 预览共用。每种动画独立图集，按实际出现的语义懒加载并缓存，主题选择仅预载静态后备图集。后备图集的每项均为对应动画首帧，画布及热点不变。制作工具与逐种动作说明见 [素材工具说明](../../crates/asset/tools/README.md)。

每个窗口独立记录当前语义及进入时间，使用真实时间推进，不受虚拟时间暂停或单帧上限影响。只有有焦点的有效 Mouse 目标窗口及有效 Drag Lease 会保持动画唤醒；清除 Hover 不会结束仍有效的 Lease / Capture。普通页面没有有效 Mouse 位置、窗口失焦、原生窗口操作或当前语义失效时停止播放和额外重绘。资源正在加载且当前窗口确实需要该语义时，保留必要更新直至进入Loaded或Failed，避免反应式窗口停在回退图。加载完成后按动画更新；失败、已加载但CPU图像不可用、失焦或语义失效时不维持加载唤醒。静态或System回退本身不产生持续动画重绘。

## 边界情况与错误处理

`WidgetryCursorHint::ReadOnly` 表达可选择、可复制但不可编辑的区域，与 Disabled 独立。System Theme 或 Kamuri 素材不可用时回退 `SystemCursorIcon::Cell`；Kamuri 使用扫描文档与闭合蓝白挂锁的循环动画，热点 `(25,56)` 固定于锁孔。Hint 只声明光标语义，应用仍负责实际编辑资格；未启用编辑的普通表格不应自动声明 ReadOnly。编辑器内部光标由编辑器自己的最近声明控制。

素材解码或加载失败应保留有意义的 System Fallback，不持续制造重绘循环。重复安装与 Bevy 官方 `CursorIconPlugin` 冲突需要报告；Window / Pointer / Lease 失效时不能残留错误的 Cursor 所有权。

## 已知限制

- 不接管 Pointer Event 派发或捕获。
- Kamuri 图片与动画要求宿主具备适当 Asset / Image 能力；Headless 环境可保留系统回退。
- OS 级光标视觉、真实 Native Move / Resize 不能只靠 Custom Pointer 模拟来证明。

