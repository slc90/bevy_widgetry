# Tree 测试总计划

## Unit Test

位置：
对应 module 内 cfg(test)

覆盖：

### Tree Model
- 空树
- 单节点
- 多层树
- flatten projection
- depth invariant

### State
- expanded transition
- selected transition

### Lazy Loading
- unknown
- loading
- loaded

## Integration Test

位置：
crate/tests

覆盖：

### Tree + ListView
验证真实组合：
- datasource mutation
- row 更新

### Interaction
stimulus：
- pointer
- expand button
- selection

### State Coupling
覆盖：
- selection × model mutation
- selection × disabled

### External ECS Change
验证：
- hierarchy 改变后 Tree 更新

## Invariants

保护：

- visible item 顺序与 hierarchy 一致
- selected 使用 Entity identity
- row 是 logical state projection
- disabled 不修改 model state
