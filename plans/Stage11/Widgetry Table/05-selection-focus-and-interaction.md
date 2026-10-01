# Selection、Focus 与 Table Interaction

## 目标

将真实 Header / Cell pointer 输入与基础 keyboard navigation 映射到 logical selection / FocusedCell，并落实整体 disabled 与 interaction events。

## 范围

负责单一 Row / Column / Cell selection、四方向 FocusedCell movement、disabled 输入 guard、对应 selection events，以及已明确 contract 的 Column resize 输入与 events。不增加编辑、多选、Range、Enter action、Corner 全选或原方案排除的其他操作。

## 预期产出

形成 Table-specific interaction 与真实 ECS 输入路径测试，覆盖 transition、重复操作、disabled / focus guard、虚拟 Cell 的 ID pair、Model identity 失效，以及明确后的 resize state machine。

## 与前后方案的关系

承接方案 04 的当前可见 Cell projection，使用方案 03 的 Header / Cell 结构与 width / style contract。下一方案以 Gallery 验收实际用户路径；本方案同时落实此前保留的 focus / selection 与 virtualization 的跨领域 case。

## 整体设计目标与非范围

`WidgetryTable` 是一个通用二维数据展示控件。

设计目标：

- data driven
- headless
- 支持异构 Cell renderer
- 支持固定 Header
- 支持虚拟化
- 支持 Row / Column / Cell selection
- 与 `WidgetryListView`、`WidgetryTreeView` 保持架构一致

不包含：

- 排序
- Cell 编辑
- Copy
- Context menu
- Double click
- Shift/Ctrl 多选
- Corner 全选

---

## 整体架构

### 外部 API

外部只看到：

```rust
WidgetryTable<T>
```

不会接触 Axis。

整体结构：

```
WidgetryTable<T>

        |
        v

WidgetryTableModel<T>

        |
        +----------------+
        |                |
        v                v

    Row 数据        Column 定义

        \              /

         \            /

          v          v

        Cell Data Source
```

---

## Axis 设计

Axis 是内部抽象。

Table 内部使用：

```
Row Axis
    |
    +-- RowId
    +-- Row data


Column Axis
    |
    +-- ColumnId
    +-- Column metadata
```

Cell：

```
(RowId, ColumnId)
        |
        v
    CellValue
```

即：

```
Cell = Row Axis × Column Axis
```

Cell 不拥有独立 identity。

---

## WidgetryTable View

保持 BSN 风格。

示意：

```rust
#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTableProps<T>)]
pub struct WidgetryTable<T>
{
    source: Entity,
}
```

外部：

```rust
@WidgetryTable<User>
{
    @source: users
}
```

Table 内部负责：

- 创建 viewport
- 创建 Header
- 创建 Cell entity
- 管理 virtualization

---

## Virtualization

采用二维 virtualization。

可见区域：

```
visible rows
      ×
visible columns
```

只生成：

```
(row_id, column_id)
```

对应 Cell entity。

采用 Cell entity 方案：

```
Table

    Cell(row,column)

    Cell(row,column)

    Cell(row,column)
```

不采用：

```
Table
 └ Row
      └ Cell
```

避免重新引入 Row-centric 层级。

---

## Selection

第一版：

```rust
enum TableSelection
{
    None,

    Row(RowId),

    Column(ColumnId),

    Cell {
        row: RowId,
        column: ColumnId,
    },
}
```

---

行为：

### Column Header 点击

结果：

```
Selection::Column
```

---

### Row Header 点击

结果：

```
Selection::Row
```

---

### Cell 点击

结果：

```
Selection::Cell
```

---

不支持：

- 多选
- Shift
- Ctrl
- Range

---

## Focus

第一版支持基础 keyboard navigation。

状态：

```
FocusedCell
```

支持：

```
Up
Down
Left
Right
```

移动：

```
Row Axis movement
+
Column Axis movement
```

不支持：

- 编辑
- Enter action

---

## Style

Style 分区域：

```rust
WidgetryTableStyle
{
    table: TableStyle,

    column_header: ColumnHeaderStyle,

    row_header: RowHeaderStyle,

    corner: CornerStyle,

    cell: CellStyle,
}
```

---

### Cell Style

Cell Style 管：

