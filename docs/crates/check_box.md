# Check Box

## 功能概述

提供二态 `WidgetryCheckBox` 和三态 `WidgetryTriStateCheckbox`，用于二值选择与部分选中等场景。两者都可接收调用方的 BSN Children。

## 当前设计思路

二态 Checkbox 复用 Bevy 官方 `Checked` State。三态 Checkbox 由 Widgetry 自有 `WidgetryCheckState` 管理状态，并提供专用的 Setter、Cycle 操作及状态通知。两套状态来源不强行统一。

二态行为可独立安装 `WidgetryCheckBoxHeadlessPlugin`，在 BSN 中组合 `WidgetryCheckBoxHeadless` 与自有 Node/Children。Headless 提供官方 Checkbox、Node、Hovered、Pointer Pressed 协调及默认 `TabIndex(-1)`，保持官方 Checkbox 的无障碍语义，不创建默认方框、勾选 SVG、颜色状态或前景输出，也不安装 Icon Plugin。Facade 的 `check_box` 模块导出这些入口。

```rust
bsn! {
    @WidgetryCheckBoxHeadless
    Node { width: px(96), height: px(36) }
    Checked
}
```

`WidgetryCheckBoxPlugin` 自动安装二态 HeadlessPlugin，并保留默认二态/三态 Style。默认二态控件在原根组合同一行为，不增加布局层。自更新 Observer 只处理具备 Headless 身份的官方 `ValueChange<bool>` 请求，且只安装一次；不接管其他原生 Checkbox。外部无需默认 Indicator 即可使用输入，即使 App 同时安装默认 Style，也不会接受默认结构检查或被改写颜色。

三态可独立安装 `WidgetryTriStateCheckboxHeadlessPlugin`，以 `@WidgetryTriStateCheckboxHeadless` 组合外部 Node/Children。它拥有原有 Pointer / Space / Enter / ActivateOnPress 输入、Pressed 清理、Commands Set/Cycle 与无障碍同步；不安装默认 Indicator 或 Icon Plugin。默认 `WidgetryTriStateCheckbox` 在原根组合该行为，完整 `WidgetryCheckBoxPlugin` 自动确保两种 HeadlessPlugin，默认颜色与布局仍由默认 Style 负责。

## 主要功能

- 二态 Checked / Unchecked 显示与输入。
- 三态 Unchecked、Checked、Indeterminate 显示与指示 Icon。
- 三态支持 Pointer Activation，以及 Focus 后 Space / Enter 切换。
- `set_state` 可指定三态值；`cycle_state` 按 Unchecked → Checked → Indeterminate → Unchecked 顺序切换。
- 状态对应的 Accessibility、Theme 与内容配色同步。

## 行为与状态契约

三态默认 Unchecked；其真实状态改变时先提交再发送 `ValueChange`，同值不通知。`Commands` 操作在真正执行时才更新。Disabled 阻止用户操作，但不阻止合约允许的程序化设置。二态仍遵循官方 `Checked` 的使用方式。

三态只读查询 `WidgetryCheckState`，通过 `WidgetryTriStateCheckboxHeadless::set_state` / `cycle_state` 排队操作；执行时以 Headless 身份及状态存在性校验，不依赖默认 Style。原默认控件类型不保留旧方法兼容层。状态是受控不可变组件，不通过运行期直接替换来代替这些 API，也不插入官方 Checked 表达任一种三态值。初始化不发送通知；`ValueChange<WidgetryCheckState>` 保持提交后语义，与二态的 bool 输入请求不同。

三态 CheckBox 角色和 False / True / Mixed 的 toggled 同步由 HeadlessPlugin 独立执行，不依赖默认 Style 的 Colors 注册。已有状态后挂载 Headless 或新增 AccessibilityNode 时，同样初始化辅助状态，不额外发送选择通知。Gallery 的自有几何符号演示三态循环、指定状态、键盘及禁用期间程序设置。

二态的 `ValueChange<bool>` 保持官方输入请求语义，不是 Widgetry 提交后通知；不承诺该请求的多个 Observer 执行顺序。Checked 是唯一状态，调用方可以直接用 BSN / World / Commands 增删；这些直接操作不产生输入请求。Disabled 阻止用户输入，不阻止原生 Checked 写入。Gallery 的自定义开关通过公开 Headless、原生 Checked 和自有 Track/Thumb 展示这些差异。

## 边界情况与错误处理

无效的三态 Root 或内部必要指示结构缺失属于错误，不应无声忽略。Theme 切换及 Indicator 加载不应改变逻辑选中状态；状态与视觉投影需要保持一致。

颜色 World API 返回 `CheckBoxFailure = WidgetryFailure<CheckBoxError>`，区分非法目标、必需颜色状态缺失和托管 Owner。Core 颜色失败保留原 CoreError、source、origin、首次回溯及 Root/状态/交互状态/字段；非法输入保留原配置。三态排队状态操作、颜色 Command 和 Scene Template 显式转为 `Severity::Error`，保留原诊断；合法同值及正常输入 Guard 保持静默。

mark/colors 各自按真实部件 Entity 和缺失原因维护统一持续异常状态，保留 Root、阶段和当前视觉状态。Query 与 SystemParam 校验失败保留原始 source。多个损坏 Root 的失败完整汇总到 Handler，健康 Root 继续更新；持续同一异常不重新创建 ID 或捕栈，修复后重试当前投影并记录一次恢复 INFO，再次损坏生成新诊断。纯传播不记录 handled，删除 Entity 不视为恢复。正常 Icon 资产等待和无目标事件不构造错误。

## 已知限制

- 三态的程序化 API 与通知不用于替代 Bevy 官方二态 Component。
- SVG Indicator 依赖 Asset 和 Icon 加载就绪；构造与内容绘制存在正常 Asset 生命周期。

