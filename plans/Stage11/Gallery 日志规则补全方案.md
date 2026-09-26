# Gallery 日志规则补全方案

## 目标

扩展 `rules/logging.md`，明确区分 Widgetry 库与 Gallery 两类日志职责，并同步补齐当前 Gallery 中缺失的用户操作日志，使现有代码符合新规则。

## 规则修改

### Widgetry 库

保留现有规则：

* 生产代码统一使用 `bevy_widgetry_log` 提供的 `widgetry_info!`、`widgetry_warn!`、`widgetry_error!`。
* 不记录正常用户交互。
* 日志主要承担库内部诊断职责。
* `debug`、`trace` 不作为永久日志。

### Gallery

新增独立的 Gallery 日志规则：

* Gallery 只能使用普通 Bevy 日志宏：

  * `info!`
  * `warn!`
  * `error!`
* 禁止 Gallery 使用 `bevy_widgetry_log` 及 `widgetry_*` 日志宏。
* 不为 Gallery 引入 `bevy_widgetry_log` 依赖。
* `debug!`、`trace!` 不作为永久日志。

等级语义：

* `info!`：记录用户明确触发、具有 Gallery 演示语义的离散操作及其必要结果。
* `warn!`：操作失败但 Gallery 仍可继续运行，或某项演示能力不可用、降级。
* `error!`：Gallery 自身 bug、内部不变量破坏或无法正常继续的失败。

当前没有确认需要新增 `error!` 的路径，不为了满足规则人为制造 ERROR 日志；以后出现相应失败路径时必须记录。

### 用户操作日志

Gallery 应记录用户实际执行的语义操作，例如：

* 激活按钮；
* 切换 Gallery 页面；
* 切换主题；
* 选择 ComboBox 项；
* 打开独立窗口；
* 打开 MessageBox 并取得结果；
* 发起文件选择、文件保存等操作及其结果。

记录的是**操作语义**，不是底层输入事件。

例如记录“打开独立窗口”“选择 ComboBox 项”，而不是“收到 `Pointer<Click>`”。

以下高频或中间状态不记录：

* hover；
* pointer move；
* press / release 本身；
* 普通 focus 变化；
* 文本框逐字符输入；
* 其他仅用于驱动控件内部状态的连续事件。

Button Demo 中按钮激活本身就是被演示的语义，因此即使按钮没有进一步修改应用状态，也需要记录。

### 消息格式

Widgetry 与 Gallery 共用现有消息格式：

* 正文使用中文；
* 动态上下文使用英文结构化字段；
* 不手工加入源码位置等 tracing 已有 metadata。

例如：

```rust
info!(page = ?page, "切换 Gallery 页面");
info!(mode = ?mode, "切换主题");
info!(demo = "button", button = "Icon + Text", "激活按钮示例");
warn!(operation = ?operation, "文件对话框无法取得主窗口");
```

Gallery 不固定 `target: "bevy_widgetry"`。

## 当前 Gallery 同步修改

### `gallery/src/main.rs`

主题选择器实际改变主题时记录一次 `info!`：

```text
切换主题
```

附带 `mode` 字段。

初始化 ComboBox 时保持静默，不把程序初始化产生的值变化记成用户操作。

### `gallery/src/gallery.rs`

用户激活侧边栏导航时记录页面切换：

```text
切换 Gallery 页面
```

附带目标 `page`。

### `gallery/src/pages/button.rs`

为可交互 Button Demo 增加 Gallery 自有的操作标识和激活处理。

每次实际激活普通按钮时记录：

```text
激活按钮示例
```

结构化字段标明具体示例，例如文本按钮、图文按钮、纯图标按钮。

禁用按钮不会产生操作日志。

### `gallery/src/pages/combo_box.rs`

给各 ComboBox 示例附加 Gallery 自有标识，并响应有效的选项变化。

用户完成一次选择时记录：

```text
选择 ComboBox 示例项
```

字段至少能够区分具体示例和所选项。

程序初始化产生的选择状态不记录。

### `gallery/src/pages/text_field.rs`

当前不新增永久操作日志。

文本逐字符编辑属于连续输入，不应为每次内容变化产生日志。

以后如果增加明确的 submit / confirm 等离散语义操作，再记录对应 `info!`。

### Window Demo

独立窗口：

* 点击入口时记录“打开独立窗口”；
* 独立窗口内部 Demo Button 被激活时记录对应按钮操作。

MessageBox：

* 用户点击入口打开 MessageBox 时记录操作及按钮组合；
* 保留已有的 MessageBox 结果 `info!`，作为该用户操作的结果日志。

File Dialog：

* 用户发起 Open File / Open Files / Select Folder / Open Image / Save File 时记录操作；
* 对选择完成或取消等最终结果记录必要的 `info!`；
* 保留当前已有的结构异常 `warn!`；
* 当前 `Main window unavailable` 分支除更新页面文本外，补充 `warn!`，因为它属于实际操作无法完成但 Gallery 可以继续运行的可恢复失败。

不因为日志而改变现有异步流程或 UI 反馈行为。

## 影响边界

本次不修改 Widgetry crate 的日志行为，只把 `rules/logging.md` 中原本主要针对库侧的规则与 Gallery 规则明确拆开。

不修改 Gallery 日志 subscriber、文件输出、过滤或格式化基础设施。

不为了日志增加轮询 system、全局输入监听或低层事件追踪；日志尽量附着在现有 Gallery 语义处理点。Button / ComboBox 示例允许增加轻量事件处理，因为这些处理本身就是为了记录其明确的 Demo 操作。

## 验证

实现后至少确认：

* Gallery 编译通过；
* Gallery 没有引用 `bevy_widgetry_log` 或 `widgetry_*` 日志宏；
* 执行各 Gallery Demo 的离散用户操作时产生对应 `info!`；
* hover、pointer move、文本逐字符编辑等不会刷日志；
* File Dialog 的可恢复失败产生 `warn!`；
* 现有 Widgetry 库日志行为没有改变。
