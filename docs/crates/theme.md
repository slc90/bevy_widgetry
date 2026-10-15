# Theme

## 功能概述

提供 Widgetry 的四种预定义配色模式，以及各 Widget / 组合部件的完整颜色数据与全局主题切换能力。

## 当前设计思路

Theme 是统一的颜色数据和切换通知源，不直接承担所有 Widget 的样式更新。每个消费者使用当前 `WidgetryThemeMode` 和自身状态选择合适的颜色输出。

## 主要功能

- Dark、Light、Pink Dream、Kamuri Violet 四套固定 Theme。
- 为 Text、Icon、Button、CheckBox、ComboBox、Tooltip、ScrollArea、ListView、Tree、Table、Window、Dialog、Waveform 等提供 Colors Type。
- 提供 World 和 Commands 主题切换入口，以及 `WidgetryThemeChanged` 通知。
- Waveform 配色包含起始 Channel Palette 与 Disabled Palette。

## 行为与状态契约

默认 Dark。主题 Setter 必须先提交新的 `WidgetryThemeMode`，再发送变化通知；合法同值设置不发送通知。Commands Queue 仅在实际执行时生效；Style Consumer 根据通知或其自身生命周期刷新对应输出。

## 边界情况与错误处理

无效目标或缺少必要 Theme Resource 的操作按公开契约返回 / 传播错误。消费者未安装或尚未构造的 UI 不由 Theme Plugin 自动补建；Theme 切换不会修改各控件的业务 State。

## 已知限制

- 当前仅提供四套固定 Theme，不包含任意运行时 Theme 编辑器。
- Colors 定义并不直接保证每个未安装的 Widget 会自动更新 UI。
- 本 crate 当前主要通过源码 Unit Test 验证，不另建 `tests/COVERAGE.md`。

