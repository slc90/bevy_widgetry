# 建立可外置 HTTP 的 Extras 兼容层

## 目标与范围

让 Gallery 能继续使用 Extras 的方法和输入、截图实现，同时自行安装 HTTP transport。这个提交建立兼容基线，不接管监听端口，不删除 150ms 分支，也不处理多帧续帧。

项目原来精确依赖 `bevy_brp_extras = "=0.22.6"`。本方案选择 `=0.22.7` 作为兼容基线，只接受这一个有明确理由的 patch 升级：修复官方发布记录确认的键盘事件窗口目标问题。Bevy 保持 `=0.19.1`，MCP server 不属于本提交的升级对象。[S1] [S11]

## 兼容包的落点

使用 `vendor/bevy_brp_extras/` 保存基于已发布 `0.22.7` crate 的本地兼容包。root manifest 保持 Workspace 统一版本声明，并使用以下形式选择该包：

```toml
[workspace.dependencies]
bevy_brp_extras = "=0.22.7"

[patch.crates-io]
bevy_brp_extras = { path = "vendor/bevy_brp_extras" }
```

上面只展示新增或变更项，不是完整 manifest。兼容包不加入 Workspace member；需要时加入 `workspace.exclude`，避免被 root Workspace 意外接管。使用发布包中已经规范化的 manifest，不能直接搬上游 monorepo 中依赖其 root 的 `workspace = true` 字段，再让它错误地继承 Widgetry 的 Workspace 配置。[S12]

保存原有 license、必要源码、发布包的来源和校验信息。`vendor/bevy_brp_extras/PATCHES.md` 记录上游版本、来源、原始 crate checksum、每项本地语义差异及未来移除补丁的条件。不要带入上游 MCP server、整个 monorepo、构建产物或无关 assets。

本地包保留 `bevy_brp_extras` 原 package 名和 `0.22.7` 版本，补丁由 path source 与记录文件区分；不伪装成上游新的正式 release。锁文件必须实际指向 path package，不允许 manifest 写了 patch 而编译仍用 registry 包。还要确认依赖解析没有额外拉入第二套 Bevy 版本。

## 新增一个真正的 methods-only 装配入口

拟新增接口形状：

```rust
BrpExtrasPlugin::without_http_transport()
```

这是本次要实现的接口，不是上游现成 API。它应延续上游的配置类型设计，使用独立的 external-transport 配置状态，而不是给所有 builder 叠加一个可能与端口配置冲突的 bool。[S10]

该入口仍安装或复用原版 `RemotePlugin`、注册全部 Extras 方法，并保留 keyboard、mouse、screenshot、diagnostics 与 deferred shutdown 等原有工作系统。它不添加 `RemoteHttpPlugin`，不解析或宣称自己拥有 HTTP 端口，不创建 listener，也不替宿主设置 Winit 模式。

上游原有的默认入口、`with_port()` 和 `with_http_plugin()` 行为不变。不要通过伪造 plugin 名称、先安装一个无用的官方 server、占位端口、删除私有 system 或修改 Bevy plugin registry 来让 Extras 误以为 transport 已存在。

这一提交不新增通用 wake API。真实工作生命周期的接缝由方案 03 定义和实现，避免提前加入没有消费者的状态结构。

## 预期产出与提交边界

产出是一个有明确上游来源、可正常构建的本地 Extras 兼容依赖，以及可被 Gallery 使用的 methods-only 入口。root manifest / lockfile 和必要的来源说明随这一能力一起提交。

Gallery 当前仍使用原有默认 Extras 入口，因此除了有意采用的 `0.22.7` 键盘目标修复外，现有 HTTP 启动方式和更新配置保持不变。新入口有确定的下一阶段消费者，不是为未知未来预留的抽象。

`docs/architecture.md` 应说明 Gallery 使用隔离的外部兼容包；不把 vendor 画成 Widgetry 新的功能 crate，不改变控件库的依赖图。普通外部版本号变化本来不要求更新 architecture；这里更新的是兼容包归属这一实际事实。[S3] [S4]

## 验证意图与完成条件

关键 invariant 是：methods-only 装配有 `RemotePlugin` 和完整 Extras 方法集合，但没有官方 HTTP plugin 或 listener；原默认装配仍具有原来的 HTTP 行为。复用已安装 `RemotePlugin` 时不能重复注册或覆盖无关方法。

需要提供依赖解析、编译和装配行为的证据。`cargo tree -i bevy_brp_extras` 应指向本地兼容包；`cargo tree -d` 中涉及 Bevy 的结果需要检查，不能把无关重复依赖一概当成错误。

