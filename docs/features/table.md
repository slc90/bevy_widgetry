# Table

Table 提供异构单元格、二维虚拟化、行列与单元格区域选择、列宽调整、冻结列、视图重排、原位编辑及 TSV 复制。业务数据保存在 Model 中；显示顺序、选择、滚动和编辑会话属于各个 View。

```rust
use bevy::prelude::*;
use bevy_widgetry::table::*;
use bevy_widgetry::text::WidgetryText;

struct Record {
    name: String,
}

app.register_widgetry_table::<Record>();
```

## 数据与单元格显示

### 注册单元格与表头 Renderer

Renderer 按值类型注册。单元格与表头是两个独立注册表，即使都使用 String，也需要分别提供显示方式。

```rust
app.register_table_cell_renderer(WidgetryTableCellRenderer::new(
    |value: &String| {
        let text = value.clone();
        bsn_list! { Text(text) WidgetryText }
    },
)).map_err(TableFailure::into_bevy_error)?;

app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(
    |value: &String| {
        let text = value.clone();
        bsn_list! { Text(text) WidgetryText }
    },
)).map_err(TableFailure::into_bevy_error)?;
```

列从业务行投影出 `WidgetryTableCellValue`。不同列可以返回不同类型，不必把所有内容预先格式化成字符串。

### 建立 Model 与 View

```rust
fn create_table(world: &mut World) -> Result<Entity, BevyError> {
    let mut model = WidgetryTableModel::<Record>::default();
    for name in ["采集任务", "分析任务"] {
        model.push_row(Record { name: name.into() })
            .map_err(TableFailure::into_bevy_error)?;
    }
    model.push_column(WidgetryTableColumn::new(
        WidgetryTableHeaderValue::new("名称".to_owned()),
        (),
        |row: &Record, _: &()| WidgetryTableCellValue::new(row.name.clone()),
    )).map_err(TableFailure::into_bevy_error)?;

    let source = world.spawn(model).id();
    Ok(world.spawn_scene(bsn! {
        @WidgetryTable::<Record> {
            @source: source,
            @layout: WidgetryTableLayout {
                row_height: 32.0,
                default_column_width: WidgetryTableColumnWidth::Fixed(180.0),
                ..default()
            },
        }
        Node { width: px(560), height: px(300) }
    })?.id())
}
```

`source` 是 Model Entity，返回值是 Table View。行和列分别拥有稳定 ID；单元格身份由 RowId 与 ColumnId 组成，不是当前显示行列号。

## 列布局与冻结

### 设置固定列宽和弹性列宽

```rust
let layout = WidgetryTableLayout {
    default_column_width: WidgetryTableColumnWidth::Flexible(1.0),
    frozen_columns: 1,
    ..default()
}.with_column_width(first_column, WidgetryTableColumnWidth::Fixed(180.0));
```

`first_column` 是 Model 中取得的 ColumnId。固定宽度与弹性权重可以混用，用户拖拽标题边缘可以调整对应列宽。宽度按列身份保存，列顺序变化不会把 A 的宽度误用到 B。

### 冻结前几个显示列

`frozen_columns` 指定当前 View 显示顺序的前 N 列。冻结区固定在左侧，非冻结区水平滚动；两区同步纵向滚动。

```rust
let mut layouts = world.query::<&mut WidgetryTableLayout>();
layouts.get_mut(world, view)?.frozen_columns = 2;
```

冻结数量不会因为视口变窄而自动减少。冻结区过宽时，超出视口的部分被裁剪；被完全裁掉的冻结列不会通过水平滚动出现。这里提供的是冻结列，不是任意业务行冻结。

## 显示顺序与筛选

### 启用列拖拽重排

```rust
bsn! {
    @WidgetryTable::<Record> {
        @source: source,
        @column_reordering: WidgetryTableColumnReordering { enabled: true },
    }
    Node { width: px(600), height: px(320) }
}
```

拖拽标题主体改变本 View 的列顺序，不改 Model 列定义。列边缘仍用于调整宽度，表头排序和筛选按钮不启动重排。

```rust
WidgetryTable::<Record>::reset_column_order(world, view)
    .map_err(TableFailure::into_bevy_error)?;
```

冻结区随新的显示顺序取前 N 列，N 本身不改变。

### 提交应用计算的行投影

Table 不执行任意业务比较和筛选。应用计算稳定 RowId 序列，再一次性提交结果及表头状态：

```rust
WidgetryTable::<Record>::update_view(
    world,
    view,
    WidgetryTableViewUpdate {
        rows: ordered_rows,
        sort: None,
        filters: Default::default(),
    },
).map_err(TableFailure::into_bevy_error)?;
```

`ordered_rows` 是当前 Model 的合法 RowId 子集及顺序。空序列表示有效的空视图，重复或失效 ID 会拒绝整次更新。受控投影不会自动接纳新行；应用在数据改变后重新计算并提交。

`reset_view` 清除受控结果并恢复默认 Model 顺序。表头 `WidgetryTableHeaderActions` 的 `sortable`、`filterable` 集合只开启请求入口，不会替应用完成排序筛选。

## 选择与键盘导航

点击行号、列标题或单元格建立相应类型的连续选区。Cell 支持拖选，Shift 扩展同类型范围；Ctrl+A 和 Corner 全选以当前 View 投影为范围。

