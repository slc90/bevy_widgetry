# 常用命令

以下命令在 Workspace 根目录执行。

## 格式与静态检查

**Rust 格式检查**

```powershell
cargo fmt --all -- --check
```

**TOML 格式化与检查**

使用 Taplo CLI。尚未安装时可执行：

```powershell
cargo install taplo-cli --locked
```

只格式化本次修改的 TOML 文件，显式指定路径。例如格式化根目录的 `Cargo.toml`：

```powershell
taplo fmt Cargo.toml
```

只检查格式，不修改文件：

```powershell
taplo fmt --check Cargo.toml
```

为兼容 Taplo，需要跨行配置时使用普通表，例如 `[workspace.dependencies.bevy]`，不要在内联表 `{ ... }` 的键值对之间换行；内联表中仍可包含跨行数组。遇到语法错误先修复，不使用 `--force` 强制格式化。

Cargo 配置修改后，可在不编译、不测试的情况下检查 Workspace 清单解析：

```powershell
cargo metadata --locked --offline --no-deps --format-version 1
```

**Workspace 编译检查**

```powershell
cargo check --workspace
```

**Workspace Clippy 检查**

```powershell
cargo clippy --workspace --all-targets
```

## 构建与测试

**Workspace 完整构建**

```powershell
cargo build --workspace
```

**Workspace 全部测试**

```powershell
cargo test --workspace
```

**指定 Crate 测试**

```powershell
cargo test -p <package-name>
```

## Gallery

**运行 Widget Gallery**

```powershell
cargo run -p widget_gallery
```

## 使用原则

根据当前任务的修改范围和验证需求选择必要命令，不要求每次执行全部检查、构建和测试。

具体开发流程、测试要求和专项验证分别遵循 `rules/development.md`、`rules/testing.md` 及对应规则。

Windows 命令执行环境遵循 `rules/project-context.md`。
