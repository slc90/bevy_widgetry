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

## 允许的注释

只允许以下三类注释，其他一律不写，包括普通声明、field、function、test function 的说明与公共 API rustdoc。

### lib.rs / main.rs 文件头部

lib.rs / main.rs 的注释写在文件头部，只描述该库或 App 的功能，不记录实现细节、architecture、开发规则或历史方案。

### 坑点记录

当代码是为了解决某个坑，而原因无法从代码中直接看出时，在相关代码前记录坑点。

注释应说明触发条件、会出现的问题，以及当前写法为什么能避开该问题；不复述代码操作或补充一般性的职责说明。

### 测试文件头部的状态机描述

测试文件头部使用 //! 描述被测行为的状态机与测试覆盖模型，记录相关 state 维度、stimuli、guards、transitions、invariants 与 couplings，具体内容遵守 [测试规则](testing.md)。

不再为每个 test function 单独编写场景或验证目标注释；测试代码中的坑点仍按上述坑点记录规则处理。

## 注释位置

所有注释必须单独成行，禁止在代码同一行末尾写注释。

禁止：

```rust
let index = 0; // 当前 index
```

需要解释时应把注释放在对应代码前方。

## Doctest

禁止编写 doctest。
