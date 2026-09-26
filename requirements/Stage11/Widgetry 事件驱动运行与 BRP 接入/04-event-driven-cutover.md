# 统一事件驱动运行并完成双窗口验收

## 目标与范围

让 Gallery 的实际入口正式采用最终运行策略，并通过没有 polling 兜底的主窗口 / 独立窗口验证。这个提交是整条链的交付点，不再保留可偷偷恢复 150ms 的内部 fallback。

前置条件是方案 01—03 都已提交：Extras 可以使用外置 transport；两个 World 的请求能主动唤醒；多帧操作由真实工作生命周期推进并能回到空闲。

## 最终入口与开关语义

`gallery/src/main.rs` 无条件插入：

```rust
WinitSettings::desktop_app()
```

删除 `WIDGETRY_BRP` 对 `focused_mode` / `unfocused_mode` 的分支，删除只服务于该分支的 `UpdateMode` / `Duration` import。不要把同样的分支移动到新 plugin，不新增“BRP 模式下使用 16ms Reactive / Continuous”的配置。

`WIDGETRY_BRP` 保留，但仅表示启用 Gallery 的 BRP 调试 runtime。为避免无关地改变原来的判定方式，仍按 `std::env::var_os("WIDGETRY_BRP").is_some()` 判断；因此值为 `0` 或空字符串但变量存在时也算启用，正常使用写成 `WIDGETRY_BRP=1`。关闭方式是移除变量，不是把它设为 0。

开启时装配 `GalleryBrpPlugin`，由其负责 methods-only Extras、自定义 HTTP、工作生命周期和续帧控制器。未开启时不安装 Extras、RemotePlugin、两个 HTTP listener 或 BRP driver，不创建它们的 timer / task / callback。

这是相对当前代码的一个明确行为变化：原代码即使没有设置变量也无条件加载 Extras，新行为下普通人工运行不暴露 BRP 端口。这个改变要写进使用说明，不能伪称原来已经如此。[S2]

除了 BRP 装配，不改变 DefaultPlugins、Gallery logging guard、transparent renderer、WindowPlugin、Widget plugin 顺序和原有界面内容。任何必须改变的顺序都应能直接解释为 BRP 初始化依赖，不顺手整理整个 main。

## 同步真实开发约定

`rules/gui-debugging.md` 当前要求设置变量以获得“低频 App polling”。本任务明确替换这一旧机制，所以该段必须在同一提交中改为“启用 BRP 调试能力，App 仍保持 desktop_app，请求与真实在途操作按需唤醒”。保留禁止全局高频 Reactive / Continuous 的约束，不借机放宽其他 GUI 调试规则。[S5]

MCP 标准启动流程仍使用 `brp_launch`，在 `env` 中设置 `WIDGETRY_BRP=1`。Main 端口配置仍采用 `BRP_EXTRAS_PORT` 的原优先级；Render 端口仍按独立端口管理。

在 `docs/` 中增加适量的 BRP 接入说明，建议为 `docs/brp-runtime.md`：描述开关、端口、为何有 Extras 兼容包、请求 wake 与活动续帧的区别、30 秒普通请求等待边界、watch 空闲行为、排错证据和上游升级检查点。它描述已实现事实，不复制整份规划到开发上下文。`docs/architecture.md` 同步 Gallery 的实际 BRP 装配职责。[S3] [S4]

不修改 `requirements/`，不把每个 Widget 的 rustdoc 都加上 BRP 说明，不新增与本目标无关的 observability 框架。

## 验收不能自己把 App 唤醒

测试某个多帧动作时，只发送发起该动作所必需的请求，随后不要持续发送 screenshot、query、diagnostics 或模拟输入来推进 App。最终观察可以在动作应当已经完成之后进行，并结合在此之前已写入的完成日志或状态记录，证明不是这次观察才使它完成。

对会立即返回“已开始”的方法，HTTP 成功不代表动作完成。需要观察最终输入状态、队列清理、最终位置或在操作完成点记录的事件。对 screenshot，要验证成功返回前完整文件已经发布；文件存在但仍未写完不算完成。

对 idle 的观察不使用高频 BRP 查询本身作为采样器。可使用低开销的内部诊断记录或单次结束后读取的计数快照。记录来源与采样范围，避免把真实鼠标移动、MCP 自动查询或运行中的 watch consumer 误当成 idle。

## 必须覆盖的运行时矩阵

