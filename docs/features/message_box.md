# MessageBox

MessageBox 创建非阻塞的模态对话框，提供固定结果按钮组合和任意 BSN 正文。调用返回后应用继续运行，用户选择通过事件交付。

```rust
use bevy::prelude::*;
use bevy_widgetry::message_box::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryMessageBoxPlugin);
```

## 对话框与正文

### 显示确认问题

```rust
let dialog = world.spawn_scene(bsn! {
    @widgetry_message_box(
        parent_native_window,
        "离开编辑器？",
        WidgetryMessageBoxButtons::YesNoCancel,
        WidgetryMessageBoxColorOverrides::default(),
        bsn_list! { Text("当前修改尚未保存。") WidgetryText },
    )
    on(decision)
})?.id();
```

`parent_native_window` 是已经绑定 Widgetry 窗口界面的原生 Window Entity。MessageBox 自己创建子窗口；不要传入普通 UI 面板根。

### 使用较长或复合正文

```rust
let body = bsn_list! {
    Text("操作将覆盖已有结果。") WidgetryText--
    Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
    Children [
        Text("已有记录会被替换。") WidgetryText--
        Text("原始输入文件保持不变。") WidgetryText
    ]
};
```

将 `body` 作为 `widgetry_message_box` 的最后一个参数。正文位于可滚动区域，结果按钮保持在滚动区域外；窄空间下按钮可以换行。

## 结果选择

### 选择按钮集合

| 配置 | 按钮 |
| --- | --- |
| `Ok` | OK |
| `YesNo` | Yes、No |
| `YesNoCancel` | Yes、No、Cancel |

当前不提供任意结果枚举和自定义按钮集合。需要复杂业务编辑的面板，应使用 Window 与普通控件组合，而不是把 MessageBox 扩成完整表单框架。

### 接收一次决议

```rust
fn decision(event: On<WidgetryMessageBoxResultEvent>) {
    match event.result {
        WidgetryMessageBoxResult::Yes => {
            info!(dialog = ?event.entity, "接受操作");
            // 需要排队业务动作时，再为 Observer 添加 Commands 参数。
        }
        WidgetryMessageBoxResult::No | WidgetryMessageBoxResult::Cancel => {}
        WidgetryMessageBoxResult::Ok => {}
    }
}
```

`event.entity` 是 Dialog Root。结果先提交，再通知；Observer 排队的操作被处理后才进入关闭回收。因此读取关联业务上下文可以在结果 Observer 中完成。

每个 MessageBox 最多决议一次。重复点击或重复 resolve 不覆盖已经接受的结果。

## 程序化决议与关闭

### 主动提交结果

```rust
WidgetryMessageBox::resolve(world, dialog, WidgetryMessageBoxResult::Cancel)
    .map_err(MessageBoxFailure::into_bevy_error)?;
```

Cancel 按钮、Escape 和原生关闭请求也提交 Cancel。直接销毁对话框或父窗口结束则是资源清理，不会无条件伪造一个用户 Cancel 结果。

应用不能靠同步等待函数返回值取得选择。需要继续的业务动作放在结果消费中，或把结果写入应用自己的状态机。

## 标题与外观

### 更新标题

```rust
use bevy_widgetry::window::WidgetryWindowTitle;
WidgetryWindowTitle::set(&mut commands, dialog, "确认覆盖");
```

默认标题绑定到原生 Window.title。更新不影响正文、结果或窗口身份。

通过 `WidgetryMessageBoxColorOverrides` 调整组合配色。MessageBox 管理自己的窗口、正文背景和结果按钮，不接管调用方正文里任意嵌套按钮的业务样式。
