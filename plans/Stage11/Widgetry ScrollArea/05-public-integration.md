# 完成 ScrollArea facade 接入与公共 API 验证

## 目标

把已经完成的 bevy_widgetry_scroll_area 作为正式 Widgetry 功能 crate 接入顶层 facade，并用外部消费者视角的 integration test 和 architecture 文档固定最终公共边界。

## 范围

本阶段不增加新的 ScrollArea 行为，只处理必须和新 crate 一起完成的 workspace / facade / docs / public API 验证。

## facade

crates/bevy_widgetry：

- Cargo.toml 增加 bevy_widgetry_scroll_area path dependency。
- src/lib.rs 增加独立 scroll_area module，并 `pub use bevy_widgetry_scroll_area::*`。
- 不把 ScrollArea type 混进 style module，也不从 core 反向暴露。

消费者最终从：

```rust
bevy_widgetry::scroll_area::{...}
```

取得完整公开 API。

## public API integration test

在 crates/bevy_widgetry/tests/public_api.rs 增加 facade 级用例，验证消费者只依赖 bevy_widgetry 即可：

- 导入 WidgetryScrollArea / Plugin / Props。
- 导入 ScrollAxis / ScrollbarPolicy / ScrollbarVisibility / WidgetryScrollIntoView / WidgetryScrollAreaViewport。
- 通过 BSN 构造空 ScrollArea 和带 content / children 的 ScrollArea。
- Props default 为 Vertical、Auto/Auto、12px。
- 从公开 Viewport marker 查询同 entity 的原生 ScrollPosition，证明“无自定义 scroll_to wrapper”仍然具备稳定的程序化访问路径。

测试只验证 facade/public contract，不重复 crate 内 style/headless test。

## architecture 文档最终同步

更新 docs/architecture.md：

- workspace tree 增加 crates/scroll_area。
- Workspace Roles 增加 ScrollArea 的最终职责说明：任意 BSN 内容、官方 ScrollArea / Scrollbar、reserved gutter、Auto policy、keyboard 与 WidgetryScrollIntoView。
- Dependency Graph 增加 facade → scroll_area，以及 scroll_area 实际生产依赖（预计 Bevy 外加 core / log；以实现后的 Cargo.toml 为准，不虚构未使用依赖）。
- core role 保持前一方案已经去除 WidgetryFocusPlugin 后的描述。
- Gallery role继续保留 BRP runtime 为运行时 GUI 验证通道。

如果新 crate 的实际内部依赖与原设计不同，只记录真实依赖，不为了让图更对称而添加兄弟 Widget dependency。

## 常规验证

本阶段结束时，新 crate 已是完整 workspace member 和 facade public API，因此至少应确保：

```text
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
```

实际执行仍遵守仓库开发流程与独立 code review 规则。

## 预期产出

ScrollArea 不再只是内部 crate，而是和 Button、ComboBox、Tooltip 等一样通过 bevy_widgetry facade 正式可用；公共 API 与 architecture 文档有消费者视角测试保护。

## 与前后方案的关系

依赖 ScrollArea crate 自身功能已经完成。下一方案开始修改 Gallery，使其成为 ScrollArea 的真实集成消费者和 BRP 测试场景。
