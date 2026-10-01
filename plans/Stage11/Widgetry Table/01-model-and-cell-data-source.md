# Model 与二维 Cell 数据投影

## 目标

建立 Row 与 Column 两个内部 Axis 的 Model，以及通过 RowId × ColumnId 查询异构 CellValue 的数据 contract。

## 范围

负责 Table 的数据表达、stable identity、Row revision、Column HeaderValue / schema 和 Cell Data Source；承接 crate 结构与 module 职责。width、selection、layout 不进入 Column Model，本阶段不生成 View 或实现输入。

## 预期产出

形成 WidgetryTableModel 与二维查询的 type / API 设计及实现边界，并具备已明确的数据 contract 的 unit / integration test。mutation、revision、失效查询等缺口必须明确后才能作为已完成行为验收。

## 与前后方案的关系

这是执行链入口。下一方案使用本方案给出的 CellValue 与 HeaderValue 作为 renderer 输入；本方案不反向依赖后续 View、virtualization 或 interaction 的实现。

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

## Crate 结构

新增：

```
crates/
└── table/
    └── src/
        ├── lib.rs
        ├── model.rs
        ├── view.rs
        ├── behavior.rs
        ├── style.rs
        ├── registration.rs
        └── virtualization.rs
```

职责：

| 文件              | 职责                         |
| ----------------- | ---------------------------- |
| model.rs          | Table 数据模型               |
| view.rs           | WidgetryTable SceneComponent |
| behavior.rs       | 输入、selection、focus       |
| style.rs          | Table 样式                   |
| registration.rs   | renderer registry            |
| virtualization.rs | 二维 viewport 生命周期       |
| lib.rs            | plugin/export                |

---

## WidgetryTableModel

参考 `WidgetryListModel`。

Model 是 ECS Component。

示意：

```rust
#[derive(Component)]
pub struct WidgetryTableModel<T>
{
    rows: Vec<TableRowEntry<T>>,
    columns: Vec<TableColumn>,
}
```

---

### Row

Row 保存：

- stable identity
- revision
- value

类似 ListModel：

```rust
struct TableRowEntry<T>
{
    id: WidgetryTableRowId,
    revision: u64,
    value: T,
}
```

---

### Column

Column 属于 Model。

负责：

- ColumnId
- HeaderValue
- column schema

不负责：

- width
- selection
- layout

示意：

```rust
struct TableColumn
{
    id: WidgetryTableColumnId,

    header: WidgetryTableHeaderValue,
}
```

---

## Cell Data Source

Table 不由 Row 生成 Cell。

Cell 由：

```
RowId
+
ColumnId
```

查询。

接口概念：

```
(row_id, column_id)
        |
        v
   CellValue
```

例如：

```
Row:
    User

Column:
    Age

Result:
    CellValue<u32>
```

---

## 总结

`WidgetryTable`

```
= 两个内部 Axis
+ Cell Data Projection
+ Type Driven Renderer
+ 2D Virtualization
+ Table-specific Interaction
```

关系：

```
ListView
    =
    Linear Axis


TreeView
    =
    Hierarchy Axis


Table
    =
    Row Axis × Column Axis
```

外部保持：

```
WidgetryXXX
+
BSN
+
SceneComponent
```

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

## 17.2 Model 与 Cell Data Source 用例

主要 owner：Model / 数据投影。局部数据 contract 使用 unit test，公共 Component 与 View 的真实协作使用 integration test。

| 场景 | stimulus / 前提 | 验证结果 |
| --- | --- | --- |
| 两个 Axis 独立保存 | 创建多个 Row、多个 Column，使用不同 Row value 与 HeaderValue | 两个 Axis 分别持有自身 ID；Column 中只保存 ID、HeaderValue / schema，不以 Column Model 承载 width、selection 或 layout |
| 二维查询不按 Row 展开 | 对同一个 RowId 查询不同 ColumnId，再对同一个 ColumnId 查询不同 RowId | 返回值来自正确 ID pair；例如 User × Age 返回对应 u32，不能把 Row value 本身固定成统一 CellValue |
| identity 与顺序分离 | 在最终 Model API 支持的 reorder / move 路径中改变位置 | 仍保留的 Row / Column identity 不因 index 变化而替换；Cell 查询仍按 identity 对应原逻辑项。Column mutation API 与 identity lifetime 需先明确 |
| Row value 与 revision | 通过最终公开的数据修改入口更新 Row value | 查询反映新值；revision 是否递增、递增粒度和 no-op 是否改变 revision 按先确定的 contract 断言，不提前规定数值规则 |
| 空 Axis | 分别构造空 Row、空 Column、两个 Axis 都空 | View 不生成不存在的 Body Cell；空数据的 Header / Corner 呈现和查询结果按明确后的空态 contract 验证 |
| 无效或 stale ID | 查询 missing RowId、missing ColumnId，以及删除后保留的旧 ID | 按先确定的查询失败 contract 验证返回结果与必要错误传播；不能误命中同 index 的其他数据 |
| 数据操作序列 | 对最终支持的 insert / move / update / remove / clear 等操作执行有意义序列 | 每一步验证 stable identity、有效查询对应关系与 revision contract；未定义的操作不为测试擅自添加 API |
| 外部 Model 更新 | Table 已创建后通过公共入口改变可见 Cell 对应的数据 | 正常 update 后 Content 对应当前 Model；如果更新不可见数据，之后进入 viewport 时也显示最新值，不只检查 Model 内部已改变 |

必要前提：Column schema、CellValue 的具体表达、Model mutation API、revision / no-op 规则、失效 ID 查询结果仍未定义。相关 case 保留测试意图，待语义确定后给出精确 expected result。

---

## 本方案的组合验证边界

Model 更新后实际 Content 的 integration case 需要方案 03 的公共 View；涉及不可见数据再次进入 viewport 的 case 需要方案 04。此处保留完整数据 stimulus 的测试意图，由能力到位后的主要 owner 验证完整流程。

## 验证边界与未决前提

实施阶段必须分别记录：已通过的 unit / integration test、Gallery 编译、实际 BRP 用户场景和未完成项目；文档中的 case、Cargo 测试成功或截图都不能互相替代证据。

以下是原方案尚未给出、但测试必须依赖的 contract。这里只标明缺口，不补出新设计：

- Model：Column schema、mutation API、Row revision 规则、ID lifetime / stale ID 查询语义。

这些缺口应随受影响的小方案保留，在对应 type 与行为实施前确定。不得为了让 checklist 看起来完整而为缺口写出臆测的 expected result，也不得把未决 case 标记为已验证。

---
