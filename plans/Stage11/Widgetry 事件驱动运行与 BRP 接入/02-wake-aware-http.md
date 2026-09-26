# 接入请求入队后唤醒 App 的 HTTP transport

## 目标与范围

把 BRP 从“请求进入 mailbox 后等待碰巧发生的 update”改为“请求成功入队后主动唤醒 App”。覆盖 Main World 和原生默认启用的 Render World transport，不修改原版 `RemotePlugin` 的方法执行逻辑。

前置条件是方案 01 已提交：本地 Extras `0.22.7` 兼容包提供 methods-only 入口，Bevy 仍为 `0.19.1`。本提交接管 HTTP，不负责判断 typing、drag 等完整操作何时结束；那是方案 03 的目标。

## 代码与依赖归属

Gallery 内新增 `brp` module，建议入口为 `gallery/src/brp.rs`，HTTP 实现放在 `gallery/src/brp/http.rs`，具体拆文件遵守项目 module 规则。拟新增的 `GalleryBrpPlugin` 装配 methods-only Extras 和 `GalleryRemoteHttpPlugin`。

`GalleryRemoteHttpPlugin` 是 Gallery 内部类型，不从 `bevy_widgetry` facade 导出，不加入 `crates/core`。只有 Gallery 和 vendor 兼容包涉及 BRP 依赖。这里也不建设通用 `AppWaker`。

HTTP 实现以 Bevy `v0.19.1/crates/bevy_remote/src/http.rs` 为固定来源，保留必要的 license 与来源说明。使用上游同一系列的 `async-channel`、`async-io`、`hyper`、`http-body-util`、`smol-hyper`、`serde_json` 等依赖；新增直接依赖集中到 root Workspace，优先沿用 lockfile 已解析的兼容版本，不引入另一个 Tokio runtime 或第二套 HTTP 框架。需要直接使用 `bevy_remote` 类型时，将它作为 `=0.19.1` 的显式 Workspace 依赖，不依靠偶然的 feature re-export。[S7] [S9] [S16]

不复制 `RemotePlugin`、built-in methods、反射处理或 watcher 执行器。HTTP headers 等字段在上游有私有封装，不能假定可以直接取其内部 map；需要的 transport 内部配置由 Gallery 自己持有，默认行为与原配置一致。

## 请求提交与唤醒的契约

正常路径是：

```text
HTTP 请求解析为 BrpRequest
  → 构造原版 BrpMessage
  → 成功提交到对应 World 的 BrpSender
  → EventLoopProxy.send_event(WinitUserEvent::WakeUp)
  → 原版 RemoteLast 处理
  → 原结果通道返回 BrpResult
  → 序列化为 HTTP / SSE 响应
```

Wake 必须发生在本条消息成功入队之后。不能只在 TCP accept、HTTP headers 到达或解析开始时唤醒，否则 update 可能跑完了，消息还没进入 mailbox。

从 `EventLoopProxyWrapper` 的 Deref 目标取得底层 proxy 的 clone，交给 I/O task 使用；不访问私有 `.0`，也不假定 wrapper 本身实现了 `Clone`。网络任务只接触 channel、配置和 proxy，不调用 `App::update()`，不持有或访问 ECS World。[S8] [S14]

有界 mailbox 不能只写一个未经分析的 `send().await`。建议把以下行为封装成 transport 私有的提交函数：

| channel 状态    | 行为                                                                |
| --------------- | ------------------------------------------------------------------- |
| `try_send` 成功 | 立即发送 WakeUp                                                     |
| 已满            | 先唤醒以处理队列中已有工作，再等待本条消息成功入队，然后再次 WakeUp |
| 已关闭          | 结束请求并报告 transport / App 不可用，不永远等待结果               |

队列已满时的前一次 wake 针对的是已经存在的消息，不替代本条消息入队后的 wake。不要加 sleep 重试、后台 `try_recv` 轮询或无界中转队列，也不要为了唤醒而从官方 receiver 抢走请求。

每条消息都保留入队后的唤醒，不做容易丢失通知的“已经 wake 过”优化。winit 自身可能合并事件；正确性不能依赖一条消息严格对应一次 update。

还要处理一个固定版本的实际边界：`process_remote_requests` 在找不到方法时可能提前返回，留下后续消息。可在 `RemoteLast` 的 `RemoteSystems::Cleanup` 之后检查本 World 的 mailbox 是否仍非空，仅在有残留时再请求一次 update。Main 和 Render 两边都要覆盖。这是在已经运行的 App 内检查剩余工作，不是引入后台 polling，也不改变原版方法执行器。[S8]

## 两个 World 与启动时机