键盘移动 Focus 并将目标显示到可见区域。冻结列只需纵向 Reveal，普通列同时考虑横向。选择基于逻辑身份，因此可以包含离屏单元格，不限于已经生成的 Cell Entity。

View 禁用阻止用户选择和编辑，不会删除业务数据或自动清空所有程序控制能力。只读单元格仍可被查看、聚焦和选择，不等于 Disabled。

## 原位编辑

### 注册编辑器

编辑器和 Renderer 分开注册。编辑器用强类型初值创建 SceneList，再从编辑根读取强类型结果。以下读取函数寻找创建的 TextField：

```rust
use bevy_widgetry::text_field::{WidgetryTextField, WidgetryTextFieldHeadless};

fn read_text_editor(world: &World, root: Entity)
    -> Result<String, WidgetryTableEditorReadError>
{
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if world.get::<WidgetryTextField>(entity).is_some() {
            return WidgetryTextFieldHeadless::text(world, entity)
                .map_err(WidgetryTableEditorReadError::diagnostic);
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    Err(WidgetryTableEditorReadError::external(std::io::Error::other(
        "编辑器缺少 TextField",
    )))
}

app.register_table_cell_editor(WidgetryTableCellEditor::new(
    WidgetryTableEditorMode::SingleLine,
    |value: &String| {
        let text = value.clone();
        bsn_list! { @WidgetryTextField { @value: text } }
    },
    read_text_editor,
));
```

随后通过 `WidgetryTableEditing::new(predicate)` 或对应 Props 开启编辑资格。只有资格允许且存在匹配 Editor 的 Cell 才可编辑。可以按 `(Source, ColumnId)` 注册列专用 Editor，覆盖类型默认值。

读取返回 `Rejected(String)` 表示用户输入不能接受，草稿保留并显示反馈；`Failed` 或 `diagnostic` 表示真实基础设施失败，不能伪装成普通校验提示。

### 提交与接收应用回执

编辑提交生成 `WidgetryTableEditRequested`，包含 View、Cell、强类型值和 `request_id`。Table 不直接修改 Model。应用先校验并写回业务值，再确认本次请求。

```rust
// request 是收到的 WidgetryTableEditRequested。
if world.get::<WidgetryTableEditState>(request.entity)
    .and_then(|state| state.pending()) != Some(request.request_id)
{
    return Err(std::io::Error::other(
        "Edit request is no longer pending",
    ).into());
}
let value = request.value.downcast_ref::<String>()
    .ok_or_else(|| std::io::Error::other("Expected a String edit value"))?;
let source = world.get::<WidgetryTable<Record>>(request.entity)
    .ok_or_else(|| std::io::Error::other("Table view no longer exists"))?
    .source();
{
    let mut models = world.query::<&mut WidgetryTableModel<Record>>();
    let mut model = models.get_mut(world, source)?;
    let index = model.row_index(request.cell.row)
        .ok_or_else(|| std::io::Error::other("Edited row no longer exists"))?;
    let row = model.row_mut(index)
        .map_err(TableFailure::into_bevy_error)?
        .ok_or_else(|| std::io::Error::other("Edited row no longer exists"))?;
    row.name = value.clone();
}
WidgetryTable::<Record>::resolve_edit(
    world, request.entity, request.request_id,
    WidgetryTableEditResult::Accepted,
).map_err(TableEditFailure::into_bevy_error)?;
```

这个单列场景中的 `source` 必须是请求 View 当前绑定的 Model；多列应用还需按 `request.cell.column` 选择字段。目标已失效时不能继续确认一个并未写入的结果。

业务拒绝使用 `WidgetryTableEditResult::Rejected(message)`，保留草稿供修改重试。Pending 阶段不重复提交；Esc 取消未提交草稿，不撤销已经发出的业务请求。

原生 Enter 提交会在当前帧文本编辑应用后读取草稿，避免漏掉最后一次输入。程序化 `submit_edit` 则读取调用时已经生效的状态。

## TSV 复制

### 导出完整逻辑选区

```rust
if let Some(tsv) = WidgetryTable::<Record>::selection_tsv(world, view)
    .map_err(TableCopyFailure::into_bevy_error)?
{
    info!(bytes = tsv.len(), "已生成选区文本");
    // 将 tsv 交给应用自己的保存或剪贴板入口。
}
```

复制读取当前完整逻辑选区，包含离屏 Cell，排除被过滤的行，顺序跟随当前 View。结果不包含行号、列标题、Corner 或装饰图标。含 Tab、换行或引号的字段按 TSV 引用规则转义。

自定义值通过 Copy Formatter 提供文本表示。复制是只读操作，不生成编辑请求，也不会改写 Model。

## 外观与更新

`WidgetryTableColorOverrides` 可分别控制 Cell、表头等区域颜色。Model 内容更新、列重排、颜色变化和 Renderer 变化具有不同作用：单纯换色不重建业务数据，滚动回收也不删除编辑目标。

编辑期间会额外保留必要的 Cell Shell，避免离屏导致未提交草稿丢失；Model 实例更换或目标真实删除则结束旧会话。应用不能依靠复用的物理 Cell Entity 保存长期业务状态。
