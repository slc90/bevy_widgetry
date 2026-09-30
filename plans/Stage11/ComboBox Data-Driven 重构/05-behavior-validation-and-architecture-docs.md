# 完成行为验证与架构文档同步

## 目标

以最终的“Button + WidgetryListView<T> + Popup lifecycle / Field projection”架构为准，重写 ComboBox 的行为验证，并同步 `docs/architecture.md` 与 dependency graph，确保仓库文档、测试和真实实现描述同一个控件模型。

这一阶段不再引入新的关键设计，只验证和固化前四个阶段已经确定的 contract。

## 范围

负责：

- ComboBox integration tests 的重构；
- ComboBox 与 ListView 真实组合边界的覆盖；
- dynamic model、stable id、Field projection、Popup lifecycle、disabled、theme 的关键 regression tests；
- Gallery 的运行时验证；
- `docs/architecture.md` 中 ComboBox 描述与 dependency graph 更新；
- 最终确认明确非目标已经从实现中消失。

不负责：

- 重复测试 ListView 自己已经保证的 virtualization、stable-id repair 等内部算法；
- 增加旧 API compatibility tests；
- 再次调整架构方向。

## 预期产出

完成后测试从 crate 外部视角验证新的公共行为，Gallery 可以真实运行新的 model-driven ComboBox，architecture 文档明确记录 `combo_box --> list_view` 的组合关系，并且仓库中不再残留旧 fixed-options architecture 的描述或测试假设。

## 与前后方案的关系

这是执行链的最后一步。它以之前各阶段已经完成的最终实现为对象，不应为了迁就旧测试而恢复已删除的 API 或双重 state。如果验证暴露实现不符合前面方案，应修正实现以回到既定 contract，而不是修改测试去模拟旧行为。

---

## 一、测试总体原则

现有 ComboBox integration tests 应按新 architecture 重写，而不是继续测试已经删除的内部 `ListBox/ListItem` 实现。

测试关注 ComboBox 自身的行为、语义和与 ListView 的真实组合边界。

不重复测试 Bevy 或 ListView 已经单独保证的内部行为。例如 ListView 自己的 visible-range 数学、stable-id repair 细节、row virtualization overlap reuse 不需要在 ComboBox crate 再复制一套测试；但“ComboBox 包含真实 ListView，并因此在大 model 下没有实例化全部 rows”属于组合边界，可以覆盖。

测试继续遵守：

- 新增或改变可观察行为必须有对应测试；
- regression 场景保留明确意图；
- 不为了测试方便扩大私有 visibility；
- 公共行为优先通过 integration test 从 crate 外部验证；
- 私有局部纯逻辑如果存在，再放对应 module unit test。

## 二、必须覆盖的行为

### 1. 泛型组合 hierarchy

验证泛型 ComboBox 正确组合：

```text
WidgetryComboBox<T>
WidgetryButton Field
ComboBoxPopup
WidgetryListView<T>
```

不再断言 ComboBox 自己直接拥有 `ListBox` direct child rows，也不依赖已删除的 `ComboBoxOption` marker。

同时验证 Popup 默认隐藏、Field 几何和 dropdown icon 仍存在。

### 2. source / renderer 构造 contract

验证有效 `source` 与 renderer 能建立 ComboBox；缺失 renderer、错误 source 或 source 上不存在对应 `WidgetryListModel<T>` 时，行为遵循 ListView/ComboBox 已确定的 invariant 与日志策略。

不要重新引入“空 options 在 Scene 构造时 panic”这种旧 factory API 前置条件。空 model 是新的合法状态。

### 3. 初始 selection

覆盖：

- 非空 model 初始化静默选择第一项；
- 初始化不产生用户 `ValueChange`；
- 第一次 Update 前已经显式设置 selection 时不被默认第一项覆盖；
- 空 model 初始化为无 selection；
- 空 model 后续插入第一项不自动选择。

### 4. 用户选择

用户在 Popup 内选择不同 item 后：

```text
内部 WidgetryListViewState.selected 更新
Field projection 更新
Popup 关闭
ComboBox root 发出一次 ValueChange<WidgetryListItemId>
```

