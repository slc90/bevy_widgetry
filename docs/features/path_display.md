# 路径转换与输出

`bevy_widgetry_path_display` 提供 Windows 路径的前缀简化及文本输出，适用于 UI、日志、报告和配置文件中的路径字段。运行时代码只依赖标准库，不需要 Bevy、Plugin 或运行时初始化；目标平台与 Workspace 一致，为 Windows 64 位。

| API | 返回值 | 用途 |
| --- | --- | --- |
| `simplify_path(&Path)` | `PathBuf` | 保留原生路径编码，供配置路径字段及其他路径消费者使用 |
| `display_path(&Path)` | `String` | 生成适合 UI、日志、报告或路径列表的有损 Unicode 文字 |

## 直接依赖

只需要路径转换能力的库可以直接依赖独立 crate：

```toml
[dependencies]
bevy_widgetry_path_display = { git = "https://github.com/slc90/bevy_widgetry" }
```

```rust
use bevy_widgetry_path_display::{display_path, simplify_path};
use std::path::Path;

let original = Path::new(r"\\?\C:\Users\用户\数据");
let path = simplify_path(original);
assert_eq!(path.as_os_str(), Path::new(r"C:\Users\用户\数据").as_os_str());
let text = display_path(original);
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
use bevy_widgetry::path_display::{display_path, simplify_path};
```

Facade 仍会引入其基础依赖。仅需格式化的库应直接依赖 `bevy_widgetry_path_display`。FileDialog 直接使用独立 crate，不要求开启 Facade 的 `path_display` Feature。

## 前缀转换契约

两个函数均借用原始路径，返回拥有所有权的结果。只按 Windows Prefix 类型简化 VerbatimDisk 和 VerbatimUNC：

| 输入 | 输出 |
| --- | --- |
| `\\?\C:\Users\用户\数据` | `C:\Users\用户\数据` |
| `\\?\C:\` | `C:\` |
| `\\?\UNC\server\share\目录` | `\\server\share\目录` |
| `\\?\UNC\server\share` | `\\server\share` |
| 普通磁盘路径、普通 UNC、相对路径和空路径 | 原文字 |
| `\\.\COM1`、`\\?\Volume{...}\` 等其他命名空间 | 原文字 |

除转换的开头外，保留分隔符、尾部分隔符、大小写、空格、中文、`.` 与 `..`，不规范化或截断正文。摘要长度由调用方控制。

两个函数均不访问文件系统，不调用 `canonicalize`，不检查路径存在性，不修改输入，不返回错误或产生日志。普通路径及不支持的命名空间保持原样；再次转换不会继续改变结果。

## 输出到文件

文本输出可以直接写入 `display_path` 的结果，例如：

```rust
use bevy_widgetry_path_display::display_path;
use std::path::Path;

let path = Path::new(r"\\?\C:\Users\用户\数据");
std::fs::write("paths.txt", display_path(path))?;
# Ok::<(), std::io::Error>(())
```

配置字段在序列化前调用 `simplify_path`。例如消费方使用 `serde_json` 时：

```rust
use bevy_widgetry_path_display::simplify_path;
use std::path::Path;

let path = Path::new(r"\\?\C:\Users\用户\数据");
let configuration = serde_json::json!({ "path": simplify_path(path) });
let bytes = serde_json::to_vec_pretty(&configuration)?;
std::fs::write("config.json", bytes)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

文件中的 JSON 为 `{"path":"C:\\Users\\用户\\数据"}`，反序列化后的字段为普通磁盘路径；UNC 路径同样输出为 `\\server\share\...`。JSON 的反斜杠转义属于文件格式本身。

转换由调用方显式执行。直接序列化原始 `PathBuf`、调用原始路径的 `display()` 或 `to_string_lossy()`，仍遵循标准库及序列化器自身的行为。

## 编码与路径语义

`simplify_path` 保留前缀以外的全部 UTF-16 单元，包括未配对代理项，不经过 Unicode 有损转换。配置格式若仅支持 Unicode，应由调用方处理无法表示的路径，例如返回序列化错误或选择原生编码格式；不能先转为有损文字再当作无损配置保存。

`display_path` 遵循 `Path::display` / `to_string_lossy` 的有损 Unicode 语义，使用替代字符；其输出适用于文本呈现，不承诺可逆。

前缀简化是词法转换，不保证 Windows Verbatim 路径与普通路径的访问语义等价。超长路径、尾随点或空格、保留设备名、`.` / `..` 和混合分隔符等可能依赖 Verbatim 语义。需要重新访问的配置路径由调用方根据用途选择是否简化；对依赖 Verbatim 语义的路径应保存原始路径。

FileDialog 的运行时路径、文件操作、路径比较、缓存身份及 Shell UTF-16 接口继续使用原始路径，不自动替换为输出结果。
