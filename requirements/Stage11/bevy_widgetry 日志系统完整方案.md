# bevy_widgetry 日志系统完整方案

## 1. 总体职责边界

日志系统分成两层：

```text
bevy_widgetry
    ↓ 产生日志事件

宿主 App / Gallery
    ↓ 决定如何过滤、格式化和输出

终端 / 文件 / 其他输出
```

`bevy_widgetry` 只负责：

- 判断哪些情况值得记录日志；
- 选择正确日志等级；
- 提供必要的结构化上下文；
- 使用统一稳定的日志 target。

`bevy_widgetry` 不负责：

- 初始化 tracing subscriber；
- 配置日志文件；
- 配置终端输出；
- 配置时间格式；
- 配置 ANSI 颜色；
- 决定是否显示源码位置；
- 解析或替代 `RUST_LOG`；
- 提供自己的日志开关或日志等级 Resource。

这些属于宿主 App 的职责。

---

# 2. Widgetry 统一日志 target

所有 Widgetry 日志统一使用：

```rust
target: "bevy_widgetry"
```

不根据 crate 或 module 拆分 target。

例如不使用：

```text
bevy_widgetry_core
bevy_widgetry_combo_box
bevy_widgetry::icon
bevy_widgetry::plugin
```

而是全部：

```text
bevy_widgetry
```

因此宿主可以统一过滤：

```text
bevy_widgetry=off
bevy_widgetry=error
bevy_widgetry=warn
bevy_widgetry=info
bevy_widgetry=debug
bevy_widgetry=trace
```

例如：

```text
bevy_widgetry=info
```

包含：

```text
INFO
WARN
ERROR
```

排除：

```text
DEBUG
TRACE
```

`target` 只承担过滤职责，默认**不显示在最终日志文本中**。

---

# 3. 新增 `bevy_widgetry_log` 内部 crate

新增：

```text
crates/log/
```

package：

```text
bevy_widgetry_log
```

它位于 Widgetry 内部生产依赖图底层。

职责只有一个：

> 为 Widgetry 内部提供统一日志宏，并固定 `target: "bevy_widgetry"`。

它不负责：

- subscriber；
- formatter；
- 日志文件；
- 日志等级配置；
- 状态保存；
- 去重状态；
- App 配置。

当前只提供：

```rust
widgetry_info!(...)
widgetry_warn!(...)
widgetry_error!(...)
```

暂时不提供：

```rust
widgetry_debug!(...)
widgetry_trace!(...)
```

因为当前没有确定的长期 `debug` / `trace` 使用场景。

以后真正出现合适用途时再增加。

---

# 4. 日志宏保持最薄封装

宏只固定 target，并完整转发原本 tracing / Bevy 日志语法。

例如：

```rust
#[macro_export]
macro_rules! widgetry_info {
    ($($arg:tt)*) => {
        bevy::log::info!(
            target: "bevy_widgetry",
            $($arg)*
        )
    };
}
```

`warn`、`error` 同理。

因此仍然支持结构化字段：

```rust
widgetry_error!(
    entity = ?entity,
    "ComboBox 内部不变量被破坏：缺少 Popup"
);
```

或者：

```rust
widgetry_warn!(
    path = %path,
    error = %err,
    "图标资源处理失败"
);
```

宏不得自动加入：

```text
crate_name
module_path
file
line
```

等额外字段。

---

# 5. Widgetry 内部禁止绕过统一日志宏

Widgetry 各生产 crate 的代码统一通过：

```text
widgetry_info!
widgetry_warn!
widgetry_error!
```

记录日志。

不得直接调用：

```rust
bevy::log::info!(...)
bevy::log::warn!(...)
bevy::log::error!(...)
```

也不得通过其他等价方式绕过统一 target。

目的不是减少输入量，而是保证：

```text
所有 Widgetry 日志
        ↓
target 永远是 "bevy_widgetry"
```

避免后续人工或 AI 开发时漏写 target。

`bevy_widgetry_log` 不从顶层 `bevy_widgetry` facade 导出，它属于内部工程基础设施。

