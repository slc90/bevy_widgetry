# HoldButton

HoldButton 用持续按住代替一次点击，适合需要明显确认过程的操作。默认外观为圆形充能按钮，也可以只使用长按行为，绘制进度条或其他形状。

```rust
use std::time::Duration;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_widgetry::hold_button::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryHoldButtonPlugin);
```

## 长按确认

### 创建确认按钮

```rust
fn confirm_button() -> impl Scene {
    bsn! {
        @WidgetryHoldButton {
            @diameter: 112.0,
            @charge_duration: Duration::from_secs(3),
            @decay_duration: Duration::from_millis(1200),
            @children: {Some(Box::new(bsn_list! {
                Text("确认") WidgetryText
            }) as Box<dyn SceneList>)}
        }
        on(confirmed)
    }
}

fn confirmed(event: On<Activate>) {
    info!(button = ?event.entity, "长按确认完成");
}
```

默认直径为 112 像素，充能 3 秒、衰减 1.2 秒。直径必须为有限正数。时长使用 `Duration`；零充能时长在有效按下后的计时更新中立即成功，零衰减时长立即清零。

### 区分按住、松开与成功

有效按住逐渐累积进度，未完成就松开会进入衰减，达到阈值才发出 `Activate`。完成确认后保持本次尝试的成功状态，不因继续按住而逐帧重复激活；下一次有效尝试再开始新的确认。

调用方把不可逆业务动作放在 `Activate` 的处理函数中，而不是根据动画高度判断成功。失焦、取消和 Disabled 会影响当前交互尝试；长按过程不是后台任务进度。

## 默认内容与外观

### 修改内容排列

```rust
bsn! {
    @WidgetryHoldButton {
        @diameter: 144.0,
        @content: {Some(Box::new(bsn! {
            Node { flex_direction: FlexDirection::Column, row_gap: px(4) }
        }) as Box<dyn Scene>)},
        @children: {Some(Box::new(bsn_list! {
            Text("长按执行") WidgetryText--
            Text("持续三秒") WidgetryText
        }) as Box<dyn SceneList>)}
    }
    on(confirmed)
}
```

默认 Style 管理圆形材质、充能表现、内容阴影与主题颜色。内容和尺寸配置在构造时确定；运行时进度由行为层维护，不通过反复重写 Props 更新。

## 自定义外观

### 只安装长按行为

```rust
app.add_plugins(WidgetryHoldButtonHeadlessPlugin);

let scene = bsn! {
    @WidgetryHoldButtonHeadless {
        @charge_duration: Duration::from_secs(2),
        @decay_duration: Duration::from_millis(500)
    }
    Node { width: px(300), height: px(52) }
    Children [Text("按住两秒") WidgetryText]
    on(confirmed)
};
```

Headless 提供相同的长按输入、计时、只读进度与尝试状态，但不创建默认材质、颜色或内容。自定义 Style 根据行为状态呈现反馈；成功仍统一监听 `Activate`。

默认 Plugin 已安装 Headless 所需行为，两者不需要在同一 App 内重复装配。只使用 Headless 不依赖默认 Shader 或圆形视觉结构。
