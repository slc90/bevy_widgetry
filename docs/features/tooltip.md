# Tooltip

Tooltip 在指针停留时展示辅助说明，支持任意 BSN 富内容、自动定位，以及独立的自定义展示层。一个 App 同时只有一个活动提示。

```rust
use bevy::prelude::*;
use bevy_widgetry::tooltip::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryTooltipPlugin);
```

## 悬停提示

### 为控件添加说明

把 `WidgetryTooltip` 组合到希望作为提示锚点的实体上，通过 `TooltipContentFactory` 提供每次展示时的内容。

```rust
use bevy_widgetry::button::WidgetryButton;

bsn! {
    @WidgetryButton {
        @children: {Some(Box::new(bsn_list! { Text("保存") WidgetryText }) as Box<dyn SceneList>)},
    }
    @WidgetryTooltip {
        @content: {
            TooltipContentFactory::new(|| {
                bsn_list! { Text("保存当前修改") WidgetryText }
            })
        },
    }
}
```

提示会从实际命中的实体向祖先寻找锚点，因此按钮内部文字和装饰不需要分别重复声明 Tooltip。禁用目标仍然可以显示说明，适合解释当前操作为什么不可用。

### 编写多行富内容

```rust
bsn! {
    @WidgetryTooltip {
        @content: {
            TooltipContentFactory::new(|| {
                bsn_list! {
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        max_width: px(280)
                    }
                    Children [
                        Text("导出结果") WidgetryText--
                        Text("保留当前选择，并生成一份独立文件。") WidgetryText
                    ]
                }
            })
        },
    }
    Node { width: px(180), height: px(36) }
    Children [Text("导出") WidgetryText]
}
```

Factory 返回新的 SceneList。持续悬停同一锚点时不会每帧重新构造内容；离开并重新展示才创建新实例。需要长期保存的业务状态应放在应用或锚点中，而不是寄存在临时提示实体里。

## 展示时序与位置

### 理解首次与连续悬停

| 情况 | 默认等待 |
| --- | --- |
| 首次进入，或更换 Owner / Window | 200ms |
| 暖状态内连续切换提示 | 50ms |
| 离开后的暖状态保留窗口 | 300ms |

计时使用真实经过时间。取消、离开、锚点删除、输入源失效都会结束展示。相同锚点持续悬停不会不断重新开始等待。

### 让提示自动选择位置

默认展示层根据锚点与窗口空间，在下、上、右、左候选位置中选择。提示实体及后代忽略 Picking，不会挡住被说明的控件，也不会因为指针进入提示而替换原锚点。

主题变化只更新现有提示的颜色，不重建内容。Default Style 必须提供 Content Factory；缺少内容属于构造错误。

## 自定义提示外观

### 只安装悬停和生命周期能力

```rust
app.add_plugins(WidgetryTooltipHeadlessPlugin);

let anchor = bsn! {
    @WidgetryTooltipHeadless
    Node { width: px(160), height: px(36) }
    Children [Text("自定义提示锚点")]
};
```

Headless 不要求默认 Popover、配色或 Content Factory。它继续维护悬停等待和全局单实例状态，应用监听 `ShowTooltip` 构造自己的提示。

`ShowTooltip` 是展示请求，不是“提示已经出现”的通知。处理排队的构造前，使用该请求的 `is_current(world)` 和 `popup(world)` 确认本次展示仍然有效；创建后通过 `register_popup(world, popup)` 登记新根。

### 管理自定义展示资源

登记的实体必须是本次新建、只承载该提示的根，不能把 Anchor 或应用主面板登记为提示。登记成功后，隐藏和锚点失效时由 Tooltip 清理整个提示子树。

`HideTooltip::presentation()` 用来辨认被关闭的是哪一次展示。需要释放非实体资源时，可以按这一展示身份处理；不要仅凭相同 Anchor 就清除较新的提示。

重复或过期请求不会替换当前展示。已登记的根保持原归属，多余的新根会被清理。这使异步或排队构造不会留下重复 Tooltip。
