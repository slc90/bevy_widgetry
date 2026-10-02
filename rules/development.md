# 开发流程规则

本文件定义仓库修改任务的进度记录要求，以及项目进行代码开发时必须遵循的基本开发流程。

`crates/` 下的库代码开发任务都应遵循：

```text
理解任务
    ↓
Type-Driven Development
    ↓
Test-Driven Development
    ↓
实现完成
    ↓
必要重构
```

开发过程中优先通过 type 和测试明确设计，再编写具体实现。

`gallery/` 作为 Widgetry 的展示应用，不要求采用 Test-Driven Development，也不要求为其行为编写自动化测试；其测试要求以 `rules/testing.md` 中的 Gallery 例外为准。

## 任务进度记录

### 适用范围

实际修改仓库的任务都必须创建任务进度记录，包括代码、配置和文档修改。纯咨询、只读分析无需创建。

### 创建时机与位置

读取适用规则、明确任务目标与范围后，在实施修改前于仓库根目录创建名为 <任务名称>-进度.md 的 Markdown 文件。

进度记录是本任务的临时执行记录，不写入 plans/，不替代源码、正式文档或验证证据，也不作为最终提交内容。

继续同一任务时复用已有记录；不得覆盖其他任务的进度记录。

### 记录内容

记录必须包括：

- 任务目标与范围边界。
- 实施步骤及各步骤的完成状态。
- 当前状态、阻塞问题与待办事项。
- 实际执行的验证及结果，包括尚未执行或不适用的验证。
- 适用的独立 Code Review 结果及 findings 的处理情况；无需 Review 时说明原因。

### 持续更新

每完成一个实施步骤、取得验证或 Review 结果，或发生任务范围变化时，及时更新记录。

记录必须反映实际完成情况。未执行的验证不得标记为通过，未解决的阻塞或 findings 不得标记为完成。

### 中断与续接

任务暂停、执行中断或尚未提交时保留记录。

继续任务时先读取记录，并核对实际工作区状态与已有验证证据，再更新进度并继续执行；不得仅凭记录中的勾选判断任务已经完成。

### 清理与提交

完成实施、必要验证与适用的独立 Code Review 后，在提交前删除本任务的进度记录，并按 rules/git.md 检查最终提交范围。

任务完成但尚未提交时继续保留记录；任务被取消时清理本任务创建的记录。

整体流程为：

```text
理解任务、读取规则
    ↓
创建任务进度记录
    ↓
设计、实施并持续更新记录
    ↓
必要验证与适用的独立 Code Review
    ↓
删除任务进度记录
    ↓
提交
```

## GUI 行为的运行时验证

当当前行为具有可观察 GUI 表现，或必须通过实际 pointer / keyboard / focus / picking / layout 等交互确认时，在相关自动化测试通过并完成必要重构后，按照 `rules/gui-debugging.md` 使用 Widget Gallery 进行运行时验证。

因此涉及 GUI 行为的开发循环可以表现为：

```text
Type
→ Red
→ Green
→ Refactor
→ BRP GUI 验证
```

BRP GUI 验证是条件阶段。纯逻辑修改或能够完全由自动化测试覆盖、且不改变 GUI 可观察行为的修改，不需要为了流程形式启动 Gallery。

BRP GUI 验证按照本次任务新增、修改及直接影响的真实用户场景选择，不要求重新执行模块完整长期测试覆盖。对于动态 UI 行为，应根据 rules/gui-debugging.md 额外关注 temporal behavior。

## 性能验证

涉及性能敏感路径、实时显示、流式处理、App startup、性能修复或性能改善声明时，按 [性能 Benchmark 规则](benchmark.md) 设计场景、确定预算并执行性能验证。

实施前明确负载、指标与计时边界；已有能力在修改前取得 baseline，修改后按相同条件比较。新增能力建立初始 baseline。性能 bug 修复应保留可重复执行的 benchmark，并对可确定性表达的工作量 invariant 增加 regression test。

Benchmark 是按需性能验证阶段，不替代 Type-Driven Development、Test-Driven Development 或适用的 BRP GUI 验证。性能验证范围限于当前任务直接影响的场景。

进度记录必须包含性能验证是否适用、实际场景与负载、测量结果、baseline 比较、预算判断以及未验证部分；不适用时说明原因。

---

## Type-Driven Development

开始实现功能之前，先从数据和 type 出发确定设计。

首先明确当前功能涉及的：

* 核心 data structure；
* Component、Resource、Event、Message 等 ECS type；
* enum 与 state 表示；
* 输入与输出；
* type 之间的关系；
* 必要的公共或 crate 内部接口。

优先让 type 表达业务语义和约束，而不是先编写行为代码，再根据实现结果补 type。

典型流程：

```text
理解需要表达的概念
        ↓
确定 state 和数据
        ↓
设计 type
        ↓
确定 type 之间的关系
        ↓
确定必要接口
        ↓
进入 Test-Driven Development
```

