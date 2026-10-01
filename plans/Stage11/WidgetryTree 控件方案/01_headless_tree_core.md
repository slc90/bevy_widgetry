# Phase 1.1 Headless Tree 核心模型

## 目标

实现不依赖 UI 的 Tree 核心能力。

## 新增 crate

新增：

crates/tree

保持和现有 widget crate 一致的模块组织：

- model.rs
- behavior.rs
- registration.rs
- lib.rs

## ECS 数据结构

### WidgetryTreeNode

只作为 marker component：

```rust
#[derive(Component)]
struct WidgetryTreeNode;
```

不保存：

- expanded
- selected
- children
- disabled

原因：

- hierarchy 使用 Bevy Parent/Children
- UI 状态属于 TreeState
- 业务数据属于用户自己的 Component

## WidgetryTreeModel

核心：

```rust
struct WidgetryTreeModel {
    root: Entity,
    state: WidgetryTreeState,
    visible_items: Vec<WidgetryTreeVisibleItem>,
    entity_to_index: HashMap<Entity, usize>,
}
```

职责：

- 从 root 遍历 ECS hierarchy
- 根据 expanded 状态生成平铺列表
- 维护 Entity 到 visible index 的映射

## WidgetryTreeState

```rust
struct WidgetryTreeState {
    expanded: HashSet<Entity>,
    selected: Option<Entity>,
}
```

只保存 UI state。

## WidgetryTreeVisibleItem

内部投影数据：

```rust
struct WidgetryTreeVisibleItem {
    entity: Entity,
    depth: u16,
    has_children: bool,
    expanded: bool,
}
```

这是 Tree 到 ListView 的桥接数据，不是业务 Node。
