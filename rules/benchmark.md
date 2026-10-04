# 性能 Benchmark 规则

本文件定义 Widgetry 与 App 的性能验证义务、benchmark 代码归属、测量方法与完成条件。

性能包括 latency、throughput、CPU / GPU 成本、allocation、memory 以及资源增长，不限于大数据或 virtualization。

## 性能验证要求

必须明确性能目标，并运行或补充与改动直接相关的 benchmark。

性能验证按当前任务新增、修改及直接影响的场景选择，不要求每次运行整个 Workspace 的全部 benchmark。

不涉及性能敏感路径的任务可以不运行 benchmark，但应在任务进度记录中说明不适用的原因。不得因为行为测试通过或 GUI 看起来流畅而免除已经触发的性能验证义务。

## 验证职责

- Benchmark 测量不同规模与输入负载下的执行成本，判断是否满足当前性能要求。
- BRP GUI 验证检查真实用户输入、可观察表现与交互体验。

两者不能互相替代。Headless benchmark 只能证明其实际包含的 CPU / ECS / UI 工作，不能据此宣称完整 rendering 或端到端交互性能已经通过。

## Benchmark 代码归属

需要长期重复运行、比较或保护的 benchmark 必须以代码或可重复执行的脚本纳入仓库，不得只留下手工计时步骤或一次测量结果。

- 单个 crate 的 benchmark 默认放在该 crate 的 benches/，由拥有该行为的 crate 维护。
- 跨 crate 的 benchmark 放在拥有真实组合行为的 crate，遵守现有 dependency 方向。
- 完整 GUI pipeline 的 benchmark 放在 gallery/，由 Gallery 维护；不得让库反向依赖 Gallery。
- 共享 fixture 优先复用已有测试基础设施；确有多个消费者需要时再提取，不提前新增通用 benchmark crate。

Benchmark 应通过真实生产路径测量。不得为了 benchmark 扩大私有实现的 visibility、增加没有业务意义的 public API，或绕过本次需要验证的 scheduling、更新与 rendering 阶段。

测量 framework 与 dependency 按当前需求选择，并遵守 [依赖规则](dependencies.md)；不因本规则提前引入 dependency。

## 场景与指标设计

实施前应明确：

- 需要满足的用户场景、目标负载与性能预算。
- 输入规模、输入速率、viewport、数据内容以及运行时长等相关维度。
- 被测操作、计时边界、包含和排除的阶段。
- 关键指标与性能预算的判定方法。

场景应覆盖典型负载、目标负载和有意义的压力负载，不得只使用小数据或最轻 renderer 证明全部场景的性能。数据内容、renderer 复杂度、visible / hidden 与 update mode 等环境必须与结论相符。

不同负载维度应分别变化，以识别成本来源；仅覆盖存在实际 coupling 的组合，不机械展开完整笛卡尔积。

### GUI 流畅度

在明确记录的目标硬件与负载下，GUI 的持续交互、动画与实时显示默认应稳定达到 60 FPS，对应 frame budget 约 16.7 ms。静止界面按需更新时，不要求持续渲染。

同时记录 frame 耗时与长帧情况，不能仅凭平均 FPS 判定流畅度通过。

### 大数据与增量更新

根据行为选择首次构建、无变化 update、scroll、局部 mutation、layout 变化、rebuild 与销毁等场景。

对 Table 等二维 Widget，分别变化 Row 与 Column 数量，并固定 viewport 比较数据规模增长时的成本；另外测量 viewport 增长的影响。仅确认可见 entity 数量有界，不能证明数据遍历、projection、allocation 或每次 update 的成本有界。

### 实时显示与流式处理

波形、实时绘图等 Widget 应根据真实需求分别变化 channel 数量、输入 sample rate、保留时间范围、viewport 与更新频率，并覆盖持续输入和有意义的突发输入。

根据任务保护以下相关指标：

- 输入与处理 throughput，以及是否能持续跟上目标输入速率。
- 数据产生到可观察显示的 latency；仅测量 CPU 更新时必须明确其边界。
- update / frame 耗时及必要的 tail latency。
- dropped sample / frame、backlog，以及 downsampling 或丢弃策略是否仍满足数据语义。
- allocation、buffer / memory / entity 数量，以及持续运行后的增长趋势。

不得通过减少输出质量、悄悄丢弃数据或改变已确定行为来取得性能改善。语义允许的 downsampling、backpressure 或丢弃策略必须明确，且有对应行为测试。

## 测量方法与可重复性

- 使用明确记录的优化构建；crate benchmark 默认使用 cargo bench 的 bench profile，App benchmark 使用与目标运行方式相符的优化 executable。
- 记录版本 / working-tree 状态、操作系统、CPU / GPU、Rust toolchain、profile / features、测量工具、场景参数与执行命令。
- 初始化、稳态操作与 cleanup 分开计时，除非其中某项正是被测场景。保证 fixture 每轮恢复到规定 state，或使用明确的连续 workload，避免后续样本变成 no-op 或因 state 累积改变工作量。
- 稳态 benchmark 应预热，并重复采样。避免与构建、其他 benchmark 或已知重负载并行执行。
- 保证被测结果实际被使用，避免被编译优化消除；计时路径中的日志与 instrumentation 开销应受控，并记录与实际 App 配置的差异。
- 记录样本数与波动，报告适合场景的统计量。需要 P95 / P99 等 tail latency 时，应有足以支撑该统计量的样本，不从少量运行中宣称可靠的 tail latency。
- 不只报告 FPS；按场景结合 latency、throughput、frame / update 耗时、allocation 和资源增长。CPU 完成不代表 GPU 完成，需要 GPU 结论时必须测量对应阶段。

GUI 性能测量必须保留目标 App 的实际 update mode、rendering 配置与交互路径。不得为了提高 BRP 响应速度改变 Gallery 全局运行模式，再把结果当作实际性能。BRP 请求耗时与 screenshot 捕获耗时不能直接当作 Widget 的交互或 rendering 耗时。

构建 profile 与 cache 测量方法可参考 [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) 和 [Hyperfine 的 warm / cold cache 说明](https://github.com/sharkdp/hyperfine#warmup-runs-and-preparation-commands)；引用工具不代表要求采用该工具，也不意味着命令执行到退出的耗时等同于 GUI readiness。

## 完成条件

每次 benchmark 应依据当前用户场景、目标负载与测量条件，检查实测结果是否满足已确定的性能预算。

除上述 GUI 默认目标外，其他性能预算应根据当前场景确定。实测结果未满足预算时，必须调查原因，完成修复或取得明确的性能 trade-off 决策后才能将任务标记为完成；Codex 不得自行放宽已确定的预算或阈值。

性能 bug 修复必须有能复现问题的负载、修复后的测量结果与长期回归保护。能够表达为确定性工作量 invariant 的问题同时增加 regression test；无法合理表达时记录原因，保留可重复执行的 benchmark。

涉及 GUI 性能的任务仍按 [GUI 验证规则](gui-debugging.md) 验证真实用户场景；直接驱动 Component 的 benchmark 不等于真实输入的端到端验收。

任务进度记录与最终结果必须明确列出实际执行的场景、负载、指标、预算判断，以及未验证部分。测量条件缺失或结果不可靠时，应记录性能验证未完成，不能用估计、功能测试或单次手工体验代替通过结论。
