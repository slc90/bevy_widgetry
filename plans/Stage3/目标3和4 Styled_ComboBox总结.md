# Stage 3：Styled ComboBox 与 Style / Visual Test 总结

## 1. 本阶段范围

Stage 3 原规划中：

- 目标 1：设计 Headless ComboBox
- 目标 2：Headless ComboBox 测试
- 目标 3：实现 ComboBox Style
- 目标 4：Style / Visual Test

目标 1 和目标 2 已经完成 Headless ComboBox 的结构、行为和测试。

本阶段继续完成：

```text
目标 3
Styled ComboBox

目标 4
ECS Style 集成测试
+
离屏 Visual Regression Test
```

最终得到：

```text
Headless ComboBox
        ↓
Styled ComboBox
        ↓
ECS 状态 / 样式测试
        ↓
Offscreen Render
        ↓
Baseline Pixel Compare
```

至此 Stage 3 的 ComboBox 从语义层、表现层到测试链路已经完整闭环。

---

## 2. Styled ComboBox 的职责边界

Headless 层继续只负责：

```text
ComboBox 结构
selection
open / closed
disabled
内部事件转换
外部 ValueChange
```

Styled 层负责：

```text
文本
布局
颜色
Dropdown 图标占位
视觉状态解析
selection → Field 文本同步
```

因此没有把文本重新塞进 Headless ComboBox。

Styled 构造函数采用：

```rust
pub fn spawn_styled_combo_box(
    commands: &mut Commands,
    options: Vec<String>,
) -> Entity
```

第一版约束：

```text
options 必须非空
默认 selected = 0
options 在构造后不动态修改
```

Styled 层根据：

```rust
options.len()
```

调用 Headless：

```rust
spawn_headless_combo_box(commands, options.len(), 0)
```

---

## 3. 文本数据属于 Styled 层

用于显示的字符串保存在 root 上：

```rust
#[derive(Component)]
struct ComboBoxOptions(Vec<String>);
```

它的作用是：

```text
Headless index
        ↓
Styled ComboBoxOptions
        ↓
实际显示文本
```

例如：

```text
Selected Option index = 1
        ↓
ComboBoxOptions[1]
        ↓
"Banana"
```

这样继续保持：

```text
Headless
→ 只理解 index

Style
→ 负责 index 对应的表现内容
```

同时 `Vec<String>` 直接 move 进 Component，不额外 clone 一份。

---

## 4. Styled ComboBox 的可见结构

在 Headless hierarchy 上补充视觉组件后，结构大致为：

```text
ComboBox Root
│
├── ComboBoxOptions
│
├── Node
│
├── Field
│   ├── ComboBoxField
│   ├── Button
│   ├── Node
│   ├── BackgroundColor
│   ├── BorderColor
│   │
│   ├── Text(current label)
│   │   └── ComboBoxFieldText
│   │
│   └── Text("v")
│       └── ComboBoxDropdownIcon
│
└── Popup
    ├── ComboBoxPopup
    ├── ListBox
    ├── Visibility
    ├── Node
    ├── BackgroundColor
    ├── BorderColor
    │
    ├── Option 0
    │   ├── ComboBoxOption { index: 0 }
    │   ├── ListItem
    │   ├── Node
    │   ├── BackgroundColor
    │   └── Text(label)
    │
    ├── Option 1
    └── ...
```

Style 层只新增：

```text
纯视觉组件
+
纯视觉 child Entity
```

没有重新创建：

```text
Button
ListBox
ListItem
Selected
Visibility
```

这些仍然属于 Headless 语义结构。

---

## 5. Style-private marker

为了区分 Field 内部的不同 Text，增加了 Style 私有 marker：

```rust
#[derive(Component)]
struct ComboBoxFieldText;

#[derive(Component)]
struct ComboBoxDropdownIcon;
```

这些 marker 不需要保存：

```text
owner: Entity
```

因为 hierarchy 已经表达了所属关系：

```text
marker
→ 这个 Entity 是什么

ChildOf / Children
→ 这个 Entity 属于谁
```

因此没有重复保存 ownership。

---

## 6. Dropdown 图标暂时使用 `"v"`

最初使用：

```rust
Text::new("▼")
```

但离屏 baseline 实际暴露了字体 glyph 问题：

```text
默认字体
→ 缺少 glyph
→ 显示为 □
```

后来尝试额外字体时，也出现字体对该字符字形不合适的问题。

因此 Stage 3 不继续扩展字体资源体系，临时改成：

```rust
Text::new("v")
```

当前原则：

```text
Stage 3
→ 使用 ASCII "v" 作为 Dropdown 占位

Stage 6
→ 专门研究 SVG 图标支持
→ 再替换为真正图标
```

