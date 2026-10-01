# Phase 3 Style / Gallery / Validation

## Style

Tree 不增加独立 Style。

复用：

- WidgetryListView style
- WidgetryButton style

Tree 自己只有：
- indent width
- expander icon 配置

## Gallery

增加：

### Basic Tree

展示：
- hierarchy
- expand/collapse
- selection

### ECS Heterogeneous Tree

展示：
- 不同 Component
- 不同 renderer

### Lazy Loading Tree

展示：
- unloaded
- loading
- loaded

## 测试规则

Gallery 不编写自动化测试。

只保证：
- 编译通过
- GUI 运行验证

## BRP GUI 验证

验证：

- TreeView 正常显示
- expand/collapse
- selection
- disabled interaction
- virtualization 大量节点
- renderer dispatch
- lazy loading 状态变化
