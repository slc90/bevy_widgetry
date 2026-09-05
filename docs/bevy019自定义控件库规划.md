# 基于 Bevy 0.19 的自定义控件库开发规划

## 1. 项目定位

目标是基于 **Bevy 0.19** 开发一套面向 **桌面 / 工具型 UI** 的自定义控件库。

这个项目同时也是一个学习过程，因此整体原则不是一开始就追求完整、通用、抽象得很漂亮，而是：

1. 先理解 Bevy 官方已有能力；
2. 能组合官方能力就尽量组合，不重复造轮子；
3. 只有官方没有的控件或语义，才自己设计 headless 层；
4. 每次只解决当前阶段真正需要的问题；
5. 有了多个真实控件以后，再从实际重复中抽象和重构；
6. 控件库只负责 **控件自身**，不扩展成桌面应用框架；
7. 不追求一次性补齐大量控件，优先实现常用控件，之后按真实需求继续增加。

## 2. 技术边界与长期原则

### 2.1 基于官方 Headless Widgets

控件行为层优先建立在：

- `bevy_ui`
- `bevy_ui_widgets`
- 官方已有的交互状态、事件、EditableText 等能力

之上。

例如：

- Button：扩展官方 `Button`，不重写它；
- TextField：扩展官方 `EditableText`，不重新实现一套文本编辑系统；
- ComboBox：官方没有时，再通过已有 primitives 组合出新的 headless 控件。

对于 keyboard、focus、accessibility 等能力，不为“完整性”提前实现；只有当某个控件本身确实需要，或者真实应用场景提出需求时，再基于官方能力补充。

### 2.2 Feathers 作为参考，而不是依赖目标

`bevy_feathers` 主要作为学习和参考对象：

- 研究它如何把 headless widget 变成 styled widget；
- 研究它如何组织样式、视觉状态和控件结构；
- 参考官方对桌面 / 编辑器 UI 的设计方向。

但自己的控件库不直接定位成 Feathers 的扩展，也不需要完全遵循它的视觉体系。

### 2.3 不进入底层 GPU 控件绘制

现阶段优先通过 Bevy UI、已有 headless widgets、普通 UI 节点和组合方式实现控件。

不把目标扩大成：

- 自定义 UI renderer；
- 自定义 GPU pipeline；
- 自己从底层绘制所有控件。

SVG Icon 也是先考虑复用现有方案，而不是一开始就自己写矢量渲染器。

### 2.4 Theme 只负责颜色

Theme 不做成通用样式系统，只服务于自己的控件库。

暂不考虑：

- CSS cascade；
- selector；
- style inheritance；
- skin 系统；
- 任意动态 token 注册；
- 第三方控件适配体系。

Theme 的目标只有：

- 使用视觉语义颜色；
- 支持 Dark / Light 等配色；
- 支持运行时切换；
- Button、ComboBox、TextField 等控件自动跟随 Theme 更新颜色。

### 2.5 控件库只负责控件自身

库可以负责：

- 控件自己的交互；
- 控件内部 Entity 的协作；
- popup / child entity 协作；
- 控件确实需要的 keyboard / focus 行为；
- style；
- theme；
- 测试。

不负责应用层的：

- 全局 Tab 导航；
- 应用级快捷键；
- 不同面板之间的 focus 跳转；
- 业务命令系统；
- 整个桌面应用的导航逻辑。

---

## 3. 阶段规划

### Stage 1：理解并扩展 Headless Button

#### 目标 1：学习官方 Headless Button 原理

以 `bevy_ui_widgets::Button` 为例，理解：

- `Button` 组件；
- `ButtonPlugin`；
- pointer input；
- keyboard activation；
- `Pressed`；
- `InteractionDisabled`；
- `Activate`；
- 官方 focus / accessibility 如何与 Button 协作；
- ECS component + observer/system + event 的组织方式。

重点不是“会用 Button”，而是理解官方为什么这样组织一个 headless widget。

#### 目标 2：扩展官方 Button，增加 Long Press

