# 让 Gallery 标题国际化使用 Window 语义接口

## 目标与范围

替换 `gallery/src/i18n.rs::refresh_window_titles` 的内部层级和旧文本匹配逻辑，使用 Window 的标题语义能力。保留现有 `LocalizedWindowTitle`、强类型文案和页面生命周期。

## 修改设计

语言或标题绑定参数变化时，计算目标字符串并调用稳定的窗口标题更新入口。不再从首个 Child 遍历，不再使用旧标题、Pickable::IGNORE、是否 EditableText、是否已有 LocalizedText 等条件识别目标，也不为识别库内标题附加猜测式 LocalizedText。

普通正文仍由 `refresh_text` 处理，EditableText 的排除和命名参数语义不改变。原生标题与绑定的可见标题在同一个明确的更新流程中保持一致，首次新建窗口采用当前语言，已经打开的 Window、MessageBox、FileDialog 能随语言切换更新。

正确安排宿主国际化与窗口标题投影的时机，但不让 Window 库反向依赖 Gallery。用户输入、文件名、文件路径和业务文本不因切换语言被翻译；正文与旧标题碰巧相同也保持原样。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

用 BRP 打开多类窗口，切换语言并检查原生标题属性、可见标题和正文同名文本。覆盖新建时机、标题参数变化、关闭后切换语言以及页面离开后的清理。原有 Focus、Modal、Session 和文本编辑状态不被重建。

Gallery 不被强制增加自身单元测试；必要的标题边界回归已属于 Window 能力本身。同步 Gallery 国际化说明，删除已经过时的启发式匹配原因注释。

## 预期产出

不再了解控件私有标题层级的 Gallery 窗口国际化实现。

## 与前后方案的关系

顺序上接 [方案 33：为 Window 提供稳定的标题文字绑定与更新入口](33-为-Window-提供稳定的标题文字绑定与更新入口.md)。

完成后进入 [方案 35：让 MessageBox 的样式归属按生命周期建立](35-让-MessageBox-的样式归属按生命周期建立.md)。

依赖已经完成的 Window 标题语义入口；Gallery 的本地化类型不会进入库。