主 transport 在 `Startup` 获取已经由 `PreStartup` 建立的 `BrpSender`。不要在 plugin build 阶段读取尚未建立的 mailbox。[S8]

Render transport 保留独立的 sender、端口和结果通道。可以把其一次性启动系统放在 `Render` schedule，使它在 `RenderStartup` 及其 deferred commands 已完成后取得 render mailbox；不要引用上游私有的 `setup_mailbox_channel` 来强行建立顺序。若本地 feature / schedule 验证表明启动顺序与预期不同，必须在该接缝修正，而不是去掉 Render 端口。

在 DefaultPlugins 已建立 RenderApp 的装配边界向它提供 clone 出来的主 event-loop proxy。无论请求进入哪个 World，唤醒的都是同一个主 event loop；绝不能从 HTTP task 单独运行 RenderApp。

保留原有的地址与端口契约：默认绑定 `127.0.0.1`；Main 默认 15702，合法 `BRP_EXTRAS_PORT` 优先；Render 默认 15703。Render 端口不自动改成 Main 端口加一，这不是原有规则。无效 Main 端口环境值沿用原 Extras 的 fallback 行为，并可记录一次诊断。端口相同、被占用或绑定失败应明确失败，不静默换端口、不留下“只有一半启动成功却报告整体就绪”的状态。[S7] [S10]

不因为创建了 RenderApp 就假定 remote render feature 必然可用。用本地 feature tree 确认实际组合，只有该能力存在时装配其 transport；本项目原生默认组合原本具有该能力，不能默认删掉。[S9]

## HTTP 与 BRP 兼容范围

保留官方请求对象、response id、结果与错误格式、单请求、batch、watch SSE、默认 headers 和 port 配置语义。解析继续使用原版 `BrpRequest` / `BrpBatch` / `BrpResponse`，不顺便实现一套“更标准”的 JSON-RPC 方言，也不修正与本任务无关的 upstream 行为。[S7]

必须分清两个概念：HTTP 是否返回 SSE，由上游 transport 的 `+watch` 请求约定决定；ECS 方法使用 `Instant` 还是 `Watching` 则由 `RemoteMethods` 决定。`brp_extras/screenshot` 是会延后完成的 Watching 方法，但名字没有 `+watch`，其普通 HTTP 请求仍等待最终一次结果。不能看到 Watching 就把截图改成 SSE。[S7] [S8] [S10]

Batch 按原行为逐条提交并汇总响应，每一条真正提交的消息都能唤醒 App。Batch 中不支持 streaming 的边界保持一致。至少覆盖未知方法之后仍有有效请求、解析错误、参数错误和并发请求，不把某个错误变成整个 server 的退出条件。

## 失败、取消与任务所有权

新 transport 不再照搬上游示例式的“detach 后忽略所有错误”。Server / connection task 由明确的 runtime owner 管理，日志能区分绑定失败、channel 关闭、event loop 已结束、请求超时及普通客户端断开。

普通等待结果的请求增加可配置的 transport deadline，默认 30 秒；这是本方案有意新增的失败边界，不宣称与上游无限等待行为完全相同。SSE 长连接不使用这个总连接寿命上限。超时返回带原 id 的可诊断错误，不能伪造成功。这个限制不改变 `send_keys` 等方法返回“操作已开始”的原语义。

连接取消或 timeout 应释放结果 receiver，让原版 watcher cleanup 有机会清理订阅；必要时发送一次 wake 促使 cleanup 运行。正常操作已经进入 World 后，HTTP 取消不等于撤销业务副作用，也不允许自动重试该 mutation。

App 退出时停止接受连接、取消不再需要的 I/O task，并释放 listener 和结果通道。不能在 World 线程同步 join 一个还等待 World 返回结果的任务。Event loop 关闭后的 wake 错误属于可识别的 shutdown 路径；运行中意外失败则要报告，不能一律 `let _ = ...` 吞掉。

绑定失败需要通过明确的启动失败 / AppExit 路径结束 BRP 启动，关闭已经启动的另一端口。不能只在被 detach 的 task 中打印一条日志，然后让 Gallery 表现为正常启用了 BRP。

## 预期产出与验证意图

产出是已经替换 Gallery 原 HTTP 路径、能在静止 App 中交付请求的双 World transport。该提交仍保留原有 `WIDGETRY_BRP` / 150ms 配置分支；彻底去掉 workaround 要等完整操作的推进能力完成。

验证应优先证明因果，而不只测一个看起来快的响应时间：记录测试 wake 回调触发时 mailbox 中已经有对应消息；用容量很小的 channel 覆盖满队列；证明关闭与取消不会留住等待者；证明一次 wake 被合并后，剩余请求仍能继续处理。

