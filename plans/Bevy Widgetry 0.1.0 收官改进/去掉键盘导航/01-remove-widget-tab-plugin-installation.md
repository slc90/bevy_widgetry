# 停止 Widget 插件自动安装 Tab 导航

多个控件在注册自身功能时，顺手安装了 Bevy 的 `TabNavigationPlugin`。这种装配方式使宿主即使只需要鼠标点击和焦点控件内部键盘操作，也会得到跨控件的 Tab 导航。此增量只解决一个问题：**Widget 插件不再擅自开启通用焦点遍历**。

其可交付结果是各相关插件在单独安装时不会引入 `TabNavigationPlugin`，且各控件原有 Pointer、焦点和内部键盘行为仍可使用。它不负责移除 TextField 自己的 Tab 跳转，也不负责改写 Window 自己的导航逻辑；这些是后续独立增量。因此，不能把本增量的局部结果描述成整个组合应用已经不存在 Tab 导航。

## 装配边界

目前明确需要移除自动安装代码的七处入口是：

| Crate | 入口 | 这次保留的能力 |
|---|---|---|
| Collapsible | `crates/collapsible/src/lib.rs` | Header / Toggle 的 Pointer 激活，获得焦点后的 Enter / Space 激活 |
| Slider | `crates/slider/src/lib.rs` | 拖动与轨道点击；获得焦点后的方向键调值 |
| ScrollArea | `crates/scroll_area/src/headless.rs` | Wheel、Thumb Drag、程序化 ScrollPosition，以及区域内部的键盘滚动 |
| Table | `crates/table/src/registration.rs` | Pointer 聚焦、Focused Cell、内部方向移动、编辑与复制 |
| Tabs | `crates/tabs/src/lib.rs` | 点击切页，已聚焦标签栏内部的方向键选择 |
| Segmented | `crates/segmented/src/lib.rs` | Pointer 选项操作和组内部方向键选择 |
| RadioGroup | `crates/radio_group/src/lib.rs` | Pointer 操作和组内部方向键选择 |

只移除类似如下的自动装配分支及不再使用的 import，不取消控件的其他插件依赖，也不移除 `InputFocusSystems` 等仍用于正确排程的约束：

```rust
if !app.is_plugin_added::<TabNavigationPlugin>() {
    app.add_plugins(TabNavigationPlugin);
}
```

`TabNavigationPlugin` 与 `FocusedInput` 不是同一个东西。后者属于焦点目标的输入分发，仍是 Slider、Table、Tabs、Segmented 等控件处理按键的基础。ScrollArea 的 `Keyboard Scrolling` 也是当前控件内部滚动，不应被误判为跨控件焦点导航。

## 原有交互测试如何迁移

需要将“由于注册这个 Widget，Tab 得以移动到下一个控件”的测试改成两类可观察契约：

- **装配契约**：单独安装相关 Widget Plugin 后，`TabNavigationPlugin` 不会被自动注册；其他插件与必要的输入焦点 Resource 仍可用。必要时在基础集成测试中覆盖几个不同装配顺序，避免某个 Widget 间接重启导航。
- **保留的控件能力**：先用 Pointer 或程序化 API 使当前控件获得焦点，再发送其真正支持的内部按键，验证业务状态、焦点和通知。例如 Collapsible 的 Enter / Space 激活、Slider 方向键调值、Tabs / RadioGroup / Segmented 内部选择、Table 的焦点单元格移动及 ScrollArea 的键盘滚动。

受影响的测试重点包括 `crates/collapsible/tests/input.rs`、`crates/tabs/tests/keyboard.rs`、`crates/tabs/tests/text_input.rs`、`crates/radio_group/tests/widgetry_radio_group.rs`、`crates/table/tests/interaction.rs`，以及对应 Slider、Segmented、ScrollArea 的输入测试。原本只验证 `TabGroup` 的路径不再是应受保护的功能；但不能顺便删除同一测试文件里的焦点丢失、可见性、Disabled 或内部按键断言。

在涉及 GUI 可观察行为的控件上，使用 BRP Pointer 点击与相应 Keyboard 输入验证；Pointer 控件既要能点击，也要能获得当前控件正确的输入焦点。测试记录与 `tests/COVERAGE.md` 随受影响的测试同步更新。

## 文档与完成条件

`docs/crates/tabs.md`、`segmented.md`、`radio_group.md`、`scroll_area.md`、`table.md`、`slider.md`、`collapsible.md` 等文档中，凡把自动安装 Tab Navigation 作为控件装配保证的表述应同步调整。保留有关内部方向键、键盘滚动和程序化操作的真实契约；不提前宣称 TextField 和 Window 已被修改。

完成时，七处自动安装分支及孤立引用都已移除；各 crate 单独装配时不引入通用 Tab Navigation；受影响控件的 Pointer 和内部键盘用例仍然通过。不得通过全局禁用键盘输入、移除 `FocusedInput`、移除焦点视觉提示等方式达到这个结果。

**与后续的关系**：本增量收回 Widget 插件对全局导航的隐式所有权。下一增量处理 TextField 直接发起的编辑器 Tab 跳转；之后 Window 还需删除其独立的 Tab 遍历路径。