通知 source 必须是 ComboBox root，value 必须是 stable id。

### 5. 重选

点击当前已经 selected 的 row：

```text
Popup 关闭
selection 不变
Field children 不重建
不发送 root ValueChange
```

这是一个重要 regression，因为内部 ListView 在同值 selection 时不会发 value-change，ComboBox 必须仍能通过有效 row click 完成关闭。

### 6. programmatic selection

覆盖 stable-id `set_selected`：

- 静默；
- 不关闭已展开 Popup；
- root disabled 时仍允许；
- target item disabled 时仍允许；
- 同值无额外 rebuild；
- 无效 root / foreign id / 已失效 id 为 no-op；
- 同帧多次排队最终收敛到最后有效 selection。

### 7. insert / move 与 stable identity

建立包含多个 id 的 model，选择其中一个，再在它之前 insert 或把它 move 到新 index。

验证：

```text
selected id 不变
Field 仍展示同一业务 item
公开 selection 不因 index 改变而漂移
```

如果 renderer 依赖 index，则 move 后 Field 必须重新 render，证明 projection cache 包含 current index。

### 8. selected item revision

通过：

```rust
model.get_mut(...)
```

修改 selected item。

验证 revision 改变后 Field renderer 重新执行、旧 children 被递归销毁、稳定 `ComboBoxFieldContent` 容器保留。

同时修改非 selected item，验证 Field 不重建。

### 9. selected item 删除

删除当前 selected entry 后：

```text
selection → None
FieldContent children 清空
Button shell 仍存在
Dropdown icon 仍存在
```

如果 model 仍非空，Field 点击仍能打开 Popup 并重新选择。

如果删除导致 model 为空，Popup 不应展开；若删除发生在 Popup 已打开时，Popup 自动关闭。

### 10. per-item disabled

通过 `WidgetryListModel::set_disabled` 设置 item disabled，验证 ComboBox 真实组合中：

- disabled row 使用 ListView 的 disabled projection；
- pointer / Space / Enter 不能用户选择该 item；
- programmatic selection 仍可选择该 item；
- ComboBox 不维护第二份 disabled metadata。

不用重新测试 ListView 内部每一种 disabled style priority，只验证组合边界成立。

### 11. 共享 model

两个 ComboBox 指向同一个 `WidgetryListModel<T>`：

- model CRUD 同时反映到两个 Popup；
- 两个 ComboBox selection 可以不同；
- 一个 ComboBox 用户选择不修改另一个的 `WidgetryListViewState`；
- item revision 更新时，各自 Field 只根据各自 selected id 决定是否重建。

### 12. root disabled

覆盖：

- root 新增 `InteractionDisabled` 后镜像到 Field Button 和内部 ListView；
- 已打开 Popup 立即关闭；
- 用户不能通过 Field 或 ListView 改值；
- model 不被写入 root disabled；
- programmatic selection 继续允许；
- root 初始就 disabled 时，新建内部 child 能读取当前权威 state；
- 同一帧 remove/re-add disabled 不被 stale RemovedComponents 覆盖。

### 13. Popup 高度与 virtualization

验证：

```text
height = min(model.len(), max_visible_items) × item_height + chrome
```

覆盖少量 item、恰好上限、大于上限和动态 shrink/grow。

大 model 下验证 Popup 内实际 rendered rows 只是 viewport 所需数量，而不是 `model.len()` 全部 rows，从组合层证明使用的是真实 ListView virtualization。

### 14. Popup lifecycle

覆盖：

- Field Activate toggle；
- empty model 不打开；
- 用户选择关闭；
- 重选关闭；
- Escape 关闭且 focus 返回 Field；
- outside click 关闭但不抢回 focus；
- 点击另一个 ComboBox 时旧 Popup 关闭、新 Popup 正常打开。

### 15. theme / Popover / dropdown icon

保留现有有价值的 UI contract 验证：

