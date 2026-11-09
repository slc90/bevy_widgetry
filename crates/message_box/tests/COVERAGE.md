# message_box 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 未决议／已决议／Closing／已清理；Parent、Modal Blocker、Result Button、Theme 和内容布局。

## Stimuli
- BSN 构造、真实按钮 Pointer／Keyboard、重复 Action、Result Observer／Command、Parent／Native 生命周期和 Theme 更新。

## Guards
- Parent 有效、Result 尚未决议；禁用 Action 不能产生业务选择；资源回收必须遵循 Owner。

## Transitions
- Button Action → 结果决议 → Result Event／Observer → Deferred Cleanup；外部关闭也回收自有资源。

## Invariants
- Result Source 为 Dialog Root，最多一次；Observer 在清理前能读取已提交结果。
- 重复 Action 不覆盖首次有效决议；多个 Dialog 和 Modal Parent Blocker 相互隔离。
- 长文本在有界宽度内换行／滚动，窄窗口结果按钮保持可达；Theme／Override 不能修改结果或内容身份。
- 失效托管部件及嵌套 Window 错误保留原始原因与 Owner，健康 Dialog 仍继续工作。

## Couplings
- Observer × Deferred Cleanup；多个 Dialog × Modal Parent；布局／Theme × 动态按钮内容。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 非法结果、托管结构、颜色和下层 Window 错误传播与恢复。 |
| `message_box.rs` | 公开构造、按钮决议、一次性结果、清理和真实 Theme／Layout。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/scene.rs` | 结果构造和内部状态建立顺序。 |
| `src/lifecycle.rs` | 决议、Observer、Closing 与重入异常。 |
| `src/style.rs` | 托管部件、颜色投影、性能边界和故障恢复。 |
