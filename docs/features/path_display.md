# 路径显示

`bevy_widgetry_path_display` 提供 `display_path(&std::path::Path) -> String`，把 Windows 路径转换为适合界面呈现的文字。它只依赖标准库，不需要 Bevy、Plugin 或运行时初始化；目标平台与 Workspace 一致，为 Windows 64 位。

## 直接依赖

只需要路径显示能力的库可以直接依赖独立 crate：

```toml
[dependencies]
bevy_widgetry_path_display = { git = "https://github.com/slc90/bevy_widgetry" }
```

```rust
use bevy_widgetry_path_display::display_path;
use std::path::Path;

let text = display_path(Path::new(r"\\?\C:\Users\用户\数据"));
assert_eq!(text, r"C:\Users\用户\数据");
```

也可以使用本地 Path Dependency。该 Package 设置 `publish = false`，不通过 crates.io 分发。

## 从 Facade 使用

已有 Widgetry 应用可以显式启用默认关闭的 `path_display` Feature：

```toml
[dependencies.bevy_widgetry]
git = "https://github.com/slc90/bevy_widgetry"
default-features = false
features = ["path_display"]
```

```rust
use bevy_widgetry::path_display::display_path;
```

Facade 仍会引入其基础依赖。仅需格式化的库应直接依赖 `bevy_widgetry_path_display`。FileDialog 直接使用独立 crate，不要求开启 Facade 的 `path_display` Feature。

## 显示契约

函数借用原始路径，返回拥有所有权的显示字符串。只按 Windows Prefix 类型简化 VerbatimDisk 和 VerbatimUNC：

| 输入 | 输出 |
| --- | --- |
| `\\?\C:\Users\用户\数据` | `C:\Users\用户\数据` |
| `\\?\C:\` | `C:\` |
| `\\?\UNC\server\share\目录` | `\\server\share\目录` |
| `\\?\UNC\server\share` | `\\server\share` |
| 普通磁盘路径、普通 UNC、相对路径和空路径 | 原文字 |
| `\\.\COM1`、`\\?\Volume{...}\` 等其他命名空间 | 原文字 |

除转换的开头外，保留分隔符、尾部分隔符、大小写、空格、中文、`.` 与 `..`，不规范化或截断正文。摘要长度由调用方控制。

函数不访问文件系统，不调用 `canonicalize`，不检查路径存在性，不修改输入，不返回错误或产生日志。

## 非 Unicode 与真实路径

无法用 Unicode 表示的路径遵循 `Path::display` / `to_string_lossy` 的有损显示语义，使用替代字符；转换不承诺可逆。

输出只用于呈现。文件操作、路径比较、缓存身份、持久化和 Shell 接口继续使用原始 `Path` / `PathBuf` 或所需的 UTF-16 单元，不能从显示字符串重新构造文件操作路径。
