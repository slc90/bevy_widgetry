# bevy_widgetry 同步当前文档并完成外部 Runtime 集成验证

**Repository:** `slc90/bevy_widgetry`

## 目标

使 Widgetry 的当前架构文档、GUI 调试规则和本地验证结果与“只消费外部 `bevy_brp_runtime`”的新事实一致。

## 范围

仅限 `slc90/bevy_widgetry` 的 `docs/architecture.md`、`docs/brp-runtime.md`、`rules/gui-debugging.md` 以及本地 workspace / Agent GUI 验证。历史 `plans/` 不整理。

## 预期产出

当前文档不再声称 Gallery 自己维护 BRP transport 或 `vendor/bevy_brp_extras`，GUI 调试流程保持 MCP 方式不变；Widgetry workspace 通过本地验证和一次 `widget_gallery` 的启动、截图/查询、输入与 shutdown 链路验证。

## 与前后方案的关系

这是整个迁移链的最后方案。它不再改变 bevy_brp 仓库，只确认 Widgetry 已完全通过 Git 依赖使用外部 Runtime，并且本地 BRP 冗余已消失。

## 方案内容

## Widgetry 当前文档同步

不整理历史 `plans/`；它们继续作为历史方案记录存在。Widgetry 自己的 `AGENTS.md` 已明确历史 plans 不是当前开发事实来源。

只同步会因迁移直接失真的当前文档：

- `docs/architecture.md`：删除本地 `gallery/src/brp*` 和 `vendor/bevy_brp_extras` 架构描述，改为 Gallery 依赖外部 `bevy_brp_runtime`。
- `docs/brp-runtime.md`：收敛成 Gallery 与外部 runtime 的集成说明，不再重复 runtime 内部 transport/activity 实现和“等待上游替代”的内容。
- `rules/gui-debugging.md`：把“Gallery 通过 `bevy_brp_extras` 暴露 BRP”改为通过 `bevy_brp_runtime` 提供完整 BRP runtime；MCP 调试流程保持不变。

### bevy_widgetry

```text
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
```

再通过 `bevy_brp_mcp` 启动 `widget_gallery`，至少完成启动、截图/查询、一次输入操作和正常 shutdown，确认 Widgetry 已完全通过 Git 依赖使用外部 runtime。

## 最终责任边界

```text
bevy_widgetry
  -> 仅依赖 bevy_brp_runtime@v0.1.0
```

Widgetry 中不再存在 BRP transport、activity tracker、vendored Extras 或其专属 transport 依赖。