---

# 6. 日志等级语义

## `error`

用于 Widgetry 自身负责的严重错误：

- 库内部不变量被破坏；
- 必须存在的内部状态或层级结构不存在；
- 明确属于 Widgetry 自身的 bug；
- 无法恢复的内部失败；
- Widgetry 自身 bug 即将导致 `panic!` / `assert!`。

如果 Widgetry 自身 bug 最终会 panic：

```text
先 ERROR
再 panic/assert
```

不能认为 panic 输出可以替代正式日志。

GUI 应用通常未必存在可见 stderr，因此错误必须先进入 App 的日志管线。

---

## `warn`

用于程序仍能运行，但实际能力受到影响的情况：

- 非预期外部操作失败；
- 失败被 Widgetry 内部吸收；
- 调用方没有其他错误通道可以得知；
- 平台或环境导致某项能力不可用；
- 某项功能发生降级；
- 某项功能被关闭。

---

## `info`

用于少量正常生命周期事件：

- Plugin 注册完成；
- 已经记录过的持续异常恢复正常。

---

## `debug`

等级保留。

当前不增加永久 `debug` 日志。

---

## `trace`

等级保留。

当前不增加永久 `trace` 日志。

---

# 7. 所有 Bevy Plugin 都记录注册完成

Widgetry 内所有实现：

```rust
impl Plugin for ...
```

的 Plugin，在 `build()` 完成时记录一次：

```text
<PluginName> 注册完成
```

例如：

```text
WidgetryAssetPlugin 注册完成
ThemePlugin 注册完成
IconPlugin 注册完成
ComboBoxPlugin 注册完成
LongPressPlugin 注册完成
```

边界是 **Plugin 本身**。

不继续记录 Plugin 内部的：

```text
resource initialized
system registered
observer registered
asset loader registered
cache initialized
```

等细节。

现有工程中 `WidgetryAssetPlugin` 就属于这种内部 Plugin，因此该规则不是只针对用户直接添加的公开 Plugin。

日志表述使用：

```text
注册完成
```

而不是：

```text
初始化成功
```

因为 `build()` 完成只说明 Plugin 注册逻辑执行完成，并不表示其未来运行时工作全部成功。

不记录库版本号。

---

# 8. Widgetry 自身 bug 必须记录

如果一个对象已经确认属于 Widgetry，随后发现其内部状态违反 Widgetry 自己定义的不变量，则属于 Widgetry 错误。

例如：

```text
ComboBox 已经被确认是有效 Widgetry ComboBox
        ↓
按设计必须存在 Popup
        ↓
Popup 不存在
        ↓
ERROR
```

而以下情况不是错误：

```text
全局 observer 收到事件
        ↓
发现目标不是当前 Widgetry 控件
        ↓
return
```

这种只是正常事件过滤。

现有 ComboBox 中存在大量这种 observer 过滤和内部层级检查，因此实现时必须区分“正常过滤”和“已确认控件后的内部异常”。

---

# 9. 调用方 API 错误不属于 Widgetry 日志

以下属于调用方责任：

- 参数非法；
- 调用顺序错误；
- 违反公开 API 前置条件；
- 错误使用 API。

Widgetry 不为这些问题额外添加库日志。

即使公开 API 因非法输入最终触发：

```rust
assert!(...)
panic!(...)
```

只要责任属于调用方，就仍然不是 Widgetry 自身 `error`。

---

# 10. 外部错误的记录原则

如果错误已经通过公开接口向上传递，例如：

```text
Result
Event
其他明确错误通道
```

则 Widgetry 不再重复打一份日志。

例如 SVG AssetLoader 当前读取或解析失败时直接通过 `Result` 返回错误，因此不需要 Widgetry 再额外 `warn!` 一次。

只有同时满足：

1. 失败是非预期的；
2. Widgetry 将其内部吸收并继续运行；
3. 调用方没有其他途径知道失败发生；

才由 Widgetry 记录。

---

