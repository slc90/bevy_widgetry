# NumberInput

NumberInput 提供强类型数值输入和加减步进。支持有符号整数、无符号整数及 f32/f64，正式值始终保留原始类型，不统一转换为浮点数。

```rust
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::number_input::*;

app.add_plugins(WidgetryNumberInputPlugin);
```

## 数值与范围

### 创建整数输入

```rust
bsn! {
    @WidgetryNumberInput::<i32> {
        @value: {Some(3)},
        @min: {Some(0)},
        @max: {Some(10)},
        @step: 3
    }
    Node { width: px(220) }
    on(changed)
}

fn changed(event: On<ValueChange<i32>>) {
    info!(root = ?event.source, value = event.value, "数值已提交");
}
```

`value: Some(...)` 是必填初值。min/max 可省略，step 默认为同类型的一。初值和边界必须合法，step 严格为正；配置构造后固定。

### 查询和程序化设置

```rust
fn reset_count(world: &mut World, root: Entity) -> Result {
    WidgetryNumberInputHeadless::<i32>::set_value_in_world(world, root, 0)
        .map_err(|error| error.into_bevy_error())?;
    let value = WidgetryNumberInputHeadless::<i32>::value(world, root)
        .map_err(|error| error.into_bevy_error())?;
    info!(value, "已重置计数");
    Ok(())
}
```

排队版本为 `set_value(&mut commands, root, value)`。Setter 校验范围和有限性，不要求步长对齐；Disabled 和 ReadOnly 不限制合法程序赋值。合法同值不发数值变化事件，但会清理旧草稿和未处理编辑请求。

## 文本编辑

### 提交或放弃草稿

键入期间可以暂时出现空串、负号或其他半成品，正式值不会随每个字符改变。

| 操作 | 合法草稿 | 非法草稿 |
| --- | --- | --- |
| Enter | 提交正式值 | 保留草稿继续编辑 |
| 焦点离开控件 | 提交正式值 | 恢复正式值文本 |
| Esc | 放弃草稿并保留编辑焦点 | 放弃草稿并保留编辑焦点 |

鼠标移出不是失焦。接受普通十进制文本，不接受指数、千位分隔、进制前缀、正号、空白或非有限浮点文本。无符号类型拒绝负号，整数类型拒绝小数。

### 设置浮点显示精度

```rust
bsn! {
    @WidgetryNumberInput::<f64> {
        @value: {Some(1.234567)},
        @step: 0.1,
        @decimals: {Some(2)}
    }
}
```

失焦时显示 1.23，聚焦后显示可往返恢复原值的完整十进制文本。显示舍入不把正式值改成 1.23；聚焦和失焦也不产生数值变化事件。整数不能配置 decimals。

## 单步与长按

### 程序调用用户步进

```rust
WidgetryNumberInputHeadless::<i32>::step(&mut commands, root, true);
WidgetryNumberInputHeadless::<i32>::step(&mut commands, root, false);
```

true 增加，false 减少。这个入口模拟用户单步，遵守 ReadOnly、Disabled 和 IME 守卫，和不受这些输入限制的 `set_value` 不同。

合法草稿作为步进起点，非法草稿使用旧正式值。运算按原类型执行，限制到范围与类型边界，不发生整数回绕。`0..10, step=3` 可以得到 `0→3→6→9→10`，反向为 `10→7→4→1→0`。

### 使用默认加减按钮

默认左右按钮按下立即执行一次，持续按住约 400ms 后每约 100ms 重复；释放、禁用、取消或窗口失焦终止重复。恢复可用后需重新按下，不续接旧计时。键盘上下键采用同一步进路径，输入法组合期间方向键留给 IME。

鼠标滚轮不改变数值，也不截断外层 ScrollArea 的正常滚动。

## 只读与禁用

### 创建可复制的只读数值

```rust
bsn! {
    @WidgetryNumberInput::<f64> {
        @value: {Some(42.125)},
        @read_only: true,
        @decimals: {Some(2)}
    }
}
```

只读保留选择、复制和完整精度的聚焦展示，但不允许文字修改和步进。Disabled 则阻止用户编辑相关输入，并放弃旧草稿及待处理 Edit/Paste/IME，重新启用后不会补交。

两种状态都允许合法程序化 Setter。只读模式在构造时确定。

## 自定义编辑区与触发器

### 绑定自己的 TextField

Headless 根通过 `WidgetryNumberInputEditor { field }` 绑定其后代 TextField Headless 根。字段可以采用自定义装饰，但必须单行、无 max_length，且只读模式匹配 NumberInput。

```rust
use bevy_widgetry::text_field::WidgetryTextField;

let custom = bsn! {
    #Number @WidgetryNumberInputHeadless::<i32> { @value: {Some(5)} }
    WidgetryNumberInputEditor { field: #Field }
    Children [
        #Field @WidgetryTextField
        Node { width: px(180) }
    ]
};
```

安装 `WidgetryNumberInputHeadlessPlugin` 提供数值与编辑行为；使用上例默认 TextField 时还需它的默认 Plugin。外部 TextField 的边框、尺寸和颜色仍由调用方管理。

### 将按钮关联到数值根

任意提供 Activate 的触发器可组合 `WidgetryNumberInputStep { root, increase }`；它不创建默认按钮，也可位于数值根之外。需要持续按住时再组合 `WidgetryNumberInputRepeat`，触发器负责首步，Repeat 只负责后续重复。
