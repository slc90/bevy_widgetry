# radio_group 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 动态 Option 集合、唯一 Checked／选中索引、Focus、Disabled、Theme，以及 Headless／默认／自定义成员组合。

## Stimuli
- BSN 构造、公开 Set、真实 Pointer／Keyboard、Option 增删／重挂及 Theme／Override 改变。

## Guards
- Option 必须是所属 Group 的直接 Child；Root、Index 和非空集合有效。
- Disabled 仅阻止用户输入，合法程序化 Set 和自动修复遵循原有状态契约。

## Transitions
- 初始选中、用户改选、程序更新、删改 Option 后按既有邻近顺序修复、空集合失效与重新恢复。

## Invariants
- 每个有效 Group 恰有一个 Checked，跨 Group／自定义 Option 不串扰；用户事件产生时 Checked 已提交，程序更新和 Repair 静默。
- 移除非当前成员保留选中身份；删除当前成员按旧位置选择邻项，空集合恢复到首项。
- 默认样式不覆盖用户自定义 Option 的 Children／颜色；父子颜色 Override、Theme、Focus 在现有成员及后挂载成员上正确投影。
- 多处结构错误独立报告并保留原始诊断，稳定故障不重复记录；健康 Group 不因邻组故障停用。

## Couplings
- 动态结构 × Checked Repair；Focus × 方向键；Group Disabled × Child 输入；Theme × Option 前景／Indicator。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `color_diagnostics.rs` | 组／选项颜色请求与原始 Core 诊断传播。 |
| `diagnostics.rs` | 多组结构错误、持续故障、修复及 Handler 行为。 |
| `headless.rs` | 独立 Headless／自定义成员、焦点与真实输入。 |
| `widgetry_radio_group.rs` | 公开 Group 构造、选择、样式、动态成员和组合交互。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/group_style.rs` | Group 配色与 Owner 的局部管理。 |
| `src/option_style.rs` | Option／Indicator 样式更新与异常修复。 |
| `src/colors.rs` | Group／Option 颜色状态的归属与错误现场。 |
