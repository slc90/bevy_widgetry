# 移除 Button Headless 的自定义无障碍节点

`WidgetryButtonHeadless` 自己处理主键 Pointer 按下、释放、取消、拖动与 `Activate`，以及获得 Focus 后的 Enter / Space。它不依赖 Bevy 原生 `ui_widgets::Button` 的输入处理。自定义 `AccessibilityNode(Role::Button)` 只提供无障碍语义，不负责上述行为，因此可以独立退出。

## 源码调整

`crates/button/src/headless.rs` 当前在 `#[require(...)]` 中主动声明 `AccessibilityNode(accesskit::Node::new(accesskit::Role::Button))`，并引入 `bevy::a11y::AccessibilityNode`。移除这两处项目级声明，保留 `Node`、`Hovered`、默认 `TabIndex(-1)` 及真实输入处理、按压归属与取消清理机制。

`crates/button/Cargo.toml` 移除这一 crate 的 `accesskit.workspace = true` 直接依赖。Button 的其他对外 API、插件与 Scene 组合方式不变；不要为了获得系统默认无障碍角色而额外挂原生 Button，因为现有实现明确避免其副键与激活语义冲突。

`docs/crates/button.md` 的「当前设计思路」中移除 Headless 提供无障碍 Button 角色的表述；其余关于输入、焦点、Content、Loading、LinkButton、Disabled 和 `Activate` 的契约保留。

## 行为和验证

完成后，普通 Button、Headless 组合以及共用 Headless 的 LinkButton / OpenUrl，仍由原先同一根实体接收 Pointer 和聚焦后的键盘输入。通过现有 Button 测试验证主键点击只触发一次、取消/拖动不误激活、副键不激活、Disabled 拦截、Loading 与链接打开行为。与 UI 输入相关的风险可通过 Gallery + BRP 做定向点击与键盘验证，同时检查 ECS 状态和可见反馈。

不新增“必须没有 `AccessibilityNode`”的断言，因为 Bevy 依赖或其他消费者可以独立附带该类型；验收对象是**项目本身不再主动构造无障碍节点**，而不是禁止 ECS 中出现节点。

## 边界

不得改变 Headless 公开类型、`Activate` 来源、输入事件截断、默认焦点可获得性或 Node / Children 结构。无需更改 Gallery 或全局 Bevy Feature。此方案不清理其他 crate，也不移除 Workspace 级 `accesskit` 声明。
