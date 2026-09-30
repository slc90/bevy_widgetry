# 建立 selection、active、focus 与 disabled 行为

## 目标

在 row entity 会随着 virtualization 创建/销毁的前提下，实现完整且稳定的单选 ListView headless behavior：pointer、keyboard、programmatic selection、focus、disabled、删除/重排后的 state 修复和 accessibility 都基于 stable item id，而不是依赖当前 visible row entity。

## Logical state 与 physical projection

`WidgetryListViewState` 中的：

```text
selected: Option<WidgetryListItemId>
active:   Option<WidgetryListItemId>
```

是权威 logical state。

内部可额外缓存 selected/active 当前 index 以便 Arrow 与 scroll math，但 id 才是 identity authority。结构变化后先检查 cached index 上的 id 是否仍匹配；不匹配时才通过 `index_of(id)` O(n) 修复。Structural CRUD 本身已经是可接受的较低频操作，不需要维护额外全局 id map。

当前 ECS row 只做 projection：

- row.id == selected -> 挂 `Selected`；
- row.id == active 且 root 有 focus -> root `ActiveDescendant(Some(row_entity))`；
- active item offscreen -> physical ActiveDescendant 为 None，logical active 保留；
- item 再滚入或 root 再获得 focus -> 恢复 physical projection。

因此调用方可以通过 `WidgetryListViewState` 查询 offscreen selection/active，而不依赖当前 row 是否实例化。

## Focus 与 accessibility

Root 是唯一 Tab stop，使用 `TabIndex::default()`；rows 不参与顺序 Tab navigation，避免长列表每一行都成为 tab stop。

初始：

```text
selected = None
active   = None
```

单纯 FocusGained 不自动把 active 设置到第一项，保持我们已确认的“没有 active 就没有 active”。FocusLost 只清 physical ActiveDescendant，不清 logical active。

Root 自己提供 `AccessibilityNode(Role::ListBox)` 与 Bevy `ActiveDescendant`，但 **不挂官方 ListBox marker**；row 使用官方 `ListItem`，继续获得 `Role::ListItem` 与 `Selectable`。

## Pointer 行为

Pointer click 可以发生在 row wrapper 或 renderer 任意 descendant 上。处理时沿 ancestor 向上找到最近的 `WidgetryListViewItem`，从而允许 renderer 任意组合 Text/Icon/Node。

普通 enabled row click：

- root 获得 focus；
- active = clicked item id；
- selected = clicked item id；
- 如果 selected 真正发生变化，root 发 `ValueChange<WidgetryListItemId>`，`is_final=true`；
- 点击已经 selected 的 row 不重复发 selection notification，但仍确保 active/focus 正确。

Disabled item click：

- active 可以变成该 item；
- selected 保持不变；
- 不发 ValueChange。

Renderer 内部如果以后包含自身可交互控件，需要由该内部控件自行停止 pointer event propagation；ListView 不猜测 descendant 业务语义。

## Hover 与 Pressed

Hover 直接使用 Bevy `Hovered`，不复制一份 Widgetry hover state。

普通 `ListItem` 不会像 Button/Menu 一样自动获得 Bevy `Pressed` lifecycle，因此 ListView 自己响应 pointer press/release/cancel，对 row 增删 **Bevy 现成 `Pressed` component**。不定义 `WidgetryPressed`。

Root/item disabled 时不进入 pressed visual state；release/cancel 必须正确清理已有 Pressed，避免虚拟 row 或 disable 切换后残留。

## Keyboard navigation

只有 root 有 focus 且 effective root 未 disabled 时处理。

第一版只支持纵向，因此：

- ArrowDown：active 向后一项，末尾 wrap 到首项；
- ArrowUp：active 向前一项，首项 wrap 到末项；
- Home：active 到 index 0；
- End：active 到最后一项；
- Space / Enter：将当前 active 设为 selected；active item disabled 时 no-op；
- PageUp / PageDown：按一个 viewport 高度修改 `ScrollPosition.y`，active 不变。

当 `active=None`：

- ArrowDown / Home -> 首项；
- ArrowUp / End -> 末项；
- Space / Enter -> no-op。

Disabled item **不会被 Arrow 跳过**；它可以成为 active，只是在用户提交 selection 时被拒绝。这与我们采用的 active/selected 分离语义一致。

当 keyboard 导航目标不完全可见时，通过固定行高直接计算 item 的 top/bottom 并修改 `ScrollPosition`，不需要先有目标 row entity。virtualization 在 scroll position 更新后创建对应 row，再恢复 ActiveDescendant projection。

## Programmatic selection

公开 API：

