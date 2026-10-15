# 独立 Code Review 规则

## 适用范围

正式开发中，凡是涉及代码、构建或测试行为的修改，都必须在实施及必要验证完成后执行独立自动 Code Review。

包括：

- Rust 源代码及测试代码。
- Benchmark、构建脚本和工程配置。
- 其他可能影响代码行为、构建或测试结果的文件。

纯 Markdown 或其他不影响工程行为的文档修改，不触发 Code Review。

方案探索阶段遵循 `AGENTS.md` 中的规则例外。探索产物进入正式开发并准备交付时，必须完成适用的 Code Review。

## 职责与独立性

Code Review 使用 `$code-review` Skill，由独立的 Reviewer Subagent 执行。

实施 Agent 负责代码修改、验证、Findings 修复，以及任务进度记录和 Baseline 的管理。

Reviewer 只负责静态代码审查，不修改代码、不修复 Findings，也不继续实施任务。

- 每轮 Review 必须启动全新的 Reviewer Subagent，不复用上一轮 Reviewer 的上下文。
- Reviewer 不执行测试、Cargo 命令、BRP、GUI 或其他运行时验证。
- Review 进行期间，不得并发修改 Reviewer 正在审查的内容。
- Reviewer 根据实际 Diff 和必要上下文判断问题，不以未验证的推测代替具体 Findings。

## Reviewer 规则读取

每轮 Reviewer 都必须读取 `rules/project-context.md`，将项目目标环境、长期工程约束及设计边界作为 Review 判断依据。

Reviewer 应先通过 `$code-review` 获取并检查本轮 Review Target / Diff，再根据实际变更内容及影响范围读取相关规则。

- Initial / Final Full Review 根据完整 Working Tree Change 选择规则。
- Incremental Review 根据本轮 Repair Delta，以及判断既有未解决 Findings 所需的上下文选择规则。
- 不根据任务名称或文件路径机械加载全部规则。
- 变更涉及某项规则的实际约束时，必须在形成对应 Review 判断前读取该规则。

### 专项规则选择

- `rules/code.md`：修改 Rust 源码时，包括测试、Benchmark、Build Script 和 Gallery Rust 代码。
- `rules/architecture.md`：涉及 Crate 职责、依赖、Cargo Dependency 配置、Module、Visibility、Asset 管理或架构边界。
- `rules/ui-composition.md`：涉及 BSN、SceneComponent、Scene Function、Props、UI 构造或组合方式。
- `rules/widget-api.md`：涉及 Widget 公开 API、自有 Runtime State、状态更新、变化通知、用户与程序化操作契约或无效请求处理。
- `rules/documentation.md`：实际修改代码注释、Rustdoc、Markdown 或其他受文档规范约束的内容。
- `rules/testing.md`：新增或修改可观察行为、修复 Bug、修改测试或调整回归保护与测试策略。
- `rules/benchmark.md`：涉及性能敏感路径、Benchmark、性能改善、性能契约或测量方法。
- `rules/logging.md`：涉及日志、错误处理、`BevyError` / `Result` 传播、失败记录与恢复，或可能引入静默吞错。
- `rules/gui-debugging.md`：修改 GUI 调试、BRP Runtime、输入或 Screenshot 验证通道，以及相关运行时验证基础设施时。

普通 Widget 的 Layout、Focus、Pointer 等行为修改不意味着必须读取 `rules/gui-debugging.md`；只有变更实际涉及其约束的调试或验证基础设施时才需要。

`rules/task-scope.md`、`rules/development.md` 和 `rules/git.md` 不属于 Reviewer 固定上下文。任务范围、开发过程及 Commit Message 分别由实施 Agent 和对应流程负责。

Reviewer 不需要检查实施过程是否遵循 TDD，也不通过静态 Review 代替编译、测试或运行时验证。

## Review 流程

完整流程采用：

```text
Initial Full Review
    │
    ├── No findings ──────────────────────→ Passed
    │
    └── Findings
          ↓
       Capture Baseline
          ↓
       Repair & Validation
          ↓
       Incremental Review
          │
          ├── Findings → Advance Baseline → Repair
          │                                  ↑
          │                                  └─ Repeat
          │
          └── No findings
                ↓
             Final Full Review
                │
                ├── Findings → Advance Baseline → Repair
                │
                └── No findings
                      ↓
                    Passed
```

### Initial Full Review

实施与适用验证完成后，启动独立 Reviewer，使用 `$code-review` 的 Full Mode 审查完整 Working Tree Change。

审查范围包括 Staged、Unstaged 和 Untracked 文件，仅排除本任务的临时进度 Markdown。

如果首次 Full Review 精确返回：

```text
No review findings.
```

Review 直接通过，不需要重复 Final Full Review，也不创建 Baseline。

如果返回 Findings，实施 Agent 必须先将完整 Findings 记录到任务进度 Markdown，再使用 `$code-review` 的 Snapshot Helper 创建 Baseline。

Baseline Ref 和完整 Git Tree SHA 都必须记录完成，才能开始修复。

### Repair 与 Incremental Review

实施 Agent 修复 Findings，并完成受影响范围内的必要验证。

随后启动全新的 Reviewer，使用 Incremental Mode 检查本轮 Repair Delta。

Incremental Reviewer 必须获得：

- 上一轮仍未解决的完整 Findings。
- 当前任务的 Baseline Ref。
- 记录中的预期完整 Git Tree SHA。
- 与 Full Review 一致的进度文件排除路径。

Reviewer 应逐项判断既有 Findings 是否真正解决，并检查修复引入的新问题。

即使原有 Finding 的问题代码不在本轮 Repair Delta 中，只要问题仍然存在，就必须继续报告，不得因不属于本轮 Diff 而忽略。

