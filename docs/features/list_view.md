# ListView

ListView 展示带稳定身份的纵向列表，支持独立选择、键盘导航、过滤投影和行虚拟化。多个 View 可以共享一个 Model，但不会共享选择、焦点或滚动状态。

```rust
use bevy::prelude::*;
use bevy_widgetry::list_view::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryListViewPlugin);
app.register_widgetry_list_view::<String>()
    .map_err(ListFailure::into_bevy_error)?;
```

泛型类型需要注册。下例使用 `String`；业务类型可以替换为自己的结构体，并提供相应 Renderer。

## 列表显示

### 建立数据源和视图

在已经完成插件装配的 World 中创建 Model，再把其 Entity 交给 View：

```rust
fn create_list(world: &mut World) -> Result<Entity, BevyError> {
    let mut model = WidgetryListModel::<String>::default();
    for label in ["待处理", "进行中", "已完成"] {
        model.push(label.to_owned())
            .map_err(ListFailure::into_bevy_error)?;
    }
    let source = world.spawn(model).id();
    let view = world.spawn_scene(bsn! {
        @WidgetryListView::<String> {
            @source: source,
            @item_height: 32.0,
            @renderer: {
                WidgetryListViewRenderer::new(|value: &String| {
                    let text = value.clone();
                    bsn_list! { Text(text) WidgetryText }
                })
            },
        }
        Node { width: px(280), height: px(180) }
    })?.id();
    Ok(view)
}
```

`source` 是 Model 实体，返回的 `view` 是界面根。应用管理 Model 的生命周期；删除一个 View 不等于删除共享数据源。

Renderer 接收业务值的引用，返回拥有内容的 SceneList。不要让返回的 Scene 借用回调结束后失效的临时引用，文字等数据可以按需要克隆。

### 显示大列表

为 View 设置有限高度。ListView 只为当前可见范围及必要的预留范围建立物理行，滚动时可以回收和复用行实体。

业务项由 `WidgetryListItemId` 标识。行实体、显示位置和业务 ID 是不同概念：滚动后，同一个物理行可能展示另一项；业务状态不应保存在该行实体中。

## 数据更新

### 插入、修改与移动条目

下面的函数接收 Model 实体，不是 View 根：

```rust
fn update_model(world: &mut World, source: Entity) -> Result {
    let mut query = world.query::<&mut WidgetryListModel<String>>();
    let mut model = query.get_mut(world, source)?;

    model.insert(0, "新任务".to_owned())
        .map_err(ListFailure::into_bevy_error)?;
    if let Some(value) = model.get_mut(1)
        .map_err(ListFailure::into_bevy_error)?
    {
        *value = "名称已修改".to_owned();
    }
    if model.len() > 1 {
        let last = model.len() - 1;
        model.move_item(0, last);
    }
    Ok(())
}
```

移动已有项保持其稳定 ID；插入新项获得新的身份。`get_by_id`、`index_of` 和 `id` 用于在身份与 Model 当前索引之间转换。不要把旧索引当作永久业务标识。

### 禁用某一项

```rust
model.set_disabled(1, true);
```

这里的 `1` 是 Model 索引。单项禁用影响用户导航和选择，但不意味着数据从 Model 删除。禁用整个 View 也不妨碍应用通过公开 API 更新数据和选择。

## 选择与导航

### 程序化选择和清空

```rust
WidgetryListView::<String>::select_index(&mut commands, view, 1);
WidgetryListView::<String>::clear_selection(&mut commands, view);
```

索引在命令执行时按数据源解释，不是屏幕上的第几条可见物理行。需要跨重排保存目标时，应以稳定 ID 组织业务状态。

### 接收选择变化

```rust
fn selection_changed(event: On<WidgetryListSelectionChanged>) {
    info!(view = ?event.source, selected = ?event.selected, "选择已改变");
}
```

在 View 的 BSN 上追加 `on(selection_changed)`。读取当前选择使用 `WidgetryListSelectionState`；Active 是键盘浏览位置，不等于已选集合。

列表支持方向键、Home / End、Page 导航，以及相应模式下的点击、范围选择和全选。Ctrl+A 的范围取决于该 View 配置的选择范围策略，过滤后的显示集合与整个数据源不能混为一谈。

## 投影与独立视图

### 共享数据，保留不同展示

创建第二个 `WidgetryListView<String>`，将 `@source` 设为同一个 Model Entity，即可共享数据。两个 View 可以提供不同 Renderer，并拥有不同的选择、滚动和显示顺序。

`WidgetryListProjection<K>` 保存 View 的有序业务键；过滤和排序改变这一序列，不复制业务项，也不改变其稳定身份。自定义数据源可以用自己的键类型接入，而不必先复制成中间列表 Model。

### 控制手动排序

```rust
WidgetryListView::<String>::set_draggable(&mut commands, view, true);
WidgetryListView::<String>::reset_view_order(&mut commands, view);
```

用户拖动产生的是当前 View 的手动顺序。重置恢复其基础投影顺序，不应把它解释为清空 Model 或恢复已删除数据。Tree 等组合消费者使用自己的结构移动语义，不等同于普通列表的视图重排。

## 内容与状态外观

通过 `@renderer` 决定每项内容，通过 `WidgetryListViewColorOverrides` 控制容器和条目状态色。颜色变化不会改变 Model、选择或物理行绑定。

```rust
let mut colors = WidgetryListViewColorOverrides::default();
colors.item.selected.background = Some(Color::srgb(0.08, 0.45, 0.42));
WidgetryListViewColorOverrides::set(&mut commands, view, colors);
```

初始化缺少 Renderer、数据源无效或公开请求引用失效身份时会返回诊断。正常离屏回收、空投影和暂时等待布局不是数据失败。
