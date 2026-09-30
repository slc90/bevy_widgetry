# 建立泛型 ComboBox 的数据与公共构造契约

## 目标

把 `WidgetryComboBox` 从“构造时接收固定 option factories 的非泛型控件”改造成直接消费 `WidgetryListModel<T>` 的泛型 data-driven 控件，并先稳定它的公共构造、item identity 和 typed registration contract。

这一阶段只解决 ComboBox 的数据来源和公开 API 边界，不负责实现 Field 的完整 projection，也不负责 Popup 的交互生命周期。

## 范围

负责：

- `WidgetryComboBox<T>` 的泛型化；
- 直接复用 `WidgetryListModel<T>`，不新建 ComboBox 专属 Model；
- 新的 `WidgetryComboBoxProps<T>`；
- `WidgetryListItemId` 成为公开 selection identity；
- ComboBox 与 ListView 的明确 crate 组合依赖；
- `WidgetryComboBoxPlugin` 与 `WidgetryComboBoxAppExt` 的 typed registration 结构。

不负责：

- selected item 如何投影到 Field；
- Popup 的显隐、focus、Escape、outside click；
- Gallery 的最终迁移；
- 完整测试重写。

## 预期产出

完成后，调用方能够为一个业务类型 `T` 注册 ComboBox，创建独立 `WidgetryListModel<T>` source，并通过 `@WidgetryComboBox::<T>` 指向该 source 和 renderer。旧的 `@options` / `WidgetryComboBoxOptionFactory` 不再属于新的构造 contract。

## 与前后方案的关系

这是整条执行链的第一步。它定义的数据 ownership、泛型类型和 public API 是后续 selection、Field、Popup、Gallery 与测试工作的前提。下一份方案在这里确定的 `source`、renderer 和 stable id 语义上收敛 selection authority。

---

## 一、总体架构方向

新的 ComboBox 不再自行实现一套列表数据、selection 和 option row，而是直接建立在现有 `WidgetryListModel<T>` 与 `WidgetryListView<T>` 之上。

最终定位：

```text
WidgetryComboBox<T>
=
WidgetryButton
+ WidgetryListView<T>
+ Popup lifecycle / Field projection
```

整体 hierarchy 最终应收敛为：

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

这一结构里，`ComboBoxPopup` 仍然可以作为 ComboBox 专属 ECS identity 存在，但它不再承担列表实现。真正的列表、row identity、selection 和 runtime reconciliation 全部交给 `WidgetryListView<T>`。

## 二、数据模型直接复用 WidgetryListModel<T>

ComboBox 不新增：

```rust
WidgetryComboBoxModel<T>
```

直接使用：

```rust
WidgetryListModel<T>
```

作为唯一 option 数据源。

数据关系为：

```text
source Entity
└─ WidgetryListModel<T>
        │
        ├───────────────┐
        │               │
        ↓               ↓
WidgetryListView<T>   Field projection
        │               │
        └── selection ───┘
```

这样 ComboBox 自动获得 `WidgetryListModel<T>` 已有的：

- stable item id；
- revision；
- `push`；
- `insert`；
- `remove`；
- `move_item`；
- `clear`；
- `get / get_mut`；
- per-item disabled；
- 同一个 model 被多个 View 共享的能力。

ComboBox 不复制这些能力，也不建立第二套 collection abstraction。

这里的原则与 ListView 相同：业务数据拥有独立生命周期，UI hierarchy 只是数据的 projection。动态增删改移不能依赖某一批已经存在的 row entities，更不能让当前 row hierarchy 重新成为 option 数据本身。

## 三、公共构造 API

`WidgetryComboBox` 改为泛型：

```rust
WidgetryComboBox<T>
```

Scene props 调整为类似：

```rust
pub struct WidgetryComboBoxProps<T> {
    pub source: Entity,
    pub item_height: f32,
    pub max_visible_items: usize,
    pub renderer: WidgetryListViewRenderer<T>,
}
```

默认约定：

```text
item_height        = 32 logical px
max_visible_items  = 8
```

`source` 与 `renderer` 必填。

典型使用：

