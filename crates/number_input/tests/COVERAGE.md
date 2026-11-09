# number_input 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 正式数值／编辑草稿、14 种原生数值类型、精度／格式／步长／范围。
- Editor Focus、ReadOnly、Disabled、IME、Stepper／Repeat 会话及 Theme。

## Stimuli
- BSN 构造、同步／排队数值 API、键盘提交／取消／失焦、真实 Pointer Step、按住 Repeat、Theme／Style 变化。

## Guards
- 不完整 Editor、类型／绑定不匹配、非法数值／步长和失效目标在写入前拒绝；失败保持原数值与草稿。
- ReadOnly、Disabled 和活动 IME 按各自契约限制用户输入；禁用不阻断合法程序化数值更新。

## Transitions
- 草稿编辑 → 验证／提交／取消；Step 单步或持续重复；Focus／Disabled／IME 状态改变中断旧交互。

## Invariants
- 数值以原始类型精度保存，包括整数极值；草稿与正式值分离，通知只对真实提交的值产生。
- 合法同值和边界饱和按既定静默规则处理，Observer 读取已经提交的数值；非法请求不得部分更新。
- Headless 支持外部 Editor／触发器而不依赖默认按钮；Editor 目标重定向不误提交草稿或改变焦点。
- Repeat 遵循固定等待与节奏、退出／取消后停止唤醒；先前会话不能因禁用恢复或重新挂载标记而继续。
- Style／Theme 更新保留草稿、正式值及其通知次数；诊断保留原类型、请求值和底层 TextField 原因链。

## Couplings
- Focus／IME × 草稿提交；ReadOnly／Disabled × 步进；Repeat × 生命周期；外部触发器 × Editor 输入映射。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `custom_repeat.rs` | 独立 Headless 自定义触发器的重复时序、取消和恢复。 |
| `custom_step.rs` | 外部步进触发器、键盘、IME／焦点与数值提交。 |
| `diagnostics.rs` | 原类型、极值、绑定及编辑器错误的领域诊断。 |
| `editing.rs` | 草稿解析、Enter／Esc／失焦、拒绝与恢复。 |
| `formatting.rs` | 原生数值的格式、精度及文本投影。 |
| `headless.rs` | 无默认框架的多层 Editor、公开 API 和状态提交。 |
| `layout.rs` | 默认 Frame、Editor 与步进部件的实际布局。 |
| `modes.rs` | 类型／模式、ReadOnly／Disabled 与编辑契约。 |
| `repeating.rs` | 默认步进长按、时钟、Pointer 会话与停止条件。 |
| `stepping.rs` | 数值步进、边界饱和、按键与触发器状态。 |
| `style.rs` | Theme／Override、托管内容及状态颜色。 |
| `value_api.rs` | 多数值类型的公开 World／Commands API 和一致性。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/number.rs` | 各数值类型的十进制解析、原生范围与非法文本拒绝。 |
| `src/value.rs` | 各整数和浮点类型的原类型安全饱和步进。 |
