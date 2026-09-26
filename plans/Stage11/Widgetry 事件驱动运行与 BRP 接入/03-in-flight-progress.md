# 让 BRP 多帧操作完成后恢复空闲

## 目标与范围

让截图、按键保持与释放、逐字输入、拖动、双击和延迟 shutdown 在没有后续真实鼠标或键盘事件时也能完成。完成或清理后，BRP 不再继续请求 update。

前置条件是方案 02 已提交，Gallery 使用自己的双 World HTTP transport，Extras 使用 `0.22.7` 本地兼容包。这个提交只解决 BRP 的真实工作生命周期，不修改 Tooltip、控件 clock、Bevy Time 资源或 WinitSettings。

## 用真实工作状态，不猜持续时间

在 Extras 兼容包增加一个小的、只服务于 BRP 集成的 `BrpExtrasActivity` Resource。对宿主提供只读的活跃状态和一个可设置的状态变化通知回调；操作侧通过 crate 内部的 RAII guard 持有工作责任。

接口名称是拟新增设计。它不是上游现成能力，也不是通用 AppWaker。默认没有宿主 callback 时，原 Extras 的使用方式仍正常；Extras 自身不依赖 `bevy_winit`，回调由 Gallery 捕获 event-loop proxy 后提供。

RAII guard 必须跟随实际异步操作，而不是 HTTP handler 的栈帧。创建 deferred 操作时就取得 guard，随后由真实 Component、Resource 或 async task 持有；正常完成、错误清理和取消销毁都释放它。工作计数从 0 变为非 0，或最后一项工作完成时，通知宿主重新检查状态。

| 操作类别            | 活跃责任结束的位置                                                 |
| ------------------- | ------------------------------------------------------------------ |
| 按键 / 鼠标按住     | release 已产生，相关 pending operation 已结束                      |
| 逐字输入            | 输入队列排空，最后的 release 已产生                                |
| drag / double-click | 最终位置、点击或 release 已完成，内部 operation 已清理             |
| screenshot          | capture、readback、编码与文件发布成功，或错误 / timeout 已完成清理 |
| deferred shutdown   | 原有延迟关闭流程到达 AppExit，或流程被明确取消                     |

只给原有状态机增加生命周期接缝，不重写按键映射、拖动插值、截图选择 / 裁剪和窗口行为。不使用 `duration_ms + 某个余量`、字符串长度乘帧数、扫描私有类型名或“每次固定跑十帧”来猜真实完成时间。

截图 guard 必须能转移到保存文件的异步任务；文件写入结束的 callback 也能唤醒主循环完成结果回传。异常的 capture / readback 等等待需要可终结的 timeout / 取消路径，默认与 30 秒请求等待边界一致。清理必须结束真实 pending 状态并产生失败结果，不能只把 busy 计数清零，却留下不会释放的操作。

HTTP 客户端断开不自动取消已经发生的输入副作用。尤其不能在 key press 之后因为断开而停止续帧，让 release 永远不发生。真实操作由自己的 guard 持续到完成或有效清理；App 退出则统一释放 runtime 及其所有工作。

## transport 等待与长期 watch 要分开

普通 HTTP 请求从成功提交直到第一个 `BrpResult` 到达之间，可以持有独立的待结果 guard。收到结果就释放，不等客户端慢慢读取完整 HTTP body；timeout、连接取消和 channel 关闭同样释放。

这解决普通结果等待和截图首次结果所需的推进，但不能替代 Extras 的操作 guard。`send_keys` / `type_text` 等可能已经返回成功，实际操作仍未结束；它们继续由 Extras 活跃状态负责。

长期 `+watch` 订阅不因为 SSE 连接仍打开而持有持续刷新责任。注册请求获得一次进入 App 的 wake；之后由真实窗口事件、BRP mutation 或其他正常工作产生 update。没有变化时允许安静地保持连接。不能为了生成 watch 数据每帧自我唤醒，也不保证暂停不动的 App 会不断产生新 watch 值。

HTTP 分类继续使用方案 02 的规则。`brp_extras/screenshot` 虽然在 RemoteMethods 中是 Watching，仍属于等待一次最终结果的普通 HTTP 请求，不能误归类为“保持连接即可、不需要推进”的长期 watch。[S7] [S8] [S10]

## 一个 Gallery 内部的按需续帧控制器

控制器只有以下工作来源：Main / Render mailbox 还有未处理消息、普通请求还在等待 World 的结果、Extras 还有真实操作、或有限的提交收尾帧。没有这些来源时，停止一切 BRP 续帧。

Main World 的判断点放在 `RemoteLast` 的 `RemoteSystems::Cleanup` 之后，确保能看到本帧刚创建的操作。Render World 在自己的对应阶段报告残余工作；跨 World / I/O thread 只共享线程安全状态和通知，不跨线程读取 World。[S8]

可见窗口的正常推进使用 `RequestRedraw`。为避免不可见或被遮挡窗口没有及时交付 redraw 而让非视觉 BRP 工作停住，增加一个仅在明确存在工作的情况下预约的单次 wake fallback：

