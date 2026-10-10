# 02 | 对齐 BSN 表达与 Scene 构造契约

[返回方案总览](00-overview.md)

## 目标

让 0.20 下的 Scene 构造、组合、属性覆盖和失败清理保持 Widgetry 既有语义。

## 范围

各 Widget 的 Scene/Props/renderer 构造、core Scene 扩展、相关测试与 Gallery 的 BSN；只清理与迁移有关的写法。

## 预期产出

一致的 BSN 使用方式，以及能证明实体引用、构造生命周期和错误传播没有退化的测试。

## 与前后方案的关系

依赖方案 01 已完成的全工作区编译切换。这里不再接收尚未迁移的编译语法；完成后进入 Pointer 生命周期核对。

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

## 按表达式的实际类型迁移 BSN

BSN 是跨仓库的共同入口，不是某个控件内部的独立语法。0.20 中引用已有 Scene 的函数、变量和表达式时使用相应的 @ 形式；SceneComponent 原来的 @Widgetry... 及其 @props 仍要保留。对已读取的 TextField 构造，text_field_base_scene() 属于 Scene，需要写成 @text_field_base_scene()；同一行的 template(...) 是模板，不应一并加上 @。[R12][R18]

同样，Gallery 中返回 Scene 的 widgetry_window(...)、页面函数或内容函数，需要按 Scene 表达处理；普通组件构造、on(...) observer 和 template(...) 不能通过一个“所有函数前加 @”的正则转换。{...} 可能是字段的 Rust 表达式，也可能在 Children 内插入 SceneList；只有实际承载 Scene 的表达式才适用对应的 Scene 插入写法。[R12][R29]

列表统一使用能表达实体边界的 -- 分隔形式，并优先采用 bsn_list! { ... } 的清晰写法。清理旧括号和逗号时，不能把一个实体上的多个组件拆成几个实体，也不能把几个原有 child 合并成一个。Children 的顺序仍然有意义，尤其是表头/内容、窗口标题栏/正文、滚动条及 Waveform camera/mesh 的组合。[R12][R24]

构造中的枚举字段应满足新版完整变体表达要求；只在实际存在旧的 VariantDefaults 或相关写法时迁移，不给所有普通结构体改 derive。最终应由具体 BSN 展开和公开构造测试确认，而不是仅对源码做字符串检查。

## 保留模板上下文、Props 和实体引用

template(...) 有时只是返回一个组件，有时还负责读取 world、载入资源、建立 ChildOf 或返回可传播错误。只对完全等价的纯值构造采用新的简写；涉及上下文、借用、闭包捕获和副作用的模板不为了“现代化”一律移除。[R12][R13][R29]

SceneComponent 的 Props 仍是一次性构造输入，运行期状态继续通过各 Widget 已有 API 维护。颜色 owner、所引用的 model/source、Renderer 注册和用户提供内容的所有权不能在迁移中变成第二份状态。允许 BSN 写法变动，不允许因为宏迁移而顺手改变这些公开契约。[R18][R33]

带 Entity 或 Handle 字段的结构必须保留必要的 FromTemplate 语义。例如 Waveform 的 RenderScene 持有 camera 和 mesh 的 Entity，并通过 #WaveformCamera、#WaveformMesh 获取构造出的实体；不能把这类 derive 简化成 Default + Clone 后，让占位 Entity 悄悄流入运行期。命名实体、向前引用、嵌套 Scene 的作用域以及传入内容列表都要在实际构造结果中验证。[R12][R24]

重点入口包括 core 的 icon/text/scene、各 Widget 的 scene/style/view/renderer、window 与 message_box 的构造函数、file_dialog 的组合界面，以及 Gallery 的 pages、color_showcase 和窗口子页面。只处理命中的构造表达，不借这个方案重排整个仓库的源码结构。

## Scene 失败必须仍然收尾正确

core/src/scene.rs 不是官方 spawn_scene 的简单别名。它把 deferred command 的错误交给宿主处理，保留 Severity::Error，避免已记录的构造错误重复打日志，并把预约 root 在执行前消失视为正常取消。这些行为是本项目真实的兼容边界，不能因为 0.20 提供了类似名称的方法就直接删除封装。[R13]

apply_scene 失败时，当前实现还会区分本次新建的空 reservation 与原先存在的空实体，回收尚未建立 ChildOf 的 child 或 forward reference。迁移 SpawnSceneError、ApplySceneError 或其嵌套错误包装时，必须保留“不会泄漏新 reservation，也不会误删既有实体”的区别。若上游已补齐部分清理，也要通过真实失败路径证明之后再简化，不能凭猜测保留双重清理或提前移除保护。[R13]

验证应覆盖成功构造及字段覆盖、嵌套/命名引用、构造失败、应用到既有 root 失败、预约 root 被取消和已记录错误的传播。测试应观察实体存续、原有状态、宿主 handler 收到的错误严重级别以及 Widgetry 日志；不把 catch_unwind 或 should_panic 当作错误传播成功的证明。[R13][R32]

主要落点为 crates/core/tests/scene.rs、crates/test_utils/src/scene.rs 与 error.rs，以及各 Widget 已有构造/公开 API 测试。Gallery 中的语法也要编译，但不为 Gallery 新增一套自动化构造测试框架。

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

[R12] Bevy v0.20.0 BSN 与 Scene 说明

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_scene/src/lib.rs`

[R13] Widgetry Scene 错误处理

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/scene.rs`

[R18] Widgetry TextField 构造、只读和禁用处理

`https://github.com/slc90/bevy_widgetry/blob/main/crates/text_field/src/style.rs`

[R24] Widgetry Waveform renderer 与资源回收

`https://github.com/slc90/bevy_widgetry/blob/main/crates/waveform/src/renderer.rs`

[R29] Gallery 入口与 BRP runtime 安装

`https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs`

[R32] Widgetry 测试规则及 Gallery 例外

`https://github.com/slc90/bevy_widgetry/blob/main/rules/testing.md`

[R33] Widgetry facade 与平台边界

`https://github.com/slc90/bevy_widgetry/blob/main/crates/bevy_widgetry/src/lib.rs`

