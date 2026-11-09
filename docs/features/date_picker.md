# DatePicker

DatePicker 提供单日期选择、月份浏览、日期范围和可选导航。选中日期与正在显示的月份相互独立：浏览其他月份不会自动改变选择。

```rust
use bevy::prelude::*;
use bevy_widgetry::date_picker::*;
use time::macros::date;

app.add_plugins(WidgetryDatePickerPlugin);
```

示例使用 `date!` 宏，在应用依赖中启用：

```toml
[dependencies]
time = { version = "0.3", features = ["macros"] }
```

## 日期选择

### 创建带初值和范围的日历

```rust
bsn! {
    @WidgetryDatePicker {
        @selected_date: {Some(date!(2026-11-09))},
        @min_date: {Some(date!(2026-11-01))},
        @max_date: {Some(date!(2026-11-30))},
        @week_start: Weekday::Monday
    }
    on(selection_changed)
}

fn selection_changed(event: On<WidgetryDatePickerSelectionChanged>) {
    info!(root = ?event.entity, date = ?event.selected_date, "日期选择已提交");
}
```

范围为闭区间，限制可以选择的日期，不把浏览月份锁死在这个范围内。范围和 week_start 构造后固定。

### 修改或清空选择

```rust
WidgetryDatePickerHeadless::set_selected_date(
    &mut commands, root, Some(date!(2026-11-15)),
);
WidgetryDatePickerHeadless::set_selected_date(&mut commands, root, None);
```

选中一个不同日期会定位到该日期所属月份；同值设置不会把当前浏览月份强行拉回。清空选择保留当前月份。公开程序化修改不受 Disabled 输入限制。

同步入口 `set_selected_date_in_world` 返回 `Result<bool, DatePickerFailure>`。初始化、同值和失败不产生选择变化事件。

## 月份浏览

### 直接定位或前后翻月

```rust
WidgetryDatePickerHeadless::set_displayed_month(
    &mut commands,
    root,
    DatePickerMonth { year: 2027, month: Month::January },
);
WidgetryDatePickerHeadless::shift_month(&mut commands, root, -1);
```

通过 `displayed_month(world, root)` 查询当前月份，监听 `WidgetryDatePickerDisplayedMonthChanged` 接收已提交变化。选择变化和月份变化是两个事件，不需要从日历文字反推状态。

初始月份优先使用显式 displayed_month，再使用初始选中日期，最后使用当前本地日期。没有初始选中值时不会自动选中“今天”。

## 日期格与今日标识

### 获取当前日期区域的数据

Headless 的 `days(world, root, today)` 根据月份和星期起点产生连续 42 个真实日期，每格包含是否属于本月、是否可选、是否选中以及是否今天。

```rust
let days = WidgetryDatePickerHeadless::days(
    world, root, Some(date!(2026-11-09)),
).map_err(|error| error.into_bevy_error())?;
for day in days {
    info!(date = ?day.date, enabled = day.is_selectable, selected = day.is_selected);
}
```

邻月格仍对应真实日期，不是不可识别的占位数字。显式传入 today 适合测试或自定义展示；默认外观使用共享本地 Today 信息。

## 呈现语言

### 切换月份和星期文字

```rust
world.insert_resource(WidgetryDatePickerLanguage::SimplifiedChinese);
// 切回英文：WidgetryDatePickerLanguage::English
```

语言是应用级呈现配置，切换时不重置选中日期和浏览月份。业务始终处理 Date，而不是解析当前显示的中文或英文标签。

## 自定义日历与导航

### 只使用日期行为

```rust
app.add_plugins(WidgetryDatePickerHeadlessPlugin);

let model = bsn! {
    @WidgetryDatePickerHeadless {
        @selected_date: {Some(date!(2026-11-09))},
        @displayed_month: {Some(DatePickerMonth {
            year: 2026, month: Month::November
        })}
    }
};
```

自定义外观读取 days 与两项状态，自行生成日期卡片、标题和按钮。日期格不能另存一份选择；点击后仍通过公开选择入口提交。

`WidgetryDatePickerNavigation` 提供可组合的日期导航能力，导航候选与日期选择分离。默认外观与外部日历可以复用日期模型，而不需要保留一套隐藏的默认 42 格 UI。
