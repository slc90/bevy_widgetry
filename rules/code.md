# 代码规则

## 源码布局

Rust 源文件中的顶层 item 按稳定职责分组，避免无意义重排。默认顺序为：

1. mod
2. use
3. const / static item
4. struct
5. enum
6. trait
7. free function
8. impl

同类 item 保持稳定顺序。不要仅为了“更漂亮”或个人偏好重排与当前任务无关的现有 item。

组与组之间使用稳定的空行分隔；struct field 等内部排布遵循标准 Rust / rustfmt 风格，不人为在每个 field 之间插入空行。

同一 module 内的所有 use（包括 pub use）必须集中在 mod 声明之后，连续排列，中间不插入空行或其他声明；与 mod 及后续声明之间用空行分隔。function 内部不得出现 use，所需 import 统一放到所属 module 的 use 分组中。此规则同样适用于 test module 和 test function。

相邻 function 或 method 之间必须保留一个空行，包括普通 function、test function 以及 impl、trait 中的 method。下一 function 或 method 带有注释或 attribute 时，空行应放在其注释或 attribute 之前，保持注释、attribute 与对应声明紧邻。

## unsafe

Workspace 自有源码禁止使用 unsafe。

- unsafe_code 应视为 forbid 级别的约束。
- 普通开发任务不得引入 unsafe。
- 若未来出现必须使用 unsafe 的底层需求，应作为独立 architecture 决策重新讨论，而不是在普通任务中自行引入。

## 主动 panic

- crates 非测试代码禁止使用 panic!、assert!、assert_eq!、assert_ne!、debug_assert!、debug_assert_eq!、debug_assert_ne!、unreachable!、todo!、unimplemented!，以及 unwrap、expect 和其他主动触发 panic 的等价写法。
- 测试代码允许使用断言、panic、unwrap 和 expect；测试基础设施仍须正确处理运行期失败。
- 不额外禁止正常 Rust 的 Result / Option 使用，也不因为本规则把正常错误处理改成复杂包装。

## 占位与调试代码

非测试生产代码：

- 禁止留下 todo!()。
- 禁止留下 unimplemented!()。
- dbg!() 不得进入最终代码。
- crates 中不存在“不可恢复错误允许 panic”的例外。

## 错误处理

绝对禁止静默吞掉错误。

处理顺序：

1. 当前层能够正确处理：在当前层处理。
2. 当前层不能处理：记录定位所需的日志，通过 Result 上抛 BevyError；system、observer 和 command 将错误交给 Bevy error handler，普通 function 交给调用方。
3. 无法恢复或内部 invariant 被破坏：同样记录 ERROR 并上抛，不得改成 panic、断言或仅记录后 return / continue。

Widgetry 创建及转换的失败使用 Severity::Error（例如 BevyError::error）；不得依赖 BevyError 默认 From 转换的 Severity::Panic。下层失败上抛时须保留错误内容并调整 severity。库不得设置宿主 App 的 error handler。

正常 Option 缺失、asset 等待、observer 目标过滤及公开 API 明确定义的 no-op 仍按原 contract 处理，不应伪造错误。SceneComponent 的构造校验应放入可失败的 template / Scene 展开路径。

deferred BSN 构造使用 core/facade scene module 的 spawn_scene_with_error_handler / apply_scene_with_error_handler，将 Scene 失败显式交给宿主 handler。不得依赖原生 Commands::spawn_scene 的仅日志处理或 EntityCommands::apply_scene 的默认 Panic severity。同步 World / EntityWorldMut Scene API 的 Result 必须由调用层处理，并转换为 Severity::Error。

以下行为如果目的是丢弃错误，均视为静默吞错并禁止：

- 用 `let _ = ...` 丢弃 Result / 错误。
- 无意义地调用 .ok() 只为忽略错误。
- 空的 `Err(_) => {}` 分支。
- 任何仅为了让编译或 lint 通过而丢掉失败信息的写法。

内部同步 renderer 展开使用 core::scene 的 spawn_scene / apply_scene 失败清理适配，再由业务 owner 记录日志并转为 Severity::Error。Scene template 中新建的空 entity 属于本次构造预约，失败时清理；独立业务 entity 应在创建时携带其 Component。已写入 Component 的业务副作用及已有 root 的部分 patch 不自动回滚。

## Lint 抑制

不得通过 `#[allow(...)]`、`#![allow(...)]` 等方式单纯绕过 compiler / Clippy 规则。

- 首选修正产生 lint 的代码。
- 只有当 lint 在当前语义下确属误报或不可避免时，才允许 suppress。
- suppress 必须限制到最小 scope。
- 必须用中文注释说明为什么此处需要例外。
- 禁止使用大范围兜底形式，例如 `#![allow(unused)]`、`#![allow(dead_code)]`、`#![allow(clippy::all)]`。

项目级例外：Workspace 明确允许 clippy::type_complexity 和 clippy::too_many_arguments，统一在 root `Cargo.toml` 中配置为 allow。这两项不受上述局部 suppress 条件、最小 scope 和逐处注释要求限制，不得作为 lint 清理擅自删除，也无需仅为满足这两项检查而重构代码。其他 lint 仍遵循上述规则。

## Warning

Workspace 自有代码必须保持零 warning。

- 正常构建、测试和 Clippy 检查不得遗留 warning。
- 不保留“以后可能会用”的 dead code；没有当前明确用途的代码不应仅为未来猜测而存在。
- 不得通过扩大 allow 范围来制造表面上的零 warning。

## 命名中的技术后缀

只规定具有稳定工程价值的后缀：

- Component type 名不追加 Component 后缀。
- Resource type 名不追加 Resource 后缀。
- 实现 Bevy Plugin 的 type 统一使用 Plugin 后缀。
- 真正表达 style 数据或解析后 style 值的 type 可以并应使用 Style 后缀。

function 名及普通 struct / enum / trait 的具体命名不额外制定项目规则，遵循正常 Rust 习惯并由实现者根据语义命名。
