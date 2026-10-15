# Git 提交规则

## 提交前检查

提交前必须完成适用的验证与独立 Code Review，并按照 `rules/development.md` 删除本任务的临时进度 Markdown。

检查最终 Working Tree、暂存区及 Diff，确保提交范围正确，且临时进度文件没有进入提交。

## Commit Message

Commit Message 统一使用英文，根据**最终实际提交的 Diff** 编写，不根据任务描述、原定计划或开发过程生成。

必须准确反映最终保留的实质修改，不包含计划但未实施、实施后又撤销或与最终提交无关的内容。

### 格式

默认使用一行 Subject 和数字编号的 Body：

```text
<Subject>

1. <Change>
2. <Change>
3. <Change>
```

当提交只有一个明确的语义修改，且 Subject 已经完整表达时，可以省略 Body。

### Subject

Subject 应简洁概括整个提交的主要变化。

- 不使用 `feat:`、`fix:`、`refactor:` 等类型前缀。
- 不添加 Stage 编号或其他项目阶段前缀。
- 不以句号结尾。
- 不使用 `Update code`、`Improve implementation` 等缺少实际信息的描述。

混合类型的提交应概括各项修改共同服务的目标，不强行归入某一种提交类型。

### Body

Body 使用数字编号，每项描述一个独立的语义修改，不使用无序列表。

- 按功能、行为、Bug 修复、结构或规则变化组织，不按文件、Module、Function 或实施步骤拆分。
- 同一语义修改涉及多个文件时，应合并为一项。
- 优先描述实际变化及其结果。原因不明显且有助于理解时，可以补充说明。
- Bug 修复应说明解决了什么问题，而不只是描述修改了哪段代码。
- 重构应说明职责、结构或处理方式发生了什么变化，不使用空泛的优化描述。
- 具有独立意义的规则与文档变化应记录；随代码变化进行的机械文档同步不单独列项。

多个修改点通常按照主要功能与行为变化、Bug 修复、结构重构、规则与文档变化的顺序排列，也可以根据实际语义调整。

例如：

```text
Unify widget interaction state handling

1. Add keyboard navigation for the new selection state.
2. Fix stale pressed state after focus loss.
3. Move pressed state updates from event callbacks into the shared synchronization system.
4. Clarify interaction state requirements in the development rules.
```

## 不记录的内容

**Commit Message 无例外地不记录以下内容：**

- 测试的新增、删除或修改。
- 测试执行过程及结果。
- 格式化、编译检查、Clippy 等验证工作。
- Import 整理及单纯代码格式调整。
- 没有独立语义的附带文件或实现变化。
- 开发过程中已经撤销的修改。

Commit Message 只描述最终提交所包含的实质变化，不记录为完成这些变化而执行的开发与验证步骤。
