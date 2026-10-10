# 05 | 对齐 UI 布局、颜色与增量更新时序

[返回方案总览](00-overview.md)

## 目标

让 0.20 下的布局、裁剪、文字和颜色在正确的帧内更新，并保留虚拟化及稳定状态不重复工作的契约。

## 范围

core UI/font/text/color/disabled、theme/asset、ScrollArea、ListView、Tree、Table、Tooltip 和相关窗口背景布局。

## 预期产出

实际命中的布局 API 完成适配；首帧和动态更新正确，滚动可见性收敛，虚拟化与内容配色没有无条件重建。

## 与前后方案的关系

接在文本与焦点方案之后，随后进行 Windows/DX12 与 Gallery 运行期验收。它不承担前面尚未修好的文本输入或编译错误。

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

## 字体、尺寸单位和圆角只在实际边界上适配

core/font.rs 的 FontSource::Monospace 测试用法需要新版构造函数；同类字体来源按实际命中迁移。默认字体仍然只替换原本使用默认来源的新 TextFont，不覆盖调用方显式选择的字体，也不把一次性 Added 处理改成每帧覆盖。默认字体落在 Materialize/Prepare 之后、文字重新布局检测之前的职责要保留。[R22]

对尺寸解析、裁剪和圆角的 API 变化，先按官方迁移索引定位命中项。Val 增加 Em/Rem，TextFont 默认尺寸与 rem 相关，单角圆角类型也有调整；这要求检查自定义解析/穷举匹配和旧的常量构造，而不是给所有 Node 统一改数值。[R6]

若 Widgetry 自己把 Val 转成像素，必须用正确的文字/根字号上下文处理新单位，或明确维持原公开 API 的有效输入限制；不能用 wildcard 分支把不认识的值转换成 0。现有明确为固定像素的视觉契约可以显式表达，其他地方不要为了模仿旧默认值而全局锁死字号。缩放、换字体以及 rem 环境变化都应按真实支持范围验收。

普通 BorderRadius::all(...) 与逐角配置要分开核对，类型适配只改变表达方式，不改变原来的圆角形状。若原先常量初始化不再合法，采用等价初始化位置即可，不为了一个构造器变化新增资源或全局缓存。Asset 的字体和 SVG 图标文件不属于此次依赖升级的替换对象。

## 裁剪、变化检测与调度不能只求类型通过

对实际读取 CalculatedClip 的代码，要区分新的裁剪结果类型及完全裁掉的状态，正确处理坐标变换；命中 contains_point 等接口时确认物理像素与 UI 局部坐标边界。这个检查只针对实际存在的读者，不假设每个 Widget 都实现了自定义裁剪。[R6]

测试重点是旋转/缩放或嵌套滚动内容不再落入错误的可见区域，完全裁掉的节点不能继续参与对应可见性判断，逻辑坐标与物理坐标不能混用。已有交互命中交给官方 picking 的地方继续使用官方管线，不为迁移另写一个平行命中系统。

core/ui.rs 已经明确了 Build → UiSystems::Prepare → Materialize → Disabled → StyleOwners → Colors → UiSystems::Propagate → ContentColors 的先后，以及 Materialize 对 visibility、stack 和文字检测的约束。不能因上游部分 schedule 改用较弱排序就把 Widgetry 的这些链也机械改弱；涉及 Commands 写入需要当帧被下游读到时，必须保留足够的顺序及 deferred 应用边界。[R21]

验证应观察首帧构造、主题切换、禁用传播和内容替换后的实际结果，不通过多调用一次 app.update() 掩盖调度缺口。若原来为了验证顺序直接读取 schedule 内部图，内部查询可以随 API 改，但测试保护的语义不能从“写入先于消费”退化成“系统存在即可”。[R21][R22]

面对 UI 渲染更新机制的变化，保留真正变化时的 change detection。现有 set_if_neq 与增量颜色写入不应被改成无条件整树更新；反过来，确实改变的材质/文本/图像必须能被渲染器看到，不能用 bypass_change_detection 逃避更新成本。

## 滚动、虚拟化和内容颜色保持原来的工作量语义

ScrollArea 的可见滚动条决策依赖 root、viewport、scrollable/content 尺寸及当前收敛状态。一个滚动条出现会占用空间并诱发另一个方向溢出；稳定后则不应持续 RequestRedraw。迁移 scrollbar 目标类型、布局单位或尺寸读取时，要保护这种收敛过程，而不是固定显示所有滚动条来绕开新布局差异。[R23]

ListView、Tree、Table 的逻辑 model/稳定 ID/selection 与物理显示行分开处理。模型改动后原来的选择修复、disabled metadata 和可见投影关系不变；ScrollArea 或 clip 变化不能让虚拟化退化成每帧生成全部行。需要修改的通常是 UI 读取和布局交汇处，不是模型算法本身。Table 的列 resize、表头/正文同步和共享滚动边界，应在已有 interaction/layout/projection/viewport 测试中核对。

颜色链路继续通过 Widgetry 的 owner、Theme 与内容标记传播。Text 和 Icon 的调用方显式颜色、只读/禁用/选中状态、嵌套组合控件的主题切换，要保持已有优先级；不能通过给每个内部实体硬写颜色使一个截图看起来正确，却破坏公共 override 语义。[R18][R21][R33]

Tooltip、ComboBox popup 和 Window 背景可能依赖新的布局结果定位。只修命中的位置计算、裁剪或时序；保留已有 popup lifecycle、背景 Stretch/Cover 与透明语义，不追加新的窗口背景更新 API。[R28]

## 布局和增量更新的验收落点

core 的 ui_schedule、color_schedule、default_font、content_colors、icon 与 effective_disabled 测试负责共同边界；ScrollArea、ListView、Tree、Table 各自的 view/rendering/virtualization/interaction 测试保护业务结果。保持测试分工，不把所有组合都塞进一个巨大的全场景断言。[R32]

至少验证构造后的首帧、主题/字号/窗口尺寸变化、内容从少变多再变少、滚动条稳定/重新收敛、虚拟行重用后的内容和样式、显式颜色覆盖及完全不可见内容。对于已有可测的工作量 invariant，继续检查稳定状态不重建、变化只影响必要范围；普通测试不增加依赖机器耗时的阈值。

既有 color_resolution、effective_disabled、icon_update、list/tree/table update 等 benchmark 目标随 API 适配并参与构建。只有具体修复触及既有性能验收规则时才运行相应场景；本次没有新建历史性能对照、先测旧版本或收集整仓基线的要求。

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

[R21] Widgetry UI 调度约束

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/ui.rs`

[R22] Widgetry 默认字体与调度测试

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/font.rs`

[R23] Widgetry 滚动区域几何与可见性收敛

`https://github.com/slc90/bevy_widgetry/blob/main/crates/scroll_area/src/layout.rs`

[R28] Widgetry 窗口公开契约

`https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/lib.rs`

[R32] Widgetry 测试规则及 Gallery 例外

`https://github.com/slc90/bevy_widgetry/blob/main/rules/testing.md`

[R33] Widgetry facade 与平台边界

`https://github.com/slc90/bevy_widgetry/blob/main/crates/bevy_widgetry/src/lib.rs`

