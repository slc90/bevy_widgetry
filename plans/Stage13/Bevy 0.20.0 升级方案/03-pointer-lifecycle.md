# 03 | 对齐 Pointer 事件与交互状态生命周期

[返回方案总览](00-overview.md)

## 目标

让新的扁平 Pointer 事件正确驱动现有 hover、pressed、拖动和禁用状态，不破坏多指针及多窗口隔离。

## 范围

core Pointer、ScrollArea、依赖这些机制的控件与组合控件、test_utils 和真实输入管线测试。

## 预期产出

新版事件与测试构造器使用一致；指针消失、窗口失效、取消和跨指针终止都能正确收尾。

## 与前后方案的关系

在方案 02 的 Scene 构造契约稳定后执行，随后进入文本与窗口输入。Scene 验证有助于排除构造问题，但普通指针逻辑并非在技术上必须等所有 Scene 清理；此处按整条迁移链固定顺序执行。

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

## 事件外形改变，交互身份不能改变

高层事件从 Pointer<Press> 这类包装形式改为 PointerPress 等具体类型。公共指针信息位于 event.pointer 中：id、target 和 position 对应指针身份、渲染目标与坐标；需要 Location 时使用目标版本提供的转换接口。被事件触发/传播到的实体仍要和指针指向的渲染目标分开理解，不把 window/camera target 当成控件实体。[R11]

新事件构造必须使用公开 API。公共 Pointer 信息包含内部传播状态，不能为了拼结构体而假定所有字段都公开，更不能引入 unsafe 或重新在 Widgetry 定义一个旧 Pointer<T> 来绕开上游类型。press、release、cancel、click、drag 等测试工厂应输出对应的新事件，保留原来用于验证的 button、hit、count、duration 和 distance。[R11][R16]

低层 PointerInput 仍承担原始输入队列，继续使用自己的 pointer_id、location、action。直接 world.trigger(PointerPress ...) 和向消息队列写 PointerInput 是两种不同刺激：前者验证 observer 局部行为，后者才能验证 picking/hover/pressed 的真实调度组合。不能用前者替换所有后者，然后宣称输入管线已经覆盖。[R15][R16]

## hover、pressed 与拖动的所有权规则

WidgetryPointerQuery 先过滤非有限坐标及已经不存在的 Window target，再给 hover 和所有权处理使用。hover writer 的替换是有意的调度集成：项目移除官方对应 writer，在 update_interactions 之后、pointer_events 之前写入所需状态。迁移后检查移除的 system/set 是否仍指向正确对象，保留合理的 SetNotFound 处理，但不能让“找不到”掩盖实际存在的两个 writer。[R14]

Pressed 所有权由 pointer、target、button 共同确定。来自其他指针、其他窗口或其他按键的 release/cancel/drag end 不应终止原 owner；同帧结束后又开始的输入也要按原始消息顺序处理。指针失效、对应按键释放、取消或实际禁用则必须清理状态，不能留下按住样式。调用方原先已维护的 Pressed 与本次由 Pointer 创建的 Pressed 必须继续区分，不能让清理逻辑覆盖外部状态。[R15]

ScrollArea thumb 还绑定 scrollbar 与 viewport。拖动期间 scrollbar 消失、target 改绑、viewport 消失、pointer 换到别的窗口或控件被禁用，都要终止属于它的拖动并清理 ScrollbarDragState/owner；其他指针的终止事件不能误清理当前拖动。不能只让新事件字段访问通过，却忘掉这些跨组件的有效性条件。[R17]

生命周期 observer 的泛型迁移不得改变 Add、Insert、Remove 各自的触发职责。对 Disabled、Pressed、renderer 或其他状态的插入/移除，保持原来的释放责任和去重处理；不要因为签名变了就把 Remove 逻辑移到每帧全量扫描里。

## 共享测试基础设施与控件覆盖

crates/test_utils/src/pointer.rs 中的 primary_press/release/cancel/drag_end/click 以及泛型 pointer_event helper 都是迁移范围。原来用“泛型 payload + Pointer<E>”建事件的 helper 不能原封不动套到新版；采用目标事件的构造方式或明确的事件工厂，并把全部调用方一起迁移。helper 应简化测试准备，不定义第二套生产事件语义。[R11][R16]

至少在 core 的 pointer/disabled_input/effective_disabled、test_utils 的 pointer_pipeline，以及 facade 的 pointer/focus 组合测试中证明正常输入和失效收尾。受影响的 Button、CheckBox、RadioGroup、ScrollArea、ListView、Tree、Table、Tooltip、窗口 controls 与 FileDialog 通过已有业务测试或必要的新回归覆盖各自真实耦合；不要求把所有独立状态做笛卡尔积。[R32]

关键刺激包括正常按下/释放、按下后离开或失去有效位置、取消、指针实体销毁、Window 销毁、禁用切换、另一个 pointer 的终止事件和目标重新绑定。断言应覆盖状态变化及不应出现的变化通知，不只检查最后一次颜色。与源码修复相关的 pointer_lifecycle、pointer_hover 等 benchmark 必须保持构建，但这里不安排性能对照测量。

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

[R11] Bevy v0.20.0 Pointer 事件定义

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_picking/src/events.rs`

[R14] Widgetry Pointer hover 与有效性处理

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/pointer.rs`

[R15] Widgetry 按下状态所有权

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/pointer/pressed.rs`

[R16] Widgetry Pointer 测试构造器

`https://github.com/slc90/bevy_widgetry/blob/main/crates/test_utils/src/pointer.rs`

[R17] Widgetry 滚动条拖动所有权

`https://github.com/slc90/bevy_widgetry/blob/main/crates/scroll_area/src/pointer.rs`

[R32] Widgetry 测试规则及 Gallery 例外

`https://github.com/slc90/bevy_widgetry/blob/main/rules/testing.md`