# 11. 正常异步等待不记录

例如 Asset：

```text
资源还没加载完成
    ↓
保持 pending
    ↓
下一帧继续尝试
```

属于正常流程。

不得每帧：

```text
WARN asset not ready
WARN asset not ready
WARN asset not ready
...
```

现有 Icon materialize 本身存在跨帧等待资源就绪的流程，这一类状态不属于异常。

如果真正发生 SVG rasterization / pixmap 等非预期失败并被内部吸收，则记录 `warn`。

实现上必须能够区分：

```text
正常等待
```

与：

```text
真正失败
```

---

# 12. 环境信息只在产生实际降级时记录

单纯检测到：

```text
OS
GPU backend
DPI
窗口环境
```

不记录。

只有这些环境因素真正造成：

```text
功能不可用
功能被关闭
功能退化
```

时记录 `warn`。

---

# 13. 正常用户交互不属于 Widgetry 日志

不长期记录：

```text
press
release
click
drag
hover
focus
blur
ComboBox open
ComboBox close
selection changed
long press triggered
```

这些都是控件正常行为。

如果宿主 App 希望记录业务行为：

```text
Widgetry Event / State
        ↓
App 监听
        ↓
App 自己决定是否写业务日志
```

现有 LongPress 中的：

```text
press
release
drag_end
cancel
trigger long press event
remove LongPressPending
```

等 `info!` 都属于正常交互或内部状态推进，应从长期日志中删除。

---

# 14. 正常内部状态推进不记录

不记录：

- Pending 插入 / 删除；
- hierarchy 正常构建；
- SVG 正常 materialize；
- cache hit / miss；
- 样式正常同步；
- system 正常执行；
- observer 正常执行；
- 内部组件正常变化。

正式日志不能变成：

```text
执行轨迹
```

也不能代替：

```text
profiler
临时 debug tracing
事件系统
```

---

# 15. 持续异常按状态边沿记录

Bevy ECS system 可能每帧执行，因此不能根据 system 调用次数记录日志。

规则：

```text
正常 → 异常       记录一次
异常 → 异常       不重复
异常 → 正常       记录一次恢复
正常 → 再次异常   再记录一次
```

因此日志描述的是：

> 逻辑状态发生变化。

而不是：

> system 又执行了一帧。

恢复日志使用 `info`。

只有此前确实进入并记录过异常状态，才记录恢复。

---

# 16. 日志正文与结构化字段

日志正文统一使用**中文**。

例如：

```rust
widgetry_error!(
    entity = ?entity,
    "ComboBox 内部不变量被破坏：缺少 Popup"
);
```

动态上下文使用结构化字段：

```rust
widgetry_warn!(
    path = %path,
    error = %err,
    "图标资源处理失败"
);
```

字段名保持英文，例如：

```text
entity
path
error
width
height
```

原则：

```text
正文：说明发生了什么
字段：提供定位问题所需的数据
```

---

# 17. Widgetry 不手工记录源码位置

Widgetry 不主动加入：

```text
file
line
module_path
crate_name
```

源码文件和行号由 tracing metadata 自带。

是否显示这些信息由宿主 formatter 决定。

---

# 18. 新增 `rules/logging.md`

新增：

```text
rules/logging.md
```

把本方案中属于长期工程规范的部分写入该文件，包括：

- 什么应该记录；
- 什么不得记录；
- Plugin 注册规则；
- 五个等级的语义；
- 持续异常状态边沿规则；
- 调用方错误边界；
- 被内部吸收错误的处理；
- panic 前的 Widgetry 自身错误日志；
- 统一 `target: "bevy_widgetry"`；
- 统一日志宏；
- 禁止 Widgetry 生产代码直接调用 Bevy 日志宏；
- 中文正文；
- 英文结构化字段；
- 不手工加入源码位置；
- Widgetry 不提供自己的日志配置系统。

---

# 19. `AGENTS.md` 增加日志规则路由

增加类似：

```text
涉及新增、修改、删除日志，或日志相关基础设施：rules/logging.md
```

