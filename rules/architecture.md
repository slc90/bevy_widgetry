# 架构规则

## Workspace / Crate 职责与依赖管理

当前 Workspace 结构与各成员职责见 `docs/architecture.md`。本节约束 crate 的职责边界、依赖方向及依赖管理方式。

### Crate 职责与依赖方向

**顶层 Facade**

`bevy_widgetry` 负责聚合和导出公共 API，不承载具体 Widget 的行为实现。

下层 crate 不得反向依赖 Facade。

**Core**

`core` 只承载具有明确跨 Widget 共享价值的能力，以及属于整个 Widgetry 库的基础设施。

Widget 专属的 Type、State、Style 和行为应保留在对应 Widget crate 中。不得仅因某项能力将来可能复用就提前放入 `core`。

`core` 不得依赖上层 Widget crate。

**Widget Crate**

各 Widget 的 Headless 行为、Style、Plugin、专属 ECS Type、内部实现、测试及 Benchmark，原则上归属于对应 Widget crate。

Widget crate 不得仅为了实现方便而依赖兄弟 Widget crate。只有当 Widget 在语义或组合关系上明确依赖另一个 Widget 时，才允许建立必要的单向依赖。

**Test Utils**

`test_utils` 只提供共享测试与 Benchmark 基础设施，不得进入生产依赖路径。

**Gallery**

Gallery 是 Widgetry 库的消费者，库 crate 不得依赖 Gallery。

Gallery 专属能力应保留在应用内部，除非该能力已经明确属于 Widgetry 库需要提供的通用功能。

### 外部依赖管理

能够统一管理的外部依赖，其版本在根目录 `Cargo.toml` 的 `[workspace.dependencies]` 中定义。

Workspace 成员通过 `workspace = true` 引用，不得在多个 crate 中分别维护同一外部依赖的版本。

各 crate 可以根据实际需要配置自身使用的 Dependency Feature，不要求所有 crate 启用完全相同的 Feature 集合。

新增依赖必须服务于当前明确需求，不得仅为了实现方便或假设中的未来用途引入依赖。

### 内部依赖与命名

Workspace 内部的 `bevy_widgetry_*` crate 使用 Path Dependency，并遵守上述职责与依赖方向约束。

`crates/` 下的目录使用简洁的功能名，对应 Cargo Package 统一使用 `bevy_widgetry_<功能名>` 格式。

例如：

```text
crates/core       → bevy_widgetry_core
crates/button     → bevy_widgetry_button
crates/combo_box  → bevy_widgetry_combo_box
```

顶层 Facade 例外，其 Package 名为 `bevy_widgetry`。

目录名与 Package 名不要求一致，也不应为追求名称一致而调整既有结构。

## Assets

### Widgetry 内建 Asset

Widgetry 库自身使用的 SVG、Image、Font 及其他静态 Asset 统一存放在 `crates/asset`，由 `bevy_widgetry_asset` 负责物理文件、Embedded 注册和语义 Asset 标识。

其他库 crate，包括 `core` 和各 Widget crate，不得自行建立或维护运行时 Asset 目录。

`asset` crate 仅依赖 Workspace Bevy 与底层内部日志 crate，不依赖 `core` 或 Widget crate，也不承担上层 SVG 等 Asset 的解析职责。

`core` 可以使用 `asset` 提供 App 级默认字体等共享基础设施。内建 Asset API 不直接通过顶层 Facade 导出。

### Asset 访问与注册

除负责 Asset 管理的 Module 外，其他库代码及测试不得直接使用内建 Asset 的物理或虚拟路径，例如 `assets/...`、`../assets/...` 或 `embedded://...`。

内建 Asset 必须通过语义标识访问，例如 `BuiltinIcon::WindowClose`。

跨 crate 所需的标识和路径接口可以使用 `pub`，但仍属于 Workspace 内部 API。

新增 Asset 类型应使用 `BuiltinFont`、`BuiltinImage` 等语义明确的 Type，不建立混合所有 Asset 种类的通用 Enum。

使用内建 Asset 的库 Plugin 必须自动确保 `WidgetryAssetPlugin` 已注册，避免重复添加，不要求宿主手动配置内部 Asset Plugin。

### 应用 Asset

应用专属 Asset 不得仅因使用 Widgetry 就迁入库 Asset 层。

Gallery 的 Logo、展示 Image、Demo 专属 Icon 等统一由 `gallery/src/assets.rs` 和 `gallery/src/assets/` 管理，并由应用入口显式装配 `GalleryAssetPlugin` 完成 Embedded 注册。

Gallery 其他代码通过自身语义标识访问应用 Asset。

只有当某项 Asset 已明确成为 Widgetry 库功能的一部分时，才允许将其迁入库 Asset 层。

## 源码与 Module 组织

### Crate 入口

Library crate 使用 `lib.rs`，Application crate 使用 `main.rs` 作为入口。

`lib.rs` 主要负责 Module 声明、Re-export 和必要的 Crate 级装配，不应堆放大量具体实现。

### Module 组织

Module 根据实际职责和语义划分，不机械采用一种 Type 对应一个文件的结构。

不得仅因为文件较长、希望目录整齐或希望各 crate 结构一致，就拆分或重组 Module。

除 Integration Test 目录外，源码统一采用现代 Rust Module 文件组织方式，不使用 `mod.rs`。

```text
src/
├── lib.rs
├── foo.rs
└── foo/
    ├── bar.rs
    └── baz.rs
```

`tests/` 下的 Integration Test 可以根据实际需要使用 `mod.rs`。

### Visibility

声明应使用满足当前需求的最小 Visibility：

- `private`：仅当前 Module 使用。
- `pub(crate)`：仅 Crate 内共享。
- `pub`：明确需要跨 Crate 访问或属于对外 API。

不得仅为了实现或测试方便而扩大 Visibility，也不得在普通开发任务中顺手增加公共 API。

跨 Crate 可访问不代表必须通过顶层 Facade 导出。公共 API 的暴露范围应由实际设计需求决定。

## UI 构造与组合

项目统一使用 Bevy BSN 构造和组合 UI。

具体的 SceneComponent、Scene Function、Props、Runtime ECS 和外部 Children 规则，统一遵循 `rules/ui-composition.md`，不在架构规则中重复定义。
