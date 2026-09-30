# 用 WidgetryListView 替换 Popup 列表实现

## 目标

删除 ComboBox 当前自己维护的 `ListBox/ListItem/ComboBoxOption` 列表实现，让 Popup 直接组合一个真正的 `WidgetryListView<T>`。ComboBox 仅保留 Popup chrome、显隐、focus 转移、选择后关闭、重选关闭和 outside click 等组合层职责。

## 范围

负责：

- 最终 Popup hierarchy；
- 删除旧 `ComboBoxOption` row implementation；
- Popup 高度约束；
- ListView 在 Popup 内的 layout patch；
- Field Activate / model empty 行为；
- 用户选择与重选后的关闭语义；
- Escape、outside click、另一个 ComboBox 点击；
- root disabled 到 Button/ListView 的镜像；
- item disabled 直接复用 ListView；
- ListView 的滚动、虚拟化和 keyboard navigation 进入 ComboBox。

不负责：

- 再设计新的 list abstraction；
- 修改 ListView virtualization 本身；
- Gallery 最终迁移与测试清单收口。

## 预期产出

完成后 ComboBox 内部不再存在第二套 option rows、`Selected` 管理或 option style。Popup 的列表行为全部由 `WidgetryListView<T>` 提供；ComboBox 只实现普通 ListView 不拥有的下拉控件语义。

## 与前后方案的关系

本方案依赖上一阶段已经确定的唯一 ListView selection authority 与 Field projection，否则 Popup 与 Field 会再次出现双状态。完成后内部 architecture 已基本稳定，下一阶段可以删除旧模块、调整 crate 依赖并迁移 Gallery 消费者。

---

## 一、最终 Popup hierarchy

新的 ComboBox hierarchy：

```text
WidgetryComboBox<T>
│
├─ ComboBoxField
│  └─ @WidgetryButton
│     ├─ ComboBoxFieldContent
│     │  └─ 当前 selected item 的 renderer projection
│     └─ ComboBoxDropdownIcon
│
└─ ComboBoxPopup
   ├─ Visibility
   ├─ Popover
   ├─ popup background / border / z-index
   │
   └─ @WidgetryListView<T>
      └─ rows...
```

`ComboBoxPopup` 仍然存在，但它不再是一个列表实现。

它只负责：

- Popup 的显隐；
- Popover placement；
- popup background / border；
- z-index；
- Popup 最大高度约束。

真正的列表完全由 `WidgetryListView<T>` 提供。

因此删除当前：

```text
ComboBoxOptions
ComboBoxOption
option.rs
ComboBox 自己的 Selected 管理
ComboBox 自己的 option style
ComboBox 自己的 ListBox row 构造
```

如果前一阶段已经先移除了 `ComboBoxOptions` 这个数据副本，那么这里继续删除它残留的 UI 依赖即可；不得为了 Popup 实现又重新保存 factories 或 row index 作为数据来源。

## 二、内部 ListView 完整接管列表职责

内部 ListView 与普通 ListView 使用同一套：

- stable identity；
- selection；
- active item；
- row renderer；
- pointer behavior；
- keyboard navigation；
- item disabled；
- ScrollArea；
- virtualization；
- row style；
- revision reconciliation。

ComboBox 不再自己实现这些功能。

这意味着 ComboBox 的 Popup 不仅“看起来像 ListView”，而是 architecture 上真正成为 ListView 的组合使用者。今后 ListView 对 stable id、revision、virtualized rows 和 input behavior 的修复应自然被 ComboBox 继承，而不是在 ComboBox 维护相似但逐渐分叉的另一套逻辑。

## 三、ListView 在 Popup 内的组合级 patch

Popup 中 ListView 做少量组合级 patch：

```text
width  = 100%
height = 100%
root border width = 0
TabIndex = -1
```

Popup 外层继续负责自己的 popup border/background，因此内部 ListView 不应再画第二层 control border。

`TabIndex = -1` 用于防止内部 ListView 成为额外的顺序 Tab stop；它仍然可以在 Popup 打开后被 ComboBox 程序化 focus。

这些 patch 是 ComboBox 对一个现有 Widget 的组合配置，不应反过来修改普通 `WidgetryListView<T>` 的默认视觉或 Tab 行为。

## 四、Popup 高度

ListView 需要有界 viewport，而当前 ComboBox Popup 是随内容直接展开，所以 ComboBox 必须明确承担 Popup 高度约束。

高度由：

```text
visible_rows = min(model.len(), max_visible_items)

content_height =
    visible_rows × item_height
```

再计入 Popup chrome 得到最终高度。

例如默认：

```text
item_height = 32
max_visible_items = 8
```

则：

```text
3 items   → 约 96px content
8 items   → 约 256px content
10000     → 仍约 256px content
```

大数据量直接使用 ListView 的 ScrollArea + virtualization，不构造全部 option entities。

model 长度变化时 Popup 高度也要收敛到新的可见行数，但不能通过重新创建整个 ListView 来完成。Popup geometry 是由 model length 派生的组合 layout state。

model 为空时 Field Activate 不展开一个空 Popup。

Popup 已经打开时如果 model 变为空，则自动关闭。

## 五、Field Activate