现有 `AGENTS.md` 已经通过任务类型将开发者/AI 路由到对应规则，因此日志规则沿用这一结构。

---

# 20. Gallery 负责实际日志输出配置

Gallery 是完整宿主 App，因此它负责演示如何消费 Widgetry 日志。

Gallery 使用 Bevy 自己的：

```text
LogPlugin
```

继续统一初始化日志系统。

不自行：

```rust
tracing_subscriber::registry().init()
```

以避免与 Bevy `LogPlugin` 抢占全局 subscriber。

结构：

```text
Bevy LogPlugin
├── fmt_layer
│   └── 终端 formatter
│
└── custom_layer
    └── 文件 formatter
```

也就是：

```text
终端 + 文件同时记录
```

---

# 21. Gallery 新增 `logging.rs`

新增：

```text
gallery/src/logging.rs
```

集中负责：

- 获取本机 UTC offset；
- 本地时间 formatter；
- 创建 `gallery/logs/`；
- 生成日志文件名；
- 创建日志文件；
- 构造 non-blocking writer；
- 保存 `WorkerGuard`；
- 创建终端 fmt layer；
- 创建文件 fmt layer；
- 提供 Gallery 使用的 `LogPlugin` 配置。

`gallery/src/main.rs` 不承载这些细节。

`main.rs` 只负责：

```text
初始化 logging
        ↓
把 LogPlugin 接入 DefaultPlugins
        ↓
启动 App
```

Gallery 当前 `main.rs` 已经承担窗口、Renderer、Plugin 和主题等装配职责，因此将日志配置单独拆出更清晰。

---

# 22. Gallery 日志目录

日志固定保存到：

```text
gallery/logs/
```

路径基于：

```rust
env!("CARGO_MANIFEST_DIR")
```

而不是当前工作目录。

也就是：

```text
<CARGO_MANIFEST_DIR>/logs
```

这样无论用户从哪里执行：

```text
cargo run -p widget_gallery
```

最终都稳定写入：

```text
<repo>/gallery/logs/
```

---

# 23. 每次启动创建一个新日志文件

不使用日志滚动。

不使用：

```text
daily
hourly
minutely
```

每次启动 Gallery 都创建一个全新的 `.log` 文件。

格式：

```text
YYYY-MM-DD_HH-MM-SS-SSS.log
```

例如：

```text
2026-09-12_18-42-07-381.log
```

最后三位是毫秒。

文件名使用**本机时间**。

---

# 24. 日志文件绝不能覆盖

创建文件时必须使用“仅创建新文件”语义：

```rust
OpenOptions::new()
    .write(true)
    .create_new(true)
```

不能使用会覆盖旧文件的：

```rust
File::create(...)
```

因此极端情况下：

```text
两个 Gallery 进程
↓
恰好同一毫秒启动
↓
生成完全相同文件名
```

第二个 Gallery 日志初始化失败。

不会覆盖第一份日志。

---

# 25. 本机时间

Gallery 的日志时间统一使用**本机时间**。

程序启动早期获取本机 UTC offset。

该 offset 同时用于：

```text
日志正文时间
日志文件名时间
```

时间显示精确到毫秒。

日志格式：

```text
YYYY-MM-DD HH:MM:SS.SSS
```

例如：

```text
2026-09-12 18:42:07.381
```

文件名：

```text
2026-09-12_18-42-07-381.log
```

两者属于同一时区。

本次程序运行期间保持启动时取得的 UTC offset。

---

# 26. 获取本机 UTC offset 失败时直接启动失败

不回退到：

```text
UTC
```

也不偷偷使用其他时间。

规则：

```text
获取本机 UTC offset 失败
        ↓
日志初始化失败
        ↓
main() -> Result 返回错误
        ↓
Gallery 不启动
```

因为 Gallery 明确要求记录本机时间，静默切到 UTC 会制造误导。

---

# 27. 日志目录或文件创建失败时直接启动失败

以下任意情况失败：

