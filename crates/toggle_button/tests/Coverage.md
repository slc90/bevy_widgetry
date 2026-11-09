# Toggle Button 测试覆盖模型

本文描述 Toggle Button 自动化测试所保护的重要行为契约及测试职责，不作为具体测试用例清单。

## States

- **选择状态**：Checked / Unchecked，以 Bevy 原生 `Checked` Component 为唯一状态来源。
- **交互状态**：Normal、Hovered、Pressed、Disabled，以及当前 Focus。
- **视觉状态**：当前 Theme、颜色覆盖配置及受管理的子元素前景色。
- **构造状态**：独立 Headless 行为与带默认 Style 的完整控件。

## Stimuli

- BSN Scene 构造、公开同步及排队状态设置 API。
- 真实 Pointer Press、Release、Click、Cancel，以及具有焦点时的 Space / Enter 键盘输入。
- 外部直接添加或移除原生 `Checked` Component。
- 本地及祖先 Disabled 状态变化。
- Theme 切换、颜色覆盖设置与清除、内部样式结构变化。

## Guards

- Disabled 阻止用户触发选择变化，但不阻止合法的程序化状态设置。
- 键盘激活要求有效焦点、对应按键的非重复 Press；Release、重复按键及无关按键不得触发选择变化。
- 单独 Press 或 Cancel 不改变选择状态。
- 无效、已销毁或缺少行为身份的目标必须拒绝；排队请求在实际执行时重新校验。
- 非法颜色配置及受其他 Style Owner 管理的颜色请求必须拒绝，不得部分修改已有配置。

## Transitions

- 用户有效激活使 Checked 与 Unchecked 相互切换。
- 公开 Setter 根据目标值更新选择状态，合法同值请求保持静默。
- Disabled 及解除 Disabled 后，保留当前选择状态并正确恢复交互能力。
- Theme、Hover、Pressed 和 Disabled 变化更新视觉状态，不改变选择结果。
- 原生 `Checked` 变化刷新显示投影，不产生额外业务通知。

## Invariants

- `Checked` 是唯一选择状态来源，不维护与其竞争的内部选择副本。
- 通过控件公开操作产生的 `WidgetryToggleButtonChanged` 在状态提交后发送，Observer 可以读取已提交结果。
- 合法同值操作不发送重复通知；直接修改原生 `Checked` 不合成 Widgetry 业务事件。
- 选中外观在 Pressed 结束后保持，Disabled 的视觉优先级高于 Pressed 和 Hovered。
- 默认样式根据选择状态、交互状态及当前 Theme 解析颜色；局部覆盖优先，清除后恢复当前 Theme。
- 受管理的子元素前景色与控件状态一致，Theme 更新不破坏业务内容。
- Headless 无需默认视觉子树或 Foreground 资源即可独立工作；与默认 Style Plugin 共存时，不重复处理输入或修改独立 Headless 实例的自有颜色。
- 错误传播保留领域分类、原始 source 和 origin；失败请求不修改已有状态，持续样式故障遵循诊断去重及实际修复后的恢复契约。
- Scene 初始化失败不遗留无效根实体；样式结构故障不妨碍其他有效控件继续更新。

## Couplings

- **Checked × Pointer / Keyboard**：原生输入与程序化操作共享选择状态，但遵守各自的通知语义。
- **Disabled × Input / API**：用户输入守卫与程序化修改权限相互独立。
- **Checked × Theme / Interaction**：选择状态、交互状态及颜色覆盖共同决定视觉投影。
- **Headless × Default Style**：独立行为、默认样式及组合安装时的职责隔离。
- **Style Failure × Diagnostics / Recovery**：样式结构损坏、错误传播、持续故障和修复后的显示恢复。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `toggle.rs` | 公开选择 API、真实 Pointer / Keyboard 交互、状态通知、Disabled 继承、Theme、颜色覆盖及内容前景传播。 |
| `headless.rs` | 独立 Headless 构造与输入、状态提交、原生 Checked 变化、程序化操作、延迟校验及 Plugin 共存隔离。 |
| `diagnostics.rs` | 无效操作与颜色配置、托管颜色权限、原始错误身份、真实 Command / Scene 错误传播，以及样式故障聚合和恢复。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/colors.rs` | 私有 ColorState 缺失时的内部错误分类、目标身份及状态保持。 |
