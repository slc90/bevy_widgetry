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

## 资源管理

### Widgetry 库资源

库自身运行所需的 SVG、图片、字体及其他随库发布或嵌入程序的静态文件，统一存放于 `crates/asset`，由 `bevy_widgetry_asset` 管理文件、嵌入注册和语义资源标识。

其他库 crate（包括 `core` 和所有控件 crate）不得自行建立或维护运行时资源目录。控件专属资源可以在语义上属于控件，但物理文件与嵌入注册仍归资源 crate 管理。

`bevy_widgetry_asset` 只依赖 workspace 的 `bevy` 和底层 `bevy_widgetry_log`，不得依赖 `core` 或控件 crate，也不承担 SVG 等上层资源解析职责。`core` 可以依赖 `asset` 以提供 App 级默认字体等共享资源基础设施；顶层 facade 不直接依赖或 re-export 内建资源 API。

### 资源访问与注册

除负责资源管理的 Asset 模块外，库代码及测试不得书写内建资源的物理或虚拟路径，如 `assets/...`、`../assets/...` 或 `embedded://...`。

其他 crate 必须通过语义标识访问内建资源，如 `BuiltinIcon::WindowClose`。跨 crate 所需的标识及路径接口可以使用 `pub`，但仍属于 workspace 内部 API，不得由 facade 导出。

新增资源类型应按语义定义 `BuiltinFont`、`BuiltinImage` 等类型，不建立混杂全部资源种类的通用枚举。

使用内建资源的每个库插件负责检查并自动注册 `WidgetryAssetPlugin`，必须避免重复添加；不得要求用户手动配置内部资源插件。

### 应用资源

应用自有资源不得因使用 Widgetry 而下沉到库资源层。Gallery 的 Logo、展示图片和 Demo 专属图标等资源统一通过 `gallery/src/assets.rs` 与 `gallery/src/assets/` 管理，由应用入口显式装配 `GalleryAssetPlugin` 完成嵌入注册。

Gallery 其他代码必须通过自身语义标识访问应用资源，不直接使用 Window 的内建图标；Window 图标由 Window 控件自身使用。

应用资源只有在已明确成为 Widgetry 库功能的一部分时才允许迁入 `bevy_widgetry_asset`，不得因为未来可能复用而提前下沉。

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

## UI 构造与组合

项目中的 UI 实体结构统一使用 Bevy BSN 进行声明、构造和组合。

新增或重构 UI 时，应优先使用 `bsn!` 表达组件挂载、实体层级、children 和控件组合，不再以手写 `Commands::spawn` / `with_children` 实体树作为常规 UI 构造方式。

BSN 作为 Widgetry 及其消费者代码的统一 UI composition language，应直接暴露和使用，不得额外设计一套以 `spawn_*`、`build_*` 等 helper 为核心的 UI 构造 API 来隐藏 BSN。

需要复用的 UI 结构，应根据其语义通过以下方式抽取：

* 返回 `impl Scene` / `impl SceneList` 的 scene 函数；
* `SceneComponent`；
* BSN 自身的 scene composition 与 patch 机制。

不得仅为了避免直接书写 BSN，而将 UI 结构机械包装为 Rust helper。

BSN 是项目统一的 UI 构造与组合方式，与类型是否实现 `SceneComponent` 无关。普通 `Component` 同样可以作为 BSN Scene 中的组成部分。

### Scene 抽象选择

是否使用 `SceneComponent`，应根据该抽象是否需要作为 ECS 身份长期存在决定。

当一个 UI 抽象本身需要：

* 作为明确的 ECS 身份存在；
* 被 `Query`、`With<T>` 等 ECS 查询识别；
* 承载需要在运行时持续存在的控件状态或语义；

应将其建模为 `SceneComponent`。

当某段 BSN 仅用于抽取和复用 Scene 结构，本身不需要在 World 中留下独立 ECS 身份时，应使用返回 `impl Scene` 或 `impl SceneList` 的函数。

不得仅因为某段 Scene 可复用，或为了统一形式，就为其额外创建 `SceneComponent`。

`SceneComponent` 与 scene 函数的选择属于 ECS 建模问题，不属于“是否使用 BSN”的选择；两者都属于项目正常的 BSN composition 方式。

### Scene props

`SceneComponent` 的 prop 只能用于 Scene 构造阶段的一次性初始化。

允许使用 prop 初始化 `Component` 字段，包括与 prop 表达相同语义的字段。prop 的语义生命周期必须在 Scene 展开完成时结束，不得继续作为运行期状态来源。

对于 Scene 创建后仍然具有意义，并需要被查询、修改、监听或用于驱动后续行为的数据，必须由持久的 `Component` 保存和维护。

当某项数据既影响初始 UI 结构，又需要在运行时继续存在时，可以通过 prop 完成初始化；Scene 展开后，必须以对应的 `Component` 作为唯一状态来源，并通过 observer、system 或其他明确的运行时机制维护其对应的 UI 结构。

禁止在运行期保留同义的 prop 状态副本，或在 prop 与 `Component` 之间持续同步状态。一次性初始化不属于运行期双重状态。

### ECS 运行时操作

本规则只约束 UI Scene 的声明与构造。

运行时对已有实体进行状态修改、组件插入或移除、despawn 等正常 ECS 操作，仍可直接使用 `Commands`、`World` 等 Bevy ECS API。

本规则约束 Rust 代码中的 BSN 使用；除非另有专门设计，不要求使用外部 `.bsn` 资源文件。

### 内容与 children

复合 UI 控件的外部内容应继续使用 BSN Scene 进行组合。

当控件需要接收用户提供的子内容、内容区或可组合 UI 片段时，应优先使用 `Scene` / `SceneList` 表达，并通过 BSN 的 children 与 scene composition 机制组合到控件内部结构中。

不得为了传递纯 UI 内容而额外设计以字符串字段、builder callback、spawn callback 或其他专用构造接口为核心的替代机制。

只有当某项输入本身属于控件的明确语义数据，而不是单纯的 UI 内容时，才应建模为 prop 或 `Component` 数据。