```text
创建 gallery/logs 失败
创建本次 .log 文件失败
create_new 检测到同名文件
其他日志文件初始化失败
```

均：

```text
main() -> Result
        ↓
返回错误
        ↓
Gallery 不启动
```

不允许静默降级成：

```text
只有终端，没有文件
```

日志设施属于 Gallery 的启动必要条件。

---

# 28. 文件日志使用 non-blocking writer

文件日志使用：

```text
tracing_appender::non_blocking
```

避免磁盘 I/O 阻塞 Bevy 主线程。

大致生命周期：

```text
初始化日志文件
    ↓
non_blocking(file)
    ↓
NonBlocking writer + WorkerGuard
    ↓
writer 交给文件 fmt layer
    ↓
WorkerGuard 一直存活到 app.run() 结束
```

`WorkerGuard` 不得提前 drop。

Gallery 正常日志量很低，因此接受 non-blocking writer 默认缓冲策略。

不为了极端日志洪水场景让 Bevy 主线程同步阻塞等待文件写入。

---

# 29. 终端和文件使用相同过滤规则

终端和 `.log` 文件记录**同一批日志事件**。

不设置：

```text
终端 INFO
文件 DEBUG
```

这种两套等级策略。

统一由 Bevy `LogPlugin` / EnvFilter 控制。

例如：

```text
bevy_widgetry=info
```

对终端和文件同时生效。

需要临时调高 Widgetry 等级时仍然通过正常环境过滤机制处理。

---

# 30. 终端日志格式

终端保持简洁：

```text
2026-09-12 18:42:07.381  INFO   ComboBoxPlugin 注册完成
2026-09-12 18:42:09.024  WARN   图标资源处理失败 path=... error=...
2026-09-12 18:42:12.517  ERROR  ComboBox 内部不变量被破坏：缺少 Popup entity=...
```

终端规则：

- 本机时间；
- 毫秒；
- level；
- 正文；
- 结构化字段；
- 不显示 target；
- 不显示源码文件；
- 不显示源码行号；
- 允许 ANSI level 颜色。

---

# 31. 文件日志格式

文件日志比终端多一个源码位置。

例如：

```text
2026-09-12 18:42:12.517  ERROR  ComboBox 内部不变量被破坏：缺少 Popup entity=42v1 crates/combo_box/src/headless.rs:137
```

规则：

- 本机时间；
- 毫秒；
- level；
- 正文；
- 结构化字段；
- 源码文件；
- 行号；
- 不显示 target；
- 禁止 ANSI escape code；
- 保持纯文本。

这样 `.log` 可以直接交给人或 AI 根据：

```text
文件:行号
```

定位源码。

---

# 32. target 只用于过滤，不显示

虽然 Widgetry 内部所有日志都具有：

```text
target = "bevy_widgetry"
```

但终端和文件 formatter 都隐藏 target。

因此不会输出：

```text
2026-09-12 18:42:07.381 INFO bevy_widgetry ComboBoxPlugin 注册完成
```

而是：

```text
2026-09-12 18:42:07.381 INFO ComboBoxPlugin 注册完成
```

---

# 33. Gallery 外部依赖

根 `[workspace.dependencies]` 增加：

```toml
tracing-appender = "0.2.5"
tracing-subscriber = { version = "0.3.23", features = ["time"] }
time = { version = "0.3.55", features = ["formatting", "macros", "local-offset"] }
```

不使用 `=` 强制锁死补丁版本。

Gallery：

```toml
[dependencies]
tracing-appender.workspace = true
tracing-subscriber.workspace = true
time.workspace = true
```

这样符合项目已有“Workspace 统一管理外部依赖版本”的规则。

用途分别为：

```text
tracing-appender
    → non-blocking 文件输出 + WorkerGuard

tracing-subscriber
    → fmt layer / 时间 formatter 等

time
    → 本机 UTC offset + 时间格式
```

---

# 34. `.gitignore`

现有 `.gitignore` 当前只有：

```text
/target
```

增加：

```text
/gallery/logs/
```