不重新实现 Button，而是在官方 Button 基础上增加一个新的 headless 行为：

- Long Press；
- 必要的内部状态；
- 长按触发事件；
- release / cancel / drag end 时取消；
- disabled 时不触发。

这一阶段完全不涉及 Style。

#### 目标 3：学习 Headless Widget 的测试

重点验证：

```text
输入
  ↓
状态变化
  ↓
事件
```

例如：

- pointer down → `Pressed`；
- 持续按住达到阈值 → `LongPress`；
- 提前 release → 不触发；
- cancel / drag end → 取消；
- disabled → 不触发。

---

### Stage 2：从 Headless Button 到 Styled Button

#### 目标 1：学习 Feathers 的 Button Style 实现

研究：

- Feathers 如何包装官方 headless Button；
- Button entity / child entity 结构；
- 如何感知 hover / pressed / disabled 等控件状态；
- 如何把控件状态映射到 Bevy UI 的视觉组件；
- 文本、边框、背景等 child entity 如何更新；
- 了解 Feathers 的 `FocusIndicator` 与官方 focus 机制如何协作，但本阶段不实现 Focus 视觉状态。

这里对 Focus 的研究属于理解 Feathers / Bevy 的现有设计，不代表自己的 `StyledButton` 第一版要实现 Focus。

#### 目标 2：实现自己的 Styled Button

基于 Stage 1 的 Button，实现第一版常用视觉状态：

- Normal；
- Hover；
- Pressed；
- Disabled。

如果后续真实使用需要 Focused 状态，再继续补充。

这一阶段：

- 可以直接写固定颜色；
- 不做 Theme；
- 不做 Design Tokens；
- 不做 Button Variant；
- 不追求复杂抽象。

#### 目标 3：学习 Style 测试

分两部分：

##### Style 逻辑测试

验证：

```text
Widget State
    ↓
Style Resolution
    ↓
BackgroundColor / BorderColor / TextColor / Node ...
```

还要测试状态优先级，例如：

- disabled > hover；
- pressed > hover。

##### Visual Regression Test

建立一套可重复使用的 Visual Regression 基础设施，并用 `StyledButton` 第一版实际支持的四种视觉状态完成验证：

- Normal；
- Hover；
- Pressed；
- Disabled。

当前已经跑通：

```text
StyledButton scene
→ Bevy UI layout
→ offscreen GPU render
→ RenderTarget::Image
→ GPU readback
→ CPU pixels
→ baseline PNG
→ 全图比较
```

同时建立独立的 baseline 生成流程，避免普通测试自动覆盖金标准。

这一阶段的目标已经不只是“理解 screenshot 测试概念”，而是把后续控件也可以继续复用的 Style Logic Test + Visual Regression 测试方法真正跑通。

---

### Stage 3：从零设计 Select-only ComboBox

这是第一个官方没有直接提供的高层控件。

原则是：**控件语义自己设计，底层能力尽量组合官方已有 primitives。**

当前第一版实际复用 / 使用的主要能力：

- `Button`；
- `ListBox`；
- `ListItem`；
- `Selected`；
- `InteractionDisabled`；
- `Visibility`；
- `ChildOf` / `Children`；
- `Activate`；
- `ValueChange<T>`；
- `Pointer<Click>`。

`Popover` 暂不放进 Headless ComboBox，等 Styled ComboBox 需要实际 popup 布局与定位时再引入。

#### 目标 1：设计 Headless ComboBox

第一版限定为 **Select-only ComboBox**，不做 Editable ComboBox。

第一版只处理鼠标交互和必要的控件状态：

- Field Activate → 打开 / 关闭 popup；
- 选择 option → 更新 `Selected`；
- 选择 option 后关闭 popup；
- 对外发出 `ValueChange<usize>`；
- 点击 ComboBox 外部 → 关闭 popup；
- disabled → 阻止用户交互；
- 已打开的 ComboBox 被 disabled → popup 立即关闭；
- 程序可以通过独立的 `SetComboBoxSelected` 路径修改 selection；
- programmatic update 不受 disabled 限制，也不产生用户 `ValueChange`；
- 维护多 Entity 控件的 ownership invariant，避免不同 ComboBox 之间串状态。

