# 电源抑制

PowerInhibit 在应用明确需要持续运行时，请求系统不要因空闲而息屏或自动休眠。通过 Facade 使用时开启 `power_inhibit` Feature。

## 请求开关

### 安装并启用

```rust
use bevy::prelude::*;
use bevy_widgetry::power_inhibit::{PowerInhibit, WidgetryPowerInhibitPlugin};

app.add_plugins(WidgetryPowerInhibitPlugin);
```

只安装插件不会开始抑制。业务操作开始时显式调用：

```rust
fn keep_awake(mut power: ResMut<PowerInhibit>) -> Result {
    power.enable().map_err(|failures| failures.into_bevy_error())
}
```

`enable()` 同时请求 Display 和 System 两类抑制；公开接口不是独立开关这两类请求。完整启用后的重复调用是合法空操作，不累积第二组请求。

### 结束请求

```rust
fn release_awake(mut power: ResMut<PowerInhibit>) -> Result {
    power.disable().map_err(|failures| failures.into_bevy_error())
}
```

应在任务完成、取消或不再需要常亮时解除。窗口隐藏、页面切换和托盘状态不会自动改变请求；业务应根据真正的任务生命周期控制。

当前 Resource 不是按任务引用计数的租约集合。多个业务任务同时需要抑制时，由应用协调最后一个需求消失的时刻，不能让其中一个任务结束就直接替其他任务关闭请求。

## 系统行为边界

抑制针对空闲息屏与自动休眠，不阻止用户主动关机、主动休眠，也不绕过系统电源策略。它与 Window、Tray 的插件独立，使用时不要求存在可见窗口。

Resource 销毁会执行最终清理，但显式 `disable()` 可以让调用者观察清理失败并决定下一步。

## 错误与恢复

`PowerFailures` 可以同时保存主操作、回滚和清理失败。启用过程中若只成功了一部分，会尝试撤销已经取得的请求，避免留下“看起来没有启用，实际仍有请求”的半成品。

关闭时分别尝试清理已有请求；某一项失败不跳过另一项。后续可以再次调用 `disable()`，只处理仍然存留的部分。领域诊断包含操作、阶段、请求类别及原始系统原因。

在 ECS 边界使用 `into_bevy_error()` 传播整个集合，不只取第一个失败或转换为字符串。