Gallery 启动时自行创建目录。

日志文件不进入 Git。

---

# 35. 清理 Gallery 当前临时日志

当前：

```text
gallery/src/main.rs
```

中存在：

```rust
info!("theme_combo_boxes: {:?}", theme_combo_boxes);
```

这属于开发阶段临时调试输出。

正式启用终端 + 文件日志后应删除，避免每次主题切换污染正式日志。

Gallery 自己以后如果确实需要 App 级日志，可以继续使用正常 Bevy 日志 API；`widgetry_*` 宏属于 Widgetry 库内部统一入口，不是 Gallery 的业务日志 API。

---

# 36. Workspace 和架构文档同步

新增：

```text
crates/log/
```

意味着 Workspace 成员变化。

根 `Cargo.toml` 需要加入：

```text
"crates/log"
```

当前 Workspace 成员列表尚不存在该 crate。

同时 Widgetry 各需要记录日志的生产 crate 增加：

```text
bevy_widgetry_log
```

内部 path dependency。

由于：

- Workspace member 改变；
- 生产依赖图改变；
- 新增共享基础设施 crate；
- Gallery 新增 `src/logging.rs`；

所以必须同步：

```text
docs/architecture.md
```

当前架构文档明确记录 Gallery 源码结构、Infrastructure crate 和 Workspace 内部依赖图。

架构角色中新增：

```text
crates/log
    Widgetry 内部日志基础设施
    提供统一日志宏
    不配置日志 subscriber 或输出
```

---

# 37. 依赖方向

目标依赖关系：

```text
                    bevy_widgetry_log
                    ↑   ↑   ↑   ↑
                    │   │   │   │
asset ──────────────┘   │   │   │
core ───────────────────┘   │   │
button ─────────────────────┘   │
combo_box ──────────────────────┘
text_field ─────────────────────┐
window ─────────────────────────┘
```

`bevy_widgetry_log` 必须保持底层。

尤其不能把统一日志宏放进：

```text
core
```

因为 `core` 当前已经依赖 `asset`，而 `asset` 自己也存在 Plugin，需要使用统一日志宏。`core -> asset` 的现有方向已经存在。

独立 `log` crate 可以避免形成反向依赖或循环依赖。

---

# 38. 实施后的关键验证

实现完成后至少验证：

```text
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
```

同时实际启动：

```text
cargo run -p widget_gallery
```

人工确认：

1. `gallery/logs/` 自动创建；
2. 每次启动产生新的毫秒时间戳 `.log`；
3. 日志正文使用本机时间；
4. 终端和文件同时出现相同事件；
5. 终端有颜色、文件无 ANSI；
6. target 不显示；
7. 文件带源码文件和行号；
8. 终端不带源码位置；
9. `bevy_widgetry=info` 能统一控制 Widgetry 日志；
10. 正常控件交互不会产生 Widgetry 日志洪水；
11. 各 Widgetry Plugin 各产生一次注册完成日志；
12. `WorkerGuard` 生命周期覆盖整个 `app.run()`；
13. 日志目录/文件或本机 offset 初始化失败时 Gallery 不启动。

---

# 39. 最终判断原则

Widgetry 新增一条日志前先问：

> 这是 `bevy_widgetry` 自己需要负责留下、供事后诊断的事实吗？

如果只是：

```text
正常用户行为
正常内部执行
调用方错误
已经向上传递的错误
正常异步等待
正常 fallback
正常 observer 过滤
```

则不记录。

如果是：

```text
Plugin 注册完成
Widgetry 自身 bug
内部吸收且调用方不可见的非预期失败
实际功能降级
已记录异常恢复正常
```

则按照本方案记录。

最终形成：

```text
Widgetry
    ↓
少量、高价值、统一 target 的结构化日志

Gallery
    ↓
Bevy LogPlugin
    ├── 本机时间终端输出
    └── 本机时间 non-blocking 文件输出
            ↓
gallery/logs/YYYY-MM-DD_HH-MM-SS-SSS.log
```