如果 Incremental Review 仍有 Findings，实施 Agent 先记录结果，再条件更新 Baseline，之后继续修复。

如果 Incremental Review 精确返回 `No review findings.`，进入 Final Full Review。

### Final Full Review

启动全新的 Reviewer，重新以 Full Mode 审查完整 Working Tree Change，而不只是此前的 Repair Delta。

Final Full Review 使用与 Initial Full Review 相同的完整静态审查标准。

- 有 Findings：记录结果，条件更新 Baseline，重新进入 Repair 与 Incremental Review。
- 无 Findings：Review 通过。

**Incremental Review 通过不代表整个 Review 已通过。**

除首次 Full Review 直接通过外，必须最终取得 Full Review 返回的 `No review findings.`。

Review 不设轮数上限。只要仍存在 Findings，就继续执行修复和独立审查，不得人为限制轮数或跳过 Final Full Review。

## Baseline 管理

Baseline 用于安全地标识修复前的代码状态，为 Incremental Review 提供准确的比较依据。

使用 `$code-review` Skill 提供的 Snapshot Helper，不自行实现第二套 Snapshot Script、Review Skill 或持久化 Review State。

### 创建与更新

- 首次 Full Review 有 Findings 时，在任何修复开始前创建本任务唯一的 Baseline Ref。
- Baseline 使用任务专属的 Private Ref，例如 `refs/code-review/<task-id>/base`，不得覆盖其他任务已有的 Ref。
- 创建后必须记录返回的完整 Git Tree SHA。
- 后续 Review 返回 Findings 时，在下一轮修复开始前更新同一个 Baseline，使其指向当前修复前的状态。
- 更新 Baseline 必须使用记录中的旧 Tree SHA 执行 Compare-and-Swap，不得无条件覆盖。
- 新 Tree SHA 必须写入进度记录后，才能开始下一轮修复。

Baseline 的创建、更新及删除由实施 Agent 负责，Reviewer 不管理持久化 Ref。

Snapshot 只操作 Git Objects 和本任务的 Private Ref，不得修改普通 Git Index、Working Tree、Commit 或 Branch。

### Review Target 与排除范围

Full / Incremental Review 的 Target Collection，以及每次 Snapshot Capture，都必须排除同一个精确的任务进度 Markdown 路径。

排除路径使用仓库根目录相对路径，并在任务进度记录中保存。

**只能排除本任务的临时进度文件。**

不得自动排除其他 Markdown、代码、配置或真实工程文件，也不得通过扩大排除范围隐藏需要 Review 的修改。

Review Target 的临时输出不作为第二份持久化任务状态文件。

### 清理

Review 通过后，如果存在 Baseline Ref，实施 Agent 必须使用记录中的完整 Tree SHA 校验 Ref，确认一致后才能删除。

任务明确取消时，同样需要校验并清理属于该任务的 Baseline。

不得删除其他任务的 Ref，也不得在 Review 尚未通过或任务未取消时提前清理 Baseline。

## 中断与续接

Review 状态统一记录在 `rules/development.md` 规定的任务进度 Markdown 中，不建立独立的 Review State 文件。

记录每轮 Review 的阶段、轮次、执行状态、完整 Findings、处理结果、Baseline Ref、完整 Tree SHA、排除路径及下一步操作。

### Review 中断

未获得完整 Verdict 的 Review 必须保持 `pending`，不得视为通过。

中断的 Reviewer 不得推进 Baseline。续接时使用相同的 Baseline 和当前 Working Tree State，启动新的 Reviewer 重新执行未完成的 Review。

不得因为 Reviewer 中断而跳过 Findings 或推断审查已经成功。

### Baseline 一致性

续接存在 Baseline 的任务时，必须先通过 Snapshot Helper 检查实际 Ref，并核对其完整 Tree SHA 与进度记录是否一致。

只有确认一致后，才能继续 Incremental Review 或条件更新 Baseline。

如果中断发生在更新 Ref 与写入进度记录之间，必须根据实际 Git State、已有 Findings 和任务进度恢复一致性。

不得静默覆盖 SHA、不经确认修改 Baseline、丢弃未解决 Findings，或猜测某轮 Review 已经通过。

无法证明 Baseline 正确时，应先厘清实际任务状态，再重新执行 Full Review，不得继续使用不可靠的 Incremental Baseline。

### 工具失败与环境续接

Review Target Collection 或 Snapshot 操作失败时，必须报告具体错误，不得将失败当作 `No review findings.`。

无法安全建立或恢复预期 Baseline 时，可以重新进行 Full Review，不能通过跳过校验继续 Incremental Review。

Private Ref 和 Git Tree Objects 保存在当前 Checkout 的 `.git` 中，可以支持同一 Checkout 下会话或账号切换后的续接。

Fresh Clone 不会自动包含原 Checkout 的 Private Ref 和未提交修改，不得假定能够直接恢复原有 Incremental Review 状态。

中断期间保留进度记录和必要的 Baseline Ref，直到 Full Review 通过或任务明确取消。

## 完成条件

只有以下两种情况可以将独立 Code Review 标记为 `passed`：

1. Initial Full Review 直接返回 `No review findings.`。
2. 所有必要的修复与 Incremental Review 完成后，Final Full Review 返回 `No review findings.`。

通过后按要求校验并清理 Baseline，更新进度记录。

任务进度 Markdown 的最终删除及 Git 提交遵循 `rules/development.md` 和 `rules/git.md`。

`$code-review` 的具体调用方式、参数及 Snapshot Helper 操作以当前 Skill 文档为准，不在工程规则中复制命令示例。
