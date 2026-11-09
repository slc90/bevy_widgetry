# Tabs

Tabs 提供固定数量的标签页，支持横向或竖向标签栏、程序切换和标签栏自动滚动。页面实体保持常驻，切换只改变活动页面的展示。

```rust
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::tabs::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryTabsPlugin);
```

## 标签页集合

### 创建成对的标题和内容

```rust
fn tab(title: &'static str, body: &'static str) -> impl Scene {
    bsn! {
        @WidgetryTab {
            @header: {Some(Box::new(bsn! {
                Children [Text(title) WidgetryText]
            }) as Box<dyn Scene>)},
            @content: {Some(Box::new(bsn! {
                Node { padding: UiRect::all(px(16)) }
                Children [Text(body) WidgetryText]
            }) as Box<dyn Scene>)}
        }
    }
}

fn settings_tabs() -> impl Scene {
    bsn! {
        @WidgetryTabs {
            @selected_index: 0,
            @children: {Some(Box::new(bsn_list! {
                @tab("基本", "基本设置")--
                @tab("高级", "高级设置")
            }) as Box<dyn SceneList>)}
        }
        Node { width: px(600), height: px(360) }
        on(changed)
    }
}
```

集合必须非空，初始索引合法。构造后页数与 Header/Content 配对固定，不支持增删或重排。需要从业务集合生成页面时，在构造前生成 SceneList。

## 选择与通知

### 读取和修改活动页

```rust
WidgetryTabs::set_selected(&mut commands, tabs_root, 1);

fn changed(event: On<ValueChange<usize>>) {
    info!(root = ?event.source, page = event.value, "活动页已改变");
}
```

同步入口 `set_selected_in_world` 返回是否真实改变。`selected_index` 查询当前索引，`header` 和 `page` 查询固定部件实体。

用户点击和程序切换都在选择、Header Checked 与页面显示提交后发出 `ValueChange<usize>`；初始化与同值不通知。不要直接写 Header 的 Checked 或页面包装层的 Display 来切页。

### 保留页面内容

切到别页不会重建之前的文本框、滚动区或数据实体。隐藏页面的业务 System 也不会自动暂停；需要暂停昂贵业务时，由应用根据活动页决定。

## 标签栏布局

### 改为竖向布局

```rust
bsn! {
    @WidgetryTabs {
        @orientation: TabsOrientation::Vertical,
        @children: {Some(Box::new(bsn_list! {
            @tab("基本", "基本设置")--
            @tab("外观", "外观设置")
        }) as Box<dyn SceneList>)}
    }
    Node { width: px(640), height: px(400) }
}
```

方向决定标签栏布局和内部方向导航。标签过多时，标签栏使用自动滚动策略，并把选中标题带入可见区域。

页面内容没有默认套 ScrollArea。页面需要滚动时，在 `@content` 中显式组合自己的滚动容器，避免标签栏滚动与页面滚动互相混淆。

## 禁用与焦点

### 限制用户切换

整组或具体标题可以拥有 Disabled。禁用不删除页面，不自动改选其他页。程序切换仍以公开 API 的索引和结构校验为准。

插件保留指针聚焦和本控件内的方向操作，不自动添加跨控件 Tab 导航。切换页面时，隐藏页面中的旧焦点不能继续作为正常用户输入目标。

## 内容外观

### 使用交互式标题和业务页面

标题与页面接受 Scene，可以组合图标、提示和编辑器。Tabs 的主题与颜色覆盖只管理自身外壳和标签头，页面业务内容的样式、滚动、数据和布局由应用管理。

页面包装层的显示是控件托管输出，页面内部自定义的 Visibility 和布局不是切页状态的替代入口。
