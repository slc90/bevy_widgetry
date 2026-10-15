# Cursor

## 功能概述

为各 Native Window 独立管理 OS Mouse Cursor，并提供 System 与 Kamuri 两种显示主题。Widget 可以通过官方 Cursor 声明或 Widgetry 扩展 Hint 请求相应样式。

## 当前设计思路

Hover 语义由 Entity / Hierarchy 提供，真正的 OS Cursor 只由有效 Mouse 身份驱动。Drag 使用有 Owner 的 `CursorLease` 临时固定样式；原生窗口 Move / Resize 期间让出 Cursor 控制权。展示主题与颜色 Theme 相互独立。

## 主要功能

- 解析最近祖先 `EntityCursor`，以及 `WidgetryCursorHint` 的十种扩展业务语义。
- 有效 Disabled 情况可展示 NotAllowed，并在遮挡、失效或生命周期变化时重新计算 Cursor。
- System Theme 使用系统样式；Kamuri 提供 34 种系统语义与 10 种扩展样式。
- Kamuri Pointer、Wait 分别使用 12 帧 / 24 帧、8 FPS 的动画，静态光标和回退图像来自内建 Atlas。
- Drag Lease 支持 Owner 失效、Cancel、Release 和失焦后的清理。

## 行为与状态契约

只有 `PointerId::Mouse` 影响 OS Cursor；Custom 与 Touch 不取得 Lease。最近有效声明优先，同一 Entity 上官方 `EntityCursor` 优先于扩展 Hint。动画进入新语义时从首帧开始，同一语义内持续显示不重启；Native 操作、失焦或离开语义时停止。未加载图集时先使用静态后备帧，再退回语义对应的 System Cursor。调用方自定义 `EntityCursor::Custom` 图像保持不变。

Kamuri 普通静态图像使用 48px 切片；Pointer / Wait 使用 64px 动画，Hotspot 分别为 `(22, 54)` 的按钮中心与 `(25, 47)` 的加载环中心。Lease 随交互 Owner 管理，正常释放、取消、失焦或 Owner 销毁都应结束固定光标控制。有焦点窗口中的有效 Mouse 光标语义驱动动画和重绘，来源包括 Hover、Drag Lease 与 Capture；清除 Hover 不会结束仍有效的 Lease / Capture 动画，窗口失焦或当前语义失效后停止动画。

## 边界情况与错误处理

素材解码或加载失败应保留有意义的 System Fallback，不持续制造重绘循环。重复安装与 Bevy 官方 `CursorIconPlugin` 冲突需要报告；Window / Pointer / Lease 失效时不能残留错误的 Cursor 所有权。

## 已知限制

- 不接管 Pointer Event 派发或捕获。
- Kamuri 图片与动画要求宿主具备适当 Asset / Image 能力；Headless 环境可保留系统回退。
- OS 级光标视觉、真实 Native Move / Resize 不能只靠 Custom Pointer 模拟来证明。