```text
本帧末确认仍有工作
  → 请求 redraw，并预约一次约 16.7ms 后的 WakeUp
  → 自然 update 提前到来：确认新一帧，替换或取消旧预约
  → deadline 到达仍没有新 update：只发一次 WakeUp
  → 没有收到后续 App update 确认：不自行反复重发
```

约 16.7ms 是续帧 fallback 的初始设计值，不是对原生窗口输入的限帧，也不是 BRP 请求首次唤醒的延迟。请求成功入队仍立即 wake；native window event 仍走原来的事件路径。

这个单次预约不是把 150ms polling 搬到另一个线程：空闲时没有预约；一次 wake 发出后没有 App 确认就不再周期重发；只有上一个真正执行过的 App update 明确报告还有工作，才可预约后续推进。OS suspend 不应造成后台空转。

状态通知需要按最新 generation 读取，不能让旧的 busy 通知覆盖较新的 idle。每个 runtime 最多保留一个有效 fallback 预约，关闭时取消。可以复用 transport 已有的 async I/O executor 与 timer，不新建独立 runtime；不在 ECS system 中 sleep。

普通请求执行于 RemoteLast，输入、layout 或 render 的消费者可能位于下一轮更早的 schedule。因此允许每次已交付请求 / 最后一项工作结束后保留最多两个完整后续 App update 作为收尾机会，然后停止。收尾 budget 的合并取剩余需求的上界，不按重复通知无限累加；持续的新请求则属于新的真实工作。

这两帧不是任何异步操作“必然完成”的保证。跨帧操作由真实 guard 负责；如果某个已知操作需要更多阶段，补其真实工作生命周期，不能把两个改成任意很大的数字遮掩问题。

## 并发、取消和退出 invariant

多个客户端、多个窗口和两个 World 的工作状态按“还有任何工作”组合，不能被某个先完成的请求清空。记录每项工作的所有权，避免一个 bool 被不同请求互相覆盖。

工作计数更新与通知必须支持：先完成后处理通知、取消与结果同时到达、主线程刚准备等待时网络请求到达，以及 Render World 在主 World 判断之后才创建后续工作。必要时直接再 wake 一次；宁可有一次有界的冗余 update，也不能丢唤醒。

持有共享锁时不 await，不等待 World，不持锁向可能重入的 callback 调用。I/O task 结束、AppExit、连接取消和正常响应不能使计数下溢、重复释放或保留永远活跃的引用环。

持续 BRP 查询本身会不断造成 update，这是实际工作，不属于 idle regression。慢客户端仅仅保持连接或读取旧数据，不能单独让 App 永久刷新。watch、HTTP keep-alive 与真实工作之间必须有清楚区别。

不修改 `Time<Real>`、`Time<Virtual>`、max delta 或时间缩放。原操作使用哪种时间语义就保持哪种；本方案不把外部主动暂停时间或 OS 挂起时的行为包装成“无论如何都按墙钟完成”。

## 预期产出与验证意图

产出是 Extras 的可维护活动状态补丁、transport 的有界待结果生命周期，以及 Gallery 的按需续帧控制器。补丁差异记入 `PATCHES.md`；描述实际接入职责的 architecture 文档同步更新，不把这些能力导出为 Widgetry 控件 API。

关键场景是“只发起一次操作，之后不再发送任何输入或 BRP 查询来帮它跑帧”。逐字输入需要最终 release，drag 需要到达终点，截图需要完整文件，shutdown 需要完成退出。不要每隔几十毫秒用 query 验证一次，否则验证本身就可能把缺失的续帧补上。

针对性验证应能证明：工作开始会推进；最后一项工作结束后只剩有界收尾；计数正确支持并发；普通等待 timeout / 取消会释放；长期 watch 静止时不持续刷新；隐藏窗口 fallback 不依赖 redraw 交付；OS 不再确认 update 时不会产生无穷重试。

可以给续帧控制器注入可控 timer / 通知回调，确定性验证 race 和 generation，而不是靠 sleep 调整到测试“恰好通过”。还需验证 guard 跟随真实操作结束，而不是只验证计数器的加减。

Gallery 此时仍可能保留原 150ms 分支，因此这些接缝验证必须有不启用该兜底的独立测试装配。最终真实 Gallery 无 polling 验收只在方案 04 完成；不能借当前中间状态宣称目标已经全部达成。

## 与前后方案的关系

这是第 03 个提交，必须在方案 02 之后。它消费 transport 的生命周期接缝，并补齐方案 04 删除 polling 的必要前提。

完成后不提前扩大到通用 UI 动画或异步任务框架。后继固定为 `04-event-driven-cutover.md`。

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
[S10]: https://github.com/natepiano/bevy_brp/blob/v0.22.7/extras/src/plugin.rs "Extras 装配与具体 RemoteHttpPlugin 配置类型"
[S15]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/git.md "提交信息规则"
[S16]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/dependencies.md "依赖管理规则"
[S17]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/task-scope.md "目标边界与禁止顺手修改"
[S18]: https://github.com/slc90/bevy_widgetry/blob/dfd86eab7bf39939a783808406c88be581fb72ae/rules/development.md "开发流程与 Gallery 例外"