这避免为了一个箭头提前引入字体资源分发问题。

---

## 7. Popup 使用 Absolute 布局

Popup 不能参与 ComboBox root 的正常纵向布局，否则打开后会把周围 UI 推开。

因此 Popup 使用：

```rust
Node {
    position_type: PositionType::Absolute,
    left: Val::Px(0.0),
    top: Val::Percent(100.0),
    width: Val::Percent(100.0),
    flex_direction: FlexDirection::Column,
    ...
}
```

第一版固定向下展开。

暂时不实现：

```text
靠近 viewport 底部时自动向上翻转
popover positioning
collision detection
```

这些不属于当前学习版 ComboBox 的必要范围。

---

## 8. Style resolver：UI = f(state)

Styled ComboBox 没有采用：

```text
Hovered
→ 只 patch hover 颜色

Pressed
→ 再 patch pressed 颜色
```

而是采用完整状态解析：

```text
UI = f(current state)
```

也就是：

> 变化检测是增量的，样式计算是完整的。

系统通过：

```text
Changed<T>
Added<T>
RemovedComponents<T>
```

判断：

```text
什么时候需要重算
```

真正计算时重新读取当前完整状态：

```text
disabled
open
pressed
hovered
selected
```

然后一次性决定最终 Style。

这样状态回退天然成立。

---

## 9. Field Style resolver

Field 当前 Style 数据：

```rust
struct ComboBoxFieldStyle {
    background: Color,
    border: Color,
    foreground: Color,
}
```

状态优先级：

```text
Disabled
   >
Open
   >
Pressed
   >
Hovered
   >
Default
```

Disabled 来自：

```text
ComboBox root 上的 InteractionDisabled
```

Open 来自：

```text
Popup 的 Visibility
```

因此没有在 Field 自己再存：

```text
disabled: bool
open: bool
```

---

## 10. Option Style resolver

Option 当前 Style 数据：

```rust
struct ComboBoxOptionStyle {
    background: Color,
    foreground: Color,
}
```

状态优先级：

```text
Disabled
   >
Hovered
   >
Selected
   >
Default
```

没有设计 Option Pressed Style。

当前 Option 点击后：

```text
选择 Option
→ selection 更新
→ Popup 立即关闭
```

Pressed 的可见时间几乎没有实际意义，因此当前 Option 只关心：

```text
disabled
hovered
selected
```

---

## 11. InteractionDisabled 只放在 root

没有把：

```rust
InteractionDisabled
```

复制到：

```text
Field
Option
```

它只存在于：

```text
ComboBox root
```

Style system 根据 hierarchy 向上找到 root，再读取：

```rust
Has<InteractionDisabled>
```

于是：

```text
root
└── InteractionDisabled
      ↓
Field resolver
Option resolver
```

都能得到 disabled 状态。

这样保持：

```text
一份语义状态
→ 多个视觉消费者
```

避免产生多份可能不同步的 disabled 状态。

---

## 12. Field 文本由 `Selected` 驱动

Field 当前文本没有监听：

```text
ValueChange<usize>
```

而是监听：

```rust
Changed<Selected>
```

原因是：

```text
用户选择
→ 会改变 Selected

程序调用 SetComboBoxSelected
→ 也会改变 Selected
```

因此：

> `Selected` 是 selection 的事实来源，Field Text 只是它的视觉投影。

同步链路为：

```text
Changed<Selected> Option
        ↓
Parent Popup
        ↓
Parent ComboBox root
        ↓
ComboBoxOptions[index]
        ↓
Field
        ↓
ComboBoxFieldText
        ↓
Text.0 = label
```

---

## 13. ECS 集成测试的边界

Style 内部类型例如：

```text
ComboBoxField
ComboBoxPopup
ComboBoxOption
ComboBoxFieldText
```

继续保持 private / crate-private。

外部集成测试没有为了方便测试而暴露这些内部实现。

测试通过公开的 Bevy 语义组件识别结构：

```text
Button
→ Field

ListBox
→ Popup

ListItem
→ Option

Selected
→ 当前被选中的 Option
```

这形成了一个重要原则：

> 集成测试尽量观察外部可见语义，不为了测试破坏封装。

---

## 14. ECS 集成测试覆盖

本阶段为 Styled ComboBox 增加了独立 ECS 集成测试。

主要覆盖：