- container
- background
- padding
- border

不管：

- 内容

结构：

```
Cell Node

    Style

    Content Widget
```

Renderer 只负责 Content。

---

### Disabled

只支持整体 Table disable。

不支持：

- Column disable
- Row disable
- Cell disable

行为：

```
WidgetryTable disabled

    |
    +-- disable picking
    +-- disable hover
    +-- disable selection input
    +-- disabled style
```

---

## Interaction Events

第一版：

```rust
enum WidgetryTableEvent
{
    ColumnSelected(ColumnId),

    RowSelected(RowId),

    CellSelected {
        row: RowId,
        column: ColumnId,
    },

    ColumnResizeStart(ColumnId),

    ColumnResized {
        column: ColumnId,
        width: f32,
    },

    ColumnResizeEnd(ColumnId),
}
```

---

## 测试设计依据

本节依据当前 rules/testing.md 补充测试意图、关键行为边界和重要 invariant。测试围绕 WidgetryTable 自身的 contract，不按源码 function 数量补测试，也不穷举所有 state 的笛卡尔积。

原方案中的 API 和 struct 是示意。以下场景不提前规定尚未定义的 method、错误类型、state 修复策略或 interaction coupling；依赖这些语义的场景在明确前提后进入对应的 Type → Red → Green → Refactor 循环。

---

## 17.1 测试层次与 Coverage Model

### Unit test

具有独立局部 contract 的逻辑放在对应源码 module 的 cfg(test) tests 中：

- Model 的 stable identity、revision 和独立数据操作语义。
- Cell Data Source 中可以独立验证的查询或投影逻辑。
- renderer registry 中具有独立语义的类型查找和隔离规则。
- 实现中实际形成独立 contract 的二维可见范围或 layout 数值算法。

不要求每个 private function 都有测试，不为了测试把 private implementation 改成 pub，也不重复测试 Bevy TypeRegistry、ECS 或 layout engine 已经保证的内部行为。

### Integration test

从 crate 外部视角验证公共行为、跨 module 协作以及真实 Bevy / ECS 组合的测试放在 crates/table/tests/。

复杂测试模块开头使用 //! module-level documentation 记录自身的 Coverage Model：

| state 维度 | 可观察内容 |
| --- | --- |
| Model | Row / Column 集合、stable identity、当前数据；分别考虑空与非空 Axis |
| Renderer | CellValue / HeaderValue 的实际类型与已注册 renderer；缺失注册行为仍需定义 |
| View / viewport | 尚未创建、已创建、可见范围变化、销毁；四个区域的实际 layout |
| Selection | None / Row / Column / Cell |
| Focus | 是否取得实际 input focus，以及 FocusedCell 的位置；二者的关系仍需定义 |
| Enabled | Table enabled / disabled |
| Style | 各区域 style；Cell shell 与 Content 的职责边界 |

每个模块继续说明相关 stimuli、guards、invariants 与 couplings。重要 stimuli 包括首次 update、source / Model 变化、renderer 注册环境、pointer、keyboard、scroll、viewport / layout 变化、style / disabled Component 变化，以及 spawn / despawn。

只覆盖已定义的业务 coupling，例如 viewport × 两个 Axis、Cell renderer × CellValue、Header renderer × HeaderValue、selection × pointer target、disabled × selection input。Focus × selection、Model mutation × selection / FocusedCell 等尚未定义的关系不得根据经验补出行为。

### Coverage Map

如果 integration test 拆成多个文件，应由主要测试模块维护轻量 Coverage Map。以下按行为领域分配主要 owner，不强制文件数量和文件名：

| 行为领域 owner | 负责的 stimuli 与 contract | 重要跨领域检查 |
| --- | --- | --- |
| Model / 数据投影 | 公共数据操作、Cell Data Source 查询、source 数据变化 | RowId × ColumnId 的对应关系、stable identity 与实际 Content 一致 |
| Renderer | CellValue / HeaderValue、类型注册、Content 生成 | 两个 registry 隔离，Content 不接管 Cell shell style |
| View / layout / style | BSN spawn、首次 update、layout / style / disabled 变化、despawn | 四区滚动对齐，style 与 Content ownership，完整生命周期 |
| Virtualization | scroll、viewport 大小变化、可见与不可见数据更新 | physical Cell 是 logical 数据的 projection，selection / FocusedCell 不依赖可见 entity identity |
| Interaction | pointer、keyboard、focus、用户选择 events；resize contract 明确后的 drag | 命中目标与通知一致，disabled guard，虚拟 Cell 的输入不使用旧 identity |

