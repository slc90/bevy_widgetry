# 04 | 恢复 TextField、焦点与多窗口 IME 的完整行为

[返回方案总览](00-overview.md)

## 目标

使普通、只读和禁用 TextField 在 0.20 输入体系下保持原有差异，并让键盘、IME 与模态窗口路由一致。

## 范围

text_field、window/input，以及使用文本输入的 ComboBox、MessageBox、FileDialog 和 Gallery 页面。

## 预期产出

正常输入、选择复制、只读保护、禁用保护、候选框位置和模态焦点行为都能独立验收。

## 与前后方案的关系

接在方案 03 之后复用已验证的 Pointer 输入；新版 TextInput 和移除接口的编译修复已在方案 01 完成。随后核对 UI 布局、颜色和增量更新。

## 执行边界

只迁移 slc90/bevy_widgetry，不修改 Bevy 或 bevy_brp 仓库。保持现有 crate 依赖方向、headless/style 分层、facade 导出、控件公共行为和 Windows 64 位定位；不借升级重做控件架构、不引入双版本 Bevy 兼容层、不扩大到其他平台或新增 CI。BRP 使用已有的兼容标签，不另外安排 BRP 升级项目。[R1][R2][R3]

不增加迁移前状态记录、快照、历史提交登记或独立性能对照任务。已有测试与 benchmark 入口必须保留并能编译，但不要求先收集旧版本测试、截图或性能数据。修复只服务于本次升级；不能靠删除测试、忽略失败、移出 workspace、关闭原有功能、临时空实现或放宽 unsafe/panic 约束换取通过。

## 每份方案的编译出口