```text
1. Styled ComboBox 创建后视觉结构存在

2. 初始 Field 显示默认选项文本

3. SetComboBoxSelected
   → Changed<Selected>
   → Field Text 更新

4. selection 切换
   → 新 Option 获得 Selected Style
   → 旧 Option 回退 Default Style

5. Field：
   Hovered
   → Pressed
   → Open

6. Field 高优先级状态移除：
   Open
   → Pressed
   → Hovered
   → Default

7. Popup 静态视觉和布局：
   Background
   Border
   Absolute
   left = 0
   top = 100%
   width = 100%
   Column

8. Selected Option：
   Selected
   → Hovered
   → Selected

9. 普通 Option：
   Default
   → Hovered
   → Default

10. Option：
    Disabled > Hovered > Selected

11. Field：
    Disabled > Open > Pressed > Hovered
```

这些测试重点验证：

```text
状态变化
        ↓
ECS system 被触发
        ↓
resolver 重新读取完整状态
        ↓
真实视觉 Component 被写回
```

---

## 15. Disabled 后 Field 的实际回退

Field Disabled 测试中出现了一个值得记录的细节。

初始：

```text
Hovered + Pressed + Open
```

加入：

```rust
InteractionDisabled
```

后：

```text
Headless ComboBox
→ 自动关闭 Popup
```

所以移除 Disabled 后，并不会回到 Open：

```text
Disabled
→ remove InteractionDisabled
→ Pressed
```

而不是：

```text
Disabled
→ Open
```

原因是此时：

```text
Popup 已经 Hidden
```

这个测试刚好验证了：

```text
Headless 行为
+
Style 完整状态 resolver
```

之间能够正确衔接。

---

## 16. Popup 没有 Disabled Style

Popup 本身没有额外：

```text
Disabled Background
Disabled Border
```

因为：

```text
ComboBox Disabled
→ Headless 直接关闭 Popup
```

因此 Styled 层只需要让：

```text
Field
Option
```

具备 Disabled Style。

Popup 继续保持静态视觉样式即可。

---

## 17. Visual Regression Test 的场景

Visual baseline 采用一张图同时展示三个 ComboBox：

```text
Default        Open                 Disabled

[ Apple  v ]   [ Apple  v ]        [ Apple  v ]
               ┌──────────────┐
               │ Apple        │ Selected
               │ Banana       │ Hovered
               │ Orange       │ Default
               └──────────────┘
```

Baseline 文件：

```text
crates/bevy_widgetry/tests/baselines/styled_combo_box.png
```

---

## 18. Baseline generator 拆分

随着 ComboBox 加入，baseline generator 改为：

```text
tests/
├── generate_baseline.rs
└── baseline_generators/
    ├── mod.rs
    ├── button.rs
    └── combo_box.rs
```

入口：

```rust
// generate_baseline.rs
mod baseline_generators;
```

模块文件：

```rust
// baseline_generators/mod.rs
mod button;
mod combo_box;
```

以后增加控件时继续新增对应 generator 文件即可。

---

## 19. Offscreen Visual Test

真正的 Visual Test 仍放在：

```text
crates/bevy_widgetry/tests/visual_test.rs
```

ComboBox 增加：

```rust
styled_combo_box_can_render_offscreen
```

流程与 Button 保持一致：

```text
创建无窗口 Bevy App
        ↓
创建离屏 Image
        ↓
Camera RenderTarget → Image
        ↓
生成 ComboBox UI 场景
        ↓
等待 Render Pipeline ready
        ↓
GPU Readback
        ↓
得到实际像素
        ↓
读取 baseline PNG
        ↓
解码
        ↓
dimensions 比较
        ↓
raw pixels 完全比较
```

最终断言：

```text
actual pixels == baseline pixels
```

因此这是严格的像素级 Visual Regression Test。

---

## 20. ComboBoxVisualCamera 的作用

Button 的 visual test 直接在测试函数里构造 UI，因此可以直接使用局部：

```rust
camera: Entity
```

ComboBox 构造 API 需要：

```rust
&mut Commands
```

所以场景构造放进 Startup system。

Startup system 无法读取测试函数的局部变量，因此增加：

```rust
#[derive(Resource)]
struct ComboBoxVisualCamera(Entity);
```

传递关系：

```text
test function
→ ComboBoxVisualCamera Resource
→ Startup system
→ UiTargetCamera
```

它只是 visual test 的桥接数据，不属于 ComboBox 自身语义。

---

## 21. GPU Readback 的 row alignment

ComboBox Visual Test 第一次使用：

```text
width = 680
height = 180
4 bytes / pixel
```

理论数据长度：

```text
680 × 180 × 4
= 489600
```

但实际 GPU Readback 得到：

```text
506880
```

原因是 texture copy 每行数据存在 256-byte alignment。

理论每行：

```text
680 × 4
= 2720 bytes
```

实际被 padding 到：

```text
2816 bytes
= 256 × 11
```

于是：

```text
2816 × 180
= 506880
```

