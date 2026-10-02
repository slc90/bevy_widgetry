# 性能 Benchmark 规则

本文件定义 Widgetry 与 App 的性能验证义务、benchmark 代码归属、测量方法与完成条件。

性能包括 latency、throughput、CPU / GPU 成本、allocation、memory、资源增长以及 App startup，不限于大数据或 virtualization。

## 适用范围与触发条件

以下任务必须读取本规则，明确性能目标，并运行或补充与改动直接相关的 benchmark：

- 新增有明确性能要求的 Widget 或能力，例如 Table、ListView、Tree、波形显示、实时绘图与流式数据处理。
- 修改性能敏感路径，例如 virtualization、model projection、layout、rendering、数据转换、downsampling、buffer 管理、增量更新与高频输入。
- 修改每次 update 执行的 system，且成本可能随数据量、entity 数量、输入速率或运行时间增长。
- 修改会直接影响 App startup 的初始化路径，例如 plugin 装配、初始 Scene 构造、asset 加载与 renderer 初始化。
- 修复性能问题，或声称改动改善了性能。

性能验证按当前任务新增、修改及直接影响的场景选择，不要求每次运行整个 Workspace 的全部 benchmark。

不涉及性能敏感路径的任务可以不运行 benchmark，但应在任务进度记录中说明不适用的原因。不得因为行为测试通过或 GUI 看起来流畅而免除已经触发的性能验证义务。

## 验证职责

- Unit test / integration test 保护行为、语义与明确的工作量 invariant。
- Benchmark 测量执行成本，比较不同规模、输入负载与版本的性能。
- BRP GUI 验证检查真实用户输入、可观察表现与交互体验。

三者不能互相替代。Headless benchmark 只能证明其实际包含的 CPU / ECS / UI 工作，不能据此宣称完整 rendering 或端到端交互性能已经通过。

可确定性表达的性能 contract 应使用普通 regression test 保护，例如无变化时不重新调用 renderer、physical entity 数量受 viewport 约束、buffer 容量有界。不得把受机器噪声影响的耗时断言直接加入普通行为测试。

Gallery 的一般自动化行为测试例外继续适用，但不免除本规则触发的 App startup 或 GUI 性能 benchmark 义务。

## Benchmark 代码归属

需要长期重复运行、比较或保护的 benchmark 必须以代码或可重复执行的脚本纳入仓库，不得只留下手工计时步骤或一次测量结果。

- 单个 crate 的 benchmark 默认放在该 crate 的 benches/，由拥有该行为的 crate 维护。
- 跨 crate 的 benchmark 放在拥有真实组合行为的 crate，遵守现有 dependency 方向。
- App startup 与完整 GUI pipeline 的 benchmark 放在对应 App，由 App 维护；Gallery 的 App benchmark 归 gallery/，不得让库反向依赖 Gallery。
- 共享 fixture 优先复用已有测试基础设施；确有多个消费者需要时再提取，不提前新增通用 benchmark crate。

Benchmark 应通过真实生产路径测量。不得为了 benchmark 扩大私有实现的 visibility、增加没有业务意义的 public API，或绕过本次需要验证的 scheduling、更新与 rendering 阶段。

测量 framework 与 dependency 按当前需求选择，并遵守 [依赖规则](dependencies.md)；不因本规则提前引入 dependency。

## 场景与指标设计

实施前应明确：

- 需要满足的用户场景、目标负载与性能预算。
- 输入规模、输入速率、viewport、数据内容以及运行时长等相关维度。
- 被测操作、计时边界、包含和排除的阶段。
- 关键指标、baseline 与可接受变化的判断方法。

场景应覆盖典型负载、目标负载和有意义的压力负载，不得只使用小数据或最轻 renderer 证明全部场景的性能。数据内容、renderer 复杂度、visible / hidden 与 update mode 等环境必须与结论相符。

不同负载维度应分别变化，以识别成本来源；仅覆盖存在实际 coupling 的组合，不机械展开完整笛卡尔积。

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

### App 首次启动

App 必须建立可重复执行的首次启动 benchmark；新增 App 或修改直接影响 startup 的路径时，运行相关场景。

先构建用于测量的优化 executable，再由 App benchmark harness 启动该 executable。编译、Cargo 检查、工具发现、MCP / BRP 请求往返与正常退出耗时不得计入 startup。

计时起点必须明确为 harness 发起进程创建请求之前的 monotonic timestamp；从 main 内部开始的阶段计时只能作为分段诊断，不能替代完整 startup。

默认终点为主要界面内容完成首次显示，且 App 已能接受目标交互。场景必须定义主要内容、必需 asset 和交互就绪条件，并提供能证明这些条件的 readiness 观测。进程存在、window 创建、App::update 返回或 BRP 连接成功均不能单独作为该终点。

