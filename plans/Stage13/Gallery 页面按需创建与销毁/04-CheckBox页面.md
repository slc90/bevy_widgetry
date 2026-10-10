# 04｜将 CheckBox 页接入按需生命周期

## 目标

只在进入 CheckBox 时创建复选框 UI，并维持其初始化状态和事件行为。

## 范围

`gallery/src/pages/check_box.rs` 的 UI Scene 和 `initialize_disabled_indeterminate` 系统。

## 预期产出

CheckBox 每次进入都从默认状态呈现，三态/禁用演示正确初始化，退出不留页面实体。

## 与前后方案的关系

承接公共页面协议和 Button 的 UI-only 模式；下一份为 ScrollArea。

## 方案正文

当前 `CheckBoxDemoPlugin::build()` 仅注册 `initialize_disabled_indeterminate` 到 Update，使用 `Added<DisabledIndeterminateDemo>` 将指定复选框初始状态设为 `WidgetryCheckState::Indeterminate`；`on_binary_change` 与 `on_tri_state_change` 在 UI 内事件驱动，不持有单独的 Model Resource。

迁入 `OnEnter(CheckBox)` 创建并挂载 `pages::check_box()`。保留 `CheckBoxDemoPlugin::build()` 中只注册一次的初始化系统，加上 `run_if(in_state(GalleryPage::CheckBox))`。由于 `OnEnter` 发生在 Update 之前且每次会创建新实体，`Added<...>` 能在进入后的首轮 Update 正确完成三态状态投影；不要为重置去复用旧实体或额外缓存按钮。`OnExit` 只同步销毁 UI 根，随之释放 `Checked`、交互态及 ColorShowcase 子树。

验证：每次进入禁用/三态示例都是指定初态，触发事件正常；退出时不存在 CheckBox 页根、返回时所有局部状态重新初始化。

**源代码：** [check_box.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/check_box.rs)。
