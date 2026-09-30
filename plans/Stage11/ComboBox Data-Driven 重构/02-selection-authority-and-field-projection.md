# 收敛 Selection 权威并实现 Field 投影

## 目标

在泛型 data-driven ComboBox 已经具有 `source + renderer` contract 的基础上，彻底消除 ComboBox 自己的 selection 副本，把内部 `WidgetryListViewState.selected` 设为唯一真实 selection，并让 Field 始终从这一真实状态与 `WidgetryListModel<T>` 派生展示内容。

这一阶段的核心不是 Popup 行为，而是保证动态 CRUD、revision、move、删除和程序化选择之后，Field 与 selection 始终遵循同一份稳定 identity。

## 范围

负责：

- selection 的唯一 authority；
- 非空初始 model 的默认第一项选择；
- 空 model、删除 selected item 后的无 selection 状态；
- stable id 与 index 的职责区分；
- Field projection cache；
- selected item revision 与 move 对 Field rebuild 的影响；
- 程序化 selection 的语义；
- 多个 ComboBox 共享一个 model 时的独立 View state。

不负责：

- Popup 打开/关闭/focus/Escape；
- Popup 最大高度与 Popover placement；
- Gallery 和最终测试迁移。

## 预期产出

完成后 ComboBox root 不再保存 selection，同一 selected id 同时驱动 ListView row projection 与 Field 内容。Field 内容消失时 Button 外壳仍然完整存在；任何数据变更都只能通过 Model + ListView state 改变展示，不存在 `ComboBox selected` 与 `ListView selected` 的双重同步。

## 与前后方案的关系

本方案承接上一阶段确定的泛型 source、renderer 与 stable id public contract。下一阶段将内部 ListView 真正放入 Popup，并把用户 pointer/keyboard selection 与 popup lifecycle 接到这里已经确定的 selection authority 上。

---

## 一、Selection 的唯一权威

ComboBox root 不保存自己的 selection component。

唯一真实 selection：

```rust
WidgetryListViewState::selected
```

也就是内部 `WidgetryListView<T>` 的 logical selection。

关系为：

```text
WidgetryListModel<T>
        ↓
WidgetryListViewState.selected
        ↓
 ┌──────┴──────┐
 ↓             ↓
Popup row    Field content
```

Popup row 的 `Selected` 仍然只是 ListView 的物理 projection。

Field 同样只是 selection 的 projection。

禁止出现：

```text
ComboBox selected
+
ListView selected
```

这种双重 state。

后续任何通知、Field 更新或 popup close 都不得反过来建立第二份 selection authority。`ValueChange` 是用户交互通知，不是 state source。

## 二、默认 selection

保留现有 ComboBox “非空时默认第一项”的体验，但改成初始化阶段的一次性行为。

规则：

```text
ComboBox 初始化
    │
    ├─ model 非空 + 尚无 selection
    │      → 静默选中 index 0
    │
    └─ model 为空
           → selected = None
```

初始化必须使用一次性 `Initialized` 语义，不能持续强制第一项选中。

因此：

- 首次构造时 model 非空：默认第一项；
- 首次构造时 model 为空：保持无 selection；
- 后来向空 model `push` 第一项：不自动选择；
- selected item 被删除：selection 变为 `None`；
- 不自动选择 successor；
- 调用方在第一次 Update 前已经显式设置 selection 时，初始化不得覆盖。

这里要利用 ListView 自己的 stable-id repair 语义，而不是在 ComboBox 再维护“当前 index”。删除 selected item 后 Field 清空是有效状态，并不意味着 ComboBox 失去交互能力。

## 三、Field projection

Field hierarchy 保持稳定：

```text
ComboBoxField / WidgetryButton
├─ ComboBoxFieldContent
│  └─ selected item 的 renderer children，或为空
└─ ComboBoxDropdownIcon
```

Field 不监听 `ValueChange` 来决定显示什么。

它始终从真实状态派生：

```text
WidgetryListViewState.selected
        ↓
WidgetryListModel<T>
        ↓
renderer
        ↓
ComboBoxFieldContent
```

当 `selected = None` 时，只清空 `ComboBoxFieldContent` 的 children。`ComboBoxField` 自己的 `@WidgetryButton`、Node、尺寸、背景、border 和 dropdown icon 都继续存在，所以 Field 仍然保留完整点击区域和 Activate 行为。没有 selected item 时表现为“选中项区域为空，只保留控件外壳与箭头”，不需要 placeholder 才能维持交互。

