# Widget Gallery：BRP 调试模式与多窗口流畅性兼容方案

## 目标

解决当前 `Widget Gallery` 中的冲突：

- 普通人工运行时，希望继续使用 `WinitSettings::desktop_app()`，保持多独立窗口拖动流畅；
- Codex 通过 BRP 调试时，需要 App 能定期 `update`，避免 BRP 请求因为没有窗口事件而长时间得不到处理；
- 不采用全局固定 `33ms` polling，因为它会让整个 Gallery 约 30Hz 定时唤醒，并对所有窗口请求 redraw，重新带来多窗口卡顿。

本次采用**简单方案**：仅在 BRP 自动化调试模式下启用低频 polling，普通运行保持原来的 desktop event-driven 模式。

---

## 实现原则

正常运行：

```text
cargo run -p widget_gallery
    ↓
WinitSettings::desktop_app()
    ↓
保持原有低功耗、事件驱动行为
```

Codex BRP 调试：

```text
brp_launch + WIDGETRY_BRP=1
    ↓
Reactive polling
    ↓
约每 150ms 最多主动 update 一次
    ↓
BRP 请求可以及时被 RemoteLast 处理
```

不要再把整个 Gallery 永久改成 `33ms` polling。

---

## 代码修改

修改 `gallery/src/main.rs`。

保留：

```rust
use bevy::winit::WinitSettings;
```

并引入：

```rust
use bevy::winit::UpdateMode;
use std::time::Duration;
```

将当前类似下面的全局配置：

```rust
app.insert_resource(WinitSettings {
    focused_mode: UpdateMode::reactive(Duration::from_millis(33)),
    unfocused_mode: UpdateMode::reactive_low_power(Duration::from_millis(33)),
});
```

替换为：

```rust
let winit_settings = if std::env::var_os("WIDGETRY_BRP").is_some() {
    WinitSettings {
        focused_mode: UpdateMode::reactive(Duration::from_millis(150)),
        unfocused_mode: UpdateMode::reactive_low_power(Duration::from_millis(150)),
    }
} else {
    WinitSettings::desktop_app()
};

app.insert_resource(winit_settings);
```

### 约定

环境变量：

```text
WIDGETRY_BRP
```

只表示：

> 当前 Gallery 是由 Codex / BRP 自动化调试流程启动，需要较短的 App update 等待时间。

值本身不重要，只判断变量是否存在。

默认 polling 间隔使用：

```text
150ms
```

不要恢复到 `33ms`。

如果实际使用后发现 Codex BRP 响应明显过慢，可以在 **100ms～250ms** 范围内调整，但本次先固定为 `150ms`，不要提前增加配置项。

---

## Codex BRP 启动方式

Codex 使用 `brp_launch` 启动 `widget_gallery` 时，应传入环境变量：

```text
target_name = "widget_gallery"

env = {
    "WIDGETRY_BRP": "1"
}
```

这样只有 Codex BRP 自动化运行才启用 polling。

人工直接运行：

```powershell
cargo run -p widget_gallery
```

不设置该变量，因此自动使用：

```rust
WinitSettings::desktop_app()
```

---

## GUI 调试规则同步

如果项目中已经新增 `rules/gui-debugging.md`，同步补充以下约束：

```markdown
Codex 通过 BRP 启动 Widget Gallery 时，必须设置：

`WIDGETRY_BRP=1`

该环境变量仅用于启用 BRP 自动化所需的低频 App polling。

普通人工运行 Gallery 不应设置该变量，并继续使用
`WinitSettings::desktop_app()`。

不得为了提高 BRP 响应速度，把 Gallery 全局改为高频 Reactive
或 Continuous update mode。
```

如果 `rules/gui-debugging.md` 中已经有 `brp_launch` 示例，也同步把 `env` 参数加进去。

不需要修改其他开发流程规则。

---

## 不做的事情

本次不要实现以下内容：

- 不自定义 BRP HTTP transport；
- 不修改 Bevy 或 `bevy_brp_extras`；
- 不实现 `EventLoopProxy` wake bridge；
- 不增加新的 crate；
- 不增加复杂配置文件；
- 不为 polling interval 增加 CLI 参数；
- 不把 BRP 调度逻辑放进 `bevy_widgetry` 库；
- 不重新设计窗口刷新机制。

这些都超出当前问题范围。

---

## 验证

修改完成后至少验证两种运行方式。

### 1. 普通人工运行

执行：

```powershell
cargo run -p widget_gallery
```

确认：

- Gallery 正常启动；
- 创建多个独立窗口；
- 拖动独立窗口时保持原有流畅性；
- 没有因为 BRP 改动重新出现明显卡顿。

### 2. Codex BRP 运行

通过 `brp_launch` 启动：

```text
widget_gallery
WIDGETRY_BRP=1
```

确认：

- BRP 能正常连接；
- screenshot 能正常返回；
- mouse / keyboard 等 BRP extras 能正常执行；
- 请求响应延迟可接受；
- 多窗口拖动性能明显好于原先全局 `33ms` polling。

最后执行项目既有必要验证，例如：

```powershell
cargo fmt --all -- --check
cargo check --workspace
```

并继续遵守项目已有 Code Review 流程。

---

## 最终预期

正常人工运行：

```text
WinitSettings::desktop_app()
→ 事件驱动
→ 多窗口流畅
```

Codex BRP 调试：

```text
WIDGETRY_BRP=1
→ Reactive 150ms
→ BRP 请求能及时处理
→ 不再让所有 Gallery 运行方式承担 30Hz polling 成本
```

这只是当前的简单兼容方案。

如果未来 `bevy_brp_extras` 或 Bevy BRP transport 能在收到请求时主动 wake Winit event loop，可以删除这套 polling 特例，恢复为完全 event-driven。
