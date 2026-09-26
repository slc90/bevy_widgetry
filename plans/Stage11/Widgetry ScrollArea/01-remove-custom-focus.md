# 去除 WidgetryFocusPlugin，切换到 Bevy 官方 focus 行为

## 目标

删除 Widgetry 自己维护的“主 pointer 点击非 EditableText 就清空 focus”全局策略，让 Widgetry 不再拥有一套与 Bevy 官方 focus 并行的全局规则。保留各 Widget 真正需要的局部 focus 适配，并通过 Bevy 0.19.1 的 InputFocus / InputDispatch / TabNavigation 体系完成 pointer focus、focus 清理和 focused keyboard dispatch。

## 范围

本方案只处理现有 focus 基础设施迁移，不实现 ScrollArea 本身。

直接受影响的位置包括：

- crates/core/src/focus.rs 与其测试。
- crates/core/src/lib.rs 的导出与 crate 说明。
- crates/bevy_widgetry/src/lib.rs 的 style facade 导出。
- crates/bevy_widgetry/tests/focus.rs 与 public_api.rs。
- crates/text_field/src/style.rs 对 WidgetryFocusPlugin 的依赖和 plugin 装配。
- crates/radio_group/src/lib.rs 对 WidgetryFocusPlugin 的依赖和 plugin 装配。
- docs/architecture.md 中对共享 pointer focus policy 的描述。

## 设计

### 删除共享自定义 focus policy

删除 WidgetryFocusPlugin、core/focus.rs、对应 core focus integration test，以及 facade 对 WidgetryFocusPlugin 的公开导出。

这属于明确的 breaking public API change：之后不存在 Widgetry 自己的全局 pointer focus plugin。

### 使用官方 TabNavigationPlugin 承担 click-to-focus

Bevy 0.19.1 中：

- DefaultPlugins 提供 InputFocusPlugin 与 InputDispatchPlugin。
- TabNavigationPlugin 提供 Pointer<Press> → AcquireFocus 的 click-to-focus observer，以及 AcquireFocus 对最近 TabIndex ancestor 的解析。
- TabIndex(-1) 可以通过 AcquireFocus 直接获得 focus，但不会进入顺序 Tab navigation。

因此需要 pointer focus 的 Widgetry plugin 应确保 TabNavigationPlugin 已安装；不重复安装 InputFocusPlugin / InputDispatchPlugin，真实应用继续依赖 DefaultPlugins 或调用方等价装配。

### TextField

WidgetryTextFieldPlugin：

- 移除 WidgetryFocusPlugin 安装。
- 改为在未安装时添加 TabNavigationPlugin。
- 保留现有 retain_text_field_focus_on_acquire 逻辑。Widgetry TextField 本身没有 TabIndex；EditableText 官方 pointer 行为先把 focus 设置到文本实体后，TabNavigationPlugin 仍会触发 AcquireFocus。这个 observer 继续负责在当前 TextField 已经获得 focus 时截住 AcquireFocus，避免事件继续到 Window 后把刚获得的 focus 清掉。
- 不因此让 TextField 自动进入顺序 Tab navigation。

测试环境如果使用 MinimalPlugins / scene_app 而不是 DefaultPlugins，需要显式装配 InputFocusPlugin，或改用已有 text_input_app；不要通过重新引入 Widgetry focus state 来满足测试。

### RadioGroup

WidgetryRadioGroupPlugin：

- 保留现有 TabNavigationPlugin 自动装配。
- 删除 WidgetryFocusPlugin import、安装和 rustdoc 描述。
- 其他 RadioGroup focus / TabGroup 行为保持不变。

### 官方行为改变

去掉自定义“点击非 EditableText 一律 clear”后，具有 TabIndex(-1) 的 Widget（例如 WidgetryButton）在 TabNavigationPlugin 存在时会成为 pointer focus target，而不是必然把 focus 变成 None。这是切换到 Bevy 官方 focus 语义后的预期行为，不应继续保留旧断言。

点击没有任何 TabIndex ancestor 的普通 Node，则 AcquireFocus 最终到达 Window 并清除当前 focus。

## 自动化测试

按 TDD 重写受影响的真实组合测试，重点验证 Widgetry 与官方 focus 的组合边界，而不是重复测试 Bevy 内部算法：

- 点击 WidgetryTextField 后获得并保持 TextField focus。
- 从 TextField 点击 WidgetryButton 后，focus 转到 Button 的 TabIndex(-1) entity。
- 从 TextField 点击没有 focusable ancestor 的普通 Node 后，focus 被清除。
- RadioGroup 仍能自动获得 TabNavigationPlugin 且既有 keyboard / selection 测试不回归。
- facade public_api 不再引用 WidgetryFocusPlugin。

删除只验证旧自定义 clear policy 的测试；不保留为了兼容已删除 API 而存在的空壳。

## 文档同步

docs/architecture.md 中 core 不再描述“共享 WidgetryFocusPlugin”；TextField 与 RadioGroup 的 role 描述改为官方 focus / TabNavigation 关系。不要把本次迁移扩展成整个库的 accessibility 或 keyboard policy 重构。

## 预期产出

项目中不再存在 WidgetryFocusPlugin；需要 pointer focus 的现有 Widget 使用 Bevy 官方机制；TextField、RadioGroup、Button 的组合行为有新的回归测试保护，后续 ScrollArea 可以直接复用同一 focus 模型。

## 与前后方案的关系

这是整条链的第一步。后续 ScrollArea 的 TabIndex(-1)、pointer focus 和 keyboard dispatch 都依赖这里确定的官方 focus 基线。完成后进入 ScrollArea headless 设计与实现。
