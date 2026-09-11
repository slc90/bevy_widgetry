# Stage 9 目标 1：工程规范与 Codex 开发约束

## 目标

建立一套适合 `bevy_widgetry` 长期维护的工程规范，用于统一后续人工开发与 Codex 开发行为。

这一目标的重点不是建立一套庞杂的“完整规范体系”，而是把项目真正需要长期保持一致的工程约束明确下来，并区分：

- 哪些规则应由工具自动执行；
- 哪些规则应由 Codex 在开发任务中遵守；
- 哪些内容属于项目设计与历史讨论，应继续放在 `docs/` 中，而不是混入规则文件。

目标 1 最终形成两类交付物：

1. 面向 Codex 的工程规则体系；
2. 面向 Rust 工具链的基础 lint / fmt 配置。

---

## 一、规则体系结构

规则不集中塞进一个巨大的 `engineering.md`，而是按职责拆分。

根目录使用 `AGENTS.md` 作为 Codex 的统一入口和规则路由文件，详细规则放在 `rules/` 中。

建议结构：

```text
AGENTS.md

rules/
├── architecture.md
├── code.md
├── dependencies.md
├── documentation.md
├── task-scope.md
└── testing.md
```

### `AGENTS.md`

`AGENTS.md` 保持足够薄，只负责：

- 说明项目的基本开发入口；
- 告诉 Codex 不同类型任务需要读取哪些规则文件；
- 不重复详细规则内容。

例如：

```text
任何代码修改
→ rules/task-scope.md
→ rules/code.md

涉及 crate / module / 可见性 / 架构
→ rules/architecture.md

涉及依赖或新增 crate
→ rules/dependencies.md

涉及注释 / rustdoc
→ rules/documentation.md

涉及测试 / bug 修复 / 新行为
→ rules/testing.md
```

---

## 二、架构约束

### 1. Workspace 层级关系

`bevy_widgetry` 是顶层 facade crate。

它负责：

- 对外统一入口；
- re-export；
- 必要的整库级组合。

它不承担具体控件实现。

下层 crate 不得反向依赖 `bevy_widgetry`。

### 2. `core` 的职责

`core` 只放：

- 明确具有跨控件共享价值的基础能力；
- 本身就属于整个库的基础设施。

不能因为“以后可能会复用”就提前把代码塞进 `core`。

如果某个能力当前只是某个控件自己的实现细节，就应继续留在该控件 crate 内。

### 3. 控件 crate 之间的依赖

默认情况下，一个控件 crate 不应仅为了实现方便而依赖另一个兄弟控件 crate。

但如果一个控件在语义或组合关系上明确建立在另一个控件之上，可以存在单向依赖，例如：

```text
spin_box → text_field
property_editor → combo_box
```

禁止循环依赖。

### 4. `test_utils`

`test_utils` 只作为测试基础设施使用。

生产代码不得依赖 `test_utils`，相关依赖应放在 `dev-dependencies` 中。

### 5. `apps/gallery`

Gallery 是库的消费者和真实使用参考。

库 crate 不得依赖 Gallery。

Gallery 同时承担项目主要的可运行使用示例职责，因此当前不额外维护一套重复的 `examples/`。

---

## 三、crate 与 module 组织

### 1. library crate

`crates/*` 下的 library crate 使用 `lib.rs` 作为入口。

`lib.rs` 主要负责：

- module 声明；
- re-export；
- 必要的 crate 级装配。

不应把大量具体实现堆在 `lib.rs` 中。

### 2. application crate

`apps/*` 下的 binary crate 使用 `main.rs`。

`main.rs` 负责应用启动和应用级装配。

### 3. module 布局

除集成测试目录外，项目源码统一使用现代 Rust module 布局，不再使用 `mod.rs`。

推荐形式：

```text
src/
├── lib.rs
├── headless.rs
├── headless/
│   ├── state.rs
│   └── behavior.rs
├── style.rs
└── style/
    ├── layout.rs
    └── visual.rs
```

`tests/` 作为集成测试目录，可按实际需要使用传统组织方式。

### 4. 模块拆分原则

模块按职责和语义拆分，不按文件长度或类型数量机械拆分。

禁止因为“文件有点长”“这样看起来更整洁”等原因主动重构目录结构。

---

## 四、Codex 修改范围约束

Codex 的任务范围必须严格 narrow 到当前明确给定的目标。

