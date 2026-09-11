# Bevy Widgetry Codex Guide

本文件是 Codex 进入项目时的规则入口。不要在这里重复详细工程规范；根据任务内容读取 `rules/` 下对应文件。

## 默认要求

任何代码修改任务都必须先读取：

- `rules/task-scope.md`
- `rules/code.md`

根据任务内容继续读取：

- 涉及 crate 职责、crate 依赖方向、module 组织、可见性：`rules/architecture.md`
- 涉及依赖、新增 crate、Cargo 配置或 crate/package 命名：`rules/dependencies.md`
- 涉及注释、rustdoc、测试注释：`rules/documentation.md`
- 涉及新增行为、行为修改、bug 修复或测试：`rules/testing.md`

一个任务可以同时命中多个规则文件；必须读取所有相关规则。

## 规则优先级

`rules/` 中的规则是项目工程硬约束。若任务目标与规则发生真实冲突，不得自行放宽规则；应明确指出冲突并把它作为设计问题处理。
