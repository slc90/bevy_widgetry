# ComboBox

ComboBox 从列表数据源选择一个条目，支持占位内容、虚拟化弹出列表、本地搜索与自定义外观。选择使用稳定 Item ID，不依赖条目当前排列位置。

```rust
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::combo_box::*;
use bevy_widgetry::list_view::{
    ListFailure, WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer,
};
use bevy_widgetry::text::WidgetryText;

app.register_widgetry_combo_box::<String>()
    .map_err(ComboFailure::into_bevy_error)?;
```

## 单项选择

### 创建数据源和下拉框

```rust
fn create_combo(world: &mut World) -> Result<Entity, BevyError> {
    let mut model = WidgetryListModel::<String>::default();
    for label in ["自动", "手动", "只读"] {
        model.push(label.to_owned())
            .map_err(ListFailure::into_bevy_error)?;
    }
    let source = world.spawn(model).id();
    Ok(world.spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source,
            @max_visible_items: 6_usize,
            @placeholder: {
                WidgetryComboBoxContent::new(|| {
                    bsn_list! { Text("请选择模式") WidgetryText }
                })
            },
            @renderer: {
                WidgetryListViewRenderer::new(|value: &String| {
                    let label = value.clone();
                    bsn_list! { Text(label) WidgetryText }
                })
            },
        }
        Node { width: px(240) }
        on(selected)
    })?.id())
}

fn selected(event: On<ValueChange<Option<WidgetryListItemId>>>) {
    info!(combo = ?event.source, item = ?event.value, "选择已提交");
}
```

初始选择为 `None`，即使 Model 非空也不会自动选择第一项。清空选择或真实删除选中项后重新显示 Placeholder。应用负责共享 Model 的生命周期。

### 程序化设置和查询

```rust
WidgetryComboBox::<String>::select_index(&mut commands, combo, 1);
WidgetryComboBox::<String>::clear_selection(&mut commands, combo);

let selected_id = WidgetryComboBox::<String>::selected_id(world, combo)
    .map_err(ComboFailure::into_bevy_error)?;
```

`select_index` 在 Commands 执行时解析原始 Model 索引。保存长期选择时使用稳定 ID，并通过 `select_id` 设置。移动条目不会改变选择身份；程序设置选择不会强制关闭已经打开的弹层。

## 本地搜索

### 增加匹配规则

在构造 Props 中提供 Matcher。Matcher 只决定当前条目是否匹配，不直接改写 Model。

```rust
let search = WidgetryComboBoxMatcher::<String>::new(|query, value| {
    value.to_lowercase().contains(&query.to_lowercase())
});

let scene = bsn! {
    @WidgetryComboBox::<String> {
        @source: source,
        @search: {Some(search)},
        @renderer: {
            WidgetryListViewRenderer::new(|value: &String| {
                let label = value.clone();
                bsn_list! { Text(label) WidgetryText }
            })
        },
        @no_results: {
            WidgetryComboBoxContent::new(|| {
                bsn_list! { Text("没有匹配项") WidgetryText }
            })
        },
    }
    Node { width: px(260) }
};
```

`source` 是已经创建的 `WidgetryListModel<String>` Entity。查询只改变这个 ComboBox 的列表投影，不影响同源的其他 View。

### 处理搜索后的选择

过滤隐藏已选项时，Field 仍显示原选择；只有真实删除该项才清空。每次打开会清空查询并聚焦搜索编辑器。无匹配结果时仍允许继续编辑。

Up / Down、Home / End 浏览候选，Enter 确认，Escape 取消。文本编辑和 IME 继续由 TextField 处理，导航键不会同时移动文本光标。

## 弹层控制

### 查询、打开和关闭

```rust
WidgetryComboBoxHeadless::<String>::open(&mut commands, combo);
WidgetryComboBoxHeadless::<String>::close(&mut commands, combo);
```

同步场景使用 `open_in_world` / `close_in_world`，通过 `is_open` 查询逻辑状态。不要通过直接修改默认 Popup 的 `Visibility` 代替开关 API。

选择、Escape、外部点击可以关闭弹层。Escape 将焦点返回触发器；外部点击不抢回已经转移给其他控件的焦点。关闭时隐藏列表立即失去交互能力，不能继续消费同批排队的确认输入。

非搜索空 Model 不打开弹层。禁用拒绝用户打开和选择，但保留合法程序化选择入口。

## 内容与颜色

### 调整状态配色

```rust
let mut colors = WidgetryComboBoxColorOverrides::default();
colors.field.open.border = Some(Color::srgb(0.48, 0.23, 0.85));
WidgetryComboBoxColorOverrides::set(&mut commands, combo, colors);
```

Field、Popup 和内部列表各有对应配色。颜色变化不会重置选择或重建内容 Renderer。清除覆盖使用 `WidgetryComboBoxColorOverrides::clear`。

### 构建自己的触发器和弹层

`WidgetryComboBoxHeadless<T>` 保存 Source 和打开状态；`WidgetryComboBoxParts { trigger, popup, list }` 绑定应用提供的部件。触发器可采用 Button Headless；列表必须使用同一 Source 和 Single 选择模式。

部件可以位于根内多层嵌套布局中，但 List 必须在 Popup 内，Trigger 在 Popup 外，且不能跨越另一个 ComboBox 根。当前不支持把弹层放到根外的 portal。

附加 `WidgetryComboBoxSearch<T> { field, matcher }` 可将自己的 TextField Headless 根作为搜索区。外部 Style 自行投影打开状态和颜色，行为层不替换其视觉结构。
