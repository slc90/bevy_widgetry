# Bevy Widgetry Codex Guide

本文件是 Codex 进入项目时的统一入口。

不要在这里重复详细工程规范或项目架构说明；根据任务内容读取对应的规则和文档。

## 默认要求

任何代码修改任务都必须先读取：

* `docs/architecture.md`
* `rules/task-scope.md`
* `rules/development.md`
* `rules/code.md`

其中：

* `docs/architecture.md` 用于了解项目当前的 Workspace 结构、架构角色和内部依赖关系；
* `rules/development.md` 用于规定项目统一开发流程，包括 Type-Driven Development 与 Test-Driven Development；
* `rules/` 下的其他文件用于约束对应开发行为。

根据任务内容继续读取：

* 涉及 crate 依赖约束、module 组织、可见性等架构规则：`rules/architecture.md`
* 涉及依赖、新增 crate、Cargo 配置或 crate/package 命名：`rules/dependencies.md`
* 涉及注释、rustdoc、测试注释：`rules/documentation.md`
* 涉及新增行为、行为修改、bug 修复或测试：`rules/testing.md`
* 涉及新增、修改、删除日志，或日志相关基础设施：`rules/logging.md`

一个任务可以同时命中多个规则文件；必须读取所有相关规则。

## 上下文边界

`requirements/` 仅用于保存项目规划、Stage 方案、历史设计背景和讨论记录，不属于 Codex 的开发上下文。

Codex 不得主动读取 `requirements/` 中的内容，也不得依据其中内容进行开发判断。

项目当前事实应以 `docs/`、`rules/`、源码和 rustdoc 为准。


## 架构文档同步

`docs/architecture.md` 描述项目当前实际架构。

如果当前任务修改了其中记录的架构事实，必须在同一任务中同步更新该文件。

典型情况包括：

* Workspace member 增加、删除或移动；
* crate 或 app 的架构角色发生变化；
* Workspace 内部生产依赖关系发生变化；
* `docs/architecture.md` 中明确记录的测试依赖关系发生变化。

普通源码修改、外部依赖版本变化或其他不影响该文档所描述架构事实的改动，不需要更新 `docs/architecture.md`。

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
