# 建立 ListModel 与 public type contract

## 目标

先把 ListView 后续所有行为依赖的数据语义和 public type 定稳：什么是一个 item 的稳定 identity、什么变化会触发内容重建、disabled 如何持久保存、offscreen selection/active 如何表达、renderer 如何 type erase、以及 generic `T` 的 runtime system 如何注册。

这一阶段的产物可以还没有真正可滚动的 rows，但后续 virtualization、selection、style 不应再重新定义这些基础语义。

## 范围

新增 `crates/list_view`，package 名 `bevy_widgetry_list_view`。该 crate 后续承载 ListView 的 headless、style、Plugin、公开 type 和测试；本阶段先完成最小 workspace/facade/architecture 接入：

- root `Cargo.toml` 增加 workspace member；
- `crates/bevy_widgetry/Cargo.toml` 增加 path dependency；
- facade 增加 `list_view` module 并 re-export public API；
- `docs/architecture.md` 增加新 crate 角色和 dependency graph；
- Gallery 仍只依赖 facade，不直接依赖新 crate。

生产依赖保持最小：Bevy、core、scroll_area，以及用于不可恢复配置错误诊断的 log。ListView 在语义上明确建立在 ScrollArea 之上，因此依赖 scroll_area 是必要的单向 Widget dependency。

## `WidgetryListModel<T>`

`WidgetryListModel<T>` 是泛型 Component，也是 ListView data 的唯一 source of truth。它不公开裸 `Vec<T>`，而是内部维护 entry：

```rust
struct ListEntry<T> {
    id: WidgetryListItemId,
    revision: u64,
    disabled: bool,
    value: T,
}

pub struct WidgetryListModel<T> {
    items: Vec<ListEntry<T>>,
    next_id: u64,
}
```

### Stable id

`WidgetryListItemId(u64)` 只要求在一个 `WidgetryListModel<T>` 生命周期内唯一，完整 identity 可理解为 `(model_entity, item_id)`。它不等同于业务对象自己的主键。

规则固定为：

- push/insert 新 entry 时由 model-local `next_id` 单调分配；
- remove 后该 id 作废；
- 后续新增不能复用已删除 id；
- `move_item()` 只是位置变化，保留原 entry 的 id；
- remove 后再插入同一个 `T`，仍然得到新 id。

不引入 UUID 或全局 atomic，因为 ListView event 和 model lookup 都已经知道 source model/entity，model-local id 足够。

### Revision

`revision` 只代表 `value: T` 的内容版本，用于解决 Bevy Component change detection 粒度过粗的问题。

调用方通过 `&mut WidgetryListModel<T>` 修改 model 会让整个 Component 进入 Changed；ListView 之后只扫描当前 rendered rows，比较当前 entry 的 `(id, revision)` 与 row 上次渲染的值，从而避免每次 model Changed 都 rerender 所有 visible rows。

`get_mut(index)` 在返回 `&mut T` 前推进目标 entry 的 revision。即使调用方最终没有做语义变化，也允许保守地产生一次 revision 变化；第一版不为了避免这种偶发误报引入复杂 guard API。

`move_item()` 不改变 revision，因为 item content 没变。

### Disabled metadata

`disabled` 是 entry 的持久 metadata，与 revision 正交：

- `set_disabled(index, bool)` 不改变 renderer 内容；
- visible row 只更新 `InteractionDisabled` projection/style；
- item 滚出 viewport 再回来后仍然保持 disabled；
- root disabled 不能改写这里的 entry.disabled。

因此每次 model Changed 时，virtualization 同步逻辑需要独立比较 id/revision/disabled，而不能把 disabled 偷塞进 revision 语义。

### Public thin API

至少提供：

- `len()` / `is_empty()`；
- `get(index)` / `get_mut(index)`；
- `id(index)`；
- `index_of(id)`；
- 按 id 读取 entry/value 的能力；
- `push()` / `insert()` / `remove()` / `clear()`；
- `move_item(from, to)`，其中 `to` 表示移动完成后的最终 index；
- `set_disabled(index, bool)` 与 disabled 查询。

业务代码应通过这些 API 修改 ListModel。整块替换 `WidgetryListModel<T>` Component 值不属于支持 contract，因为那会破坏 model-local id 的持续 identity 语义。

多个 `WidgetryListView<T>` 可以引用同一个 model，但每个 ListView 自己拥有 selection、active、scroll position 和 renderer。

## ListView public type surface

### `WidgetryListView<T>` / Props