为了正确完成目标所必需的修改，Codex 可以自主完成，不需要逐项询问，包括：

- 必要的代码调整；
- 必要的局部重构；
- 为完成目标必须修复的已有 bug；
- 必要的跨 crate 修改。

判断标准：

```text
不做这个修改
↓
当前目标无法正确完成
↓
允许直接做
```

如果不做某项修改，当前目标仍然可以完整正确完成，则不应修改。

包括但不限于：

- opportunistic cleanup；
- 顺手重构；
- 顺手修复无关 bug；
- 无关命名调整；
- 无关目录调整；
- 无关公共 API 修改；
- 无关依赖整理。

发现无关问题时，可以报告，但不应顺手修改。

---

## 五、代码硬约束

### 1. 禁止 `unsafe`

Workspace 自有源码统一禁止 `unsafe`。

建议在 workspace lint 中使用：

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
```

如果未来真的出现底层互操作等必须使用 `unsafe` 的需求，应作为独立架构决策重新讨论，而不是普通开发任务自行引入。

### 2. 禁止占位实现

非测试代码中禁止：

- `todo!()`
- `unimplemented!()`

`dbg!()` 不得出现在最终提交代码中。

### 3. `panic!()`

`panic!()` 不作为普通错误处理方式。

默认情况下普通开发任务不应新增 `panic!()`。

只有在错误已经无法处理、无法继续合理向上抛出、并属于不可恢复的内部不变量破坏时，才允许在记录足够错误上下文后 `panic!()`。

### 4. 禁止静默吞错

错误处理遵循：

```text
能处理
→ 当前层处理

不能处理
→ 向上返回

无法恢复且无法向上返回
→ 记录日志 + panic
```

禁止通过以下方式静默丢弃错误：

- 无意义的 `.ok()`；
- `let _ = ...` 丢弃关键错误；
- 空的 `Err(_) => {}`；
- 仅为了通过编译而忽略失败结果。

### 5. 零 warning

Workspace 自有代码在正常构建、测试和 Clippy 检查中应保持零 warning。

不得遗留 unused import、unused variable、dead code 或其他 compiler / Clippy warning。

### 6. lint suppression

禁止通过大范围 `#[allow(...)]` 或 `#![allow(...)]` 绕过 lint。

应优先修正真实问题。只有 lint 确实属于误报或不可避免时，才允许将 suppress 限制在最小作用域，并使用中文注释明确说明原因。

---

## 六、源码布局与格式

项目希望源码 item 的组织稳定，避免 Codex 产生无意义的重排 diff。

源码应按稳定的职责块组织，例如：

```text
mod
use
const
struct
enum
trait
fn
impl
```

具体规则以项目工程规则文件为准。

这类 item 级排序和空行约束，rustfmt 无法完整表达，因此需要作为工程规则由 Codex 遵守。

结构体字段内部遵循标准 Rust 风格，不人为插入多余空行。

---

## 七、注释与 rustdoc

项目中的人工代码注释和文档注释统一使用中文。

`struct`、`enum`、`trait`、`fn`、`const` 等具有独立语义的声明，应有说明其职责和语义的注释。注释应解释它是做什么的、为什么存在、以及关键语义或约束，禁止把名字机械翻译一遍作为注释。

`struct` 字段应说明字段的实际用途。

`impl` 块本身无需为了形式统一而额外写注释。

函数内部只在关键步骤、非显然逻辑、重要约束或需要解释“为什么这样做”的地方写注释，禁止给普通赋值、调用、循环等写旁白式注释。

每个测试本体应明确说明测试场景、验证行为或不变量，不能只是把测试函数名翻译一遍。测试函数内部同样只在关键步骤写注释。

所有注释必须单独成行，禁止行尾注释。

项目禁止 doctest。

rustdoc 仍然保留，因为它会成为外部项目中的 AI 理解 Widgetry 公共 API 的重要上下文。源码中的 `///` / `//!` 应说明 public API 的职责、使用语义、关键约束、调用前提和重要行为差异。

生成后的 rustdoc 不作为项目内部额外维护的一套文档。

---

## 八、命名规范

不对普通函数、struct、enum、trait 的具体命名施加额外限制，遵循正常 Rust 习惯即可。

只保留少量具有项目统一价值的规则：

