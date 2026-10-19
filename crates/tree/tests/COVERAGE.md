# tree Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 每 View 的 Expanded / Collapsed、Selection Entity 集合 与 Unknown / Loading / Loaded。
- 可见 / 离屏 Node、View Focus / Disabled、Renderer Subtree。

## Stimuli

- BSN Scene、公开 Expand / Collapse / Selection API。
- ECS Hierarchy / Component Mutation、真实 Pointer / Keyboard / Wheel、Lazy Reply、Theme。

## Guards

- Source / Node / Renderer 有效、配置合法。
- Disabled 阻止用户输入；同值选择不重复通知。
- Expander 不等于 Row Selection。
- BSN 构造中间态可延后同步；完整 Shell 的失效 Source 或配置须记录 ERROR 并传播，Indent 有限非负、Item Height 有限正数。

## Transitions

- Expand / Collapse、Lazy Request / Completion、Selection / Clear。
- 外部 Hierarchy Mutation 后可见顺序重建，删除节点 Repair。
- View Spawn / Despawn 不销毁业务 Node。

## Invariants

- Node Entity 直接作为内部 ListView 的 Row、Selection 和 Active 身份；折叠、展开、滚动复用和重挂接保留原 Entity，行展示深度与子节点标记跟随实际层级。
- View 的 Expanded / List Selection Authority 先提交再通知，自动 Repair 静默。
- 多 View 独立 Expanded / Selection / Active / Focus / Disabled / Scroll，共享原始 ECS 树与 Loading 状态。
- 内部 ListView / Expander 的颜色 Owner 是实际 TreeView；共享或不同 Model 的多个 View 不串用归属，滚动复用、折叠 / 展开和 Renderer 内容替换后仍在颜色消费前就绪。
- Renderer 在动态 Component / Theme 更新时生成正确可见内容。
- Tree 的 View Override、独立 Selection / 展开、外部 Hierarchy Repair 与 Disabled View 组合配色保持一致；Override 清除使用当前 Theme，局部输入不改变其他 View 调色板。
- Collapse 可隐藏仍选中的 Node，删除才清除失效 Selection；Lazy 请求不因重新展开重复，Selection 直接存于内部 ListView，没有 TreeModel 选择副本。
- 新建 / 重建 Renderer 的 Text 在同一次 Update 完成 Font、Visibility、Stack、Measurement 和 Layout；仅准备 Font / SVG 时允许等待，被测操作后不额外推进 Frame。

## Couplings

- Lazy Loading × Expand / Collapse。
- Hierarchy Mutation × Visible Projection / Selection Repair。
- 多 View × Shared Model，独立 Disabled 边界。

## Coverage Map

- `contract.rs`：Scene / Source / Props / Shell 错误处理。
- `model_behavior.rs`：面向 View 的 Headless Expansion、Selection、Lazy 与外部 Hierarchy。
- `view_state.rs`：双 View 独立展开、选择、可见性、隐藏选择与真实删除清理、销毁隔离；共享 Lazy 首次请求一次、加载完成只投影展开 View、Disabled View 程序化操作、无效成员拒绝与队列执行。
- `view.rs`：真实输入、共享 Source、虚拟化、Renderer、Focus / Disabled。
- 共享 Source 基础用例同时覆盖双侧 Pointer 选择与实际 Keyboard Focus 转移、独立 Wheel / Disabled、禁用侧业务 Component 与层级更新、插入 / 删除后的静默 Selection Repair，以及销毁一个 View 保留另一个 View 和业务 Node。
- `rendering.rs`：真实 UI Pipeline 中的新 Text / Expander 同帧准备。
- `view.rs` 检查独立 Override 的共享 View 在展开、选择 / 清空、Focus、禁用侧业务更新、外部层级变化、静默 Repair、Renderer 替换与滚动复用后的颜色；`rendering.rs` 检查缓存调色板下新 Text 和 Icon 的同帧前景。源码局部测试保护稳定零配色解析 / Expander 更新、单按钮 Hover / Pressed / Disabled 移除的局部工作量，以及缺失输出持续 Error 与修复后的稳定状态。
- `view.rs` 同时检查共享 / 不同 Model、滚动与内容替换后的部件 Owner。源码局部测试保护稳定帧零重复归属发现、单部件 Owner 修复、延后挂接与中间子树换父级、嵌套 View、标记移除 / 恢复、后安装 Plugin、View 销毁及业务 Button 不被接管。
- 源码局部测试同时保护稳定 View 零额外 Source / Hierarchy 诊断、共享 Model 移除 / 恢复 / 再次故障、失效 View 销毁不伪造恢复，以及内部列表 / Expander 所属失效后的局部恢复。持续故障仍从实际业务校验传播 Error，健康 View 不产生无依据的恢复信息；`contract.rs` 保留输入前失效 Source 与完整 Shell 故障检查，`rendering.rs` 保留新投影当帧准备验证。

## 覆盖边界

共享 Model 的多 View 输入、数据和生命周期耦合由上述用例保护；Gallery 的中英文实时切换、不同视口展示和离页资源清理由任务级 BRP 验证，不作为库自动化测试结论。