- Popup 使用 Bottom/Top Popover 候选；
- window margin、全宽、共享 popup z-index token 保持；
- `Visibility` 切换驱动 ChevronDown / ChevronUp；
- icon entity/尺寸不因切换而重建；
- Field 完全使用 Button 的 hover/pressed/disabled/theme 配色；
- Popup 外层 background/border 响应 current `ThemeMode` 和 `ThemeChanged`；
- row style 由 ListView 提供，不再测试 ComboBox option style module。

### 16. arbitrary renderer content

继续使用嵌套 SceneList、文本、icon 等内容验证 renderer 在：

```text
Popup row
Field selected projection
```

中各生成独立 UI subtree。

Field 与 row 不能共享同一批 entity；renderer 生成的内容允许被销毁和重建，因此持久业务 state 仍然应该位于 model 或其他 ECS state，而不是 renderer children。

## 三、Gallery 运行时验证

Gallery 不要求增加自动化测试，但涉及展示效果和交互行为的改造必须通过项目既有 GUI debugging 流程做实际运行验证。

至少人工验证：

- Text / Icon + Text / Icon renderer 正常；
- Popup placement 与高度正常；
- 大数据滚动时 rows virtualized；
- 动态 insert/remove/move 即时反映；
- selected item edit 更新 Field；
- selected item 删除后 Field 为空但仍可点击；
- item disabled 与 root disabled 行为正确；
- Theme ComboBox 能切换主题；
- 两个 ComboBox 连续点击时 Popup lifecycle 正常。

Gallery 修改只需保证编译和运行时行为，不需要单独增加 Gallery tests。

## 四、architecture 文档同步

同步更新：

```text
crates/combo_box/Cargo.toml
docs/architecture.md
```

Architecture 文档中的 ComboBox 描述改为表达真实结构，例如：

```text
泛型 data-driven ComboBox；
使用 WidgetryListModel<T> 作为数据来源；
由 WidgetryButton 与 WidgetryListView<T> 组合；
ComboBox 仅负责 Field projection 与 Popup lifecycle。
```

Dependency Graph 增加：

```text
combo_box --> list_view
```

同时删除旧文档中类似：

```text
option 由可重复调用的 SceneList factory 提供
options 固定
Popup 自己保留 Bevy ListBox / ListItem 行为
```

这些已经不再成立的描述。

facade 的 `list_view` 与 `combo_box` module 说明如有旧术语，也要同步到新的 stable-model / generic-composition contract，但 facade 自身不承载行为实现。

## 五、明确不做的事情

最终实现与测试都要确认本次重构没有引入以下内容：

- 新建 `WidgetryComboBoxModel<T>`；
- 新建第二套 item id；
- 新建第二套 renderer abstraction；
- 在 ComboBox root 复制 selection state；
- 修改 ListView virtualization 设计；
- 为 ComboBox 重新实现 keyboard list navigation；
- 保留旧 `options` API 作为兼容层；
- 保留旧 `ValueChange<usize>`；
- 为旧 ComboBox API 做迁移兼容。

这是一次直接 architecture replacement。

## 六、最终职责边界验收

完成后应可以把 ComboBox 理解为：

```text
                   WidgetryListModel<T>
                           │
                           │ source
                           ↓
              ┌────────────────────────┐
              │  WidgetryComboBox<T>   │
              │                        │
              │  ┌──────────────────┐  │
              │  │ WidgetryButton   │  │
              │  │                  │  │
              │  │ selected value ◄─┼──┼──┐
              │  └──────────────────┘  │  │
              │                        │  │
              │  ┌──────────────────┐  │  │
              │  │ ComboBoxPopup    │  │  │
              │  │                  │  │  │
              │  │ WidgetryListView │──┼──┘
              │  │       <T>        │  │
              │  └──────────────────┘  │
              └────────────────────────┘
```

ListView 负责“列表是什么”。

Button 负责“Field 是什么”。

ComboBox 自己只负责：

```text
打开 / 关闭 Popup
selection → Field projection
ListView 用户选择 → ComboBox value notification
```

如果最终测试或代码仍然需要 ComboBox 专属 option collection、row selection、keyboard list navigation 或 index 作为公开 selection identity，说明重构没有真正完成，应回到前面阶段修正，而不是把这些残留合理化。
