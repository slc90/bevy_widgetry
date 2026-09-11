# Stage 9 目标 3～6：Codex 上下文、开发与验证方案

## 目标 3：让 Codex 稳定获取项目上下文

现有项目结构已经能够满足 Codex 获取开发上下文的需求，不额外建立索引、导航文档或上下文系统。

Codex 的主要开发上下文来自：

```text
AGENTS.md
├── docs/architecture.md
├── rules/
└── 源码 / rustdoc
```

### `requirements/` 边界

`requirements/` 仅用于保存：

* 项目规划；
* Stage 方案；
* 历史设计背景；
* 讨论记录。

它不属于 Codex 的开发上下文。

在 `AGENTS.md` 中明确：

```md
## 上下文边界

`requirements/` 仅用于保存项目规划、Stage 方案、历史设计背景和讨论记录，不属于 Codex 的开发上下文。

Codex 不得主动读取 `requirements/` 中的内容，也不得依据其中内容进行开发判断。

项目当前事实应以 `docs/`、`rules/`、源码和 rustdoc 为准。
```

除这项边界外，目标 3 不新增其他机制。

---

## 目标 4：建立开发与验证流程

原“建立自动验证体系”调整为“建立开发与验证流程”。

项目不建立 GitHub CI。

Rust 构建成本较高，验证主要由 Codex 在本地开发环境执行。

### 新增 `rules/development.md`

所有代码开发任务默认读取该规则。

开发流程以：

```text
Type-Driven Development
        ↓
Test-Driven Development
```

为基本原则。

整体开发循环为：

```text
理解任务
    ↓
设计当前需要的数据与类型
    ↓
Red
    ↓
Green
    ↓
Refactor
    ↓
进入下一个行为
```

实际开发允许反复进行：

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

### Type-Driven Development

在行为实现之前，先明确当前功能需要的数据模型和类型关系，包括：

* 核心数据结构；
* Component；
* Resource；
* Event / Message；
* enum 与状态表示；
* 输入与输出；
* 类型之间的关系；
* 必要接口。

类型应优先表达实际语义和约束，但不得为了类型安全本身制造没有实际价值的包装、泛型或抽象。

类型设计不是不可修改的前期设计。如果测试或实现证明原设计不合理，应调整类型后继续开发。

### Test-Driven Development

类型设计达到能够表达当前行为的程度后，进入：

```text
Red
→ Green
→ Refactor
```

#### Red

新的可观察行为先通过测试表达，并确认测试因为该行为尚未实现而失败。

#### Green

只实现让当前测试通过所需要的最小能力。

如果实现过程中暴露类型设计问题，应返回 Type-Driven Development 调整类型，而不是绕过类型问题。

#### Refactor

测试通过后，在测试保护下进行当前实现真正需要的重构。

不得借重构扩展任务范围或为未知未来需求提前抽象。

### Bug 修复

采用：

```text
复现问题
→ regression test
→ 确认失败
→ 修复
→ 确认通过
→ 必要重构
```

### 纯重构任务

纯重构不人为制造失败测试。

重构前确认已有测试能够保护需要保持的行为；覆盖不足时先补必要测试。

### 与 `rules/testing.md` 的职责划分

`rules/development.md` 负责：

> 代码按照什么流程开发。

`rules/testing.md` 继续负责：

> 测试应该覆盖什么、采用什么形式以及放在哪里。

两者不重复维护。

---

## `AGENTS.md` 调整

### 默认读取规则

任何代码修改任务默认读取：

```text
docs/architecture.md
rules/task-scope.md
rules/development.md
rules/code.md
```

其他规则继续根据任务内容读取。

### Windows 命令环境

增加：

```md
## 命令执行环境

在 Windows 环境下执行项目命令时，统一使用 PowerShell 7（`pwsh`）。

禁止使用 Windows PowerShell 5（`powershell.exe`）。
```

### 常用命令

增加：

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

这些命令作为项目常用开发与验证入口，不额外建立 verify script、xtask 或 CI。

同时修复当前 `AGENTS.md` 末尾未完成的“规则优先级”语句。

---

## 目标 5：Review / 修改控制

不新增机制。

现有 `rules/task-scope.md` 已负责：

* 限制任务范围；
* 禁止顺手重构；
* 禁止无关改名与移动；
* 禁止无关公共 API 修改；
* 允许完成目标真正必要的配套修改。

`rules/code.md` 已进一步限制无意义的源码重排。

项目的重要设计与 API 方案在交给 Codex 之前已经由方案讨论确定，因此不再为 Codex 增加额外的“人工设计确认”流程。

不增加 PR 模板、Review checklist 或 diff 工具。

---

## 目标 6：反馈沉淀机制

不新增专门的 feedback 文档或反馈系统。

实际开发中发现问题后，由后续讨论判断是否具有长期价值，再根据问题性质沉淀到现有体系：

```text
开发方式 / 工程约束
→ rules/

当前架构事实
→ docs/architecture.md

具体 API / 行为语义
→ 源码 / rustdoc

行为 bug
→ regression test

可机械检查的规则
→ lint / Cargo 配置 / 自动检查

重复机械操作
→ script / 工具
```

`requirements/` 不作为 Codex 反馈沉淀或开发知识来源。

目前先让 Codex 按现有规范实际开发。

后续只有在实践中发现真实问题时，才调整或新增对应规则、测试、文档或工具，不提前制造额外机制。