第一版明确不做：

- keyboard navigation；
- Arrow key active option；
- Enter 提交；
- Escape 关闭；
- active / highlighted option；
- ComboBox 自己的 focus 状态机；
- popup 关闭后的 focus 恢复；
- 完整 accessibility 行为；
- 动态增加 / 删除 option 的公开 API。

这些以后根据实际需求再决定是否增加，不作为当前 Headless ComboBox 的欠账。

#### 目标 2：Headless ComboBox 测试

目标 1 和目标 2 在实际学习中交叉推进：每设计一块行为，就立即增加相应测试。

当前重点覆盖：

- 构造后 Root / Field / Popup / Option hierarchy 正确；
- 初始 `Selected` 正确；
- Field `Activate` → popup open；
- 再次 `Activate` → popup close；
- ListBox `ValueChange<Entity>` → selection 更新；
- 新 option 获得 `Selected`，旧 option 移除 `Selected`；
- 选择后 popup 关闭；
- 用户选择后对外发 `ValueChange<usize>`；
- 点击 ComboBox 外部 → popup close；
- 点击 ComboBox root / descendant → 不被 outside-click 逻辑误关；
- disabled → Field Activate 无效；
- disabled → ListBox 用户 selection 无效；
- disabled 时不改变 selection、不发 `ValueChange`；
- open 状态下增加 `InteractionDisabled` → popup 立即关闭；
- disabled 状态下 programmatic selection 仍然有效；
- `SetComboBoxSelected(Some(valid))` → 修改 selection；
- `SetComboBoxSelected(None)` → 清空 selection；
- `SetComboBoxSelected(Some(invalid))` → 忽略并保持原 selection；
- programmatic selection 不产生 `ValueChange<usize>`；
- Popup A + Option B 这种跨 ComboBox 错误组合 → 整个事件忽略。

另外在 `tests/combo_box.rs` 增加最小 integration test：

```text
ComboBox A 已打开
        ↓
用户点击 ComboBox B 的 Field
        ↓
A Popup → Hidden
B Popup → Visible
```

单元测试尽量直接从自己的语义边界开始：

```text
Activate
ValueChange<Entity>
SetComboBoxSelected
```

集成测试再真正走：

```text
Pointer
→ ButtonPlugin
→ Activate
→ ComboBoxPlugin
```

从而避免在单元测试中重复测试 Bevy 官方 widget 已经负责的内部行为。

#### 目标 3：实现 ComboBox Style

在已经完成的 Headless ComboBox 上增加实际视觉结构。

包括：

- ComboBox field；
- popup；
- option；
- 文本；
- 下拉提示图标；
- normal / hover / pressed / open / disabled；
- selected option 的视觉状态；
- popup 实际布局与定位。

这一阶段再研究并引入 `Popover` 或其他适合的 popup positioning 方案。

第一版 Style 同样不要求：

- focus visual；
- keyboard active / highlighted visual。

如果后续真实需求需要，再继续扩展。

#### 目标 4：Style / Visual Test

继续使用：

- 状态 → Style Components 的逻辑测试；
- screenshot visual test。

重点覆盖第一版实际支持的状态，而不是为了完整状态表增加当前没有的行为。

这一步也用来验证 Stage 2 的 Style 方法是否真的可以复用。

---

### Stage 4：学习官方抽象，并重构自己的控件

这一阶段不新增控件。

#### 目标 1：研究 Bevy 官方已有的跨控件抽象

横向比较：

- Button；
- Checkbox；
- RadioButton；
- Slider；
- ListBox；
- MenuButton 等。

重点观察：

- 哪些状态被复用；
- 哪些事件被复用；
- 哪些行为通过组件组合；
- 哪些概念被放进 `bevy_ui`；
- 哪些放进 `bevy_ui_widgets`；
- Plugin / Observer 怎么组织；
- 官方在哪里选择了“不抽象”。

