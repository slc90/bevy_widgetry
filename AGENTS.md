# Bevy Widgetry Codex Guide

本文件是 Codex 进入项目时的统一入口。

不要在这里重复详细工程规范或项目 architecture 说明；根据任务内容读取对应的规则和文档。

## 默认要求

任何代码修改任务都必须先读取：

* `docs/architecture.md`
* `rules/task-scope.md`
* `rules/development.md`
* `rules/code.md`

其中：

* `docs/architecture.md` 用于了解项目当前的 Workspace 结构、architecture 角色和内部依赖关系；
* `rules/development.md` 用于规定项目统一开发流程，包括 Type-Driven Development 与 Test-Driven Development；
* `rules/` 下的其他文件用于约束对应开发行为。

根据任务内容继续读取：

* 涉及 crate 依赖约束、module 组织、visibility 等 architecture 规则：`rules/architecture.md`
* 涉及依赖、新增 crate、Cargo 配置或 crate/package 命名：`rules/dependencies.md`
* 涉及注释、rustdoc、测试注释：`rules/documentation.md`
* 涉及新增行为、行为修改、bug 修复或测试：`rules/testing.md`
* 涉及新增、修改、删除日志，或日志相关基础设施：`rules/logging.md`
* 涉及 Git 提交或编写 commit message：`rules/git.md`

一个任务可以同时命中多个规则文件；必须读取所有相关规则。

## 上下文边界

`requirements/` 仅用于保存项目规划、Stage 方案、历史设计背景和讨论记录，不属于 Codex 的开发上下文。

Codex 不得主动读取 `requirements/` 中的内容，也不得依据其中内容进行开发判断。

项目当前事实应以 `docs/`、`rules/`、源码和 rustdoc 为准。


## Architecture 文档同步

`docs/architecture.md` 描述项目当前实际 architecture。

如果当前任务修改了其中记录的 architecture 事实，必须在同一任务中同步更新该文件。

典型情况包括：

* Workspace member 增加、删除或移动；
* crate 或 app 的 architecture 角色发生变化；
* Workspace 内部生产依赖关系发生变化；
* `docs/architecture.md` 中明确记录的测试依赖关系发生变化。

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

* 源代码；
* 测试代码；
* 构建脚本；
* 项目配置文件；
* 其他会影响项目行为、构建或测试的工程文件。

仅修改 Markdown、方案文档等不影响代码或工程行为的文件时，不需要执行此流程。

### Review 流程

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

最多执行 2 轮 Review。

如果第二轮返回 `No review findings.`，Review 阶段通过。

如果第二轮仍然存在 findings：

* 不再启动第三轮 Review；
* 不继续进入自动修复循环；
* 不得将剩余 findings 隐藏或视为已经解决；
* 在任务最终结果中明确报告剩余 findings。

### 职责边界

施工 agent 负责实施任务以及根据 reviewer findings 修改代码。

reviewer subagent 只负责调用 `$code-review` 进行独立审查，不负责修改代码、修复 findings 或继续实施任务。