每一种重要 stimulus 必须有主要 owner。跨领域 case 可以放在最直接负责完整 contract 的模块，不把同一个流程拆成多个文件后遗漏整体结果。

### 公共 invariant

- Row 与 Column 是两个内部 Axis；外部公共使用路径不需要操作 Axis。
- Cell identity 是 RowId 与 ColumnId 的组合，不引入独立 Cell identity。
- Cell Data Source 通过两个 ID 查询，不以 Row entity 层级代替二维投影。
- Column Model 不承担 width、selection 或 layout state。
- Cell 与 Header 使用不同 registry；Cell shell style 与 renderer Content 保持职责边界。
- 可见 physical Cell 对应当前 logical ID pair，不能残留上一次可见区域的数据或命中身份。
- 用户 Header / Cell 点击产生与对应 selection 一致的事件。
- disabled 时不接受 selection input，不产生 enabled hover 表现，并使用 disabled style。

上述 invariant 应在每一个可能破坏它的 transition 之后检查，可以使用针对性的共享 assertion helper，不要求所有测试调用一个巨大的 assert_everything()。

### 测试方法

- 通过真实公共入口创建 Model、Table 和 renderer，并运行正常的 ECS schedule，不直接调用 private system 代替 integration contract。
- 输入 fixture 安装真实行为需要的 Bevy / Widgetry plugins、input dispatch 和调用方前置条件；不能手动写入 selection 或 focus，再把结果当成 pointer / keyboard 输入已通过。
- 多 crate 或多 integration test 共用的基础设施放入 test_utils，不复制同类 App、输入或生命周期 helper。
- 每个 test function 的 test attribute 之前用中文注释说明场景和验证目标，技术术语保留英文。
- 明确 contract 使用 deterministic test。二维范围计算、数值边界和合法操作序列适合按需考虑 proptest；逐步检查相应 invariant，不能替代明确 regression test，也不预先增加 dependency。
- 如果最终 API 定义了错误 contract，验证错误内容、Severity::Error、Widgetry 日志和失败后的业务 state；system、observer、command 通过捕获错误的宿主 handler 验证真实 ECS 传播，不使用 catch_unwind 或 should_panic 代表错误传播成功。
- 库行为按小步 Type → Red → Green → Refactor 推进；本节是覆盖设计，不要求先一次写完全部测试。
- 本次补充文档不执行下列测试，也不代表 Table 已实现。

---

## 17.6 Selection、Focus、Disabled 与 Events 用例

主要 owner：Interaction。先覆盖每个 state 维度的 transition，再覆盖已定义的 coupling。fixture 必须从实际 pointer / keyboard dispatch 路径进入，不直接修改内部 state 冒充用户输入。