- `Component` / `Resource` 不额外加技术类型后缀；
- 实现 Bevy `Plugin` 的类型统一使用 `Plugin` 后缀；
- 真正表示样式数据或解析后样式值的类型使用 `Style` 后缀；
- 控件内部专属类型保留控件语义前缀，例如 `ComboBoxField`、`ComboBoxPopup`、`ComboBoxOption`。

---

## 九、crate 目录与 package 命名

`crates/` 下目录使用简洁功能名，Cargo package 统一使用 `bevy_widgetry_<功能名>`，顶层 facade crate 使用 `bevy_widgetry`。

例如：

```text
crates/button
→ bevy_widgetry_button

crates/combo_box
→ bevy_widgetry_combo_box
```

目录名与 package name 不一致是有意设计的命名层次。

---

## 十、依赖管理

外部依赖只要适合 workspace 统一管理，其版本统一定义在根 `Cargo.toml` 的 `[workspace.dependencies]` 中。

子 crate 使用 `.workspace = true`。

内部 `bevy_widgetry_*` crate 继续使用 path dependency。

---

## 十一、最小可见性

所有声明默认使用满足当前需求的最小可见性：

```text
仅当前 module 使用
→ private

crate 内共享
→ pub(crate)

明确属于对外 API
→ pub
```

不得仅为了实现方便扩大可见性。

哪些内容最终属于公共 API，不在目标 1 中决定，留到 Stage 10 统一整理。

---

## 十二、测试与质量

新增或改变可观察行为时必须有对应测试；修复 bug 时应增加能够复现该 bug 的 regression test；纯结构重构如果行为不变，不要求为了重构本身新增测试。

测试重点验证 Widgetry 自身的行为和语义，不重复验证 Bevy 或第三方库已经保证的内部行为。

后续开发中，人工讨论重点放在功能应该保证什么、哪些行为是关键语义、哪些边界条件会影响设计、哪些 regression case 必须固定。Codex 负责把这些测试意图落成测试代码，并补齐机械性的覆盖和必要的参数化 case。

内部模块逻辑测试放在对应源码模块内的 `#[cfg(test)] mod tests` 中。

从 crate 外部视角验证公共行为、多模块协作或真实 Bevy/ECS 组合行为时，放在 `tests/` 中做 integration test。

多个 crate / integration test 共用的测试基础设施放在 `test_utils`，不要复制相同 helper，也不要为了测试方便把私有实现改成 `pub`。

当前测试增强工具只考虑 `proptest`。它不是所有测试的强制方案，而是在大量输入组合、数值边界、状态空间、不变量和操作序列等场景下优先考虑。普通明确行为仍使用常规单元测试 / 集成测试。

---

## 十三、工具链配置

### workspace lint

根 `Cargo.toml` 中至少加入：

```toml
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
```

每个 workspace member 显式继承：

```toml
[lints]
workspace = true
```

### Clippy

根目录 `clippy.toml`：

```toml
allow-unwrap-in-tests = true
```

因此正常生产代码禁止 `unwrap()`，测试代码允许使用 `unwrap()`。当前不额外禁止 `expect()`。

### rustfmt

根目录 `rustfmt.toml` 保持简洁：

```toml
edition = "2024"
style_edition = "2024"

newline_style = "Unix"

reorder_imports = true
reorder_modules = true
```

不为了 import 分组等格式特性引入 nightly rustfmt。无法通过 rustfmt 表达的源码 item 排序和空行规则继续由工程规范约束。

---

## 目标 1 的最终状态

Stage 9 目标 1 最终建立的是一套适合 AI-first 开发的工程约束体系：

```text
AGENTS.md
→ Codex 规则入口与路由

rules/
→ 详细工程规范

Cargo.toml / clippy.toml / rustfmt.toml
→ 可以由工具自动执行的约束
```

其核心原则是：

- 让 Codex 的工作范围保持 narrow；
- 允许它自主完成目标所需的全部必要修改；
- 禁止顺手重构、顺手修 bug 和无关 cleanup；
- 能自动检查的规则交给工具；
- 无法自动检查但长期有价值的规则写入 `rules/`；
- 项目设计、Stage 和讨论结果继续留在 `docs/`；
- 公共 API 的语义尽量贴近代码，通过 rustdoc 提供给未来使用 Widgetry 的 AI。

至此，Stage 9 目标 1「建立工程规范」具备了明确的规则边界、文档结构和工具链落地方式，可以进入下一目标。
