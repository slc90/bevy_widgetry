# 移除 Segmented Group / Option 的无障碍角色

Segmented 的组与选项交互由私有方向状态、原生 `Checked` / `Checkable`、Root Focus、Pointer 输入和互斥选择机制实现。`RadioGroup` / `RadioButton` 的 AccessKit 角色只是当前 Scene 创建时额外挂载的描述，并不参与严格单选提交。

## 源码调整

`crates/segmented/src/group.rs` 中 `WidgetrySegmentedHeadless::scene` 在初始化方向状态时还额外创建 `AccessibilityNode(Role::RadioGroup)`。移除无障碍模板，保留用于布局和键盘方向映射的 `OrientationState(props.orientation)`。

`crates/segmented/src/option.rs` 中 `WidgetrySegmentedOptionHeadless::scene` 只为挂载 `AccessibilityNode(Role::RadioButton)` 而存在。移除模板；若 `SceneComponent` 宏仍需要 Scene 入口，按实际类型与编译约束保留最小空 Scene，而不是新建包装 Component。保留 `Checkable`、`Hovered`、Pointer Pressed、Owner 校验和默认 Style 所需部件。

从 `crates/segmented/Cargo.toml` 移除 `accesskit.workspace = true`；同步修改 `docs/crates/segmented.md` 中“标准 Accessibility Role”的描述，其余方向键、Focus、Checked、选项限制与组合边界保持原样。

## 行为和验证

用 Segmented 现有行为测试检查严格单选初始化、多个 Checked 冲突错误、合法选择及通知、增加/删除选项、Disabled 拦截、Pointer 点击嵌套内容、水平左右与垂直上下方向键、边界不循环、Tab 不触发跨控件导航。必要时通过 BRP 按真实输入确认选中浮块与 Focus 样式，不能只修改 Checked 后观察外观。

不引入 Bevy 原生 `RadioGroup` / `RadioButton` 作为替代，因为当前 Segmented 刻意保留独立输入和非循环导航规则。它与按钮类方案仅执行顺序相邻。
