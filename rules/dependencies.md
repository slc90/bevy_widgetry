# 依赖与 Crate 命名规则

## Workspace 外部依赖

能够由 Workspace 统一管理的外部 crate，其版本统一定义在根 `Cargo.toml` 的 `[workspace.dependencies]` 中。

子 crate 使用：

```toml
some_dependency.workspace = true
```

不得在多个子 crate 中分别维护同一外部依赖的版本号。

## Workspace 内部依赖

内部 `bevy_widgetry_*` crate 使用 path dependency 指向对应 workspace crate。

是否允许某个内部 crate 依赖另一个内部 crate，必须同时符合 `rules/architecture.md` 的依赖方向约束。

## Crate 目录名与 Package 名

`crates/` 下目录名使用简洁的功能名：

```text
crates/
├── core/
├── button/
├── combo_box/
├── text_field/
└── window/
```

对应 Cargo package 名统一使用：

```text
bevy_widgetry_<功能名>
```

例如：

```text
crates/button     -> bevy_widgetry_button
crates/combo_box  -> bevy_widgetry_combo_box
crates/core       -> bevy_widgetry_core
```

顶层 facade crate 是例外，其 package 名直接为 `bevy_widgetry`。

目录名与 package 名不要求一致；这是刻意的命名层次，而不是需要消除的不一致。

## 新增依赖

不得因为实现方便、可能以后会用或为了引入某个小工具函数而随意增加依赖。新增依赖必须服务于当前明确目标，并遵守 Workspace 统一版本管理。
