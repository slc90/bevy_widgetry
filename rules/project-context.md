# 项目背景规则

本文件记录会直接影响工程设计、实现和 Code Review 判断的长期项目背景。

这些背景属于项目当前事实和明确边界，不是未来兼容性目标。

设计、实现和 Review 都必须基于这些事实进行判断，不得为了项目当前并不支持、也没有明确要求支持的场景增加额外代码或工程复杂度。

## 目标平台

bevy_widgetry 当前只面向 Windows 64 位环境。

项目不以跨平台兼容为当前目标。

除非当前任务明确要求改变平台支持范围，否则：

- 不为 Linux、macOS、Web 或其他非 Windows 平台设计兼容路径；
- 不为非 Windows 平台增加 fallback、stub、空实现或错误分支；
- 不为了潜在的跨平台需求增加 abstraction layer；
- 不增加仅用于区分 Windows 与非 Windows 的 `#[cfg(windows)]` / `#[cfg(not(windows))]` 分支；
- 可以直接依赖 Windows 平台提供的能力，以及适用于 Windows 的 crate 和 API；
- 不得为了使项目能够在非 Windows target 上编译，而增加当前需求不需要的代码。

例如，如果实现只服务于当前项目：

```rust
#[cfg(windows)]
fn foo() {
    // ...
}

#[cfg(not(windows))]
fn foo() {
    // ...
}
```

通常应直接写成：

```rust
fn foo() {
    // ...
}
```

因为非 Windows target 不属于当前项目需要支持的环境。

这条规则只用于排除为了未支持平台产生的额外复杂度。

如果某段代码本身自然具有跨平台能力，并且没有因此增加额外实现、分支、抽象或维护成本，不需要为了强调 Windows-only 而主动改写成 Windows-specific 实现。

## 目标指针宽度

项目仅支持 target_pointer_width 为 64 的 target，指针、usize 和 isize 均为 64 位。

32 位 target 不属于项目支持范围。除非当前任务明确改变该范围，否则不为 32 位 target 增加兼容分支、fallback、替代算法或额外抽象。

每个 Workspace crate 的入口必须包含以下编译检查，统一拒绝非 Windows 或非 64 位 target：

```rust
#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");
```

Library 在 lib.rs 中检查，Application 在 main.rs 中检查，不在内部 module 重复添加。

此检查用于拒绝不支持的 target，不属于为未支持环境增加的兼容实现。

## 当前目标环境

当前开发、构建、运行和调试环境以 Windows 64 位环境为准。

Workspace 当前针对 `x86_64-pc-windows-msvc` 配置 Rust linker，并使用 DX12 作为 wgpu backend。

除非当前任务明确涉及 target、平台支持范围或 rendering backend 调整，否则工程设计应以这一环境作为默认前提。

不得为了没有明确需求的其他平台：

- 增加额外 Cargo feature；
- 增加 target-specific dependency；
- 增加平台 abstraction；
- 增加 fallback implementation；
- 修改当前 Windows 实现以保持其他平台可编译。

## 不为假设中的未来需求设计

项目只实现当前已经明确存在的需求。

不得仅因为以下理由增加代码：

- “以后可能支持其他平台”；
- “作为 library 最好更通用”；
- “以后迁移时可能方便”；
- “先把平台差异抽象掉”；
- “顺便保留一个 fallback”；
- “以后可能会有第二种 backend”。

如果未来确实需要支持新的平台、target 或 backend，应作为独立的 scope / architecture 变更，根据当时的真实需求重新设计。

不要提前为未知未来保留当前没有明确职责的代码、接口、分支或抽象。

## 与其他规则的关系

本文件描述：

```text
项目处于什么背景和边界之中
```

`docs/architecture.md` 描述：

```text
项目当前实际 architecture 和 Workspace 结构
```

`rules/code.md` 描述：

```text
代码本身应遵守什么实现规范
```

`rules/task-scope.md` 描述：

```text
当前任务允许修改什么
```

项目背景不是具体代码风格规则。

例如，本文件并不是抽象地禁止使用 `#[cfg(...)]`。

如果当前任务真实需要区分某些 Windows target、Windows capability 或 feature，且该条件分支属于当前明确需求，则可以使用对应的条件编译。

禁止的是为了项目明确不支持的环境增加额外兼容代码。

## Code Review

Code Review 必须将本文件描述的项目背景作为判断当前 change 是否合理的依据。

如果当前 change 因为项目实际不支持的场景而新增了不必要复杂度，应作为 finding 指出。

典型问题包括：

- 新增仅用于支持非 Windows 平台的 `#[cfg(windows)]` / `#[cfg(not(windows))]` 分支；
- 新增非 Windows fallback、stub 或空实现；
- 新增仅用于兼容 32 位 target 的分支、fallback、替代算法或抽象；
- 新增仅用于未来跨平台支持的 trait、wrapper、adapter 或 abstraction；
- 为不支持的平台增加 dependency、Cargo feature、build configuration 或测试；
- 为保持非 Windows target 可编译而增加额外 control flow；
- 为假设中的未来 backend 或平台预留当前没有职责的接口；
- 因追求不需要的通用性而使当前 Windows 实现明显更复杂。

仅仅因为代码本身能够跨平台运行，不构成问题。

只有当 change 为项目不支持、也没有明确要求支持的场景承担了额外复杂度时，才应作为 Review finding。
