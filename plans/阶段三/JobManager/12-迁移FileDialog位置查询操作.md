# 迁移 FileDialog 位置查询操作

文件对话框的系统位置查询（常用位置、侧栏位置等）是可独立请求并完成的一项 I/O 操作。它和创建文件夹有不同的错误、副作用和缓存语义，因此单独改为受管 Job，不再让位置查询依赖旧的 I/O worker 线程存活。

## 异步位置查询与业务边界

当前 `worker.rs` 的 `IoWork::Locations(Control)` 调用 `backend.locations()`，将结果限制在最多 128 项，并通过 `Output::Locations(Control, Result<Vec<WidgetryFileDialogLocation>, String>)` 回传。`runtime.rs` 的 `Session::locations_admitted` 限制相同 Session 的重复请求，出错时 FileDialog 原有侧栏和状态逻辑决定如何显示。新的一个 Locations Job 使用已有 JobManager I/O Future，结果可通过 `JobReceiver<T>` 发送一条 `Locations` 业务项并最终给出 `Finished`；不要求通道为 128 个位置逐条广播，业务可以把一次位置查询视为一个 `T`。

```mermaid
flowchart LR
    Session[当前 Session] --> Job[Locations I/O Job]
    Job --> Result[位置集合 / 查询失败]
    Result --> Guard[Session / Generation 守卫]
    Guard --> Sidebar[FileDialog 侧栏]
```

新 Job 仍要尊重 FileDialog 的 Session/Token 身份和请求次数；用户已关闭、导航到新 Session 或 Root 已无效时，旧位置结果不能修改当前 UI。对单 Session 有重复位置查询需求时，是否重新 Admission 由业务自己的状态决定，不加入统一 JobManager 的优先级/重试/最新请求队列。`WidgetryFileDialogRuntimeOptions` 的自动服务开关、每帧消息/时间预算与 Headless 手工结果注入继续生效。

## 错误与取消

系统位置不可获得、部分入口无效、返回超过 128 项均走现有可观察业务错误路径，不因 Job 失败后自动隐藏原 UI；具体是否允许部分位置由 FileDialog backend 已有契约决定，不在通用 Manager 定义。Job 被取消不能代表 OS 查询已中止，管理器应等待实际退出，ECS 仍可拒绝所有过期结果。若查询已返回完整结果、业务未确认取消，则为 `Completed`；若主动终止查询并通过 `JobContext::acknowledge_cancel()` 明确确认，且真实退出后才为 `Canceled`，顶层错误为 `Failed`。无活跃订阅者时新结果丢弃，不会由于旧 Dialog 已销毁而拖住后台查询。

## 交付与验收

替换 `IoWork::Locations` 和 `Output::Locations` 的旧专有线程传输，保留其业务 Reply/状态适配。测试成功位置列表、空列表、128 项容量、 backend 失败、Session Cancel/重开、新旧 Session 并发、重复初始化不重复提交、取消后不越权更新侧栏、Headless 模式不自动发起 I/O。对应 FileDialog Runtime/Contract 测试保留，并在 Gallery 的文件对话框侧栏可见行为发生变化时进行 BRP 验证。更新 `docs/crates/file_dialog.md`；接下来的偏好读写收口才能完全退役旧的 I/O Worker。
