# Bevy Widgetry Codex Guide

本文件是 Codex 进入项目时的统一入口。

不要在这里重复详细工程规范或项目 architecture 说明；根据任务内容读取对应的规则和文档。

## 默认要求

任何实际修改仓库的任务都必须先读取 rules/development.md，并在实施修改前按其中的“任务进度记录”要求，于仓库根目录创建进度 Markdown。执行期间持续更新记录，提交前删除。

任何代码相关任务，包括代码实现、代码修改和独立 Code Review，都必须先读取：

- `rules/project-context.md`

`rules/project-context.md` 描述项目长期背景、目标环境和工程决策边界。这些事实必须作为设计、实现和 Review 判断的前提。

任何代码修改任务还必须先读取：

- `docs/architecture.md`
- `rules/task-scope.md`
- `rules/development.md`
- `rules/code.md`

其中：

- `rules/project-context.md` 用于了解项目长期背景、目标环境以及工程决策边界；
- `docs/architecture.md` 用于了解项目当前的 Workspace 结构、architecture 角色和内部依赖关系；
- `rules/development.md` 用于规定任务进度记录与项目统一开发流程，包括 Type-Driven Development 与 Test-Driven Development；
- `rules/` 下的其他文件用于约束对应开发行为。

根据任务内容继续读取：

- 涉及 Widget 的公开 API、state 更新或输入输出语义：`rules/widget-api.md`
- 涉及 crate 依赖约束、module 组织、visibility 等 architecture 规则：`rules/architecture.md`
- 涉及依赖、新增 crate、Cargo 配置或 crate/package 命名：`rules/dependencies.md`
- 涉及注释、rustdoc、测试注释：`rules/documentation.md`
- 涉及新增行为、行为修改、bug 修复或测试：`rules/testing.md`
- 涉及有明确性能要求的能力、性能敏感路径、成本随规模或运行时间增长的 update system，或性能修复与改善声明：`rules/benchmark.md`
- 涉及 Widget / Gallery 的可视表现、GUI 交互、focus、picking、layout、theme 或运行时界面验证：`rules/gui-debugging.md`
- 涉及新增、修改、删除日志，或日志相关基础设施：`rules/logging.md`
- 涉及 Git 提交或编写 commit message：`rules/git.md`

一个任务可以同时命中多个规则文件；必须读取所有相关规则。

## 上下文边界

`plans/` 仅用于保存项目规划、Stage 方案、历史设计背景和讨论记录，不属于 Codex 的开发上下文。

Codex 不得主动读取 `plans/` 中的内容，也不得依据其中内容进行开发判断。

项目当前事实应以 `docs/`、`rules/`、源码和 rustdoc 为准。

## Architecture 文档同步

`docs/architecture.md` 描述项目当前实际 architecture。

如果当前任务修改了其中记录的 architecture 事实，必须在同一任务中同步更新该文件。

典型情况包括：

- Workspace member 增加、删除或移动；
- crate 或 app 的 architecture 角色发生变化；
- Workspace 内部生产依赖关系发生变化；
- `docs/architecture.md` 中明确记录的测试依赖关系发生变化。

普通源码修改、外部依赖版本变化或其他不影响该文档所描述 architecture 事实的改动，不需要更新 `docs/architecture.md`。

## 规则优先级

`rules/` 中的规则是项目工程硬约束。

若任务目标与规则发生真实冲突，不得自行放宽规则；应明确指出冲突并把它作为设计问题处理。

## 命令执行环境

在 Windows 环境下执行项目命令时，统一使用 PowerShell 7（`pwsh`）。

禁止使用 Windows PowerShell 5（`powershell.exe`）。

## 常用验证命令

代码修改完成后，根据任务影响范围执行必要验证。

```bash
# 格式检查
cargo fmt --all -- --check

# Workspace 编译检查
cargo check --workspace

# Workspace Clippy
cargo clippy --workspace --all-targets

# Workspace 完整构建
cargo build --workspace

# Workspace 全部测试
cargo test --workspace

# 单个 crate 测试
cargo test -p <package-name>

# 运行 Widget Gallery
cargo run -p widget_gallery
```

## 自动 Code Review

任何产生代码相关改动的任务，在实施和必要验证完成后，都必须执行独立 Code Review。

代码相关改动包括但不限于：

- 源代码；
- 测试代码；
- 构建脚本；
- 项目配置文件；
- 其他会影响项目行为、构建或测试的工程文件。

仅修改 Markdown、方案文档等不影响代码或工程行为的文件时，不需要执行此流程。

### Reviewer 规则读取

独立 Code Review 的规则读取按本节执行，不套用前文面向施工任务的规则读取要求。

每轮 reviewer subagent 都必须读取 `rules/project-context.md`，并作为判断当前 change 是否符合项目长期背景、目标平台和工程决策边界的 Review 依据。

