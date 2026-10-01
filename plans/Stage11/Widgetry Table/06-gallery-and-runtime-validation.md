# Gallery 展示与 BRP 运行时验收

## 目标

在 Widget Gallery 中展示 Table 的完整设计能力，并通过真实用户输入确认当前实现的视觉、交互与 temporal behavior。

## 范围

负责 gallery/src/pages/table.rs、TableDemoPlugin、TableDemoSources，以及原方案列出的 Basic、CellValue、Header、Selection、Column Layout、Virtualization 和 Disabled Demo。只消费前面方案的公共接口，不由 Gallery 补出库的缺失 contract。

## 预期产出

形成能够编译的 Gallery Table 页面及有实际输入 / screenshot / 必要 state 证据的 BRP 验收记录；按 Gallery 例外不新增 gallery/ 自动化测试，记录缺失条件和未完成项。

## 与前后方案的关系

这是唯一执行链的最后一份方案，消费方案 01–05 的已定义能力。Gallery 是真实运行时验收入口；长期 unit / integration / regression 保护仍归库行为的主要 owner。

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

## Gallery

新增：

```
gallery/src/pages/table.rs
```

注册：

```
TableDemoPlugin
TableDemoSources
```

---

Demo 内容：

### Basic Table

展示：

- Row Axis
- Column Axis
- Cell renderer

### CellValue Renderer

展示：

- String
- Number
- Bool
- Progress
- Icon

### Header Renderer

展示：

- 普通 header
- 自定义 header

### Selection

展示：

- Column selection
- Row selection
- Cell selection

### Column Layout

展示：

- fixed width
- flexible width
- resize

### Virtualization

展示：

- 大量 rows

### Disabled

展示：

- 整体 Table disabled

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

## 17.7 Gallery 与 BRP 运行时验收

Gallery 遵守 rules/testing.md 的例外：不为 gallery/ 编写 unit test、integration test 或其他自动化测试，保证能够编译，并按 rules/gui-debugging.md 通过真实 Widget Gallery 进行运行时验收。

BRP GUI 验证不属于长期自动化测试覆盖体系，不替代 crates/table 的 unit / integration test，也不以 BRP 场景数量判断模块覆盖是否全面。场景根据当前阶段新增、修改及直接影响的 GUI 行为选择。

标准流程：使用 brp_list_bevy 确认 widget_gallery，通过 brp_launch 启动，获取 baseline screenshot，执行需要的 pointer / keyboard / drag / scroll 输入，以 screenshot 和必要 ECS / Component / Resource state 检查结果，最后通过 BRP 正常关闭 Gallery。

| Gallery 场景 | 实际输入与验收 |
| --- | --- |
| Basic Table / CellValue Renderer / Header Renderer | 检查两个 Axis 的数据投影，以及 String、Number、Bool、Progress、Icon 和自定义 Header 的实际 Content；screenshot 确认视觉，不只确认应用存活 |
| Selection | 实际点击 Column Header、Row Header、Cell，结合截图与 state / events 确认 ID 和选择表现；通过真实输入取得 focus 后发送四方向键 |
| Column Layout | 验证 fixed / flexible width 的实际布局，并实际 drag resize；结合 width state / events 与截图检查 Header / Body 对齐 |
| Virtualization | 使用同时超过 viewport 的 Row 与 Column 数据，实际水平、垂直及连续 scroll，检查四区滚动和二维 Content；大量 rows Demo 仍保留，验收另需覆盖 columns 超过 viewport 的输入条件 |
| Disabled | 实际 pointer movement / click 验证无 enabled hover、无 selection input 和 disabled style，再恢复 enabled 检查输入；keyboard 分支按最终 disabled contract 验证 |
| 动态变化 | 对 renderer Content 更新、scroll 后 Cell 创建 / 替换、layout / focus / disabled 转换，按 temporal risk 尽快观察操作后和必要中间 state，再确认最终 state，注意短暂空白、旧 Content、flicker 或 Header / Body 延迟对齐 |

不得直接修改 ECS selection / focus / scroll state 后把结果当作真实输入验收，不默认使用 PowerShell 截图或 Win32 鼠标控制替代 BRP。保留 Gallery 的 desktop_app update mode，不为验证响应速度改成全局高频 update。

如果 BRP 不可用，应明确报告 GUI 验证未完成和缺失条件。BRP 发现能够稳定复现、且可由 headless 测试合理表达的问题时，应提炼 crates/table 的永久 regression test。

---

## 本方案的组合验证边界

BRP 发现可稳定复现且适合 headless 表达的问题时，提炼为对应库行为的永久 regression test；不得把只看最终截图或直接写 ECS state 的结果当成用户路径已通过。

## 验证边界与未决前提

实施阶段必须分别记录：已通过的 unit / integration test、Gallery 编译、实际 BRP 用户场景和未完成项目；文档中的 case、Cargo 测试成功或截图都不能互相替代证据。

以下是原方案尚未给出、但测试必须依赖的 contract。这里只标明缺口，不补出新设计：

- Model：Column schema、mutation API、Row revision 规则、ID lifetime / stale ID 查询语义。
- Renderer：CellValue / HeaderValue 表达、注册 API、缺失与重复注册、Content 更新 / ownership。
- View / layout：Row Header 内容与 renderer、Corner 内容、source 失效 / 重绑定、entity ownership、width state owner、fixed / flexible 分配、resize 边界。
- Virtualization：Cell 几何、可见边界 / overscan、scroll clamp、回收 / 复用、Header virtualization、数据变化后的 viewport 修复。
- Interaction：selection 初始 / 清除 / 修复、重复选择 events、Focus 获得 / 丢失、方向键边界、Focus × selection、disabled × keyboard、programmatic events、resize 开始 / 移动 / 结束 / 中断。

这些缺口应随受影响的小方案保留，在对应 type 与行为实施前确定。不得为了让 checklist 看起来完整而为缺口写出臆测的 expected result，也不得把未决 case 标记为已验证。

---