可以用只在收到测试 wake 后才推进的最小 App 验证队列接缝，不能使用后台定时 `app.update()` 把问题掩盖。该测试不替代最终真实 winit 验收。协议回归关注我们替换 transport 后的组合行为，不需要重测全部 Bevy 反射内部实现。[S6]

本提交至少应有编译、依赖和协议接缝证据；真实 Gallery 的全面输入 / 截图 / 空闲验证留给方案 04，不能把仍有 150ms 兜底时的表现当作最终通过。

## 与前后方案的关系

这是第 02 个提交，必须建立在方案 01 上。它消费 methods-only 入口，为方案 03 提供明确的请求、结果、取消和退出生命周期边界。

完成后停止，不在本提交提前实现按参数估算帧数的“临时”续帧，不删除 polling 分支。后继固定为 `03-in-flight-progress.md`。

## 所有提交都要遵守的边界

本任务只涉及 Gallery 的 BRP 接入和隔离的 Extras 兼容层。不修改 Tooltip、其他 Widget 的时间逻辑、Window 的 ownership / modal 语义或 renderer / present mode。不新增 Workspace 功能 crate，不把 HTTP、winit wake 或调试依赖加进控件库的生产依赖路径。Gallery 原有 logging guard、资产装配和窗口配置保持不变。[S2] [S3]

所有新接口名称都属于本方案拟新增的接口，不能当成上游已经存在的 API。内部命名可按项目规则调整；改变 transport 归属、激活条件、工作生命周期或端口范围则是设计变化，不能静默替换。

每个实现任务先读取 `AGENTS.md`、`docs/architecture.md`、`rules/task-scope.md`、`rules/development.md`、`rules/code.md`，并根据修改范围读取 architecture、dependencies、documentation、testing、gui-debugging、logging 和 git 规则。不要主动读取 `plans/`。当前小方案应作为本次明确提供的任务附件使用，而不是要求 Codex 去历史规划目录寻找依据。[S4] [S17] [S18]

Gallery 的测试例外继续保留，不把展示页面改造成强制 TDD 项目，也不为了放测试而新建一个生产 crate。本方案给出的是并发接缝的验证意图与可选的针对性自动化方式；实现者需提供足以证明 invariant 的证据。GUI 结果仍需按项目规则进行 BRP 验证，真实 OS move / resize / 跨应用 focus 等行为另由人工验证，不能冒充 BRP 已覆盖。[S5] [S6]

每个提交都要可编译、范围独立，不留下跨提交的半成品引用或无用途开关。外部依赖版本由 root `[workspace.dependencies]` 统一管理；只更新必需的 lockfile 项。Vendored 包作为外部兼容包保留自己的已解析 manifest，不冒充新的 Widgetry Workspace member。[S16]

代码相关改动完成必要验证后，按 `AGENTS.md` 使用全新的 reviewer subagent 调用 `$code-review`，只审查完整 working-tree change、不修改代码；有 findings 时由施工 agent 修复，再换一个全新的 reviewer，直到没有 findings。环境缺少该能力时必须报告缺失，不能声明 Review 已通过。[S4]

提交信息由最终实际 diff 产生：中文 subject、不加 `feat:` / `fix:` 或阶段前缀，其他格式遵守 `rules/git.md`。本文只定义提交边界，不预写可直接复制的 commit message。不把计划当成已经实现的变更，不自动 push，也不混入用户原有的无关修改。[S15]

验证记录必须区分：已执行且通过、已执行但失败、未执行。当前文档没有预先宣称任何编译、性能指标或 GUI 场景通过。

[S2]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/gallery/src/main.rs "Gallery 实际装配与 150ms 分支"
[S3]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/docs/architecture.md "Gallery 与控件库的依赖边界"
[S4]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/AGENTS.md "开发上下文与独立 Review 规则"
[S5]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/gui-debugging.md "BRP 调试约定与 OS 级验证边界"
[S6]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/testing.md "Gallery 测试例外"
[S7]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_remote/src/http.rs "固定版本 HTTP transport 基线"
[S8]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_remote/src/lib.rs "RemotePlugin、mailbox、RemoteLast 与 watcher 处理"
[S9]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_remote/Cargo.toml "Remote 默认 features 与 transport 依赖"
[S10]: https://github.com/natepiano/bevy_brp/blob/v0.22.7/extras/src/plugin.rs "Extras 装配与具体 RemoteHttpPlugin 配置类型"
[S14]: https://bevy.org/examples/window/low-power/ "官方事件驱动与外部唤醒示例；API 仍以固定版本源码为准"
[S15]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/git.md "提交信息规则"
[S16]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/dependencies.md "依赖管理规则"
[S17]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/task-scope.md "目标边界与禁止顺手修改"
[S18]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/development.md "开发流程与 Gallery 例外"
