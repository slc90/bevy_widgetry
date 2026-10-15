# Bevy Widgetry

## 项目资料与约束

仓库中的资料按以下职责区分：

- `rules/`：工程约束，包括开发流程、代码规范、架构边界、测试、验证与提交要求。
- `docs/`：当前有效的架构、功能、设计及可观察行为文档。
- `plans/`：方案规划、探索结论与历史设计背景。

`rules/` 中的适用规则必须遵守，但方案探索阶段适用下文规定的例外。

`docs/` 用于理解当前设计与功能，具体实现以实际源码和工程配置为准。

`plans/` 不作为当前实现事实，也不需要默认读取。只有任务涉及历史设计依据、已有方案或相关探索结论时，才按需查阅。

根据任务目标、涉及范围和实际变更选择相关资料，不机械读取全部规则与文档。

如果任务要求与现有工程规则发生实质冲突，应先明确冲突并解决设计问题，不得自行放宽正式开发阶段的规则。

## 规则导航

### 基础规则

- `rules/project-context.md`：项目背景、目标环境、平台边界与 UI 构造原则。正式代码开发及独立 Code Review 均须读取。
- `rules/task-scope.md`：限定当前任务的修改范围，避免无关改动。
- `rules/development.md`：开发流程、任务进度记录、工程化开发方法及专项验证衔接。

实际修改仓库前，必须按照 `rules/development.md` 创建并维护任务进度记录。纯只读分析不需要创建记录。

### 专项规则

根据任务实际涉及的内容读取：

- `rules/code.md`：Rust 代码规范、错误处理、命名及 lint 约束。
- `rules/widget-api.md`：Widget API、运行时状态、状态变化通知及交互契约。
- `rules/architecture.md`：Workspace、crate 职责、依赖管理、Assets 与源码组织。
- `rules/ui-composition.md`：Bevy BSN、Scene、Props、Runtime ECS 与 UI 组合边界。
- `rules/documentation.md`：代码注释、rustdoc 及文档规范。
- `rules/testing.md`：行为测试、回归保护、覆盖模型与 Property-based Testing。
- `rules/benchmark.md`：性能敏感路径、Benchmark 与性能验收。
- `rules/gui-debugging.md`：GUI 运行时行为及 BRP 自动验证。
- `rules/logging.md`：日志职责、记录条件、错误日志与结构化字段。
- `rules/git.md`：Git 提交范围、变更分组及 Commit Message。
- `rules/code-review.md`：独立自动 Code Review 的适用范围、执行流程及中断续接。

相关规则可以同时适用。规则选择应依据真实任务内容和变更影响，而不是仅凭文件所属目录判断。

常用开发与验证命令见 `rules/common-commands.md`。命令列表仅供按需选择，不代表每次修改都必须执行全部命令。

### 当前功能与架构文档

- `docs/architecture.md`：Workspace 当前结构及各成员职责。
- `docs/crates/`：各 crate 的功能、设计思路、行为契约及限制。
- `docs/gallery.md`：Gallery 的功能与展示行为。

涉及特定 crate 或 Gallery 时，优先阅读相应功能文档，并结合公共接口及必要源码确认实际行为。

文档不能替代代码事实。发现文档与当前实现不一致时，应依据实际实现核实，并在任务范围内修正文档。

## 任务执行

### 方案探索

解决方案尚不明确时，允许自主开展实验、原型开发、技术调查和 Benchmark，以实际结果探索可行方案。

**探索阶段只强制遵守任务进度记录要求，可以不遵守 `rules/` 中的其他工程规则，包括正式开发流程、代码规范、测试要求和验证流程。**

探索可以采用任何适合当前问题的方法，不必遵循 Type-Driven Development 或 Test-Driven Development，也不要求实验代码达到正式工程质量。

探索期间仍须维护任务进度记录，保存重要发现、实验结果、失败原因、已确认的设计决策及未解决问题，确保过程可以中断和续接。

需要保存探索方案时，将其记录在 `plans/`：

- 一个独立方案对应一份 Markdown 文件或一个目录。
- 复杂方案使用 `@Plan Decomposer` Skill，将已有总方案拆分为按执行顺序排列的独立方案文档。
- 不同的独立方案分别提交，避免将无关设计混在一起。
- 区分已验证事实、设计决策和未解决问题，不将实验结果直接视为正式功能。

探索结束、解决方案明确后，**自主进入正式开发，不需要等待人工确认**。

探索阶段的规则豁免不延续至正式开发。需要保留的实验代码必须按照正式工程规则整理、验证和 Review，不能直接以探索阶段的标准交付。

### 正式开发

正式开发遵循 `rules/development.md`，采用 Type-Driven Development、Test-Driven Development 和小步迭代，并根据任务影响范围执行必要验证。

具体测试、GUI 验证和性能 Benchmark 分别遵循对应专项规则。

纯重构、Gallery 开发等特殊情况按适用规则处理，不为满足流程形式制造无意义的测试或步骤。

### 任务进度与中断续接

实际修改仓库的任务必须维护根目录下的临时进度 Markdown，记录任务范围、实施进度、验证结果、Review 状态及必要的续接信息。

中断后根据进度记录和实际工作区状态继续，不重复已完成的工作，也不将未经核实的记录视为完成证据。

具体记录格式、更新时机、Review 状态管理及提交前清理要求遵循 `rules/development.md`。

### 独立 Code Review

影响代码、构建或测试行为的修改，在实施和必要验证后必须执行独立自动 Code Review。纯不影响工程行为的 Markdown 修改不触发。

每轮 Review 使用新的独立 reviewer subagent，按真实 diff 选择相关规则，并始终读取 `rules/project-context.md`。

完整 Review 流程、findings 修复循环、baseline 管理及中断续接遵循 `rules/code-review.md`。具体调用方式以当前 `$code-review` Skill 文档为准。

## 文档同步

修改当前架构事实时，必须同步更新 `docs/architecture.md`，例如 Workspace 成员、crate 职责或文档记录的依赖结构发生变化。

修改 crate 或 Gallery 的对外功能、可观察行为、重要设计决策或已知限制时，应同步更新 `docs/crates/` 或 `docs/gallery.md` 中受影响的内容。

纯内部重构且不改变文档所描述事实的，不要求机械修改文档。

正式文档描述当前已经实现的功能和行为；未实施的方案保留在 `plans/`，不得作为现有功能写入 `docs/`。
