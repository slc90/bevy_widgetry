# PowerInhibit

`bevy_widgetry_power_inhibit` 提供 Windows 64 位空闲息屏与系统自动休眠抑制。Facade 通过默认关闭的 `power_inhibit` Feature 和同名模块导出，不依赖窗口或托盘插件。

```toml
bevy_widgetry = { path = "../bevy_widgetry/crates/bevy_widgetry", default-features = false, features = ["power_inhibit"] }
```

宿主显式添加 `WidgetryPowerInhibitPlugin`。插件仅注册默认关闭的 `PowerInhibit` Resource，不创建电源请求、不添加更新 System。应用根据任务生命周期同步调用：

```rust
use bevy::prelude::*;
use bevy_widgetry::power_inhibit::{PowerFailures, PowerInhibit, WidgetryPowerInhibitPlugin};

fn start_work(mut power: ResMut<PowerInhibit>) -> Result {
    power.enable().map_err(PowerFailures::into_bevy_error)
}

fn finish_work(mut power: ResMut<PowerInhibit>) -> Result {
    power.disable().map_err(PowerFailures::into_bevy_error)
}

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, WidgetryPowerInhibitPlugin))
        .add_systems(Startup, start_work)
        .run();
}
```

`enable()` 成功表示 Windows 已接收同一个对象上的 DisplayRequired 和 SystemRequired 两类请求。重复开启不增加请求计数；`disable()` 清除两类请求并释放句柄，重复关闭无操作，之后可以再次开启。没有公共 `enabled` / `active` 字段，也没有每帧续期或定时轮询。

内部以 RAII 持有 Windows 请求句柄，只记录真正成功的请求类型。第二类请求设置失败时撤销已建立的第一类请求并关闭对象；回滚失败时仍关闭句柄，由 Windows 回收关联请求，并将主错误与回滚错误一起返回。普通 `disable()` 清除失败时继续尝试另一类型，只保留失败类型和句柄以供重试。此时调用 `enable()` 会先尝试清理剩余请求，清理成功后重新建立完整请求；清理失败则返回错误，不重复设置已有类型。

普通 `enable()` / `disable()` 返回 `Result<(), PowerFailures>`。非空集合内的每个 `PowerFailure = WidgetryFailure<PowerError>` 独立保存操作 `PowerOperation`、阶段 `PowerPhase`、请求 `RequestKind`、失败时 Display/System 计数状态以及原始 `io::Error`。主失败、回滚失败和双类型 clear 失败分别保留 ID、origin 与回溯，不拼成唯一诊断字符串。源头各记录一次 ERROR；集合迭代 `failures()` 供业务匹配，只有执行边界调用 `into_bevy_error()` 转为 Error Severity，并让宿主逐项访问。

移除 Resource 或释放 App 时尽力清除剩余请求，随后由私有 `OwnedHandle` 调用 `CloseHandle`。Drop 中的 clear 失败先保存领域诊断，真正释放请求对象后才记录处理 DEBUG，说明执行了最终关闭措施，不声称 clear 调用成功。显式 disable 的失败仍返回拥有者，保留待重试状态；enable 失败后成功回滚并不表示主失败已消费。库不配置宿主 error handler 或日志订阅。

请求属于进程内核对象，资源可通过 `ResMut` 跨线程使用；所有修改均需独占借用，裸句柄只存在于局部允许 unsafe 的私有 FFI 模块。线程安全和单次关闭由标准库 `OwnedHandle` 支持，无手写 `unsafe impl Send/Sync`。原因字符串使用调用期间有效的 UTF-16 文本。

窗口最小化、失去焦点、隐藏至托盘均不改变请求。应用正常退出并释放 App 时清理，异常进程终止由 Windows 回收句柄。仅发送 AppExit 但继续持有 App 的宿主仍拥有该 Resource，应显式调用 `disable()` 或移除 Resource。

仅阻止空闲超时触发的自动息屏与休眠，不阻止用户主动睡眠、合盖、电源按钮或关机，不修改电源计划，也不提供独立屏保设置。普通用户可调用电源请求 API；系统仍可能按策略忽略或限制请求，Modern Standby 电池供电时 SystemRequired 存在额外时限。参见 [PowerSetRequest](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-powersetrequest)。

实机验收需在开启前、开启后、最小化/失焦/托盘隐藏期间、关闭后与退出后检查 `powercfg /requests` 的 DISPLAY/SYSTEM 本进程条目。部分 Windows 环境要求该诊断命令以管理员身份运行；这与请求 API 的普通用户权限要求不同。API 测试通过不代表已观测到系统列表或实际空闲超时行为。

`cargo run -p widget_gallery` 后进入“休眠抑制 / Power Inhibit”页，可通过按钮开启或关闭真实请求，查看同步调用的成功或错误状态。Gallery 在更新失败读数与重试状态后，按集合内各个 error_id 记录处理 DEBUG；显示文字只是已消费诊断的投影。Gallery 默认关闭该能力，不保存开关偏好；切换页面、失焦、最小化或关闭到托盘均保留请求，手动关闭或彻底退出 Gallery 才释放。可先在 Windows 设置中缩短自动息屏和睡眠等待时间，开启后停止输入并等待超时，再关闭进行对照；Gallery 不修改系统电源设置。
