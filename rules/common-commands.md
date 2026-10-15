# 常用命令

以下命令在 Workspace 根目录执行。

## 格式与静态检查

**Rust 格式检查**

```powershell
cargo fmt --all -- --check
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
