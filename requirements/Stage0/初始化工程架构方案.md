# Stage 1 / 目标 1 工程架构方案

## 1. 当前目标

当前工程架构仅服务于：

> **Stage 1 / 目标 1：学习官方 Headless Button 原理**

这一阶段重点是理解 `bevy_ui_widgets::Button` 的组织方式，包括：

- `Button` 组件；
- `ButtonPlugin`；
- pointer input；
- keyboard activation；
- `Pressed`；
- `InteractionDisabled`；
- `Activate`；
- focus；
- accessibility；
- ECS component + observer/system + event 的组织方式。

当前不为后续 Stage 提前设计完整架构。

---

## 2. 总体原则

仓库采用渐进式开发。

早期学习代码不要求长期保留：

- 可以修改；
- 可以重构；
- 可以删除；
- 不需要为了未来兼容而提前设计抽象。

当后续 Stage 出现真实需求时，再调整工程结构。

---

## 3. Cargo 组织

当前使用 **单 crate**。

暂时不建立 Cargo workspace，也不拆分多个子 crate。

未来如果在实际开发过程中出现稳定且明确的模块边界，再考虑拆分。

当前结构：

```text
project/
├─ Cargo.toml
├─ src/
│  └─ lib.rs
└─ examples/
   └─ button.rs
```

---

## 4. Bevy 依赖

使用顶层 `bevy` crate，方便学习和实验。

为了避免完整默认 features 带来的额外编译开销，关闭默认 features，只启用 UI profile：

```toml
[dependencies]
bevy = {
    version = "=0.19.1",
    default-features = false,
    features = ["ui"]
}
```

这样既保留：

```rust
use bevy::prelude::*;
```

等方便的使用方式，也避免无关的 3D、Audio 等功能进入当前工程。

---

## 5. `src/lib.rs`

当前阶段 `src/lib.rs` 不承担实际功能。

可以保持为空：

```rust
// 暂时为空
```

Stage 1 / 目标 1 只是学习官方 Button，还没有开始实现自己的控件库功能。

等 Stage 1 后续目标真正开始添加自己的行为时，再向 `src/` 中加入正式代码。

---

## 6. `examples/button.rs`

`examples/button.rs` 是 Stage 1 / 目标 1 的唯一实验入口。

第一版从空的 Bevy App 手动搭建，不直接复制官方 Button example。

基本结构：

```text
App
├─ DefaultPlugins
├─ 必要的 UI 场景
├─ 一个官方 Button
│  └─ Text
└─ 用于观察 Activate 等行为的逻辑
```

后续研究过程中，可以直接在这个文件中逐步添加：

- pointer 行为观察；
- `Pressed` 状态观察；
- `Activate` 事件观察；
- keyboard activation；
- focus；
- disabled；
- accessibility；
- observer / system 调试代码。

当前保持单文件。

只有当实际代码明显变得难以阅读时，再考虑拆分辅助模块。

---

## 7. Plugin 策略

示例程序直接使用：

```rust
DefaultPlugins
```

当前不追求构造 Button 所需的最小 plugin 集合。

原因是 Stage 1 / 目标 1 的重点是理解 Button 本身，而不是研究最小 Bevy App 依赖。

Button 内部真正依赖哪些 plugin，可以在后续阅读 `ButtonPlugin` 实现时逐步理解。

---

## 8. 当前明确不做的内容

当前阶段暂不引入：

- `theme` 模块；
- `style` 模块；
- `headless` 模块；
- `behavior` 模块；
- 多 crate；
- Cargo workspace；
- helper 模块；
- `tests/` 目录；
- 最小 plugin 集合；
- 正式公共 API 设计。

其中测试相关工程结构等到 **Stage 1 / 目标 3：学习 Headless Widget 测试** 时再建立。

---

## 9. 当前最终结构

```text
project/
├─ Cargo.toml
├─ src/
│  └─ lib.rs
└─ examples/
   └─ button.rs
```

当前阶段的原则可以概括为：

> **只搭够 Stage 1 / 目标 1 使用的最小工程骨架。**
>
> 不提前为未来 Stage 设计结构；随着实际开发逐步演化工程架构。