#### 目标 2：比较自己的 Button 与 ComboBox

检查真实重复：

- 状态；
- 事件；
- Observer；
- Style 状态解析；
- Entity 结构；
- Plugin 注册；
- 测试辅助代码。

重构原则：

1. 官方已有表达方式 → 优先沿用；
2. 只有真实重复出现以后才考虑自己的抽象；
3. 不为了“看起来统一”而提前制造复杂泛型或 trait。

---

### Stage 5：Color Theme

Theme 只负责颜色。

#### 目标

建立一套自己的 `ThemeColors`，采用 **视觉语义颜色**，而不是按控件分别定义颜色。

例如：

```text
window_bg
panel_bg
surface
control_bg
control_hovered
control_pressed
control_disabled
border
border_focused
text
text_disabled
accent
selection
```

具体字段到实现阶段再根据实际控件调整；没有真实使用场景的颜色 token 不提前加入。

#### Theme 要求

- 支持 Dark Theme；
- 支持 Light Theme；
- 支持运行时切换；
- Button / ComboBox 自动更新；
- 后续加入的控件按需接入；
- Theme 不改变 padding、radius、尺寸、entity 结构等。

#### 不做

- CSS cascade；
- selector；
- inheritance；
- skin；
- 通用 style engine；
- 任意第三方主题系统。

---

### Stage 6：SVG Icon 支持

桌面控件大量依赖图标，而 PNG 在缩放时容易失真，所以需要建立 SVG Icon 能力。

#### 目标

支持：

- SVG asset；
- 作为 Bevy UI 节点 / child 使用；
- 常见尺寸下清晰显示；
- 保持宽高比；
- 单色 tint；
- 跟随 Theme runtime 切换颜色。

#### 实现原则

这一阶段开始时再调研并试验方案，例如：

- Bevy 生态现有 SVG 库；
- rasterize-to-image；
- mesh / tessellation；
- 其他可用方案。

不提前绑定具体 crate。

通过最小 prototype 比较：

- UI 集成难度；
- 缩放清晰度；
- runtime tint；
- asset loading；
- Theme 切换；
- 性能；
- 维护成本。

控件库对外最好暴露 `Icon` 概念，而不是把具体 SVG renderer 暴露成公共 API。

---

### Stage 7：Widget Gallery

建立一个长期维护的控件展示 / 试用程序。

#### 定位

Gallery 是：

- 控件查看器；
- 交互 Playground；
- 人工集成验证工具；
- 未来的活文档。

#### 仓库结构

Gallery 与控件库在同一仓库 / workspace 中，但不是 Rust library 的一部分。

建议结构：

```text
repo/
├── Cargo.toml
├── crates/
│   └── my_widgets/
└── apps/
    └── gallery/
```

依赖方向：

```text
gallery → my_widgets
```

不能反向依赖。

#### 第一版只展示

- Button；
- ComboBox；
- Dark / Light Theme runtime switch。

不要在这个阶段为了 Gallery 扩充更多控件。

#### 自定义窗口

Gallery 使用简单的自定义窗口 chrome：

- 去掉系统标题栏；
- 自定义 title bar；
- 拖动窗口；
- 最小化；
- 最大化 / restore；
- 关闭。

SVG Icon 可以首先用于这些窗口按钮。

暂时不进入复杂平台窗口定制。

#### 测试

Gallery 不设置正式自动测试目标。

它主要用于人工集成验证。

只要求：

- 能编译；
- 能启动；
- 基础功能可正常操作。

---

### Stage 8：TextField

TextField 继续遵循“官方已有能力优先扩展”的原则。

#### 目标 1：研究官方 EditableText

理解：

- text buffer；
- cursor；
- selection；
- clipboard；
- IME；
- `TextEdit`；
- `TextEditChange`；
- EditableText 自身依赖的 focus / 输入机制；
- 编辑期状态与 application value 的区别。

这里研究 focus 是因为文本输入本身确实依赖它，不代表控件库要扩展成应用级 focus / navigation 系统。

