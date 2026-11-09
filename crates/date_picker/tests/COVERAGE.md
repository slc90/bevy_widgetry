# date_picker 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 选中日期、当前展示月份及临时月／年候选；日／月／年三个视图、Today 与边界范围。
- Focus、Disabled、Theme、颜色覆盖和本地化语言。

## Stimuli
- BSN 构造、公开同步／排队选择与浏览、真实 Pointer／Keyboard、Clear、Today 和语言／Theme 切换。

## Guards
- 无效日期、范围、Root／Navigation 结构和失效命令必须拒绝；Disabled 阻止相应用户输入，但程序化更新可用。
- 月年浏览不等同最终选择，不应发送虚假的日期业务通知。

## Transitions
- 选择／清空、跨月／跨年导航、临时候选确认／放弃及三视图切换；语言变更保留现有业务状态。

## Invariants
- 选中日期与展示月份相互独立；日期网格覆盖完整六周投影，包含闰年、月界及不同周起始日。
- 状态先提交再通知；同值与候选浏览静默；Queued 命令以执行时实际状态为准。
- Theme／语言更新不改变 Selection、候选或实体身份，文字标签、邻月日期和清除按钮的配色与可用状态一致。
- Today 由显式可用的日期输入决定；本地日期失败的 UTC fallback 不能冒充业务 Today。
- Headless／独立导航组合不依赖默认子树或无关 Plugin。

## Couplings
- Selection × 展示月份／导航；语言 × 日期投影／标签；Disabled × Pointer／Keyboard；Theme × 动态内容。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 日期／导航／颜色请求的领域错误、诊断身份及 Command 传播。 |
| `headless.rs` | 公开日期状态、日历投影、范围校验与通知。 |
| `independent.rs` | 独立 Headless 场景的日期 API 与状态隔离。 |
| `independent_navigation.rs` | 自定义导航、候选、命令和缺省部件边界。 |
| `language.rs` | 语言切换、星期排列、标签与状态保留。 |
| `navigation.rs` | 日月年视图切换、候选及提交语义。 |
| `view.rs` | 真实 UI 输入、Theme／Override、Disabled 和布局投影。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/model.rs` | 日期来源异常、UTC fallback 与 Today 的独立语义。 |
