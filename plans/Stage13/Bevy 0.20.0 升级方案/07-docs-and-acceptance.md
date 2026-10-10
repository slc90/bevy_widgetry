# 07 | 同步当前说明并完成全仓验收

[返回方案总览](00-overview.md)

## 目标

让当前使用说明、工具安装方式与已经迁移完成的代码一致，并确认整个仓库不存在遗漏目标。

## 范围

当前有效的项目说明、依赖/工具注释和使用示例，以及全工作区最终构建、测试、clippy 和 Gallery 验收。

## 预期产出

准确的 0.20 使用入口及通过最终验收的工作区；不追加历史资料改写或新迁移记录体系。

## 与前后方案的关系

在方案 06 的运行期验收完成后执行，这是唯一的最终收口方案。前面的编译和行为问题必须已经闭合，不能把它变成未完成工作的兜底清单。

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

## 只同步当前有效的版本说明

根 Cargo.toml 的 Bevy/bevy_remote/BRP 和 MCP metadata、工具安装注释应已在版本切换时同步；此处核对说明没有落后于代码。AGENTS.md、rules/project-context.md、docs/architecture.md 及实际描述构建/GUI 调试的当前文档，若写有旧版依赖、旧 API 示例或旧工具安装方式，应改成与本次结果一致的内容。[R1][R2]

当前使用示例按 0.20 的 Scene/observer/Pointer/文本输入接口更新，尤其是会被复制到新代码里的片段。window/input.rs 中解释旧 FocusedInput 构造限制的注释要依据新代码修订；若反射边界仍然存在，就写清现在的理由，而不是只把 0.19.1 数字替换为 0.20.0。[R20]

.codex/ 等当前工具配置只在实际固定旧标签、旧二进制路径或旧启动参数时处理。没有命中就不制造变更，不把 runtime 依赖更新等同于工具已经在本机更新。

历史 plans/Stage... 文件及其他明确记录过去设计的材料保留原文，不做全仓 0.19 → 0.20 字符串替换。版本搜索得到旧字符串时按用途判断，不把“仓库里没有任何 0.19 字样”设为验收标准。项目自己的包版本、历史计划、字体许可证与资源内容不属于本次升级改写范围。

## 最终验收覆盖构建、测试和实际桌面使用

最终仍从 Windows x86_64-pc-windows-msvc 验收，Rust 至少 1.98.1。公共编译出口必须全部通过，并补齐库/集成测试、doc tests 和 workspace clippy：

```powershell
cargo test --workspace --lib --tests --target x86_64-pc-windows-msvc --locked
cargo test --workspace --doc --target x86_64-pc-windows-msvc --locked
cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc --locked -- -D warnings
```

测试命令分开写是为了清楚区分库/集成测试、文档示例与 benchmark。benchmark 的构建已经由 cargo build --all-targets 覆盖，这里不把执行所有 benchmark 伪装成普通测试。clippy 出现与新编译器有关的告警时修正实际问题，不全局关闭原有禁用规则。[R1][R32]

最终判断包括：依赖图实际一致；22 个成员以及测试、benchmark 和 Gallery 构建通过；Widgetry 自有 contract 的测试没有通过删除/忽略被削弱；Scene/Pointer/文本/布局/窗口/Waveform 的相关回归闭合；Gallery 使用匹配 BRP 后能够完成既有交互。不存在“只剩某个测试工具还在旧版”或“暂时禁掉一个窗口功能”的收尾状态。

需要报告实际验证结果时，直接说明执行过的检查及未执行的部分即可，不为本次升级新增当前基线、历史对照表或长期维护的迁移记录文件。文档可以确认修改范围和验收要求，但只有真正执行过的 Windows 构建与运行才可以写为通过。

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

[R20] Widgetry 多窗口键盘、IME 与焦点路由

`https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/input.rs`

[R32] Widgetry 测试规则及 Gallery 例外

`https://github.com/slc90/bevy_widgetry/blob/main/rules/testing.md`

