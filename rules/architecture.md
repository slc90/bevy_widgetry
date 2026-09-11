# 架构规则

本文件只定义项目在架构层面必须遵守的约束。

当前 workspace 结构、crate 组成、依赖关系和架构角色等描述性信息，应记录在 `docs/architecture.md` 中，不在这里重复维护。

## Crate 依赖约束

### 顶层 facade

下层 crate 禁止反向依赖顶层 `bevy_widgetry` facade crate。

顶层 facade 不应承载具体控件的行为实现。具体控件实现应位于对应控件 crate 中。

### `core`

`core` 只允许承载：

* 明确具有跨控件共享价值的基础能力；
* 本质上属于整个库的基础设施。

不得仅因为某段代码“以后可能复用”就提前放入 `core`。

控件专属的类型、状态、样式和行为应继续留在对应控件 crate 中。

`core` 不得依赖上层控件 crate。

### 控件 crate

一个控件自己的以下内容，默认归属于该控件 crate：

* headless 行为；
* Style；
* Plugin；
* 控件专属 Component、Resource、Event、Message 和其他类型；
* 控件内部实现；
* 对应测试。

控件 crate 默认不得仅为了实现方便而依赖兄弟控件 crate。

当一个控件在语义或组合关系上明确建立在另一个控件之上时，可以形成必要的单向依赖。

禁止 crate 之间形成循环依赖。

### `test_utils`

`test_utils` 只用于共享测试基础设施。

生产代码不得依赖 `test_utils`。

需要使用 `test_utils` 的 crate，应通过测试相关依赖使用，不得将其引入正常生产依赖路径。

### `apps/*`

`apps/*` 属于库的消费者。

库 crate 不得依赖 `apps/*`。

应用层代码不得为了实现方便下沉到库中，除非该能力本身已经明确成为库需要提供的通用能力。

## Crate 内部组织

Library crate 使用 `lib.rs` 作为入口。

Application crate 使用 `main.rs` 作为入口。

`lib.rs` 主要用于：

* module 声明；
* re-export；
* 必要的 crate 级装配。

不应在 `lib.rs` 中堆放大量具体实现。

模块应按职责和语义划分。

不得仅因为：

* 文件较长；
* 希望目录更整齐；
* 希望不同 crate 结构看起来一致；

就主动拆分或重组 module。

不得机械采用“一种类型一个文件”的组织方式。

## Module 布局

除集成测试目录外，源码统一使用现代 Rust module 布局，不使用 `mod.rs`。

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

该结构只是常见组织方式，不要求所有 crate 强行采用相同目录形态。

`tests/` 下的集成测试可以根据实际需要使用 `mod.rs` 组织测试模块。

## 可见性

所有声明默认使用满足当前需求的最小可见性。

```text
仅当前 module 使用
→ private

仅 crate 内共享
→ pub(crate)

明确属于对外 API
→ pub
```

不得仅为了：

* 实现方便；
* 跨 module 调用方便；
* 测试方便；

而扩大可见性。

普通开发任务不得顺手扩大公共 API 面。

哪些内容属于正式公共 API，应由专门的公共 API 设计工作决定。
