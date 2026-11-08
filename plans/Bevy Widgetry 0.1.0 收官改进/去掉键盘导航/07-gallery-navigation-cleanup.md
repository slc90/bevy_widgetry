# 清理 Gallery 的跨控件导航配置与演示

Gallery 是 Widgetry 的应用层消费者。库已经取消自动 Tab 导航后，Gallery 还保留了 Root `TabGroup`、示例输入区的 `TabIndex(0)`，以及切换 Table 示例时动态改写 Tab 顺序的代码。这些配置原本用于展示跨控件遍历，不再符合“Pointer 选择控件，键盘操作当前控件”的交互约定。

本增量把 Gallery 自己维护的导航顺序配置清理掉，同时保留它作为控件展示、交互和 BRP 验证应用的能力。它不是单独的最终测试阶段：Gallery 的 Scene 和 Demo 状态管理会有明确的实际代码改动；全组合回归作为该实施增量的完成条件。

## 删除不再使用的应用层导航配置

| Gallery 位置 | 当前行为 | 预期处理 |
|---|---|---|
| `gallery/src/gallery.rs` | Gallery Root 包含 `TabGroup::default()` | 移除该 Component 与不再需要的 import；Sidebar、内容区和页面生命周期保持原样 |
| `gallery/src/pages/tabs.rs` | 某演示输入框附加 `TabIndex(0)` | 删除纯顺序配置；保留 TextField / Tabs 的 Pointer、编辑与组内方向键演示 |
| `gallery/src/pages/text_field.rs` | `TextFieldSlots` 附加 `TabIndex(0)` | 删除额外 Tab Stop；槽内真正可交互对象仍按自身焦点契约工作 |
| `gallery/src/pages/table.rs` | Basic Demo 的 Table 被赋 `0`，其他为 `-1`；`change_demo()` 修改每个 Table 的 TabIndex | 移除这套动态顺序同步。Demo 切换只改变业务上需要的页面 Display 与状态，不再维护当前页面的 Tab Stop |
| `gallery/src/pages/segmented.rs` | `FocusBefore` / `FocusAfter` Button 附加 `TabIndex(0)` 用于验证组前后遍历 | 删除导航专用配置与测试语义；仅为旧 Tab 演示存在的控制内容可移除，仍有独立 Pointer/业务用途的按钮应保留 |

Table Demo 的处理应一起更新 `change_demo()` 的 `Query<(&DemoTable, &mut TabIndex)>` 与写入循环，不能只删 Scene 上的标记却留下对已不存在组件的查询。其 `DemoPanel` 显隐切换、Table 独立 View 状态、Selection 和复制反馈不应改变。隐藏的 Table 不再通过设置 `TabIndex(-1)` 表达“不可 Tab 聚焦”，因为应用已经不提供这种路径。

### Gallery 仍要能测试内部键盘操作

Gallery 的 TextField、NumberInput、Slider、ScrollArea、ComboBox、Tabs、Segmented、ListView、Table、FileDialog 等页面仍然应能先由 Pointer 选择目标，再针对焦点控件测试：键入与 IME、方向键调值、组内选项选择、列表/表格定位、Esc 取消或 Enter 激活。控件自身的 `InputFocus`、FocusedInput、Hover/Pressed 配色、Disabled、Reveal、Modal 及 Pointer Capture 继续按已有契约验证。

不要仅因为 Navigation 出现在某个 Gallery 页面标题、目录导航、列表业务操作或 FileDialog 目录历史中，就删除对应功能。这些操作可以在没有跨控件键盘焦点导航的情况下正常工作。

## 组合回归与 BRP 验证

这一增量完成时，除了 Gallery Scene 清理，也应通过聚合层的真实组合测试确保前面各增量在一起成立：

- 装配完整 Widgetry / Gallery 时不会自动添加 `TabNavigationPlugin`；已清理的 TextField/Window 不再自行处理跨控件 Tab。
- Pointer 点击 Button、TextField、ListView、Table、Modal 控件时焦点归属正确；Pointer 点击不可聚焦背景仍按既有规则清空焦点；`InputFocus` 与 FocusGained/FocusLost 不重复或遗漏。
- 焦点控件接收的 Tab / Shift+Tab 不导致跨独立 Widget 跳转。Slider、NumberInput、RadioGroup、Segmented、Tabs、ListView、Table、ScrollArea 和 FileDialog 的**内部**方向键及 Enter/Space/Escape 语义不因此丢失。
- TextField 真实 editor、Table 单元格 Editor、ComboBox 搜索框和 FileDialog Path/Filename 的键盘输入目标正确；Modal 与多窗口作用域、IME 输入和焦点恢复仍保持隔离。
- 业务 Disabled、ReadOnly、隐藏 Scene、Popup 开合、虚拟化 Row 与焦点清理路径没有因为删除 Tab 顺序而把旧事件投到失效对象。

可以在 `crates/bevy_widgetry/tests/focus.rs` 的聚合场景补充必要的装配与 Pointer/Keyboard 回归。Gallery 使用 BRP 进行真实 Pointer、Keyboard、Screenshot 及 ECS 查询：包括点击、键入、弹窗、Tab 不切控件、内部方向键和焦点视觉反馈。操作完成后按当前 GUI 验证规则释放 BRP Pointer 并确认已恢复 inactive。BRP 成功提交输入不等于 UI 业务结果正确，应检查目标状态与画面。

应用展示以 `gallery/` 源码为准，不单独建立集中 Gallery 功能文档。只有当应用层清理同时改变了已记录的库契约或 Workspace 结构，才同步 `docs/crates/` 或 `docs/architecture.md` 的相关事实；对应测试覆盖模型随实际新增或调整的 Integration Test 更新。

## 范围与完成条件

Gallery 的页面选择仍靠原有 Sidebar/Button Pointer 操作；不添加键盘导航设置项、可选导航 Feature 或为了 Agent 自动化而准备的焦点遍历路径。BRP 能访问 ECS 和控制 Pointer，不要求每个业务界面仅凭键盘可达。

这是整条拆分链最后一个有实际实施内容的增量。在这里确认：前序七处 Widget 安装点、TextField 的一个安装点、Window 和 TextField 自定义遍历、各控件顺序标记、FileDialog 动态 Tab Stop 与 Gallery 配置都已按目标收敛。保留的 `TabIndex(-1)` 是 Bevy 0.20 Pointer Focus 所需的官方兼容标记，而不是导航遗漏。