```text
WidgetryListView::<T>::set_selected(commands, list_entity, index)
```

行为固定：

- invalid list entity / invalid index -> no-op；
- selected 指向目标 entry；
- active 同时指向同一 entry；
- 即使 selected 已经是该 item，但 active 不同，也要修正 active；
- ensure target visible；若完全可见则不改 ScrollPosition；若部分/完全不可见则按固定行高滚到可见位置，之前讨论倾向 top-align 并 clamp；
- root disabled 时仍允许；
- item disabled 时仍允许；
- 不发送用户 `ValueChange`。

Programmatic API 表示应用主动设置 state，不受 `InteractionDisabled` 的“禁止用户输入”语义限制；这与现有 ComboBox/RadioGroup 的 programmatic selection contract 保持一致。

## Structural data change 后的 state 修复

Selection 与 active 独立处理。

### Remove

- selected item 被删除 -> `selected=None`；
- active item 被删除 -> 优先选择删除位置上滑入的 successor；
- 如果删除的是最后一项，没有 successor -> predecessor；
- 列表变空 -> `active=None`；
- 如果 selected/active 原本是同一 item，则分别应用上述规则，不人为绑定两者。

### Insert

Stable id 未变，selected/active 保持同一个 logical item；只更新 cached index。

### Move/reorder

`move_item()` 保持 entry id/revision，因此 selected/active 都按 id 跟随原 item 到新 index；不发送用户 selection event。

### Clear

所有原 id 失效，selected/active 都变 None。

## Root disabled

在 ListView root 插入 `InteractionDisabled` 只代表 **禁止用户交互**，不冻结 ECS state。

Root disabled 时：

- pointer click/press 不改变 active/selected；
- keyboard 不改变 active/selected；
- 用户 wheel/trackpad scroll 应被阻止；
- hover/pressed 等 interaction visual 被 disabled style 覆盖；
- programmatic `set_selected` 仍有效；
- programmatic 直接修改 ScrollPosition 仍有效；
- ListModel CRUD 仍有效。

实现上需要把 effective root disabled 投影到当前 rendered rows 和 ScrollArea 的用户交互面，但 **不能**把 model entry 的持久 `disabled` 改成 true。解除 root disabled 后，每个 row 恢复其 entry 自身 disabled state。

## Item disabled

Entry disabled 来自 `WidgetryListModel<T>` metadata：

- 可以 active；
- click / Space / Enter 不能让它成为用户 selected；
- programmatic set_selected 可以选择；
- 滚出/滚回保持；
- enable/disable visible row 时不 rerender renderer content，只改变 interaction/style projection。

如果 programmatic selected item 后来被 disable，logical selected 可以保留；disabled style 优先，不因 disable 自动清 selection。

## Selection event

真实用户 selection change 统一从 ListView root 发：

```text
ValueChange<WidgetryListItemId>
```

使用 stable id 而不是 index，避免 event 刚发出后 insert/remove 就使 index 失去 identity 含义。调用方可以通过 source model + id lookup 获得当前 value/index。

Programmatic selection 和结构变化不发这个 event。

## 本阶段测试

### Unit test

状态机/纯逻辑重点：

- 初始 selected/active None；
- active=None 时四个导航键的起点；
- Arrow wrap；
- Space/Enter selection；
- disabled active 不可用户 select；
- remove selected；
- remove active successor/predecessor；
- move 后 stable id 跟随；
- cached index 校验/修复；
- ensure-visible scroll target 与 clamp。

### Integration test

使用真实 ECS event/input 组合覆盖：

- click row 和 click renderer descendant；
- click 已 selected 不重复通知；
- click disabled 只 active；
- Focus/ActiveDescendant 的 visible/offscreen projection；
- Arrow/Home/End/Space/Enter；
- PageUp/PageDown 只 scroll、不改 active；
- `ValueChange<WidgetryListItemId>` source/value/is_final；
- `set_selected()` 静默并自动滚入；
- root disabled 拦用户输入但 programmatic API 仍可用；
- item disabled 的 user/programmatic 差异；
- offscreen active 滚回后恢复 projection。

测试从 public state/behavior 观察，不依赖 private runtime component。

## 预期产出

完成完整可用的 headless ListView behavior：virtualization 下 selection/active 不丢失，pointer/keyboard/programmatic 语义一致，root/item disabled 与 accessibility 有明确边界。

## 与前后方案的关系

本方案建立在第三方案的 rendered row projection 上。下一方案只读取这里已经确定的 Hovered/Pressed/Selected/InteractionDisabled/focus/active state 来计算视觉，不再修改行为语义。
