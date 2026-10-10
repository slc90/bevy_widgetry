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

reviewer 应先通过 $code-review 确定并检查本轮 Review target / diff，再判断适用规则。
initial / final full reviewer 根据完整 working-tree change 选择规则。
incremental reviewer 根据本轮 repair delta，以及判断上一轮未解决 findings 所需的影响上下文选择规则。
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

本流程只适用于上文已要求自动独立 Code Review 的任务，不扩大触发范围。
施工 agent 完成当前任务的实施和适用验证后，按以下阶段执行。
使用已安装且支持 full / incremental mode 与 snapshot helper 的 $code-review Skill，具体接口以 Skill 为准。
不在仓库中新增 snapshot script、额外 Skill 或第二份 Review state 文件。

1. 首次 full Review：启动新的独立 reviewer subagent，调用 $code-review 的 full mode，审查当前完整 working-tree change，包括 staged、unstaged 和 untracked 文件。施工 agent 必须传入本任务进度 Markdown 的精确 root-relative path，reviewer 通过 --exclude-path 排除该文件。
2. 首次通过：若首次 full Review 精确返回 `No review findings.`，Review 阶段直接通过，不再安排重复的 final full Review，也不创建 baseline。
3. 首次 findings：施工 agent 将本轮阶段与 findings 原文写入现有任务进度 Markdown。任何修复开始前，使用 Skill 的 snapshot helper 创建任务唯一的 baseline ref，例如 `refs/code-review/<task-id>/base`。将命令返回的 ref 和完整 Tree SHA 一起记录到同一进度 Markdown，二者记录完成前不得开始修复。
4. 修复：原施工 agent 根据 findings 修改，并完成适用验证。独立 reviewer 只负责审查，不修改代码。
5. incremental Review：启动全新的独立 reviewer subagent，传入上一轮未解决 findings、baseline ref、记录中的预期完整 Tree SHA，以及相同的进度文件排除路径。reviewer 调用 $code-review 的 incremental mode，检查本轮 repair delta 和必要的影响上下文，逐项确认旧 findings 是否已修复，并报告仍未解决的 findings 与新引入的问题。即使错误行不在 repair diff 中，未解决的 finding 仍须报告。
6. incremental findings：施工 agent 更新进度记录。下一轮修复前，用 snapshot helper 的 --expect-old-tree 参数传入记录中的 Tree SHA，条件更新同一 baseline ref，使其指向当前修复前的 state。记录返回的新完整 Tree SHA 后，重复修复与 incremental Review。
7. incremental 通过：只有当 incremental Review 精确返回 `No review findings.`，才能进入 final full Review。启动全新的 reviewer subagent，用 full mode 和原有完整 static review 标准审查全部当前 working-tree change，不得只确认 incremental diff。
8. final full findings：保存阶段与 findings 原文，在下一次修复前条件更新 baseline 并记录新的 Tree SHA，返回修复与 incremental Review 循环。incremental 再次通过后，重新安排全新的 final full Review。
9. final full 通过：当 full Review 精确返回 `No review findings.`，将 Review 阶段记录为 passed。若本任务存在 baseline ref，施工 agent 使用 --expected-tree 校验记录中的完整 Tree SHA 后删除本任务的 ref。随后遵守原有提交前删除任务进度 Markdown 的要求，确认没有生成的进度记录被 staged。

每轮 Review 都必须使用新的 reviewer subagent，包括 initial full、incremental 和 final full，不得复用上一轮 reviewer 的上下文。
Review 期间不得并发修改 reviewer 正在读取的实施内容。
reviewer 遵守 $code-review 的 static review 边界，不执行测试、Cargo 命令、BRP、GUI 或其他运行时验证。

### Skill 命令与 snapshot lifecycle

`<skill_dir>` 是已安装的 $code-review Skill 目录，`<repo>` 是当前本地 checkout 的根目录。
`<progress>` 是本任务现有根目录进度 Markdown 的精确 root-relative path，沿用 rules/development.md 的命名要求。
`<ref>` 仅标识当前任务的 snapshot，`<tree-sha>` 与 `<old-tree-sha>` 必须是记录中的完整 Git Tree SHA。
`<target-json>` 是仓库外的临时 Review target 输出，不作为第二份任务 state 文件。

