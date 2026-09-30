# ListView 拆分方案总览

## 整体目标

在 Bevy 0.19.1 / Widgetry 现有 architecture 内新增一个 data-driven、ECS-native、固定行高虚拟化的 `WidgetryListView<T>`。调用方继续使用 BSN；业务数据由独立的 `WidgetryListModel<T>` 持有，ListView 只引用 source entity，并只实例化当前 viewport 实际可见的 row。

第一版明确只支持纵向、固定 `item_height`、单选；不做 multi-select、动态行高、overscan、横向 ListView 或 UI drag reorder。程序化 reorder 通过 model 的 `move_item()` 提供。

## 唯一执行顺序

1. [建立 ListModel 与 public type contract](01-list-model-and-public-types.md)  
   先固定 data identity、revision、CRUD、renderer、logical state、generic registration 等类型边界，并完成新 crate 的最小 architecture 接入。
2. [扩展 ScrollArea contract 并建立 BSN ListView 外壳](02-scroll-area-contract-and-bsn-shell.md)  
   让 ListView 复用现有 ScrollArea，同时解决 content marker、keyboard ownership、focus root 和 generic BSN 构造问题。
3. [实现固定行高 virtualization 与 renderer lifecycle](03-virtualization-and-renderer-lifecycle.md)  
   建立 visible range、spacer、range overlap reconciliation、精确 rerender、scroll clamp 与 bootstrap/scheduling contract。
4. [建立 selection、active、focus 与 disabled 行为](04-selection-active-focus-disabled.md)  
   在 row entity 会动态销毁的前提下完成 stable logical state、pointer/keyboard/programmatic selection、disabled 与 accessibility。
5. [完成 ListView 与 ListItem style/theme](05-list-view-item-style-and-theme.md)  
   统一 root/row 的 border、圆角、interaction priority、foreground、active border 与 ThemeChanged 投影。
6. [完成 Gallery 与 BRP 验证闭环](06-gallery-and-brp-validation.md)  
   用 Small/Large/Disabled root/Disabled item 四类场景验证 UI、programmatic、style 与 Theme，并完成最终自动化测试矩阵、workspace 验证和 code review。

## 跨方案约束

- 所有新增 public Widgetry type 使用 `Widgetry` 前缀；不顺手重命名现有 public API。
- ListView 自己的 headless/style/state 留在 `crates/list_view`；只复用现有 core 与 ScrollArea 能力，不把专属逻辑下沉到 core。
- UI hierarchy 使用 BSN；Scene prop 只用于构造期，runtime state 必须落到 Component。
- 第一版 `rendered range == visible range`，不提供 overscan API；是否需要 overscan 由 Gallery/BRP 真实结果决定。
- virtualization 下 row subtree 是 ephemeral UI；必须长期存在的业务 state 不能依赖 row entity identity。
- 自动化测试按现有 Type-Driven Development + TDD 规则分布到对应方案，不留到最后统一补。
- `gallery/` 自身不新增自动化测试，按现有 rule 通过编译 + BRP 做 runtime 验证。
- `rules/testing.md` 的后续改进是独立任务，不夹带进本次 ListView 实现。
