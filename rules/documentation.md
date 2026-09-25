# 注释与 Rustdoc 规则

## 语言

项目内人工编写的代码注释、rustdoc 与 Markdown 文档统一使用中文作为主要叙述语言。

本节的全部语言规则同样适用于后续编写的 Git commit message。

明确的技术术语保留英文原词，不翻译为中文。即使某个技术术语已经存在常见中文译法，也仍然使用英文。

该规则适用于编程语言、框架、API、架构、UI、领域模型以及项目代码中的技术概念。例如 Widget、Popup、hover、focus、state、system、component、event、layout、ownership、borrowing、borrow checker、lifetime、trait、closure、generic、dispatch 等。

不得为了使文本看起来更中文而翻译技术术语，也不得自行创造中文译词、简称或缩写。

中文用于描述技术概念之间的关系、行为、原因、条件和约束，不用于替换技术概念本身。

是否属于技术术语必须根据实际语义判断，不得进行机械字符串替换。同一个词只有在表达明确技术概念时才使用英文；普通自然语言中的同名词仍正常使用中文。

例如：

- “这个 system 更新 component 的 state”中的 system、component 和 state 是技术概念，应使用英文。
- “系统启动时读取配置”中的“系统”是普通自然语言，不应替换为 system。
- “窗口获得 focus”中的 focus 是技术概念，应使用英文。
- “这是当前工作的焦点”中的“焦点”是普通自然语言，不应替换为 focus。

技术术语、API 名称和代码标识符在中文正文中直接作为普通英文文本使用，不因为其技术属性额外添加 Markdown 反引号。

该规则适用于项目现有内容以及后续新增或修改的内容。整理已有代码注释、rustdoc 和 Markdown 文档时，必须根据上下文语义逐项判断，不得通过全局字符串替换机械修改。

## 声明级注释

struct、enum、trait、fn、const 等具有独立语义的声明应编写注释，说明：

- 它负责什么。
- 为什么存在。
- 需要调用方或维护者知道的关键语义与约束。

禁止仅把标识符名称翻译一遍作为注释。

对需要进入 rustdoc 的 API 使用 `///` / `//!` 文档注释；私有实现也应按其维护价值写清楚职责，但不要为了形式制造冗余文字。

impl block 本身不要求为了形式额外写注释；其 method 按照 function 规则处理。

## Struct field

struct field 需要说明 field 的实际职责和语义，尤其是 field 用途无法从名称完整推断时。

禁止只写“xxx field”“保存 xxx”之类对名称进行机械复述的注释。

## Function 内部注释

function 内部只在以下位置写注释：

- 关键步骤。
- 非显然逻辑。
- 重要约束。
- 需要解释“为什么这样做”的地方。

禁止给普通赋值、调用、循环等显然代码添加旁白式注释。

## 测试注释

每个 test function 上方都应使用中文注释写清楚以下内容，注释放在 `#[test]`、`#[rstest]` 等 test attribute 之前：

- 测试场景。
- 要验证的行为、invariant 或 regression 问题。

禁止仅翻译或复述 test function 名。

test function 内部只对关键步骤、非显然逻辑和测试方法的必要原因写注释；内部注释不能代替 function 上方的测试场景与验证目标说明，也不应重复这些说明。

## 注释位置

所有注释必须单独成行，禁止在代码同一行末尾写注释。

禁止：

```rust
let index = 0; // 当前 index
```

需要解释时应把注释放在对应代码前方。

## Rustdoc 的定位

Rustdoc 的主要职责是把公共 API 的语义直接附着在 API 上，使其他项目中的开发者或 AI 在读取依赖源码 / API 文档时能够正确理解和使用 Widgetry。

公共 API 的 rustdoc 应覆盖正确使用该 API 所必需的信息，例如：

- API 的职责。
- 重要前置条件。
- 必需 Plugin / component 关系。
- 关键 state 语义。
- 程序化修改与用户交互在行为上的差异。
- 参数中特殊值的含义。
- 其他会影响调用正确性的约束。

不要求把 rustdoc 写成完整用户手册，也不使用它承载 Stage、architecture 决策或开发规则。

## Doctest

禁止编写 doctest。

可以正常维护 rustdoc 文档注释，但不使用可执行 doctest 作为项目测试机制。