| 场景 | stimulus / guard | 验证结果 |
| --- | --- | --- |
| Column Header 选择 | enabled Table 中点击不同 Column Header | selection 指向所点 ColumnId，发出对应 ColumnSelected；之后点击另一 Column 不保留隐含多选 |
| Row Header 选择 | enabled Table 中点击不同 Row Header | selection 指向所点 RowId，发出对应 RowSelected |
| Cell 选择 | enabled Table 中点击不同 Cell | selection 为正确 row / column pair，发出对应 CellSelected；Content 子 entity 命中也按实际 picking contract 路由到所属 Cell |
| selection 类型切换 | 真实输入按 Row → Column → Cell → Row 切换 | 每一步只保留当前 TableSelection variant，事件类型与实际点击目标一致；None 的初始和清除入口仍需定义 |
| 重复选择同一目标 | 连续点击同一 Header / Cell | state 不形成重复选择项；是否重复通知按先明确的 event contract 验证，不能提前认定只通知一次 |
| 虚拟 Cell 再次命中 | 先滚动改变可见 pair，再点击当前 Cell | selection 与事件使用当前 logical ID pair，不使用复用前的 RowId / ColumnId |
| 四方向 navigation | 通过实际输入取得 focus，在至少有相邻 Row / Column 的位置发送 Up / Down / Left / Right | FocusedCell 沿对应 Axis 移到相邻位置；测试不能只覆盖其中一个方向或只检查 keyboard message 已入队 |
| navigation 边界 | 在第一 / 最后一 Row 或 Column 发送对应方向键；另测空 Axis | 按先明确的 boundary contract 验证 no-op、clamp 或其他结果；不发明 wrap 行为，不产生不存在的 ID pair |
| focus guard | 比较已取得和未取得 Table input focus 时的 keyboard 输入 | 验证真实 focus dispatch 与 Table 的 guard；focus 获取、释放、初始 FocusedCell 和 FocusedCell 与 input focus 的关系需先定义 |
| 禁用与恢复 | enabled → disabled 后执行 pointer movement、Header / Cell click，再恢复 enabled 执行相同输入 | disabled 不产生 hover 表现、不接受 selection input、使用 disabled style，Table interaction target 的 picking 符合禁用 contract；恢复后真实输入重新有效 |
| disabled 与 keyboard / 既有 state | 先建立 focus 和 selection，再 disable 并发送方向键 | disabled 是否阻止 navigation、是否保留 selection / FocusedCell 需先明确；selection input 的禁用规则不能仅测 pointer 分支后就声称所有 guard 已覆盖 |
| Model identity 失效 | 真实输入选择 / focus 后删除或替换相关 Row / Column | 按先确定的修复策略验证 selection、FocusedCell 与事件；不能让旧 ID 错指同 index 的新数据，也不能擅自决定选中邻项 |
| 程序化 state 修改 | 如果最终 API 提供 programmatic selection / focus 修改，再执行 valid、same、missing / stale ID 操作 | 验证其独立 contract 以及是否产生用户 events；原方案没有规定静默或通知策略，不能把示例当成已确定要求 |
| Column resize events | 通过真实 drag 开始、移动、结束；补充 resize 期间 disable / despawn 等中断前提 | 按明确后的 resize state machine 验证 Start / Resized / End 的 ID、width、时序、重复或中断语义；当前仅有 event enum，不能只测试直接发送 event |
| 非范围操作 | 在正常 selection / focus 场景加入 Shift / Ctrl，或点击 Corner、发送 Enter | 不出现多选、Range、Corner 全选或 Enter action；modifier 下究竟保持原 state 还是按普通输入处理仍需明确，不新增编辑、排序、Copy、Context menu、Double click 行为 |

必要前提：selection 初始 / 清除与无效 ID 策略、重复选择通知、keyboard 对 selection 的影响、focus 路径与边界、disabled 的 keyboard guard、programmatic 通知语义、resize state machine 仍未定义。每项必须在所影响的 behavior 实施前明确，再补齐精确断言。

---

## 本方案的组合验证边界

本方案承担真实 pointer / keyboard / focus / drag 流程的完整 integration contract，联合验证前面方案的 layout 和 virtualization 结果。BRP 真实运行验收由方案 06 承接，不能用它替代长期自动化测试。

## 验证边界与未决前提

实施阶段必须分别记录：已通过的 unit / integration test、Gallery 编译、实际 BRP 用户场景和未完成项目；文档中的 case、Cargo 测试成功或截图都不能互相替代证据。

以下是原方案尚未给出、但测试必须依赖的 contract。这里只标明缺口，不补出新设计：

- View / layout：Row Header 内容与 renderer、Corner 内容、source 失效 / 重绑定、entity ownership、width state owner、fixed / flexible 分配、resize 边界。
- Virtualization：Cell 几何、可见边界 / overscan、scroll clamp、回收 / 复用、Header virtualization、数据变化后的 viewport 修复。
- Interaction：selection 初始 / 清除 / 修复、重复选择 events、Focus 获得 / 丢失、方向键边界、Focus × selection、disabled × keyboard、programmatic events、resize 开始 / 移动 / 结束 / 中断。

这些缺口应随受影响的小方案保留，在对应 type 与行为实施前确定。不得为了让 checklist 看起来完整而为缺口写出臆测的 expected result，也不得把未决 case 标记为已验证。

---
