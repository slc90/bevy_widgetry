# table 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Model Row／Column Stable ID、Cell／Row／Column Selection、Focused Cell 与 Source Revision。
- X／Y Viewport、可见／离屏 Cell、Renderer Registry、冻结列、列显示顺序、Sort／Filter／Projection。
- Editing／Clipboard、Header Action、Resize／Reorder Gesture、Theme／Disabled 和各 View 的状态。

## Stimuli
- BSN Scene、Model／Schema CRUD、受控行投影、Renderer 注册／替换和 Column Order 更新。
- 真实 Pointer／Keyboard／Wheel、选择／范围／全选、拖动 Resize／Reorder、编辑／粘贴／复制、Header 排序／筛选。
- Viewport／Scale／Theme／Override 变化与 Source 失效、恢复或销毁。

## Guards
- Source、Schema、Renderer、Stable ID 与当前请求有效；Disabled 和当前编辑焦点决定用户操作资格。
- Resize／Reorder／Selection Gesture 的 Primary Pointer 和目标必须仍存在，失效时清理捕获。
- Clipboard／Editing 的格式、类型、目标与草稿校验失败不得部分写入 Model 或覆盖原有外部内容。

## Transitions
- Model／Projection → 可见 Cell／Header；Selection／Focused Cell 变化、静默 Repair 和 Reveal。
- Resize／Column Reorder 的开始 → 预览／提交 → 结束／取消，受控投影／列顺序的设置与重置。
- Editor 打开 → 草稿／验证 → 提交或取消；Header Sort／Filter 请求；TSV Copy 与 Clipboard 交付。

## Invariants
- Row／Column Stable ID 在移动／重排后不漂移，删除后 Selection 和 Focused Cell 按当前投影修复；状态先提交再通知，同值／自动 Repair 静默。
- 二维虚拟化只创建与当前可见矩形相交的 Cell／Header，重叠部分复用实体；Revision／Type／Renderer 变化只重建相关内容。
- Cell／Header 的实际强类型值、Renderer Registry 和显示行列顺序相符；缺失 Registry 报错，不偷偷用 Fallback 替代。
- 多个共享 Model 的 View 各自维护投影、列顺序、宽度、Selection／Focus／Scroll／Theme，互不修改对方。
- 冻结列与滚动区的横向位置分离、纵向位置同步；不同 DPI／UiScale 与动态布局下 Header、Cell 和行号对齐。
- Resize／Reorder 捕获和提示层在取消、目标删除、失焦、Disabled 和虚拟化变化后正确清理；每次合法手势终止一次。
- 逻辑 Selection／Range／All 包含离屏数据，并按受控显示顺序投影；清空或删除端点后同帧修复，不依赖额外刷新帧。
- TSV Copy 使用完整逻辑选择及强类型格式化，不损坏 Tab／换行／Unicode 等字符；失败不覆盖旧 Clipboard，也不抢占活动 Editor 的复制。
- Editor 的 Focus／输入映射、草稿与正式提交分离；只读、Disabled、类型错误和虚拟化离屏不能产生伪提交。
- Header 排序／筛选和手动列序发出对应 View／Column 身份的请求，未确认前不提前修改 Source；布局样式不破坏状态。
- 动态样式、颜色 Owner 和 Renderer 输出在当帧同步，持续结构故障保留 source／origin 并隔离健康实例。

## Couplings
- X／Y Scroll × Header／冻结区域；Projection × Selection／Virtualization／Column Order。
- Resize／Reorder × DPI／捕获；Editor／Clipboard × Focus／Selection；Renderer Registry × Cell Type／Revision。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `clipboard.rs` | 逻辑选区 TSV、强类型格式化、原生 Clipboard 和 Editor 归属。 |
| `column_order.rs` | 手动列重排、显示顺序、冻结边界和列生命周期。 |
| `copy_diagnostics.rs` | 复制格式化／选区／Clipboard 失败的来源与传播。 |
| `diagnostics.rs` | 公共构造、Selection／Width／Renderer 等错误契约。 |
| `edit_diagnostics.rs` | Editor／类型／提交失败的领域上下文和宿主处理。 |
| `editing.rs` | 真实 Editor、草稿、Focus、ReadOnly／Disabled 和提交／取消。 |
| `frozen.rs` | 冻结列／滚动区对齐、虚拟化、Resize 与编辑内容。 |
| `header.rs` | Header Sort／Filter 真实按钮、请求与确认语义。 |
| `interaction.rs` | 真实选择／焦点、范围／全选、Resize 和 Pointer／Keyboard 手势。 |
| `renderers.rs` | Cell／Header Typed Renderer Registry 与异构内容。 |
| `runtime_diagnostics.rs` | 动态 Runtime／Renderer／Style 持续故障和恢复。 |
| `view.rs` | Scene／Source、Model 投影、宽度、共享 View 状态与布局。 |
| `virtualization.rs` | 二维可见范围、Cell／Header 复用与动态投影。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/view_state.rs` | 手动列顺序、结构变化后的 Stable ID 与投影状态。 |
| `src/clipboard.rs` | Clipboard 提交次数、类型和失败原子性。 |
| `src/editing.rs` | Editor 输入目标解析、生命周期及失效焦点映射。 |
| `src/selection.rs` | Selection／Range 的局部状态与修复。 |