`WidgetryListView<T>` 是长期存在的 generic SceneComponent / ECS identity。Props 只表达构造期配置：

- `source: Entity`：必填，指向 `WidgetryListModel<T>`；
- `item_height: f32`：默认 32 logical px，创建后固定；
- `renderer: WidgetryListViewRenderer<T>`：必填，创建后固定。

这些数据若运行时仍有语义，应落到持久 Component field；不能让 Scene props 在运行期继续作为第二份 state。

`item_height` 必须是有限正数。source 必须在 ListView 生命周期内存在且类型匹配；缺少 required prop、非法 item_height 或 source invariant 失效属于配置错误，按项目错误处理规则先记录 ERROR，再终止，而不是静默 no-op。

### Renderer

`WidgetryListViewRenderer<T>` 对 closure 做 type erasure，语义为：

```rust
Fn(usize, &T) -> Box<dyn SceneList>
```

public constructor 可以接受任意 `S: SceneList + 'static`，内部保存成 `Arc<dyn Fn(...) + Send + Sync>`。

Renderer 返回 row wrapper 的 **direct children**，只负责业务内容，不负责：

- Bevy `ListItem`；
- `WidgetryListViewItem`；
- row height/padding/border；
- Hovered/Pressed/Selected；
- disabled；
- foreground/theme。

Renderer scene 必须 owned/'static，不能持有 `&T` 的长期引用。

虚拟 row subtree 生命周期是 ephemeral：滚出 viewport 或 revision 变化后可能被递归销毁并重建。必须跨滚动存活的业务 state 应放到 ListModel 或其他 ECS state，而不是 renderer subtree。

### Logical state

增加 public `WidgetryListViewState`，至少暴露：

```text
selected: Option<WidgetryListItemId>
active:   Option<WidgetryListItemId>
```

原因是 virtualization 下物理 row entity 可能不存在，`Selected`/`ActiveDescendant<Entity>` 只能作为当前 hierarchy projection，不能作为业务权威 state。

index 不需要作为 public authority；内部可以缓存 index 并在结构变化后通过 id 修复。

### Public row marker

`WidgetryListViewItem` 是公开 rendered-row identity component，至少让外部能够读取当前 `id` 与 `index`。它既是 renderer descendant click 向上解析的边界，也是 Gallery/BRP 在不暴露 private runtime 的前提下观察 rendered row count/range 的入口。

rendered revision 等只为内部 reconciliation 服务的字段不应为了测试或 Gallery 扩大 visibility。

## Generic type registration

`WidgetryListView<FileEntry>` 与 `WidgetryListView<Player>` 是不同 Component/system monomorphization，不能靠一个非 generic system 自动处理未来所有未知 `T`。

公共入口固定为：

```rust
app.add_plugins(WidgetryListViewPlugin)
    .register_widgetry_list_view::<FileEntry>()
    .register_widgetry_list_view::<Player>();
```

`WidgetryListViewAppExt` 提供注册方法。一个 `T` 注册一次即可支持任意多个同类型 ListView。更适合在对应业务 Plugin 中注册，而不是把所有业务 item type 堆到 `main.rs`。

`WidgetryListViewPlugin` 负责与 `T` 无关的 common dependency/observer/style infrastructure；`register_widgetry_list_view::<T>()` 负责 typed systems。内部可用 private generic plugin 或等价机制避免同一 `T` 重复注册。

## 本阶段测试

Unit test 贴近 model module，锁定：

- push/insert 生成不同且不复用的 id；
- remove 后 lookup 失效；
- `get_mut()` 只推进目标 revision；
- `move_item()` 保持 id/revision；
- disabled 修改不改变 revision；
- clear 后旧 id 全部失效；
- index ↔ id lookup；
- `move_item(from,to)` 的边界语义。

Integration/compile coverage 验证 public generic registration surface：

- 相同 `T` 不产生重复 typed registration；
- 不同 `T` 可以各自注册；
- facade 能从正常消费路径访问 public type。

这一阶段不为了测试方便把 private entry/runtime type 改成 public。

## 预期产出

得到一个 architecture 接入完整、data identity/revision/disabled/public state/renderer/generic registration 都已确定的 `list_view` crate。它可能还没有生成动态 rows，但后续方案不再需要改变这些基础语义。

## 与前后方案的关系

这是整条执行链的类型基础。下一方案会把这里的 generic SceneComponent/config 接到 ScrollArea；第三、四方案分别依赖 stable id/revision 与 logical state。若这里 identity contract 不稳，后续 virtualization 和 selection 都无法可靠实现。