#### 目标 2：只扩展真正需要的 Headless 行为

不重新实现自己的 Headless TextField。

基于官方 `EditableText` 组合 / 扩展需要的功能。

第一版限定为：

- 单行普通文本输入。

暂不做：

- password；
- number input；
- search box；
- validation；
- multiline；
- 复杂 formatter。

具体缺什么 headless 行为，在研究官方实现后再决定，不提前假设。

#### 目标 3：测试新增行为

官方已有能力主要是理解和验证；自己新增的行为需要重点测试。

#### 目标 4：实现 Style

处理第一版实际需要的：

- Normal；
- Hover；
- Focused；
- Disabled；
- text；
- cursor；
- selection highlight。

这里 `Focused` 属于文本输入本身需要的交互状态，与应用级 focus 导航是两回事。

#### 目标 5：Style / Visual Test

继续沿用前面建立的测试方法。

#### 目标 6：接入 Theme 和 Gallery

- Dark / Light runtime Theme；
- 加入 Widget Gallery。

---

### Stage 9+：按需持续扩充控件

Stage 8 之后不提前决定完整控件清单，也不追求把常见 UI 库里的控件一次性补齐。

原则是：

```text
真实项目需要某个控件
        ↓
先研究 Bevy 官方现有能力
        ↓
能组合 / 扩展就不重写
        ↓
确实缺少时再实现
        ↓
补测试、Style、Theme、Gallery
```

未来可能遇到：

- NumberInput / SpinBox；
- TreeView；
- TabView；
- Menu；
- Toolbar；
- Property Editor；
- 其他桌面工具常用控件。

这些只是可能的需求，不是预先排好的 TODO，也不设置固定实现顺序。

#### 新控件的默认开发流程

```text
研究官方已有能力
        ↓
能组合 / 扩展就不重写
        ↓
官方没有的才设计新的 headless 控件
        ↓
Headless Test
        ↓
Style
        ↓
Style / Visual Test
        ↓
接入 Theme
        ↓
加入 Gallery
```

每增加若干真实控件后，如果出现实际重复，再进行小规模抽象和重构。

---

## 4. 当前规划的整体学习路径

```text
Stage 1
官方 Headless Button
→ 理解行为模型
→ Long Press 扩展
→ 行为测试

Stage 2
Headless Button
→ 学习 Feathers Style
→ Styled Button
→ Style / Visual Test

Stage 3
官方没有的 ComboBox
→ 组合已有 primitives
→ Headless + Test 交叉推进
→ 鼠标交互 / selection / disabled / programmatic update
→ Style
→ Visual Test

Stage 4
横向研究官方抽象
→ 比较自己的 Button / ComboBox
→ 重构真实重复

Stage 5
Color Theme
→ 语义颜色
→ Dark / Light
→ Runtime Switch

Stage 6
SVG Icon
→ 调研方案
→ Icon 能力
→ Theme Tint

Stage 7
Widget Gallery
→ 独立应用
→ 自定义窗口
→ Button + ComboBox

Stage 8
EditableText
→ 扩展 TextField
→ Test
→ Style
→ Theme
→ Gallery

Stage 9+
真实项目缺什么
→ 再按需增加什么
```

---

## 5. 目前明确不做的事情

为了控制范围，目前明确不把项目做成：

- 通用 GUI 框架；
- 桌面应用框架；
- CSS / Web 风格样式系统；
- 通用 Theme Engine；
- 自定义 UI Renderer；
- GPU 矢量控件框架；
- 全局 focus / Tab 导航框架；
- 全局快捷键 / 应用导航框架；
- 跨平台深度窗口系统；
- 一开始就覆盖大量控件；
- 为了“功能完整”提前实现暂时没有真实需求的 keyboard / focus / accessibility 行为。

当前更重要的是：

> 通过少量真正会使用的代表性控件，把 Bevy 0.19 的 headless widget、style、theme、test、icon、gallery 这条完整链路真正学懂并跑通；之后再根据实际项目需求持续扩充。