Field 继续复用完整 `WidgetryButton`。

Field Activate：

```text
Field Activate
→ 检查 root disabled
→ 检查 model 非空
→ toggle Popup Visibility
```

即使当前 `selected = None`、`ComboBoxFieldContent` 没有 children，Button 自己仍然存在完整点击区域，因此只要 model 非空就可以打开 Popup 重新选择。

打开时将 focus 移交内部 ListView，使其已有的：

```text
ArrowUp / ArrowDown
Home / End
Space / Enter
PageUp / PageDown
```

可以直接工作。

这里不为 ComboBox 重新实现一套 keyboard navigation。

## 六、用户选择不同 item

内部 ListView：

```text
更新 WidgetryListViewState
→ 发出内部 ValueChange<WidgetryListItemId>
```

ComboBox：

```text
接收内部通知
→ 关闭 Popup
→ 从 ComboBox root 重新发出 ValueChange<WidgetryListItemId>
```

Field 内容由上一阶段定义的 state projection system 更新，不依赖通知。

公开消费者只需要监听 ComboBox root，而不需要知道内部 ListView entity。

通知值使用 stable `WidgetryListItemId`，不能重新转换成 index。

## 七、重选当前 item

ListView selection 不发生变化，因此不产生 value change。

ComboBox 仍要识别这个有效 row click：

```text
关闭 Popup
不发送 root ValueChange
不重建 Field
```

保留当前 ComboBox 的 reselect 行为。

这里不能简单只监听 ListView 的 `ValueChange` 来关闭 Popup，否则点击已选项不会产生通知，Popup 就无法关闭。

实现可以观察 Popup 内真实 row 的 primary click，但判断必须基于它确实属于该内部 ListView / 当前 ComboBox，避免其他嵌套或 foreign row 冒充本控件的有效重选。

## 八、Escape

Popup 打开且 ListView 获得 focus 时：

```text
Escape
→ 关闭 Popup
→ focus 返回 Field
→ selection 不变
```

Escape 属于 ComboBox 的 Popup lifecycle，而不是普通 ListView selection behavior，因此留在 ComboBox typed runtime。

关闭过程中不发 `ValueChange`，也不重置 active/selected。

## 九、Outside click 与多个 ComboBox

保持现有规则：

```text
点击 ComboBox hierarchy 外部
→ 关闭 Popup
```

用原始 pointer target 判断 hierarchy，内部任意 descendants 均算 ComboBox 内部点击。

如果用户点击另一个 ComboBox：

```text
旧 ComboBox 的 outside-click 路径关闭旧 Popup
新的 Field click 再打开新 Popup
```

最终一次点击应能完成“关闭旧下拉 + 打开新下拉”。

outside click 不强制把 focus 抢回 Field，避免破坏被点击目标本来应该获得的 focus。只有 Escape 这种明确的 ComboBox 取消行为要求 focus 返回 Field。

## 十、Disabled 语义

### ComboBox root disabled

`InteractionDisabled` 仍只要求调用方设置在 ComboBox root。

ComboBox 将其镜像到：

```text
WidgetryButton
WidgetryListView<T>
```

同时关闭已经打开的 Popup。

不修改：

```text
WidgetryListModel<T>
```

程序化 selection 仍允许。

镜像逻辑必须继续处理“root 早已 disabled，内部 child 后创建”的初始化情况，以及同一帧 remove/re-add disabled 时最终权威 state 不能被 stale removed event 覆盖的边界。

### item disabled

不再由 ComboBox 自己实现。

直接使用：

```rust
WidgetryListModel::set_disabled(...)
```

ListView 已负责：

- pointer 禁止选择；
- Space / Enter 禁止选择；
- disabled style；
- programmatic selection 仍允许。

因此 ComboBox 本次改造顺带获得真正的 per-item disabled。

## 十一、Dropdown icon 与 theme

Popup `Visibility` 继续作为 open state 的唯一来源，dropdown icon 不保存第二份 open bool。

```text
Visibility::Visible → ChevronUp
Visibility::Hidden  → ChevronDown
```

Field 仍使用完整 `WidgetryButton` 的 hover/pressed/disabled/theme style；Popup 打开不额外把 Field 改成 active 配色。

Popup 外层的 background / border 继续由当前 `ThemeMode` 初始化，并响应 `ThemeChanged`；内部 rows 的 selected/hover/disabled/foreground 由 ListView 自己的 style 系统处理，不再保留 ComboBox `option.rs` 那套样式解析。

## 十二、不修改 ListView 的泛用语义

ComboBox 需要有界 Popup、隐藏内部 root border、排除顺序 Tab navigation 等需求，优先通过 BSN composition 和 Node/component patch 表达。

不得为了让 ComboBox 更方便而把普通 ListView 改造成 ComboBox-specific behavior，例如：

- 不让普通 ListView 点击 selection 后自动隐藏；
- 不让普通 ListView 默认 `TabIndex = -1`；
- 不把 Popover 逻辑下沉到 ListView；
- 不把 ComboBox root disabled 语义塞进 ListView；
- 不改变普通 ListView 的公开 notification source。

两者共享的是 ListView 已经拥有的 list/view 行为，而不是让 ListView 感知 ComboBox。
