# 二维 Viewport 与 Cell 生命周期

## 目标

仅为 visible rows × visible columns 生成当前 Cell projection，并在两轴 scroll、viewport 与数据变化中维护正确 Content 和生命周期。

## 范围

负责二维可见范围、Cell entity 生成 / 回收或复用、ID pair 与 Content 的同步，以及对应生命周期。保持 Cell entity 方案，不引入 Table → Row → Cell 层级，不自行设计 Header virtualization 策略或 selection 修复规则。

## 预期产出

形成二维 virtualization 与明确后的数值边界、projection / 生命周期测试；包括两轴连续 scroll、可见和不可见数据更新、viewport 扩缩、空 Axis 和销毁。

## 与前后方案的关系

依赖方案 03 的 viewport / layout，以及方案 01、02 的数据与 renderer。下一方案把用户输入绑定到当前 logical identity，覆盖虚拟 Cell 与 selection / FocusedCell 的真实组合。

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

## Layout

Table 分为四个区域：

```
+-------------+----------------+
|   Corner    | Column Header  |
+-------------+----------------+
| Row Header  | Body           |
|             |                |
+-------------+----------------+
```

---

### Header 滚动行为

Column Header：

- 不跟随垂直滚动
- 跟随水平滚动

Row Header：

- 不跟随水平滚动
- 跟随垂直滚动

Body：

- 水平 + 垂直滚动

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

## 17.5 二维 Virtualization 用例

主要 owner：Virtualization。覆盖 viewport、数据 projection 和 Cell entity 生命周期，不以长列表只验证垂直 virtualization。

| 场景 | stimulus / 前提 | 验证结果 |
| --- | --- | --- |
| 二维可见区域 | 数据量在两个 Axis 上都超过 viewport，形成部分可见 Row 与 Column | 实际生成的 Cell ID pair 对应 visible rows × visible columns；不生成完整二维数据集，不用 Row entity 包裹 Cell |
| 单轴移动 | 分别水平、垂直滚动穿过可见范围边界 | 进入区域的 Cell 显示正确数据，离开区域的 Cell 按生命周期 contract 移除或复用；不能把原 Column / Row 的 Content 留给新 ID pair |
| 两轴连续移动 | 同时改变两轴位置，并快速往返可见区域 | 最终与中间已提交的 projection 不出现重复 pair、旧命中 identity 或旧 Content；不预设复用还是重建策略 |
| 可见边界 | viewport 恰好落在 Cell / Row / Column 边界及少量部分可见位置 | 根据先确定的可见范围规则验证边界；零尺寸、超出范围、overscan 是否存在及数值输入合法性需先定义 |
| viewport 扩缩 | 保持非零 scroll 后改变 viewport 大小 | 新的可见 pair 集合与 layout 一致；无需改动 Model identity，已有 Content 不因重复 update 无限增殖 |
| 可见 / 不可见数据更新 | 更新可见数据，再更新不可见数据并滚入其所在区域 | 可见 Content 正确刷新；不可见数据进入 viewport 时使用最新值，不使用旧 revision 的内容 |
| selection / FocusedCell 离开 viewport | 先通过真实输入建立 logical state，再滚出并滚回 | state 仍引用 logical identity，不转成当前可见 index 或 physical entity；是否保留、清除或自动 scroll 由先确定的 contract 决定 |
| 空 Axis 与销毁 | 将最终 API 允许清空的 Axis 清空，或销毁已滚动的 Table | 不残留已失效数据的 Cell / Content；销毁清理完整 owned subtree，不破坏外部 source |

可见范围算法若形成独立局部 contract，应补充 deterministic 边界 unit test；适合数值输入或合法 scroll / viewport 操作序列的 invariant 可以按需使用 proptest。Property test 不能代替具体二维 projection 与生命周期 integration case。

必要前提：Cell 尺寸、可见范围边界、overscan、entity 复用与回收、scroll clamp、数据删除后的 viewport / logical state 修复策略仍需明确。Header 的 virtualization 策略也未定义，不擅自要求其与 Body 共用回收算法。

---

## 本方案的组合验证边界

selection / FocusedCell 离开 viewport 的完整输入流程随方案 05 的公共 interaction 入口到位后落实。该 case 的数据 projection 与 lifecycle 仍由 Virtualization owner 负责，不通过写入 private state 提前冒充真实输入，也不为本阶段实现后续 selection 行为。

## 验证边界与未决前提

实施阶段必须分别记录：已通过的 unit / integration test、Gallery 编译、实际 BRP 用户场景和未完成项目；文档中的 case、Cargo 测试成功或截图都不能互相替代证据。

以下是原方案尚未给出、但测试必须依赖的 contract。这里只标明缺口，不补出新设计：

- View / layout：Row Header 内容与 renderer、Corner 内容、source 失效 / 重绑定、entity ownership、width state owner、fixed / flexible 分配、resize 边界。
- Virtualization：Cell 几何、可见边界 / overscan、scroll clamp、回收 / 复用、Header virtualization、数据变化后的 viewport 修复。
- Interaction：selection 初始 / 清除 / 修复、重复选择 events、Focus 获得 / 丢失、方向键边界、Focus × selection、disabled × keyboard、programmatic events、resize 开始 / 移动 / 结束 / 中断。

这些缺口应随受影响的小方案保留，在对应 type 与行为实施前确定。不得为了让 checklist 看起来完整而为缺口写出臆测的 expected result，也不得把未决 case 标记为已验证。

---
