# message_box Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 未决议 / 已决议 / Closing / 已清理。
- Parent / Modal Blocker、Result Button、Theme 与内容 Layout。

## Stimuli

- BSN 构造、真实 Button Pointer 输入、重复 Action、Result Observer 与 Deferred Operation。
- Parent / Native 生命周期、Disabled Action、Theme 更新。

## Guards

- 有效 Parent 及当前未决议 Result。
- 禁用 Action 不产生有效选择；资源只能按 Owner 归属释放。

## Transitions

- Button → 决议 → Result Event → Observer / Command → Cleanup。
- 重复 Action 保留首次接受的结果；外部结束走资源清理。

## Invariants

- Result Source 为 Dialog Root，最多一次。
- Observer 在清理前可读取状态；其他 Dialog 与 Parent Blocker 不受影响。
- 长正文在有界宽度内换行并可通过 ScrollPosition 滚动，不覆盖独立结果按钮区域；窄窗口下结果按钮换行并保持在窗口范围内，正文和结果按钮的配色与生命周期保持既有契约。

## Couplings

- Result Observer × Deferred Cleanup。
- 多 Dialog × Modal Parent Blocker。
- Theme × Window / Body / Action 内容。

## Coverage Map

- `message_box.rs`：公开构造、真实按钮、一次性 Result、异步清理、Parent 失效、Theme / Layout。
- `message_box.rs` 同时保护默认 Window 标题的显式绑定和标题更新不修改同名正文。
- `message_box.rs` 保护结果按钮 Hover / Pressed 移除、继承 Disabled / 重新启用与 Override 清除，不产生结果或关闭对话框。
- `style.rs` 的工作量回归保护稳定帧零组合配色解析与部件处理，以及单个结果按钮 Hover / 移除只处理自身。
- `scene.rs`、`lifecycle.rs` 源码 Unit Test 补充内部结果顺序与同步 Observer 时机。
- `style.rs` 源码 Unit Test 保护稳定帧零重复归属检查、所有结果组合与业务 Button 隔离、延后挂接 / 容器重挂、Owner 恢复与标记移除释放、销毁清理、后安装 Plugin 补偿，以及缺失受管 Button 的 Error / 重试 / 诊断去重和恢复。
- `diagnostics.rs` 覆盖非法 resolve 的 Root / 请求结果 / 缺状态分类、单次源头与真实 Command / Handler 的同 origin 和 Severity::Error；Cancel 与重入重复决议静默，Observer 命令在清理前执行。多个结果按钮结构失败各自保留 origin，真实修复恢复、销毁不伪恢复；下层 Window 故障保留原 origin / 原因链与补充上下文，非法嵌套配色保留 Core cause / origin 与字段路径且不提交。
- `lifecycle.rs` Unit Test 验证已识别结果 Action 缺少决议状态时抵达真实 Observer Handler，不创建 Closing。`style.rs` Unit Test 验证派生样式 / 调色板丢失后的独立诊断、补建后重新绑定已有部件并重新应用实际配色。