当前阶段没有实现通用的 row-padding 剥离逻辑，而是让测试尺寸主动满足对齐要求。

因为：

```text
4 bytes / pixel
```

所以 width 选 64 的倍数即可满足：

```text
width × 4
```

为 256 的倍数。

最终使用：

```text
width = 704
```

以后如果要支持任意宽度，再实现正式的 row padding 去除逻辑。

---

## 22. Rust 测试默认并行带来的 Bevy 全局插件冲突

增加第二个 Visual Test 后，全量：

```bash
cargo test
```

出现：

```text
Could not set global logger and tracing subscriber as they are already set.

Skipping installing Ctrl+C handler as one was already installed.
```

原因是 Rust test runner 默认并行执行测试。

两个 Visual Test 可能同时启动两套 `DefaultPlugins`，于是同时尝试初始化进程级全局设施。

Visual Test 本身不需要这些能力，所以继续保留并行测试，并在测试 App 中禁用：

```rust
.disable::<LogPlugin>()
.disable::<TerminalCtrlCHandlerPlugin>()
```

同时原本已经禁用：

```rust
.disable::<WinitPlugin>()
.disable::<PipelinedRenderingPlugin>()
```

这样无需强制：

```bash
cargo test -- --test-threads=1
```

仍可正常并行执行。

---

## 23. Baseline 第一次必须人工确认

Baseline generator 成功生成 PNG，并不代表图片一定正确。

本阶段第一次生成时，人工检查就发现：

```text
Dropdown "▼"
→ missing glyph
```

因此正确流程应该是：

```text
生成 baseline
        ↓
人工打开图片
        ↓
确认布局 / 颜色 / 文本 / 状态正确
        ↓
认可 baseline
        ↓
之后 Visual Test 机械比较
```

也就是说：

> Baseline 是“已经被人确认正确的视觉结果”，不是“程序第一次碰巧生成的结果”。

---

## 24. 当前测试职责划分

Stage 3 完成后，ComboBox 的测试分成三层：

```text
Headless tests
→ 语义行为
→ selection / open / disabled / events

Styled ECS integration tests
→ ECS 状态变化是否正确投影到 Style / Text

Visual regression tests
→ 最终渲染结果是否和已确认 baseline 完全一致
```

可以理解为：

```text
语义正确
        ↓
ECS 表现状态正确
        ↓
最终像素正确
```

三层分别保护不同边界。

---

## 25. 关于 Style 断言完整度的暂缓决定

目前 ECS Style 集成测试主要断言：

```text
BackgroundColor
```

以及 Popup 中明确需要验证的：

```text
BorderColor
Node layout
```

讨论过未来是否应该对每个 Style 状态完整比较：

```text
background
border
foreground
padding
...
```

随着后续越来越多代码可能由 Codex 快速生成，测试很可能需要承担更多自动 review 的职责。

这个方向倾向于以后加强，但本阶段暂时不扩展，留到后续测试体系进一步成熟时再专门讨论。

---

## 26. 本阶段形成的几个重要认识

### 26.1 Style 不应该创建第二套语义状态

继续坚持：

```text
Selected
→ selection truth

Visibility
→ open truth

InteractionDisabled
→ disabled truth
```

Style 只读取并投影视觉结果。

### 26.2 变化通知和最终状态要分开理解

```text
Changed / Added / Removed
→ 告诉 system “什么时候需要工作”

resolver
→ 读取当前完整状态决定最终 UI
```

因此：

```text
增量触发
+
完整计算
```

并不冲突。

### 26.3 Text 也应该从真实状态派生

不是：

```text
ValueChange
→ 顺便改 Text
```

而是：

```text
Selected
→ Text
```

这样程序修改和用户修改走同一条视觉同步路径。

### 26.4 集成测试不要为了方便破坏封装

测试通过：

```text
Button
ListBox
ListItem
Selected
```

识别公开语义，而不是把 Style-private marker 暴露成 public。

### 26.5 Visual Test 会暴露 ECS 测试看不到的问题

本阶段实际暴露了：

```text
字体 glyph
GPU row alignment
并行测试的全局 logger / Ctrl+C handler
```

这些都不是普通 ECS 测试容易发现的问题。

因此 Visual Test 不是重复测试，而是在保护另一层真实运行环境。

---

## 27. Stage 3 目标 3 和目标 4 完成状态

最终可以认为：

```text
目标 3：实现 ComboBox Style
✅ 完成

目标 4：Style / Visual Test
✅ 完成
```

现在 ComboBox 已经具备：

```text
Headless 语义
+
Styled UI
+
状态 resolver
+
Field Text 同步
+
ECS 集成测试
+
离屏像素级 Visual Regression Test
```

Stage 3 的 ComboBox 主线至此完成。
