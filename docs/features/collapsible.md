# Collapsible

Collapsible 提供可折叠的 Header 与内容区。内容始终保留在 ECS 中，折叠只是隐藏，不会丢掉表单草稿或重新创建业务实体。

```rust
use bevy::prelude::*;
use bevy_widgetry::collapsible::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryCollapsiblePlugin);
```

## 折叠内容

### 创建可展开区域

```rust
bsn! {
    @WidgetryCollapsible {
        @expanded: true,
        @header: {Some(Box::new(bsn! {
            Text("高级设置") WidgetryText
        }) as Box<dyn Scene>)},
        @children: {Some(Box::new(bsn_list! {
            Text("这里放置需要保留状态的设置内容") WidgetryText
        }) as Box<dyn SceneList>)}
    }
    on(expanded_changed)
}

fn expanded_changed(event: On<WidgetryCollapsibleChanged>) {
    info!(root = ?event.entity, expanded = event.expanded, "展开状态已改变");
}
```

默认展开，Header 和 Content 可以为空。Header 末端保留切换按钮；调用方提供的 Header 内容占据剩余空间。

### 程序化折叠和展开

```rust
WidgetryCollapsible::set_expanded(&mut commands, root, false);
WidgetryCollapsible::set_expanded(&mut commands, root, true);
```

同步入口 `set_expanded_in_world` 返回是否真实改变，`expanded(world, root)` 查询当前值。无效根或必要结构缺失时，返回领域失败而不发出成功通知。

## 状态保留

### 在内容区放置编辑器

```rust
use bevy_widgetry::text_field::WidgetryTextField;

bsn! {
    @WidgetryCollapsible {
        @header: {Some(Box::new(bsn! {
            Text("备注") WidgetryText
        }) as Box<dyn Scene>)},
        @children: {Some(Box::new(bsn_list! {
            @WidgetryTextField { @value: {"未提交的草稿".to_owned()} }
        }) as Box<dyn SceneList>)}
    }
}
```

折叠再展开后，同一个 TextField 和正文仍然存在。隐藏不暂停其业务 System；需要停止后台工作时，由应用控制自己的任务。

折叠通过 Content 包装层的 Display 与 Visibility 实现，不支持第一版折叠动画。不要直接改包装层显示来代替展开 API。

## 变化通知

### 在实际改变后响应

`WidgetryCollapsibleChanged` 包含根实体和新 expanded。事件发出时状态已经提交，布局可能还未重新计算。初始化、同值与失败不产生变化通知。

业务需要“打开时加载数据”时，可在 expanded 变为 true 后发起自己的请求，同时自行决定是否复用已加载数据。控件不自动把每次展开当成重建或重载。

## 外观与输入

### 组合自己的标题内容

Header 可以放文字、图标和其他交互内容，Content 接收任意 SceneList。卡片配色、箭头方向与切换按钮分别随当前状态更新，不覆盖业务子树的样式。

根或祖先的 Disabled 限制交互。Header 的局部禁用只作用于对应控制入口，不等同于销毁内容或清除 Expanded。