| 场景 | 观察目标 |
| --- | --- |
| 未设置 WIDGETRY_BRP | 无 BRP listener / driver；原有主窗口和独立窗口可人工交互 |
| 已启用 BRP，但无人请求 | 空闲更新策略与普通运行一致，没有新的 150ms 或高频周期刷新 |
| 主窗口静止时单次查询 | 不移动鼠标也得到结果；日志可关联 enqueue、wake 与后续处理 |
| 主窗口与独立窗口切换 focus | 任一窗口的正常输入能继续处理；不只依据 PrimaryWindow 的 focus 判断更新需求 |
| 两个窗口都失焦但 App 未被 OS suspend | Main / Render 普通请求仍能通过 user event 得到处理；不依赖鼠标偶然唤醒 |
| MessageBox 打开、点击、关闭、反复创建 | 窗口呈现、modal 阻挡与关闭后的父窗口恢复都保持原语义 |
| 按键保持 / 逐字输入 / drag / double-click | 发起后不追加帮助性查询也会完成；最后的 release 不遗失 |
| screenshot 与普通查询交错 | 截图不阻塞整个 transport；完整文件、结果和清理状态一致 |
| 长期 watch 后无变化 | 连接可保持，但不永久续帧；之后一次真实 mutation 仍能产生相应数据 |
| Main 与 Render 并发请求 | 各自使用正确 mailbox；一侧结束不会清空另一侧的工作责任 |
| 未知方法、满队列、超时、客户端取消 | 后续请求仍可服务；等待者、订阅和活动责任能清理 |
| 正常 shutdown、异常断开、端口占用 | 无悬挂 server / driver；失败原因明确；无“部分启动但整体就绪” |

主窗口隐藏、最小化或完全被遮挡需要单独验证。非视觉请求的进展应由 WakeUp / 按需 fallback 保证到 App 的可运行边界；截图是否能得到有效窗口图像还取决于 OS 是否继续提供可呈现 surface。不能承诺所有平台在最小化后仍得到正常截图。失败应走明确结果或有界 timeout，而不是永久挂住。

真实 OS move / resize、跨应用 focus 和系统对话框无法全部通过 BRP 忠实模拟，按项目规则保留人工验证项。缺少这项证据时明确标记未验证，不静默改用 Win32 / PowerShell GUI 自动化并宣称等价。[S5]

## 如何判定“不卡”和“确实去掉 polling”

必须同时看配置、调度证据和可观察行为，不能只看一张截图。

在固定机器、相同 build / renderer 下比较启用与关闭 BRP 的操作后静止状态。记录 App update 来源、请求 enqueue 到实际处理的时间、最后一项工作完成到 driver 停止的时间。对多个静止后请求记录 P50 / P95 或原始样本，不预先编造性能数据，也不把单次低于 150ms 当成机制已正确的证明。

`desktop_app()` 本身并不表示永远没有 timeout wake，Bevy 或原有控件也可能请求 redraw。验收标准是没有新引入的 BRP idle 周期，不能要求所有平台的空闲 update 数绝对等于零。[S13] [S14]

日志应能说明“收到真实工作才唤醒”和“工作结束后停止”，但不打印完整请求正文、键入文本或截图内容，不逐帧输出大量 info 日志。复用 Gallery 当前日志设施。

如果独立窗口仍然卡住，要分辨 App 根本没有 update、update 已开始但耗时很长、还是 render / present 阻塞。后两类不能靠调短 wake 间隔掩盖，也不能仅因 BRP 请求返回就宣布弹窗问题解决。保留复现与 trace；若确属独立的 renderer / OS 问题，本阶段的相应窗口验收仍标记未通过，不把它伪装成已修复，也不在这个提交顺手重构渲染后端。

## 构建、Review 与预期产出

使用项目已有命令验证实际影响范围。最终集成至少考虑：

```text
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
cargo build --workspace
```

Vendored 包自己的新增接缝验证需另外覆盖，Workspace 命令不自动等于执行了该外部包的所有测试。Windows 执行环境使用 `pwsh`，不使用 Windows PowerShell 5。保留原有 Gallery 测试例外，不为展示页面无差别补测试。[S4] [S6]

执行已有 BRP 工具进行真实 Gallery 验证。缺少 MCP、桌面 session、GPU / DX12 条件或独立 reviewer 时，记录准确的缺失项和已完成的验证；不能宣称这些检查通过。

预期产出是最终 main 装配、移除的 polling 分支、同步的规则与实际运行说明，以及足以复核本次变更的验证记录。文档中的结果只记录真实执行证据；不要把当前计划里的验收表原样改成“全部通过”。

完成条件是：静止请求能进入 App，多帧操作能自主完成，结束后能停，独立窗口在已声明支持的环境下通过交互验收，两个端口与失败路径没有回归。任何缺失都应明确标记，而不是启用隐藏 polling 作为临时通过手段。

## 与前后方案的关系

这是第 04 个、也是最后一个提交，必须在方案 03 之后。它消费前三个提交的全部能力，正式改变 Gallery 的启动与运行策略，并完成最终交付验证。

当前提交结束后停止，不自动追加通用调度平台、其他外部事件源支持或上游提案。回退只能是明确回退相关代码提交，不能保留运行时自动恢复 150ms polling 的隐藏路径。

## 所有提交都要遵守的边界

