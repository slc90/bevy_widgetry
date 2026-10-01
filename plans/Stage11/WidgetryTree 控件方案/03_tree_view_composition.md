# Phase 2.1 WidgetryTreeView 组合

## 目标

实现用户直接使用的 Tree widget。

## 组合结构

WidgetryTreeView：

- WidgetryTreeModel
- WidgetryListView<WidgetryTreeVisibleItem>

Tree 不重新实现：
- virtualization
- scroll
- row lifecycle

这些全部复用 ListView。

## TreeRow

TreeRow 只是 view 概念，不新增独立数据结构。

结构：

- left padding(depth)
- WidgetryButton(expander)
- renderer content

## Expander

复用 Button。

默认 icon：
- expand svg
- collapse svg

不要自己实现按钮行为。

## Disable

只支持 TreeView 整体 disable。

传播：

TreeView disabled
-> ListView disabled

不支持 node disable。

## 测试

Integration Test：
- TreeView 创建内部 ListView
- expand 更新 ListView datasource
- collapse 删除对应 rows
- selection 与 view 状态同步
- disabled 后 interaction 无效
