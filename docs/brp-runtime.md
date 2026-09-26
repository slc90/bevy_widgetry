# Gallery BRP runtime

Gallery 默认安装 BRP runtime，为 Codex CLI 运行时验证提供调试能力。该能力是 Gallery 的调试基础设施，不属于 Widgetry 库的生产依赖路径。

## 启用方式

Gallery 每次启动都会安装 Extras、RemotePlugin、Main / Render HTTP listener 与 BRP 续帧 controller。BRP 调试直接使用 `brp_launch` 启动 `widget_gallery`，不需要额外的环境变量开关。

Gallery 仍始终使用 `WinitSettings::desktop_app()`。BRP runtime 不会切换为 Reactive 或 Continuous update mode，也不会安装 idle 周期 polling。

## Port 与等待边界

Main World 默认监听 `127.0.0.1:15702`。`BRP_EXTRAS_PORT` 可覆盖 Main port；该变量未设置时使用 15702，无法解析为 `u16` 时记录 warning 并回退到 15702。Render World 使用独立的 `127.0.0.1:15703`，不跟随 `BRP_EXTRAS_PORT`。

普通非 watch 请求从成功入队起最多等待 30 秒的首个 result。超时会返回明确的 BRP error，释放 result receiver 并请求 cleanup update，不会无界持有工作责任。watch 使用 SSE connection 生命周期，不套用该 30 秒边界。

## 事件驱动运行

请求 wake 与活动续帧是两个不同的责任：

- HTTP transport 把 message 成功放入 Main 或 Render mailbox 后，通过 Winit `WakeUp` user event 请求 App 处理它。mailbox 满时会先 wake 已排队工作，成功入队后再次 wake。
- keyboard、mouse、drag、double-click、screenshot 和 deferred shutdown 等跨帧操作由 Extras activity guard 表达真实在途 lifecycle。Main / Render mailbox、普通请求 result 等待与 Extras activity 共同决定是否还需要后续 update。
- 存在真实工作时，controller 使用 `RequestRedraw` 推进下一次正常 update，并为当前 generation 预约一次有界 Winit `WakeUp` fallback。fallback 只保证在途工作的跨帧进展，不是 idle polling。
- 最后一项工作结束后只保留有界的 cleanup update，随后恢复 `desktop_app()` 的 idle 行为。长期 watch connection 在没有变化时不单独构成续帧来源。

## 排错证据

调试时先检查 Main 与 Render port 是否成功监听。一侧 bind 或 accept 发生 fatal failure 时，共享 lifecycle 会关闭两个 listener，Gallery 记录 endpoint 与底层 error 并结束 App，不会保留“部分就绪”的 runtime。

对静止后请求，区分以下证据：请求是否成功入队、Winit event loop 是否被 wake、对应 World 是否消费 mailbox，以及最终 result 或 timeout。对多帧操作，HTTP 已返回“开始”不代表操作已完成；应等待预期完成时间后再做一次最终观察，并使用最终输入 state、队列清理、最终位置、完成日志或已完整发布的 screenshot 文件作为证据。不要在等待期间连续发送 query 或 screenshot 来人为推进 App。

最小化、隐藏或完全遮挡 window 时，非视觉请求仍由 WakeUp 和按需 fallback 推进；screenshot 能否获得有效图像取决于 OS 是否继续提供可 present 的 surface。该情况应获得明确失败或有界 timeout，不应永久挂起。

## 上游升级检查点

Gallery 通过 Git tag `v0.1.0` 依赖外部 `bevy_brp_runtime`，仓库内不再维护 Extras、HTTP transport 或续帧 controller 源码。

升级 Bevy 或 `bevy_brp_runtime` 前，必须在上游比较 method registration、HTTP framing、Main / Render 启动阶段、mailbox 行为、screenshot publication 与 Extras 操作 lifecycle，并在 Widgetry Gallery 重新完成对应接缝与运行时验证。