### Type 设计原则

type 应尽量直接表达实际语义。

如果某种 state 在语义上具有明确区别，应优先考虑通过 type system 表达，而不是依赖隐含约定、特殊值或松散组合。

例如：

```text
明确 state
→ enum / 独立 type

有独立语义的数据
→ 独立 struct / Component

跨 system 共享 state
→ 根据语义选择 Component / Resource

一次性交互或通知
→ 根据 Bevy 当前语义选择 Event / Message 等机制
```

不要为了追求 type safety 而制造没有实际价值的 wrapper type、generic 或抽象层。

type 设计只需要满足当前已经明确的需求和约束，不提前为假设中的未来需求设计复杂扩展能力。

### 先确定 type，不等于一次设计完所有实现

Type-Driven Development 的目标是先确定当前功能的基本 data model 和接口形状。

它不要求在实现前预测所有内部细节。

如果后续测试或实现证明原有 type 设计不合理，可以调整 type，再继续开发。

不得因为已经开始写测试或实现，就把早期 type 设计视为不可修改。

---

## Test-Driven Development

基本 type 设计明确后，使用 Test-Driven Development 开发行为。

复杂 state 模块进入具体 test case 之前，先识别当前行为涉及的 state 维度、stimuli、guards、invariants 与跨 state couplings，再依据 rules/testing.md 选择当前 Red 阶段需要的 unit / integration test。

不要求一次设计完整功能的全部 state space，仍以当前小步保持 Type → Red → Green → Refactor 循环。

采用：

```text
Red
↓
Green
↓
Refactor
```

### Red

在实现新的可观察行为之前，先编写能够表达该行为的测试。

测试首先应该失败，并且失败原因应当对应当前尚未实现的行为。

测试用于明确：

* 当前需要实现什么行为；
* 输入与预期输出；
* 关键边界；
* 需要保持的 invariant。

不要先完成实现，再补一个只用于覆盖现有代码的测试。

### Green

编写满足当前失败测试所需的实现。

这一阶段优先让行为正确，不为了提前追求完美结构而扩大修改范围。

只实现当前测试和任务真正要求的能力。

```text
失败测试
    ↓
最小必要实现
    ↓
测试通过
```

如果实现过程中发现 type 设计无法合理表达行为，应回到 Type-Driven Development 阶段调整 type，而不是使用临时绕过方式强行完成实现。

### Refactor

测试通过后，可以在测试保护下进行必要重构。

重构只能改善当前实现，例如：

* 消除已经出现的重复；
* 改善命名；
* 简化 control flow；
* 调整已经证明不合理的内部结构；
* 提取已经具有明确职责的代码。

重构不得改变已经确定的可观察行为。

不要仅因为“以后可能需要”而提前增加抽象、扩展点或通用基础设施。

重构完成后，相关测试必须继续通过。

---

## 一个开发循环

一个功能可能包含多个独立行为。

不要先写完整功能的所有测试，再一次性实现全部代码。

优先以较小行为循环推进：

```text
设计当前需要的 type
        ↓
写一个行为测试
        ↓
确认失败
        ↓
实现该行为
        ↓
确认通过
        ↓
必要重构
        ↓
进入下一个行为
```

如果下一个行为需要新的 data model 或改变已有 type，再进行对应的 type 设计。

因此实际开发过程通常是：

```text
Type
→ Red
→ Green
→ Refactor
→ Type
→ Red
→ Green
→ Refactor
→ ...
```

而不是：

```text
一次设计全部 type
→ 一次写完全部测试
→ 一次写完整个实现
```

---

## Bug 修复

Bug 修复同样遵循 Test-Driven Development 流程。

优先：

```text
理解问题
    ↓
编写能够复现问题的 regression test
    ↓
确认测试失败
    ↓
修复实现
    ↓
确认测试通过
    ↓
必要重构
```

如果 bug 的根源是 data model 或 type 设计错误，应先调整对应 type 设计。

不得只修改实现而不留下能够防止该问题再次出现的测试，除非该问题确实无法通过自动测试合理覆盖。

---

## 重构任务

纯重构任务不要求为了形式上的 TDD 人为制造失败测试。

开始重构前，应先确认现有测试能够覆盖当前需要保持的关键行为。

如果现有测试不足以保护本次重构涉及的行为，应先补充必要测试，再进行重构。

重构过程中保持行为不变。

---

## 与其他规则的关系

本文件负责定义：

```text
代码应该按照什么开发流程完成
```

具体测试规则由：

```text
rules/testing.md
```

负责，包括：

* 哪些行为需要测试；
* unit test 与 integration test 的选择；
* 测试放置位置；
* 测试基础设施；
* property-based testing 等测试约束。

代码、architecture、依赖和文档仍分别遵守对应的其他规则文件。

本文件不重复这些规则。
