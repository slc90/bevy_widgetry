# slider 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 连续／步进值、范围端点、方向／刻度、Disabled、Focus，以及拖动会话和外部交互策略。

## Stimuli
- BSN／Headless 构造、公开 World／Commands 数值 API、真实 Pointer／Keyboard、滑块抓取／释放、Theme／Override。

## Guards
- 数值、范围和步长必须合法且有限；非法 Root／结构／失效命令不部分提交。
- Disabled 阻止用户操作，但合法程序化更新不受阻；非当前 Pointer、非主键和错误捕获不能启动手势。

## Transitions
- 初始化 → 修改值／合法同值 → 通知；拖动 Start → 值更新 → Completed／Cancelled；外部策略可等待或拒绝开始。

## Invariants
- Slider Authority 先提交再通知；非法请求原子拒绝，合法同值不产生额外业务变化。
- 水平／垂直、连续／步进以及窗口 Scale 下的 Thumb 抓取偏移和轨道映射一致，范围端点可达。
- Pointer 取消、失焦、隐藏、Disabled、Capture 丢失等结束会话时，生命周期通知最多一次且已清理捕获。
- 独立 Headless／自定义策略不要求默认 Track／Thumb；用户 Theme／布局更新不改变已提交数值。
- 不同实例输入和诊断归属隔离；错误报告保留原始 source 与请求上下文。

## Couplings
- Pointer Capture × Drag Lifecycle；方向／步进 × 值映射；Disabled × 程序更新；Theme／Layout × Thumb。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 数值、运行时结构、样式及命令错误传播。 |
| `headless.rs` | 独立无视觉的公开 Slider 状态与数值 API。 |
| `input.rs` | 默认真实 Picking／Keyboard、拖动会话和边界行为。 |
| `interaction.rs` | 外部策略、独立 Headless 触发器和手势通知。 |
| `style.rs` | 真实 UI 方向、尺寸、Theme、Override 和 Slider 投影。 |
| `value_api.rs` | 公共 World／Commands 的数值合法性、提交和失败原子性。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/value.rs` | 步进数值、浮点误差和边界校验。 |
| `src/gesture.rs` | 局部拖动方向、手势状态和中断。 |