```rust
@WidgetryComboBox::<Item> {
    @source: source,
    @item_height: 32.0,
    @max_visible_items: 8,
    @renderer: {
        WidgetryListViewRenderer::new(|index, item: &Item| {
            // arbitrary SceneList
        })
    },
}
```

不再存在：

```rust
@options
WidgetryComboBoxOptionFactory
```

也不创建 ComboBox 专属 renderer wrapper；直接复用 `WidgetryListViewRenderer<T>`。

继续遵守当前项目的 Scene props 规则：props 只承担 Scene 构造阶段的一次性初始化。`source`、renderer 等在 Scene 展开后仍具有运行时意义的内容必须由持久 Component 保存，不得把 props 自身保留成第二份运行时 state。

## 四、Stable ID 作为公开 selection identity

ComboBox 不再以 index 作为公开 selection value。

用户 selection 通知改为：

```rust
ValueChange<WidgetryListItemId>
```

来源仍为 ComboBox root，而不是要求消费者观察内部 ListView entity。

程序化 API 相应改成 stable id 版本：

```rust
WidgetryComboBox::<T>::set_selected(
    &mut commands,
    combo_box,
    item_id,
);
```

`WidgetryListItemId` 继续保持 ListModel-local identity；所属 model 由 ComboBox 的 `source` 上下文确定。

Index 仅作为当前 model 顺序的临时 projection，不作为 selection identity。

例如：

```text
[A(id=7), B(id=12), C(id=18)]

selected = id12
```

如果执行：

```text
move B -> index 0
```

selection 仍然是 `id12`。

这条约束对整个重构是跨阶段约束：后续 Field projection、Popup 用户选择、Gallery 的事件消费和测试都不得重新把 index 当成选择权威。

## 五、Plugin 与 typed registration

因为 ComboBox 现在泛型化，并包含 `WidgetryListView<T>`，采用和 ListView 相同的 typed registration 模式。

保留：

```rust
WidgetryComboBoxPlugin
```

负责非泛型公共基础设施，并自动安装：

```text
WidgetryButtonPlugin
WidgetryListViewPlugin
WidgetryAssetPlugin
WidgetryIconPlugin
PopoverPlugin
```

旧 ComboBox 自有 popup rows 删除后，不再需要因为 ComboBox 本身而安装 `ListBoxPlugin`；ListView 自身所需基础设施由 `WidgetryListViewPlugin` 管理。

新增：

```rust
WidgetryComboBoxAppExt
```

调用方式：

```rust
app.add_plugins(WidgetryComboBoxPlugin)
    .register_widgetry_combo_box::<MyItem>();
```

`register_widgetry_combo_box::<T>()` 内部：

```text
确保 WidgetryComboBoxPlugin 已安装
→ register_widgetry_list_view::<T>()
→ 注册 TypedComboBoxPlugin<T>
```

调用方不需要再额外手动：

```rust
register_widgetry_list_view::<T>()
```

同一个 `T` 重复注册保持幂等；不同 `T` 各自拥有必要的 typed systems。

## 六、crate 组合依赖

`combo_box` 新增生产依赖：

```text
combo_box --> list_view
```

这不是为了实现方便引入兄弟 crate，而是新的 ComboBox 在语义和组合关系上明确建立在 ListView 之上：ListView 成为它正式的内部子控件和 selection/list behavior provider。

因此这个单向依赖符合当前 workspace 的 Widget crate 约束：只有当一个 Widget 在语义或组合关系上明确建立在另一个 Widget 之上时，才建立必要的兄弟依赖。

不得因此把 ComboBox 专属内容下沉到 `core`，也不得形成 `list_view -> combo_box` 反向依赖。

## 七、需要淘汰的旧数据 API

本次是 architecture replacement，不保留旧数据模型作为兼容层。

在新 API 成立后，以下旧 contract 应退出：

```text
WidgetryComboBoxOptionFactory
@options
ComboBoxOptions
ValueChange<usize>
基于 index 的公开 selection identity
```

后续方案删除旧 row 与 popup 实现时，不需要维护一条旧 API 到新 Model 的兼容桥。这样可以避免同时保留“固定 factory options”和“外部 model source”两套互相竞争的数据来源。
