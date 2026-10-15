# UI 构造与组合规则

## Bevy BSN

项目统一使用 Bevy BSN 声明、构造和组合 UI，包括 Widgetry 库及其消费者代码。

新增或重构 UI 时，应使用 `bsn!` 表达 Component 挂载、Entity Hierarchy、Children 和 Widget 组合，不以手写 `Commands::spawn` / `with_children` 构建 Entity Tree 作为常规 UI 构造方式。

BSN 是项目统一的 UI Composition 方式，不得额外建立以 `spawn_*`、`build_*` 等 Helper 为核心的替代构造 API 来隐藏 BSN。

UI 结构复用应使用：

- 返回 `impl Scene` 或 `impl SceneList` 的 Scene Function。
- `SceneComponent`。
- BSN 自身的 Scene Composition 与 Patch 机制。

普通 Component 同样可以直接用于 BSN Scene，不要求所有 UI 构造都通过 `SceneComponent`。

`core` 提供的 Scene Command 错误适配只负责 Deferred 错误传播、Severity 和失败 Root 清理，不改变 BSN 的构造与组合模型。具体错误处理遵循 `rules/code.md`。

## Scene 抽象选择

`SceneComponent` 与 Scene Function 的选择取决于是否需要独立、持久的 ECS 身份，而不是 UI 结构是否可复用。

当 UI 抽象需要作为独立 ECS 身份存在、被 Query 识别，或承载持续存在的 Widget State 与业务语义时，应使用 `SceneComponent`。

当某段 UI 结构仅用于构造与复用，本身不需要留下独立 ECS 身份时，应使用返回 `impl Scene` 或 `impl SceneList` 的 Function。

不得仅为了复用 Scene 结构或统一代码形式，就额外创建 `SceneComponent`。

## Props 与 Runtime State

`SceneComponent` 的 Props 仅用于 Scene 构造阶段的一次性初始化。

Props 可以初始化 Component Field，包括与 Props 表达相同语义的字段，但 Props 的生命周期必须在 Scene 展开完成时结束。

Scene 创建后仍需查询、修改、监听或驱动运行时行为的数据，必须由持久的 Component 保存和维护。

当某项数据同时影响初始 UI 结构和后续运行时行为时，可以通过 Props 完成初始化；Scene 展开后，以对应 Component 作为唯一 State 来源，并通过 Observer、System 等运行时机制维护相关 UI。

不得保留同义的 Props State 副本，也不得在运行期间持续同步 Props 与 Component State。

## Runtime ECS 操作

BSN 约束 UI Scene 的声明、构造与组合，不禁止正常的运行时 ECS 操作。

运行时可以直接使用 `Commands`、`World` 等 Bevy ECS API 修改已有 Entity 的 Component、执行 Insert / Remove、Despawn 及其他必要的结构操作。

Widgetry 自有 Runtime State 的公开更新仍须遵守 `rules/widget-api.md`，不得以直接 ECS 操作为由绕过其公开更新契约。

库内部维护、Bevy 官方 Component 以及其他正常 ECS 结构操作遵循各自契约。

## 外部内容与 Children

复合 Widget 的外部 UI 内容应通过 BSN Scene 进行组合。

当 Widget 需要接收调用方提供的 Children、内容区域或其他可组合 UI 片段时，应使用 `Scene` / `SceneList`，通过 BSN 的 Children 和 Scene Composition 机制嵌入 Widget 内部结构。

不得为了传递纯 UI 内容而额外设计 String Field、Builder Callback、Spawn Callback 或专用构造接口来替代 BSN Composition。

只有当输入本身属于 Widget 的明确语义数据，而不是单纯的 UI 内容时，才应建模为 Props 或 Component 数据。
