# 后台任务

Job 提供 I/O、CPU 与独立线程任务，统一查询状态、接收结果、协作取消和关闭。通过 Facade 使用时开启 `job` Feature。

```rust
use bevy::prelude::*;
use bevy_widgetry::job::*;

app.add_plugins(JobManagerPlugin);
```

宿主先安装 Bevy 任务池；普通应用的默认插件已提供任务池。`JobManagerPlugin` 在 App 中建立共享 `JobManager`。从 Resource 克隆 Manager 可以交给后台使用，克隆不会创建第二个任务注册表。

## 任务提交

### 提交 CPU 计算

下面把结果分批发回，而不是让主线程等待整个计算。

```rust
let manager = world.resource::<JobManager>().clone();
let (job_id, receiver) = manager.spawn_cpu::<u64, _, _>(
    JobOptions {
        label: Some("计算平方".into()),
        buffer_capacity: Some(16),
    },
    |context| async move {
        for value in 0_u64..100 {
            if context.is_cancelled() {
                context.acknowledge_cancel();
                return Ok(());
            }
            match context.send(value * value).await? {
                SendStatus::Sent => {}
                SendStatus::Canceled => {
                    context.acknowledge_cancel();
                    return Ok(());
                }
            }
        }
        Ok(())
    },
).map_err(|error| error.into_bevy_error())?;
```

将 `job_id` 和 `receiver` 保存在发起操作的业务状态中。结果项类型是 `u64`；复杂任务可以改成包含进度、批次或完成数据的业务枚举。

`spawn_io` 接受相同形式的异步闭包。需要同步、长期阻塞的服务时使用 `spawn_thread`，不要把阻塞 I/O 放进 ECS System。

### 提交独立线程

```rust
let (job_id, receiver) = manager.spawn_thread::<String, _>(
    JobOptions { label: Some("独立工作线程".into()), ..default() },
    |context| {
        for index in 0..10 {
            if context.is_cancelled() {
                context.acknowledge_cancel();
                return Ok(());
            }
            if context.send_blocking(format!("批次 {index}"))? == SendStatus::Canceled {
                context.acknowledge_cancel();
                return Ok(());
            }
        }
        Ok(())
    },
).map_err(|error| error.into_bevy_error())?;
```

`send_blocking` 用于普通同步工作线程，不用于 ECS 主线程。线程任务的终态要等线程真实退出并完成收割后才成立。

## 结果接收

### 在 UI 更新中非阻塞读取

```rust
fn drain(receiver: &mut JobReceiver<u64>) -> Result {
    loop {
        match receiver.try_recv().map_err(|error| error.into_bevy_error())? {
            JobRead::Item(value) => {
                info!(value = *value, "收到计算结果");
            }
            JobRead::Finished(JobOutcome::Completed) => {
                info!("任务完成");
                break;
            }
            JobRead::Finished(JobOutcome::Canceled) => break,
            JobRead::Finished(JobOutcome::Failed(error)) => {
                return Err(error.into_bevy_error());
            }
            JobRead::Empty | JobRead::Closed => break,
        }
    }
    Ok(())
}
```

`Empty` 表示当前没有数据，不表示失败或结束。每份 Receiver 先读完属于自己的数据，再收到一次 `Finished`，之后返回 `Closed`。异步消费者可使用 `recv().await`；同步后台线程可使用 `recv_blocking()`。

结果以 `Arc<T>` 交付，不要求 `T: Clone`。接收者长期持有这些 Arc 时，相应数据也会继续占用内存。

### 增加订阅者

```rust
let second_receiver = manager.subscribe::<u64>(job_id)
    .map_err(|error| error.into_bevy_error())?;
```

新订阅只接收注册之后的数据，不回放历史。只有 Pending 或 Running 的任务接纳订阅；类型必须与提交时一致。

缓冲容量按结果条目数计算。最慢的活跃订阅者会产生背压；不覆盖尚未读取的数据。没有活跃订阅者时，发送不会为了等待订阅者而阻塞。终态通知独立保存，不占数据缓冲容量。

## 状态与取消

### 查询任务

```rust
let status = manager.status(job_id)
    .map_err(|error| error.into_bevy_error())?;
info!(state = ?status.state, label = ?status.label, "任务状态");
```

状态为 Pending、Running、Stopping、Completed、Canceled 或 Failed。任务顶层返回错误会进入 Failed；业务内部已经处理的局部问题不会自动改变任务终态。

### 请求取消

```rust
let result = manager.cancel(job_id)
    .map_err(|error| error.into_bevy_error())?;
info!(?result, "提交取消请求");
```

取消不是强行终止线程。工作体检查 `is_cancelled()`，停止业务操作并完成收尾，再调用 `acknowledge_cancel()` 确认。`send` 返回 Canceled 也不会自动替工作体确认取消。

已完成任务不会因为迟到的取消请求改成 Canceled。取消后仍在执行的工作保持 Stopping；UI 可以停止显示进度，但仍需保留接收或收尾路径。

### 移除完成记录

```rust
if manager.status(job_id)
    .map_err(|error| error.into_bevy_error())?
    .state.is_finished()
{
    manager.remove(job_id).map_err(|error| error.into_bevy_error())?;
}
```

移除记录不是取消操作。活动任务不能移除，JobId 在同一个 Manager 中不复用。允许对象已经结束的幂等清理可使用 `cancel_if_present`、`remove_if_present`，而不是忽略所有访问错误。

## 全局关闭

### 停止接纳并等待已有任务

```rust
use std::time::Duration;

let shutdown = manager.begin_shutdown(Duration::from_secs(5))
    .map_err(|error| error.into_bevy_error())?;
```

保留 `ShutdownHandle` 观察关闭进程。开始关闭后，Manager 拒绝新任务并向已有任务请求取消。超时不是线程已退出的证明，也不会强行释放仍在执行的资源；应用据实际状态决定等待或结束进程。

关闭期间继续接收任务结果，特别是取消后才暴露的真实失败。重复发起关闭复用已有关闭过程。
