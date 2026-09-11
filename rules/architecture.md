# 架构规则

## Crate 职责

### `bevy_widgetry`

- 作为顶层 facade / 聚合 crate。
- 负责统一公共入口、re-export，以及真正属于整个库级别的组合。
- 不承载具体控件的行为实现。
- 下层 crate 禁止反向依赖 `bevy_widgetry`。

### `core`

- 只放真正具有跨控件角色的共享基础能力，或本质上属于整个库的基础设施。
- 不得仅因为“以后可能复用”就把代码放入 `core`。
- 控件专属类型、状态、样式、行为必须留在对应控件 crate。
- `core` 不得依赖上层控件 crate。

### 控件 crate

- 一个控件自己的 headless 行为、样式、Plugin、控件专属类型、事件/消息/组件及测试，默认归属于该控件 crate。
- 不得为了实现方便而依赖兄弟控件 crate。
- 当一个控件在语义或组合关系上明确建立于另一个控件之上时，允许形成单向依赖。
- crate 之间禁止循环依赖。

### `test_utils`

- 只承载共享测试基础设施。
- 生产依赖不得依赖 `test_utils`；需要时通过 dev-dependency 使用。

### `apps/*`

- `apps/*` 是库的消费者。
- 库 crate 不得依赖 `apps/*`。
- Gallery 属于实际使用方和可运行参考，不是底层库实现的一部分。

## Crate 内部组织

- library crate 使用 `lib.rs` 作为入口。
- application crate（如 `apps/gallery`）使用 `main.rs` 作为入口。
- `lib.rs` 主要负责模块声明、re-export 和必要的 crate 级装配，不应堆放大量具体实现。
- 模块按职责和语义划分，不按“一个类型一个文件”机械拆分，也不以文件行数作为强制拆分依据。
- 不得仅为了“更整洁”“更统一”或减少单文件长度而主动重构目录结构。

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

`tests/` 下的集成测试允许根据需要使用 `mod.rs` 组织测试模块。

## 可见性

- 所有声明使用满足当前需求的最小可见性。
- 仅当前 module 使用：保持私有。
- 仅 crate 内共享：优先使用 `pub(crate)`。
- 只有明确属于库对外 API 的内容才使用 `pub`。
- 不得为了实现方便、跨 module 调用或测试方便而扩大可见性。
- 哪些内容应成为正式公共 API，由公共 API 设计阶段决定，普通任务不得顺手扩大 API 面。
