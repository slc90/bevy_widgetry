# Tree

Tree 将真实 ECS 层级显示为可展开、可选择、可虚拟化的树。业务节点可以携带不同组件，由对应 Renderer 决定内容；多个 TreeView 可以共享一棵业务树，同时保留独立的展开与选择状态。

```rust
use bevy::prelude::*;
use bevy_widgetry::tree::*;
use bevy_widgetry::text::WidgetryText;

#[derive(Component)]
struct EntryLabel(String);

app.register_renderer::<EntryLabel>(WidgetryTreeRenderer::new(
    |_, value: &EntryLabel| {
        let label = value.0.clone();
        bsn_list! { Text(label) WidgetryText }
    },
)).map_err(TreeFailure::into_bevy_error)?;
```

## 树形显示

### 创建业务树和 View

```rust
fn create_tree(world: &mut World) -> Result<Entity, BevyError> {
    let root = world.spawn_empty().id();
    let folder = world.spawn((
        WidgetryTreeNode,
        EntryLabel("项目".into()),
        ChildOf(root),
    )).id();
    world.spawn((
        WidgetryTreeNode,
        EntryLabel("说明.md".into()),
        ChildOf(folder),
    ));
    let source = world.spawn(WidgetryTreeModel::new(root)).id();
    Ok(world.spawn_scene(bsn! {
        @WidgetryTreeView { @source: source }
        Node { width: px(320), height: px(260) }
    })?.id())
}
```

`root` 是业务层级的根，`source` 是保存该根的 Model 实体，返回值是 View 根。它们不是同一个对象。显示行由 TreeView 临时创建，不能用显示行实体代替业务节点。

应用管理业务树和 Model 的生命周期。删除一个 View 不应被当作删除共享业务数据。

### 使用不同节点类型

为文件夹、文件等不同 Component 分别注册 `WidgetryTreeRenderer`。Renderer 按节点携带的组件选择，不要求所有节点用同一种字符串或枚举表示。节点的内容类型必须能明确匹配 Renderer；缺失或歧义会返回诊断。

## 展开与选择

### 程序化展开、折叠和选中

```rust
WidgetryTreeView::expand(world, view, folder)
    .map_err(TreeViewFailure::into_bevy_error)?;
WidgetryTreeView::select(world, view, Some(file))
    .map_err(TreeViewFailure::into_bevy_error)?;
WidgetryTreeView::collapse_all(world, view)
    .map_err(TreeViewFailure::into_bevy_error)?;
```

`folder` 和 `file` 都是业务节点 Entity。批量展开使用 `expand_all`，选择当前可见节点使用 `select_visible`。

展开只改变当前 View 的显示范围，不修改业务层级；折叠也不销毁隐藏的后代。不同 View 可以一边展开文件夹，一边保持折叠。

### 读取状态与接收事件

```rust
let selection = WidgetryTreeView::selection(world, view)
    .map_err(TreeViewFailure::into_bevy_error)?;
let selected_nodes = selection.selected_ids();
```

`WidgetryTreeState` 提供 `visible_items()` 和 `expanded_entities()`。选择的权威通过 `selection` 查询，不要另外维护一份与它竞争的集合。

```rust
fn tree_event(event: On<WidgetryTreeEvent>) {
    match &event.kind {
        WidgetryTreeEventKind::Expanded(node) => {
            info!(view = ?event.entity, ?node, "展开节点");
        }
        WidgetryTreeEventKind::SelectionChanged { selected, .. } => {
            info!(count = selected.len(), "选择变化");
        }
        _ => {}
    }
}
```

将 `on(tree_event)` 组合到 View 上。选择、Active 和展开是不同状态，不能用展开集合推断选中集合。

## 延迟加载

### 声明尚未加载的子节点

```rust
let remote = world.spawn((
    WidgetryTreeNode,
    EntryLabel("远程目录".into()),
    WidgetryTreeChildrenState::Unknown,
    ChildOf(root),
)).id();
```

用户展开时，应用通过 `WidgetryTreeEventKind::ChildrenRequested(node)` 接收加载请求。加载动作由应用执行；Tree 不自行访问文件系统或网络。

### 完成一次加载

收到结果后，把真实业务节点挂到请求节点下面，再通过完成入口标记已加载：

```rust
world.spawn((
    WidgetryTreeNode,
    EntryLabel("远程文件.txt".into()),
    ChildOf(remote),
));
WidgetryTreeChildrenState::set_loaded(world, remote)
    .map_err(TreeViewFailure::into_bevy_error)?;
```

异步结果返回时先确认节点及当前请求仍然属于应用要完成的加载。未知、加载中和已加载是不同状态；空目录也需要明确结束加载，而不是一直保留 Unknown。

## 节点移动

### 启用拖放

```rust
bsn! {
    @WidgetryTreeView { @source: source, @draggable: true }
    Node { width: px(320), height: px(260) }
}
```

运行期可以通过 `WidgetryTreeView::set_draggable(world, view, enabled)` 修改。拖放在节点之前、之后或内部形成移动请求，最终修改真实业务层级，而不是只移动当前 View 的行。

移动会验证节点成员资格、目标父链、循环关系、禁用状态与子节点加载状态。不能把节点移入自己的后代，也不会因为落点是 Unknown / Loading 就自动启动加载并暂存移动。

### 观察移动结果

```rust
fn moved(event: On<WidgetryTreeEvent>) {
    if let WidgetryTreeEventKind::NodeMoved {
        movement, old_parent, new_parent,
    } = &event.kind {
        info!(?movement, ?old_parent, ?new_parent, "层级已修改");
    }
}
```

其他共享 View 在正常更新阶段重新投影层级，但各自展开与选择仍独立。同父同位置的合法请求不产生多余移动通知。

## 外观与可见范围

通过 View 根的 `Node` 限定尺寸。Tree 借助 ListView 虚拟化可见行，展开的大树不等于同时生成所有行实体。

```rust
let mut colors = WidgetryTreeColorOverrides::default();
colors.item.selected.background = Some(Color::srgb(0.08, 0.45, 0.42));
WidgetryTreeColorOverrides::set(&mut commands, view, colors);
```

Renderer 管理业务内容；Tree 管理行、缩进和展开器。折叠、滚动回收和配色刷新不应让业务节点失去自己的数据。