本任务只涉及 Gallery 的 BRP 接入和隔离的 Extras 兼容层。不修改 Tooltip、其他 Widget 的时间逻辑、Window 的 ownership / modal 语义或 renderer / present mode。不新增 Workspace 功能 crate，不把 HTTP、winit wake 或调试依赖加进控件库的生产依赖路径。Gallery 原有 logging guard、资产装配和窗口配置保持不变。[S2] [S3]

所有新接口名称都属于本方案拟新增的接口，不能当成上游已经存在的 API。内部命名可按项目规则调整；改变 transport 归属、激活条件、工作生命周期或端口范围则是设计变化，不能静默替换。

每个实现任务先读取 `AGENTS.md`、`docs/architecture.md`、`rules/task-scope.md`、`rules/development.md`、`rules/code.md`，并根据修改范围读取 architecture、dependencies、documentation、testing、gui-debugging、logging 和 git 规则。不要主动读取 `requirements/`。当前小方案应作为本次明确提供的任务附件使用，而不是要求 Codex 去历史规划目录寻找依据。[S4] [S17] [S18]

Gallery 的测试例外继续保留，不把展示页面改造成强制 TDD 项目，也不为了放测试而新建一个生产 crate。本方案给出的是并发接缝的验证意图与可选的针对性自动化方式；实现者需提供足以证明 invariant 的证据。GUI 结果仍需按项目规则进行 BRP 验证，真实 OS move / resize / 跨应用 focus 等行为另由人工验证，不能冒充 BRP 已覆盖。[S5] [S6]

每个提交都要可编译、范围独立，不留下跨提交的半成品引用或无用途开关。外部依赖版本由 root `[workspace.dependencies]` 统一管理；只更新必需的 lockfile 项。Vendored 包作为外部兼容包保留自己的已解析 manifest，不冒充新的 Widgetry Workspace member。[S16]

代码相关改动完成必要验证后，按 `AGENTS.md` 使用全新的 reviewer subagent 调用 `$code-review`，只审查完整 working-tree change、不修改代码；有 findings 时由施工 agent 修复，再换一个全新的 reviewer，直到没有 findings。环境缺少该能力时必须报告缺失，不能声明 Review 已通过。[S4]

提交信息由最终实际 diff 产生：中文 subject、不加 `feat:` / `fix:` 或阶段前缀，其他格式遵守 `rules/git.md`。本文只定义提交边界，不预写可直接复制的 commit message。不把计划当成已经实现的变更，不自动 push，也不混入用户原有的无关修改。[S15]

验证记录必须区分：已执行且通过、已执行但失败、未执行。当前文档没有预先宣称任何编译、性能指标或 GUI 场景通过。

## 最终边界与维护责任

本方案面向当前 Gallery 的原生桌面环境，Windows 是必须实际验收的平台。未测平台不自动获得兼容保证；WASM transport、通用 headless runner 与其他外部事件源不在本次实现范围。

保留三项有意的兼容变化：Extras 从 0.22.6 提升到 0.22.7；未设置 `WIDGETRY_BRP` 时不再无条件启动 BRP；普通等待结果的 HTTP 请求有默认 30 秒的失败边界。其余协议、方法、端口和控件行为按文中固定基线保持。不能在交付时把这些变化藏在“内部重构”里。

两处来源需要维护：Extras 本地兼容包、从 Bevy 0.19.1 提取的 HTTP transport。升级前比较方法 / HTTP framing / Main 与 Render 启动阶段 / Extras 操作生命周期的差异，并重新执行对应接缝与运行时验证，不直接覆盖本地文件。上游具备相同能力后再计划移除补丁，不在当前任务里自动发 PR、发布 crate 或扩展架构。

当前没有待用户选择的架构分叉。仍需要实施环境确认的是实际 lockfile / feature 解析、Windows winit 与 Render 行为、并发接缝的运行结果和 GUI 证据。这些是明确的验证条件，不是已经取得的结果。

## 资料说明

正文中的 S 编号链接指向本次读取的项目基线或上游资料。以固定版本源码为主要依据，官方 low-power 示例只用来说明外部 wake 的使用方向，不拿滚动更新的示例替代 0.19.1 API 验证。新增接口、deadline、活动状态与续帧策略是本方案的设计，不是对上游现有能力的陈述。

[S2]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/gallery/src/main.rs "Gallery 实际装配与 150ms 分支"
[S3]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/docs/architecture.md "Gallery 与控件库的依赖边界"
[S4]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/AGENTS.md "开发上下文与独立 Review 规则"
[S5]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/gui-debugging.md "BRP 调试约定与 OS 级验证边界"
[S6]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/testing.md "Gallery 测试例外"
[S13]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_winit/src/state.rs "Winit 事件更新判断与 RequestRedraw 处理"
[S14]: https://bevy.org/examples/window/low-power/ "官方事件驱动与外部唤醒示例；API 仍以固定版本源码为准"
[S15]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/git.md "提交信息规则"
[S16]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/dependencies.md "依赖管理规则"
[S17]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/task-scope.md "目标边界与禁止顺手修改"
[S18]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/development.md "开发流程与 Gallery 例外"
