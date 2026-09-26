# bevy_widgetry 切换外部 Runtime 并移除本地 BRP 基础设施

**Repository:** `slc90/bevy_widgetry`

## 目标

让 Widgetry 从 BRP transport/activity 的维护者变为 `bevy_brp_runtime@v0.1.0` 的单纯消费者，删除 vendored Extras、Gallery 内 BRP 实现以及只为这些实现服务的依赖。

## 范围

仅限 `slc90/bevy_widgetry` 仓库中 Cargo 依赖、`vendor/bevy_brp_extras/`、`gallery/src/brp*` 和 Gallery 启动接缝。不再回头修改 bevy_brp 内部设计。

## 预期产出

Widgetry 只通过 workspace Git tag 依赖 `bevy_brp_runtime`，Gallery 只安装 `BrpRuntimePlugin::default()`，仓库内不再有 vendored Extras、BRP transport/progress 源码与其专属依赖。

## 与前后方案的关系

依赖方案 05 已经产生 `bevy_brp` 的 `v0.1.0` 可消费基线。完成后，方案 07 再只在 Widgetry 内同步当前文档并验证外部 Runtime 集成。

## 方案内容

## 8. bevy_widgetry 清理

Widgetry 删除本地 BRP 实现：

```text
vendor/bevy_brp_extras/
gallery/src/brp.rs
gallery/src/brp/
```

根 `Cargo.toml` 删除：

```toml
exclude = ["vendor/bevy_brp_extras"]

[patch.crates-io]
bevy_brp_extras = { path = "vendor/bevy_brp_extras" }
```

删除只为本地 BRP transport 服务的 workspace / Gallery 依赖：

```text
async-channel
async-io
bevy_brp_extras
bevy_remote
futures-util
http-body-util
hyper
serde_json
smol-hyper
```

改为 Git tag 依赖：

```toml
[workspace.dependencies]
bevy_brp_runtime = {
    git = "https://github.com/slc90/bevy_brp",
    tag = "v0.1.0",
}
```

`gallery/Cargo.toml`：

```toml
bevy_brp_runtime.workspace = true
```

Gallery 启动只保留：

```rust
use bevy_brp_runtime::BrpRuntimePlugin;

app.add_plugins(BrpRuntimePlugin::default());
```