每份方案完成后，验收对象都是整个 workspace，而不是本方案主要修改的几个 crate。使用 Windows 64 位的 x86_64-pc-windows-msvc，Rust 不低于 1.98.1，并具备现有 rust-lld.exe 配置所需的 MSVC/Windows SDK 环境。Bevy 自身的最低 Rust 要求是 1.97.1，但选用的 BRP v0.4.0 把本项目实际下限提高到 1.98.1。[R3][R4][R5]

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --target x86_64-pc-windows-msvc --locked
cargo build --workspace --all-targets --target x86_64-pc-windows-msvc --locked
```

check 用来尽早发现类型和宏展开问题；build 用来确认实际代码生成、链接及测试/benchmark 目标也能构建。只执行 check 不能声称完整构建通过。这里的 all-targets 指 Cargo 目标，不是扩大到其他操作系统；也不添加会改变原有 feature 范围的 all-features。

本方案触及的既有测试和新增回归测试必须通过。第一份方案还要让全部既有库/集成测试通过，不能把已知失败转交给后续方案；后续方案负责更深入的契约核对和补充覆盖，而不是补交前面的编译结果。编译失败时就在当前方案范围内闭合修复，不进入下一份方案。

## 证据与不确定性的处理

“确定必改”表示在已读取的项目代码中找到了与 0.20 不兼容的用法；“命中后修改”表示上游有变更，但还不能断言仓库每个候选文件都使用了它。实现时要对 src、tests、benches、Gallery 和可编译文档示例一起搜索。未命中的迁移条目不制造修改；契约已经满足的模块不为了让方案看起来有工作量而重写。

文中的编译命令和运行期检查是实施者的验收要求，不是本次已经取得的通过结果。任何具体 API 字段、构造器或调度点仍有疑问时，以锁定的 v0.20.0 源码和真实编译诊断为准，不能用默认值吞掉错误或猜一个 API 名称补洞。

## TextInput 是输入能力，EditableText 是编辑数据

WidgetryTextField 和 WidgetryReadOnlyTextField 的基础 Scene 当前只插入 EditableText。0.20 的交互处理位于 TextInput，且 TextInput 会要求 EditableText；仅保留旧组件可能仍能显示文字，却不再进入官方键盘和指针输入查询。因此普通和只读输入根都要接入 TextInput，不能通过让只读框缺少 TextInput 来阻止修改，否则选择、导航和复制也会一起丢失。[R18][R19]

只读与禁用不是同一种状态。只读保留允许的选择、移动与复制，同时拒绝会改变文本的输入；禁用继续按 Widgetry 已有契约阻止交互。上游 TextReadWriteMode 与 TextEdit::is_destructive 可以作为对齐入口，但是否取代现有过滤器，要看它是否覆盖本项目的所有输入通道，不能仅凭枚举名称判定完全等价。[R18][R19]

当前 Widgetry 还会清理 pending_edits 和 pending_paste；只读过滤包括 Cut/Paste/Insert、各种删除以及 IME compose/commit。迁移时既检查真实键盘输入，也检查已经排队的编辑和异步粘贴，防止输入先排队、随后状态变为只读或禁用时，旧任务仍写入文本。新的破坏性编辑类型不能因为旧的黑名单没有列到就穿过保护。[R18]

这些守卫仍必须处于禁用状态确定之后、EditableText 实际消费编辑之前。不要只改查询组件而丢掉原有调度先后。TextField 的只读配色、焦点配色、光标和选区颜色继续通过既有样式 owner 管理，不借 TextInput 接入重新建立一套状态。[R18][R21]

## 编辑视口和多窗口 IME 必须一起对齐

window/src/input.rs 目前直接读取 TextScroll，并在旧的 scroll_editable_text 之后计算候选框位置。位置计算组合了编辑器的 ime_cursor_area、ComputedNode.content_box、UiGlobalTransform、UiScale 与目标 scale factor。这不是单纯替换一个 import：要改成读取 EditableText 的新编辑视口，并核对新视口更新与文字布局的调度关系，保证取到当帧的滚动偏移。[R20]

不能凭字段名称假设新 viewport 的单位和旧 TextScroll.0 相同。应以目标版本的实际定义确定偏移方向、内容坐标和缩放含义，在短文本、横向滚动后的长文本、多行内容、DPI/UiScale 变化以及窗口移动/缩放时验证候选框贴合光标。避免重复乘除 scale，避免把主窗口 scale factor 套给其他 native Window。

当前项目按 Ime 消息携带的 Window 判断是否发给当前焦点，只有有效且可用的目标才排入 compose/commit；同时在存在受管窗口时抑制官方相应 IME 系统，防止重复分发。升级后要核对 ImeSystems 的名称与所属 schedule，保持“每条输入只有一个实际处理路径”，但不能影响没有 Widgetry 受管窗口时的默认行为。[R20]

IME 开关与位置只更新正确的 native Window；模态切换、焦点丢失、输入实体销毁和只读/禁用切换时不继续把旧 composition 写回原目标。若新官方输入查询要求 TextInput 或读写模式，Widgetry 自己的路由也要遵守同样的实际输入资格，而不是看到 EditableText 就把任意文本当成可交互输入框。[R19][R20]

## 焦点、Tab、Escape 和反射构造边界

保留 TextField 的 AcquireFocus 传播控制：没有 TabIndex 的输入框通过 pointer 获得焦点后，不应又因事件继续传播而被 window 清空。模态窗口继续以实际 native Window、最内层/最近有效的 modal scope 选择输入接收者，Tab 在允许的控件中循环，关闭后只恢复仍有效的旧焦点。[R18][R20][R28]

IME composing 期间的 Tab 和按键不能被 Widgetry 提前当作窗口导航处理。当前 keyboard 路由会检查正在组合及已排队的相关编辑，新 TextInput 也会处理 composing；二者要协作而不是发生两次导航或先改焦点后提交 composition。[R19][R20]

0.20 的 Escape 输入行为可能先折叠选择、清除焦点再继续传播。因此 ComboBox popup、MessageBox、FileDialog 或宿主快捷键的处理不能仅通过“此刻 InputFocus 是否为空”猜测原始输入来源。涉及来源判定时检查事件的原始目标及 trigger 语义，不把冒泡到的当前实体误认成发起输入的文本框。是否需要一次 Escape 只退出编辑、再次才关闭窗口，应按已有 Widgetry 契约和测试决定；这次迁移不擅自增加一个新的两段式交互规则。[R6][R19][R20]

focused_event helper 目前通过 DynamicStruct 的 focused_entity、input、window 字段构造 FocusedInput。核对 0.20 是否提供合适的公开构造器；有则采用公开接口并保留窗口身份，没有则验证现有反射字段/类型是否仍匹配。反射失败必须记录日志并返回原有严重级别的错误，不能返回空事件或退回 primary window。更新这段明确写有 0.19.1 的兼容注释，使它只描述实际仍存在的边界。[R20]

## 文本输入方案的验收重点

主要自动化入口是 crates/text_field/tests/disabled.rs、read_only.rs、pointer.rs、widgetry_text_field.rs 及相关 support，以及 facade focus 和 window/组合控件的输入测试。覆盖正常插入删除、只读选择复制、禁用阻挡、待处理粘贴/IME 被状态切换阻断、pointer 获焦、Tab 导航和 Escape 传播；针对真实耦合补回归，不复制官方编辑器所有内部测试。[R32]

真实 Windows Gallery 中验证中文等 IME 的 composition 与候选框位置，至少包括一个非主窗口或 modal 内的输入框。FileDialog 中的路径、文件名和搜索/过滤等实际已提供输入场景，要以界面现有功能为准，不为了测试扩展新的业务能力。检查一次输入只产生一次文本变化、窗口关闭结果不重复、焦点不会落入被挡住的父窗口。

headless 测试可以验证路由与状态，但不能代替操作系统候选框和真实窗口焦点验证。Gallery 依然按项目规则做运行期验收，不新增必须长期维护的 Gallery 单元/集成测试体系。[R28][R29][R32]

## 参考来源

[R1] Widgetry 工作区、成员与依赖声明

`https://github.com/slc90/bevy_widgetry/blob/main/Cargo.toml`

[R2] Widgetry 项目范围与架构约束

`https://github.com/slc90/bevy_widgetry/blob/main/rules/project-context.md`

[R3] BRP v0.4.0 工作区版本与 Rust 要求

`https://github.com/slc90/bevy_brp/blob/v0.4.0/Cargo.toml`

[R4] Bevy v0.20.0 工作区与 Rust 要求

`https://github.com/bevyengine/bevy/blob/v0.20.0/Cargo.toml`

[R5] Widgetry Windows 构建配置

`https://github.com/slc90/bevy_widgetry/blob/main/.cargo/config.toml`

[R6] Bevy 0.19 → 0.20 官方迁移索引

`https://bevy.org/learn/migration-guides/0-19-to-0-20/`

[R18] Widgetry TextField 构造、只读和禁用处理

`https://github.com/slc90/bevy_widgetry/blob/main/crates/text_field/src/style.rs`

[R19] Bevy v0.20.0 TextInput 输入处理

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui_widgets/src/text_input.rs`

[R20] Widgetry 多窗口键盘、IME 与焦点路由

`https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/input.rs`

[R21] Widgetry UI 调度约束

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/ui.rs`

[R28] Widgetry 窗口公开契约

`https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/lib.rs`

[R29] Gallery 入口与 BRP runtime 安装

`https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs`

[R32] Widgetry 测试规则及 Gallery 例外

`https://github.com/slc90/bevy_widgetry/blob/main/rules/testing.md`