可在兼容包内用针对性的装配测试验证 plugin 存在性、方法集合和两种入口的差异。Gallery 展示页不需要补一整套单元测试。必要的项目验证包括格式、Workspace 编译及受影响目标的 Clippy；具体执行结果必须如实记录。[S6]

不能完成的情况包括：只能在上游 monorepo 根目录构建、没有原始来源信息、patch 没有被解析器选中、methods-only 入口仍开端口，或升级导致必须升级 Bevy 才能编译。遇到这些情况应停在当前兼容边界修正，不能偷偷扩大到引擎升级。

## 与前后方案的关系

本方案是唯一执行链的第 01 个提交，没有代码前置提交。它为方案 02 提供外置 HTTP 的合法接入点，也为方案 03 提供可维护的小范围 Extras 修改位置。

完成后只提交当前目标，不提前复制 HTTP server，不删除 `WIDGETRY_BRP` 分支。后继固定为 `02-wake-aware-http.md`。

## 所有提交都要遵守的边界

本任务只涉及 Gallery 的 BRP 接入和隔离的 Extras 兼容层。不修改 Tooltip、其他 Widget 的时间逻辑、Window 的 ownership / modal 语义或 renderer / present mode。不新增 Workspace 功能 crate，不把 HTTP、winit wake 或调试依赖加进控件库的生产依赖路径。Gallery 原有 logging guard、资产装配和窗口配置保持不变。[S2] [S3]

所有新接口名称都属于本方案拟新增的接口，不能当成上游已经存在的 API。内部命名可按项目规则调整；改变 transport 归属、激活条件、工作生命周期或端口范围则是设计变化，不能静默替换。

每个实现任务先读取 `AGENTS.md`、`docs/architecture.md`、`rules/task-scope.md`、`rules/development.md`、`rules/code.md`，并根据修改范围读取 architecture、dependencies、documentation、testing、gui-debugging、logging 和 git 规则。不要主动读取 `requirements/`。当前小方案应作为本次明确提供的任务附件使用，而不是要求 Codex 去历史规划目录寻找依据。[S4] [S17] [S18]

Gallery 的测试例外继续保留，不把展示页面改造成强制 TDD 项目，也不为了放测试而新建一个生产 crate。本方案给出的是并发接缝的验证意图与可选的针对性自动化方式；实现者需提供足以证明 invariant 的证据。GUI 结果仍需按项目规则进行 BRP 验证，真实 OS move / resize / 跨应用 focus 等行为另由人工验证，不能冒充 BRP 已覆盖。[S5] [S6]

每个提交都要可编译、范围独立，不留下跨提交的半成品引用或无用途开关。外部依赖版本由 root `[workspace.dependencies]` 统一管理；只更新必需的 lockfile 项。Vendored 包作为外部兼容包保留自己的已解析 manifest，不冒充新的 Widgetry Workspace member。[S16]

代码相关改动完成必要验证后，按 `AGENTS.md` 使用全新的 reviewer subagent 调用 `$code-review`，只审查完整 working-tree change、不修改代码；有 findings 时由施工 agent 修复，再换一个全新的 reviewer，直到没有 findings。环境缺少该能力时必须报告缺失，不能声明 Review 已通过。[S4]

提交信息由最终实际 diff 产生：中文 subject、不加 `feat:` / `fix:` 或阶段前缀，其他格式遵守 `rules/git.md`。本文只定义提交边界，不预写可直接复制的 commit message。不把计划当成已经实现的变更，不自动 push，也不混入用户原有的无关修改。[S15]

验证记录必须区分：已执行且通过、已执行但失败、未执行。当前文档没有预先宣称任何编译、性能指标或 GUI 场景通过。

[S1]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/Cargo.toml "项目依赖基线"
[S2]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/gallery/src/main.rs "Gallery 实际装配与 150ms 分支"
[S3]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/docs/architecture.md "Gallery 与控件库的依赖边界"
[S4]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/AGENTS.md "开发上下文与独立 Review 规则"
[S5]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/gui-debugging.md "BRP 调试约定与 OS 级验证边界"
[S6]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/testing.md "Gallery 测试例外"
[S10]: https://github.com/natepiano/bevy_brp/blob/v0.22.7/extras/src/plugin.rs "Extras 装配与具体 RemoteHttpPlugin 配置类型"
[S11]: https://github.com/natepiano/bevy_brp/releases/tag/v0.22.7 "2026-09-23 发布记录及键盘事件目标修复"
[S12]: https://github.com/natepiano/bevy_brp/blob/v0.22.7/extras/Cargo.toml "Extras 兼容基线 manifest"
[S15]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/git.md "提交信息规则"
[S16]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/dependencies.md "依赖管理规则"
[S17]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/task-scope.md "目标边界与禁止顺手修改"
[S18]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/development.md "开发流程与 Gallery 例外"