```pwsh
# initial / final full：完整 change，只排除本任务临时进度记录
python "<skill_dir>/scripts/collect_review_target.py" "<repo>" --mode full --exclude-path "<progress>" --output "<target-json>"

# 首次 findings 后、任何修复前创建 baseline
python "<skill_dir>/scripts/review_snapshot.py" capture "<repo>" --ref "<ref>" --exclude-path "<progress>"

# incremental：还须在 reviewer prompt 中传入上一轮未解决 findings
python "<skill_dir>/scripts/collect_review_target.py" "<repo>" --mode incremental --base-ref "<ref>" --expected-base-tree "<tree-sha>" --exclude-path "<progress>" --output "<target-json>"

# 后续 incremental / final full 返回 findings 后、新修复前推进 baseline
python "<skill_dir>/scripts/review_snapshot.py" capture "<repo>" --ref "<ref>" --expect-old-tree "<old-tree-sha>" --exclude-path "<progress>"

# 检查或续接时确认 ref 对应的实际 Tree SHA
python "<skill_dir>/scripts/review_snapshot.py" inspect "<repo>" --ref "<ref>"

# full Review 通过后清理本任务 ref，或任务明确取消时校验后清理
python "<skill_dir>/scripts/review_snapshot.py" delete "<repo>" --ref "<ref>" --expected-tree "<tree-sha>"
```

full / incremental 的 target collection 和每次 snapshot capture 都必须使用相同的 --exclude-path。
只能排除明确指定的本任务临时进度记录，不得自动忽略其他 Markdown、真实代码、配置或文档改动。
baseline 的 capture、推进、进度记录与删除由施工 agent 负责，独立 reviewer 不管理 persistent ref。
snapshot 管理只写入 Git objects 和本任务 private ref，不改变普通 Git index、working tree、commit 或 branch。
创建 ref 时不得覆盖其他任务已有的 ref，后续推进必须使用记录中的旧 Tree SHA 执行 compare-and-swap，删除前同样核对归属与 SHA。

Git tree objects 与 refs/code-review/... 位于同一本地 .git 数据中，不依赖 Codex session 或账号。
切换账号后可以在同一 checkout 续接，fresh clone 不自动包含 private refs 或未提交的 change。
续接时先读取现有进度 Markdown，再 inspect ref，确认其实际 Tree SHA 与记录一致后，才能进行 incremental Review 或推进 baseline。

中断的 reviewer 不得推进 baseline，未获得完整 verdict 不能视为成功。
记录为 pending 的 Review 必须使用相同 baseline 和当前 working-tree state 重新执行。
若中断发生在更新 ref 与写入进度 Markdown 之间，续接时必须发现并明确处理 SHA mismatch，不得静默覆盖 SHA、丢弃待处理 findings 或猜测 Review 已通过。
应根据实际 Git state 与已记录 findings 核对并恢复一致性。若不能证明 baseline 正确，则在厘清当前任务 state 后改用新的 full Review。
snapshot 或 target collection 失败时必须报告具体错误，不得宣称 `No review findings.`。无法建立预期 baseline 时也应采用新的 full Review 作为 fallback。
中断时保留任务 ref 与进度记录，只在 full Review 通过或任务明确取消后校验并清理本任务 ref。

### Review 轮数

Review 不设轮数上限。

只要 reviewer 仍然返回 findings，就必须记录 findings，在修复前创建或条件推进 baseline，再由原施工 agent 修复并启动全新的 incremental reviewer。

incremental 返回 `No review findings.` 只表示上一轮 findings 已解决且本轮修复未引入可报告问题，不能单独使整个 Review 阶段通过。
除首次 full 直接通过外，必须持续执行“修复 → incremental Review → 无 findings 后 final full Review”的循环，直到 full Review 返回 `No review findings.`。

### 职责边界

施工 agent 负责实施、适用验证、根据 reviewer findings 修复，以及进度 Markdown 与 baseline ref 的 lifecycle。

reviewer subagent 只负责调用 `$code-review` 进行独立审查，不负责修改代码、修复 findings 或继续实施任务。