## 四、Field projection cache

Field 保存一个私有 projection cache，用来避免每帧无条件重建 renderer 内容：

```text
selected id
current index
revision
```

只有三者之一改变时才重建 `ComboBoxFieldContent`。

这三个维度各自承担不同语义：

- `selected id`：选择是否换成另一个业务 item；
- `current index`：同一个 stable id 是否因为 `move_item` 改变了位置；
- `revision`：同一个 item 的业务内容是否通过 `get_mut` 等路径发生变化。

因为 `WidgetryListViewRenderer<T>` 接口本身接收 `(index, &T)`，即使 value revision 没变，只要 selected item 的 index 改变，也必须重新 render Field，不能只缓存 id/revision。

## 五、selected item 内容修改

```rust
model.get_mut(index)
```

推进该 entry revision。

如果该 item 当前被选中：

```text
revision changed
→ Field renderer 重新执行
```

旧的 Field renderer children 应被递归 despawn，再展开新的 SceneList；`ComboBoxFieldContent` 这个稳定容器 entity 本身保持不变。

如果修改的是非 selected item，则不会导致 Field rebuild。Popup 内该 row 是否重建由 ListView 自己的 revision reconciliation 决定，ComboBox 不参与。

## 六、selected item move

例如：

```text
[A(id=7), B(id=12), C(id=18)]
selected = id12
```

执行：

```text
move B -> index 0
```

selection authority 仍然是 `id12`，不会因为 index 改变而错误选择另一个 item。

但是 renderer 接收 index，因此 Field projection cache 看到：

```text
selected id 相同
revision 相同
current index 改变
```

仍然必须重新 render Field。

这保证 renderer 如果根据 index 决定展示内容，Field 与 Popup row 的 projection 仍然一致。

## 七、selected item 删除

当 selected item 从 `WidgetryListModel<T>` 删除后，ListView 的 state repair 将：

```text
selected → None
```

ComboBox 不自动选择 successor，也不保存旧 value 的展示副本。

Field 随后：

```text
selected = None
→ despawn ComboBoxFieldContent 当前 children
→ FieldContent 为空
```

Button 外壳与 dropdown icon 保持，因此 ComboBox 仍可点击。如果 model 中仍有其他 items，打开 Popup 后可以重新选择；如果 model 已为空，则下一阶段的 Popup lifecycle 规则会禁止展开空 Popup。

## 八、程序化 selection

保留之前的重要语义，但目标从 index 改成 stable id：

```rust
WidgetryComboBox::<T>::set_selected(
    &mut commands,
    combo_box,
    item_id,
);
```

必须：

- 静默修改；
- 不发 `ValueChange`；
- 不主动关闭 Popup；
- root disabled 时仍允许；
- target item disabled 时仍允许；
- 无效 root / 不属于当前 model 的无效 id 为 no-op。

实际 selection 最终仍通过内部 ListView state 完成，ComboBox 不直接维护另一套 state。

同一帧连续排队多个 programmatic selection 时，最终 Field 只需要收敛到最后真实存在的 ListView selection；不应因为中间状态产生用户通知。

## 九、用户通知与 Field state 解耦

后续 Popup 中真实用户选择会由内部 ListView 发出：

```rust
ValueChange<WidgetryListItemId>
```

ComboBox 可以将它重新从 root 发出给消费者，但 Field projection 不依赖这个通知。

这样可以继续满足一个重要 invariant：

> 直接改变真正的 selection state、程序化选择、Model repair 和用户选择都走同一个 Field 派生路径；只有真实用户改值才产生公开通知。

不能重新出现“只有收到 ValueChange 才刷新 Field”的实现，否则 programmatic path 和数据删除/repair 会再次形成漏同步。

## 十、共享 Model

允许：

```text
WidgetryListModel<T>
       │
       ├── ComboBox A
       └── ComboBox B
```

两个 ComboBox：

- 使用相同数据；
- 看到相同 CRUD / revision / disabled metadata；
- 拥有各自独立 `WidgetryListViewState`；
- 可以选择不同 item；
- 后续 Popup scroll/active state 也互不影响。

这与 ListView “Model 与 View 生命周期分离”的原则完全一致。

Model 中没有“ComboBox 当前选择”这一业务字段；selection 属于具体 View。共享 source 不应该导致两个 ComboBox 被迫共享 selection。