reviewer 应先通过 $code-review 确定并检查当前完整 Review target / diff，再根据收集到的当前完整 change 判断适用规则。
除 `rules/project-context.md` 外，不无条件读取全部 `rules/`。
当前 change 命中某项规则的适用范围时，必须在形成对应 Review 判断前读取该规则。
不得根据任务名称机械选择规则，也不得仅以文件路径作为触发条件。

独立 Code Review 不要求读取 `rules/task-scope.md`、`rules/development.md` 和 `rules/git.md`。
task scope 由施工任务和施工 agent 控制，development 约束实施流程，提交前的独立 Review 不负责 commit message。
这些规则不作为独立 reviewer 的固定 Review 上下文。

#### 根据当前 change 动态读取规则

- `rules/code.md`：当前 change 包含 Rust 源码时必须读取，包括 *.rs、Rust test、benchmark Rust code、build.rs 和 Gallery Rust code。用于判断源码布局、item 组织、错误处理、panic、lint suppression、warning 和命名等代码约束。只有 Cargo 配置、Markdown 或其他不包含 Rust 源码的 change 不要求读取。
- `rules/architecture.md`：change 涉及 crate 职责或边界、module 组织、visibility、facade / core / Widget crate / Gallery 之间的职责、Workspace 内部依赖方向、asset 归属或访问方式、文件或实现职责在 crate / module 之间移动，或其他 architecture 约束时读取。
- `rules/dependencies.md`：change 涉及 Cargo.toml 中 dependency 变化、新增、删除或调整 dependency、Workspace dependency、crate/package 新增、删除或命名变化、path dependency 或 dependency 配置变化时读取。
- `rules/documentation.md`：change 实际新增、删除或修改代码注释、rustdoc、Markdown 文档、测试说明类注释或其他受项目文档语言和注释规则约束的文本时读取。不要仅因为 Rust 文件中存在未修改的注释就读取。
- `rules/testing.md`：change 涉及新增或修改可观察行为、bug fix、新增、删除或修改 unit / integration test，或修改测试策略、regression protection 时读取。纯内部重构且没有行为变化、测试变化时不要求机械读取。
- `rules/benchmark.md`：change 涉及 benchmark 实现或 fixture、性能修复或改善、性能敏感路径、成本随输入规模、数据量或运行时间增长的 update / rendering / streaming 路径，或影响性能 contract、性能测量方法时读取。
- `rules/widget-api.md`：change 涉及 Widget 的 public API、自有 runtime state、state setter / query、event / message、state change notification、用户输入与程序化输入语义、invalid target / invalid input，或对外可观察 state contract 时读取。不要仅因为代码位于 Widget crate 中就读取。
- `rules/logging.md`：change 涉及新增、删除或修改日志、log level、structured context、Widgetry / Gallery 日志入口、新增或改变错误路径、BevyError / Result 传播、失败被记录、吸收或恢复的行为，或可能引入静默吞错时读取。不能仅以是否出现日志 macro 判断，错误处理和失败传播变化也必须纳入判断。
- `rules/gui-debugging.md`：只有 change 本身修改或依赖该规则规定的 GUI 调试 / 运行时验证基础设施时读取，包括 Gallery 的 BRP runtime、GUI 调试基础设施、Winit update mode、BRP 输入或 screenshot 验证通道、Gallery runtime 的验证机制，以及与该规则明确约束的运行方式直接相关的代码或配置。普通 GUI Widget 的 layout、focus、pointer 等行为变化不要求机械读取。

### Review 流程

reviewer 遵守 $code-review 的 static review 边界，不执行测试、Cargo check、BRP 或其他运行时验证。

第一轮 Review：

1. 施工 agent 完成实施后，启动一个新的 reviewer subagent。
2. reviewer subagent 必须调用 `$code-review` Skill，审查当前完整 working-tree change。
3. reviewer subagent 只负责审查并返回 findings，不得修改代码。

如果 reviewer 返回 `No review findings.`，Review 阶段通过。

如果 reviewer 返回 findings：

1. findings 交回原施工 agent；
2. 由原施工 agent 根据 findings 修改代码；
3. 修改完成后，启动一个全新的 reviewer subagent；
4. 新 reviewer subagent 再次调用 `$code-review`，重新审查当前完整 working-tree change。

每轮 Review 都必须使用新的 reviewer subagent，不得复用上一轮 reviewer 的上下文。

### Review 轮数

Review 不设轮数上限。

只要 reviewer 仍然返回 findings，就必须由原施工 agent 完成修复，并启动一个全新的 reviewer subagent 重新审查当前完整 working-tree change。

持续执行“Review → 修复 → 使用全新 reviewer 再次 Review”的循环，直到 reviewer 返回 `No review findings.`，Review 阶段才算通过。

### 职责边界

施工 agent 负责实施任务以及根据 reviewer findings 修改代码。

reviewer subagent 只负责调用 `$code-review` 进行独立审查，不负责修改代码、修复 findings 或继续实施任务。