Harness 与 App 的计时必须使用可比较的 clock，或明确包含 readiness 通知传输开销；轮询延迟及其测量误差必须记录。不得用任意 sleep 时长作为 startup 时间。启动失败或超时应记录为失败，不能丢弃后仅报告成功样本；readiness 后应正常关闭本次进程，shutdown 不计入 startup。

首次启动与后续启动分开报告：

- 首次启动使用预先定义的新用户配置与 App 自有 cache 状态，每个样本恢复相同初始状态；需要一次性生成的用户数据或 cache，其成本应包含在 startup 中。
- 后续启动保留规定的用户配置与 cache，单独测量已有状态下的 startup。
- OS file cache、GPU / driver cache 与 App 自有 cache 的控制范围必须说明。新进程或新用户配置不自动等于完整 cold start；未控制的条件应如实记录，不得宣称完整 cold start 已验证。

记录启动参数、初始页面 / Scene、数据负载、window 配置、renderer backend 与 readiness 定义。若另行测量 window 首次出现或首帧提交，应作为单独阶段，不能替代默认终点。

## 测量方法与可重复性

- 使用明确记录的优化构建；crate benchmark 默认使用 cargo bench 的 bench profile，App benchmark 使用与目标运行方式相符的优化 executable。修改前后使用相同构建配置。
- 记录版本 / working-tree 状态、操作系统、CPU / GPU、Rust toolchain、profile / features、测量工具、场景参数与执行命令。
- 初始化、稳态操作与 cleanup 分开计时，除非其中某项正是被测场景。保证 fixture 每轮恢复到规定 state，或使用明确的连续 workload，避免后续样本变成 no-op 或因 state 累积改变工作量。
- 稳态 benchmark 应预热，并重复采样；首次启动场景不得通过预热改变规定的首次状态。避免与构建、其他 benchmark 或已知重负载并行执行。
- 保证被测结果实际被使用，避免被编译优化消除；计时路径中的日志与 instrumentation 开销应受控，并记录与实际 App 配置的差异。
- 记录样本数与波动，报告适合场景的统计量。需要 P95 / P99 等 tail latency 时，应有足以支撑该统计量的样本，不从少量运行中宣称可靠的 tail latency。
- 不只报告 FPS；按场景结合 latency、throughput、frame / update 耗时、allocation 和资源增长。CPU 完成不代表 GPU 完成，需要 GPU 结论时必须测量对应阶段。

GUI 性能测量必须保留目标 App 的实际 update mode、rendering 配置与交互路径。不得为了提高 BRP 响应速度改变 Gallery 全局运行模式，再把结果当作实际性能。BRP 请求耗时与 screenshot 捕获耗时不能直接当作 Widget 的交互或 rendering 耗时。

构建 profile 与 cache 测量方法可参考 [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) 和 [Hyperfine 的 warm / cold cache 说明](https://github.com/sharkdp/hyperfine#warmup-runs-and-preparation-commands)；引用工具不代表要求采用该工具，也不意味着命令执行到退出的耗时等同于 GUI readiness。

## Baseline、Regression 与完成条件

已有场景应在相同测量环境、负载与计时边界下比较修改前后。新增能力没有历史 baseline 时，应建立初始 baseline，并检查目标负载下的性能预算。

Baseline 必须能追溯到源码版本、场景与测量配置。版本控制中保留 benchmark 代码、必要参数、baseline 摘要与结论；体积较大的原始报告可保存为 artifact，但必须在结果记录中说明位置。不得自动覆盖 baseline 后将 regression 隐藏为新常态。

可接受变化应根据场景预算与实测噪声确定，不统一规定所有场景使用同一个百分比或 frame budget。出现可重复、超出噪声范围的 regression，必须调查原因，完成修复或取得明确的性能 trade-off 决策后才能将任务标记为完成；Codex 不得自行放宽已确定的预算或阈值。

性能 bug 修复必须有能复现问题的负载、修改前后测量与长期回归保护。能够表达为确定性工作量 invariant 的问题同时增加 regression test；无法合理表达时记录原因，保留可重复执行的 benchmark。

涉及 GUI 性能的任务仍按 [GUI 验证规则](gui-debugging.md) 验证真实用户场景；直接驱动 Component 的 benchmark 不等于真实输入的端到端验收。

任务进度记录与最终结果必须明确列出实际执行的场景、负载、指标、baseline 比较、预算判断，以及未验证部分。测量条件缺失或结果不可靠时，应记录性能验证未完成，不能用估计、功能测试或单次手工体验代替通过结论。
