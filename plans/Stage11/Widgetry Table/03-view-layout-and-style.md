# BSN View、四区 Layout 与 Style

## 目标

建立通过 BSN / SceneComponent 使用的 Table View，以四区 Layout 协同 Body 与 Header 的滚动，并保持区域 style 与 Content 的边界。

## 范围

负责 source 接入、viewport / Header / Cell 的 View 结构、四区 layout、Column width 的展示 contract 和 table / header / corner / cell style。disabled style 与 picking 结构保留其约束，完整 selection / keyboard guard 由方案 05 落实；本阶段不提前实现二维回收或交互 state machine。

## 预期产出

形成可由公共 BSN 入口创建的 View、四区滚动 layout 与独立区域 style，覆盖首次 update、source 隔离、layout / style 变化和明确 ownership 后的生命周期测试。Column width、Row Header 与 Corner 等缺口须作为 layout 前提明确。

## 与前后方案的关系

使用方案 01 的 source / Model 和方案 02 的两个 renderer。下一方案在本方案 viewport 与二维 layout 的基础上建立可见 Cell 生命周期；方案 05 承接 Header / Cell 输入目标和 resize 的实际交互。

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

## 17.4 View、Layout 与 Style 用例

主要 owner：View / layout / style。通过 BSN / SceneComponent 的公共创建入口与真实 ECS 组合验证，不把示意 struct 当成已经确定的完整构造 API。

| 场景 | stimulus / 前提 | 验证结果 |
| --- | --- | --- |
| BSN source 接入 | 使用 @WidgetryTable<User> 并提供 users source，执行首次 update | 生成 viewport、Header 和实际需要的 Cell；数据来自指定 source，调用方不需要创建 Axis |
| 多 Table source 隔离 | 创建绑定不同 source 的两个 Table，更新其中一个 source | 各自显示对应 Model；selection、viewport 和 renderer Content 不串到另一个 Table |
| 四区结构 | 创建带可见数据的 Table | Corner、Column Header、Row Header、Body 各自承担原设计职责；Body 不重建 Row-centric Cell 层级 |
| 水平滚动 | 通过实际 scroll 输入改变水平位置 | Body 与 Column Header 水平同步；Row Header 不跟随水平滚动 |
| 垂直滚动 | 通过实际 scroll 输入改变垂直位置 | Body 与 Row Header 垂直同步；Column Header 不跟随垂直滚动 |
| 两轴与 viewport 变化 | 先形成非零两轴 scroll，再改变 viewport 大小 | Header 与 Body 仍对齐；可见范围与实际 layout 一致。clamp、Corner 固定位置等未定义细节先明确再断言 |
| 区域 style 与 Content | 改变 table、column_header、row_header、corner、cell 的已定义运行期 style | style 落在对应区域；Cell background / padding / border 变化不把 renderer Content 改成 Table 固定内容；若支持 theme 输入，再覆盖实际 theme stimulus |
| Column width 与 resize | 根据明确后的 width state owner 配置 fixed / flexible width，并执行 drag resize | 验证宽度、Header / Body 对齐与既定边界；当前缺少 width 分配和 resize contract，不能仅凭 Gallery 名称发明预期算法 |
| source 失效与生命周期 | 按先确定的 ownership contract 移除 source、销毁 Table；若允许重绑定 / rebuild，再使用其公开入口 | 失败处理和清理符合 contract，Table 自己创建的 viewport / Header / Cell / Content 无遗留；不能删除调用方仍拥有的外部 source |

必要前提：Row Header 数据来源与 renderer、Corner 内容、source invalid / rebinding、创建 entity 的 ownership、layout width 和 runtime style 接口仍未完整定义。对应场景不能用直接构造 private hierarchy 或修改内部 cache 绕过公共路径。

---

## 本方案的组合验证边界

四区同步滚动验证由本方案负责 layout 结果，二维 Cell 集合由方案 04 验证。Column resize 的真实 drag 和事件流程在方案 05 的输入能力到位后联合作为完整 contract 验证，不以手动修改 width 代替 drag。

## 验证边界与未决前提

实施阶段必须分别记录：已通过的 unit / integration test、Gallery 编译、实际 BRP 用户场景和未完成项目；文档中的 case、Cargo 测试成功或截图都不能互相替代证据。

以下是原方案尚未给出、但测试必须依赖的 contract。这里只标明缺口，不补出新设计：

- Renderer：CellValue / HeaderValue 表达、注册 API、缺失与重复注册、Content 更新 / ownership。
- View / layout：Row Header 内容与 renderer、Corner 内容、source 失效 / 重绑定、entity ownership、width state owner、fixed / flexible 分配、resize 边界。

这些缺口应随受影响的小方案保留，在对应 type 与行为实施前确定。不得为了让 checklist 看起来完整而为缺口写出臆测的 expected result，也不得把未决 case 标记为已验证。

---
