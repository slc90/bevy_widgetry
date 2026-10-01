# Phase 1.2 Headless Tree 行为与事件

## expand / collapse

Tree 自己负责：

- expand(entity)
- collapse(entity)
- toggle_expand(entity)
- select(entity)

不负责：
- scroll
- focus
- keyboard navigation

这些由 ListView 体系处理。

## ListView 数据源操作

不要通过 TreeChanged 全量刷新。

Tree 已经知道具体变化：

展开：

visible items insert

收起：

visible items remove

更新：

visible item update

Tree 输出细粒度 datasource 操作给 ListView。

## Tree Event

定义：

- Expanded(Entity)
- Collapsed(Entity)
- Selected(Entity)
- ChildrenRequested(Entity)

事件表达语义，不暴露内部 ListView 操作。

## Lazy Loading

参考 Bevy headless Tree。

不要把 loader 塞进 WidgetryTreeNode。

节点：

Entity
- WidgetryTreeNode
- lazy children state
- business components

流程：

展开节点
-> ChildrenRequested
-> 外部加载
-> 创建 Entity hierarchy
-> Tree 更新 visible items

## 测试

Unit Test：
- flatten 正确性
- depth 计算
- expand/collapse transition
- repeated operation no-op
- selection state

Integration Test：
- TreeModel 与 ListView datasource mutation 协作
- lazy loading 流程
