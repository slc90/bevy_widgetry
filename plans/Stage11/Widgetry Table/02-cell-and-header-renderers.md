# 异构 Cell 与 Header Renderer

## 目标

建立两个语义独立的 type-driven registry，把 CellValue 和 HeaderValue 投影为各自的 Content Widget。

## 范围

负责 Cell / Header 的 renderer 注册、派发和 Content 职责边界，支持原方案列出的异构内容与非 String Header。Table container、padding、border、viewport 与用户输入不由 renderer 接管。

## 预期产出

形成独立的 CellRendererRegistry 与 HeaderRendererRegistry 及对应公共注册入口，具备类型派发、registry 隔离和已明确失败策略的测试。

## 与前后方案的关系

承接方案 01 的 Model、CellValue 和 HeaderValue。下一方案通过 renderer 生成 View 内的 Content；两个 registry 的独立语义是后续 Header / Cell 显示的前提。

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

## Cell Renderer

采用 TypeRegistry。

流程：

```
CellValue
    |
    v
Cell Renderer Registry
    |
    v
Content Widget
```

例如注册：

```rust
register_table_cell_renderer::<String>()

register_table_cell_renderer::<u32>()

register_table_cell_renderer::<Icon>()
```

支持：

- Text
- Number
- Bool
- Progress
- Icon

等不同类型。

---

## Header Renderer

Header 同样采用 type-driven。

流程：

```
HeaderValue
    |
    v
Header Renderer Registry
    |
    v
Header Widget
```

Cell 和 Header 使用不同 registry：

```
CellValue
    |
CellRendererRegistry


HeaderValue
    |
HeaderRendererRegistry
```

原因：

两者语义不同。

Header 不固定为 String，可以支持：

- 普通文字
- 图标
- 组合内容
- 自定义展示

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

## 17.3 Cell / Header Renderer 用例

主要 owner：Renderer。使用可识别输出的测试 renderer 验证 Widgetry 自身的类型派发与 Content 生成，不重测 TypeRegistry 的底层实现。

| 场景 | stimulus / 前提 | 验证结果 |
| --- | --- | --- |
| 异构 Cell | 同一 Table 查询得到 String、u32、Bool、Progress、Icon 等受支持 value，注册对应 renderer | 每个 Cell 使用其实际类型的 renderer；Content 显示正确值，不受其他 Cell 类型影响 |
| Cell 与 Header registry 隔离 | 同一 value type 在两个 registry 中注册不同 Content 输出 | Cell 只调用 Cell renderer，Header 只调用 Header renderer；注册一方不等于另一方已经注册 |
| 非 String Header | 为图标、组合内容或自定义 HeaderValue 注册 renderer | Header 按 type 生成 Content，不强制转换成 String |
| 多 Cell 共用类型 | 多个 ID pair 使用同一种类型但不同 value | 每个 Content 使用自己的 value，不串用前一个 Cell 的数据或 entity |
| renderer 与 shell 分工 | renderer 生成有自身外观的 Content，Table 为 Cell 设置 background、padding、border | renderer 负责 Content；Cell container 与 shell style 由 Table 维护，二者不互相覆盖 |
| value 更新或类型变化 | 使用最终支持的数据更新路径，更新相同类型值；若允许类型变化，再切换类型 | Content 对应最新 value / type，旧 Content 不残留；如果类型变化不受支持，不为此扩展 API |
| 缺失或重复注册 | 只注册 Cell renderer 后生成同类型 Header；或按最终 API 重复注册同类型 renderer | 按先明确的 fallback / error / replacement contract 断言，不自行指定占位内容或覆盖策略 |

必要前提：注册 API、重复注册和缺失 renderer 的策略、Content ownership / replacement contract、运行期 value type 能否变化仍需确定。相关失败场景遵守 17.1 的错误测试规则。

---

## 本方案的组合验证边界

完整 Table 中的异构显示与 Cell shell style coupling 在方案 03 提供公共 View 后验证；运行期类型变化仅在最终数据 contract 支持时纳入，不因此扩展 API。

## 验证边界与未决前提

实施阶段必须分别记录：已通过的 unit / integration test、Gallery 编译、实际 BRP 用户场景和未完成项目；文档中的 case、Cargo 测试成功或截图都不能互相替代证据。

以下是原方案尚未给出、但测试必须依赖的 contract。这里只标明缺口，不补出新设计：

- Model：Column schema、mutation API、Row revision 规则、ID lifetime / stale ID 查询语义。
- Renderer：CellValue / HeaderValue 表达、注册 API、缺失与重复注册、Content 更新 / ownership。

这些缺口应随受影响的小方案保留，在对应 type 与行为实施前确定。不得为了让 checklist 看起来完整而为缺口写出臆测的 expected result，也不得把未决 case 标记为已验证。

---
