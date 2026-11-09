# segmented 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Option 集合、初始 Checked 声明、唯一选中成员、Focus／Disabled、结构故障和恢复。

## Stimuli
- BSN 构造、真实 Pointer／Keyboard、程序 Commands、动态 Option 增删、Theme／Override 与布局改变。

## Guards
- 初始冲突 Checked、空 Group、非法 Child／Index／Root 明确拒绝；Disabled 阻止用户交互。
- 方向键按 Group 主轴导航，跳过 Disabled 并在首尾停止；Tab 不改变当前控件焦点。

## Transitions
- 缺省选择首项或按有效 Checked 初始化；用户改选、批量增删修复、冲突解除和程序化设置。

## Invariants
- 有效 Group 维持唯一 Checked，先提交后发用户通知；程序设置、重复点击和自动 Repair 静默。
- 删除当前 Option 后依既定邻接位置补选，合法新成员与自定义 Option 不被默认样式误写。
- 同一命令批次的非法请求不部分更新或阻断其他健康 Group；错误保留原始身份，恢复后继续工作。
- 自然／等宽布局、Theme／Override 和托管状态配色不改变选择或用户内容。

## Couplings
- 动态 Option × 选择修复；Focus × 方向键；Disabled × 自定义成员；Theme × Highlight／Layout。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 组／成员结构与样式诊断、持续故障和恢复。 |
| `external.rs` | 独立行为插件、自定义 Group／Option 的真实输入。 |
| `headless.rs` | 初始化、选择、Disabled、队列和输入／事件隔离。 |
| `layout.rs` | 自然宽度／等宽布局与动态内容。 |
| `style.rs` | 选中高亮、Hover／Focus／Disabled、Theme 和覆盖色。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/group.rs` | 局部结构维护、后装插件与选择状态。 |
| `src/style.rs` | 部件配色、托管结构和稳定帧行为。 |
