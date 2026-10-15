# 为 Window 提供稳定的标题文字绑定与更新入口

## 目标与范围

在 `crates/window/` 建立由控件负责的标题文字语义绑定和更新操作，并让 MessageBox、FileDialog 的库内默认标题构造使用这一绑定。Facade 只重导出必要的公开能力。

## 修改设计

目前稳定的公开入口只能取得 Native Window，而可见标题 Text 缺少明确身份。新能力通过语义标记或等价绑定识别标题，不以 Children 顺序、原有字符串、Pickable 或 EditableText 特征猜测目标。标记只表示真实运行期绑定身份，不成为另一套 UI Builder。

更新操作以有效 Widgetry Root 和新标题为输入，使用 Native Window 的标题作为真实文本来源；可见标题是显示投影，不保存第二份可独立写入的业务标题。先验证绑定和目标，再完成原生标题与已绑定显示文字的同步。无效 Root、失效 Native Window、失效或歧义绑定按 Error 契约报告，不能先改一半再发现目标错误。

任意自定义 TitleBar SceneList 仍然合法，不要求从它的所有 Text 中挑一个标题。没有显式标题绑定时，原生标题可单独更新；库提供的默认标题必须有明确绑定。标题节点延后创建时应从最新的原生标题初始化，不能因此丢失创建期间发生的语言变化。具体公开符号名称在实施时按已有 API 风格确定，但这些行为边界不再留给 Gallery 猜测。

库不依赖 Gallery 的 Language、LocalizedText 或 TextKey。绑定维护不每帧遍历整个窗口子树，不影响标题中的 Icon、操作控件和正文同名文字。保留现有 BSN TitleBar／Body 组合和 Native／Camera 生命周期。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

回归覆盖默认 Window／MessageBox／FileDialog 标题、带图标或额外文本的自定义标题栏、无绑定的自定义内容、正文同名文本、多个窗口、重复同值、无效 Root 和延迟构造。原生标题与绑定文字在约定的有效呈现阶段一致，正文不被修改。

验证公开入口不暴露 WindowRoot 或私有层级布局，Scene 与错误用例按现有宿主 Handler 捕获。同步 Window 功能文档和相关 Coverage，不增加无实际需求的统一标题事件协议。

## 预期产出

应用可稳定使用、与 Gallery 解耦的窗口标题绑定和同步能力。

## 与前后方案的关系

顺序上接 [方案 32：让 Window 标题栏与系统按钮配色按变化更新](32-让-Window-标题栏与系统按钮配色按变化更新.md)。

完成后进入 [方案 34：让 Gallery 标题国际化使用 Window 语义接口](34-让-Gallery-标题国际化使用-Window-语义接口.md)。

本项提供下一项 Gallery 标题国际化所需的稳定入口，窗口配色改动不是标题 API 的替代品。
