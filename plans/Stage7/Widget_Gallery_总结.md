# Stage 7：Widget Gallery 总结

## 1. 阶段目标

Stage 7 的目标不是继续扩充大量控件，而是建立一个长期维护的 **Widget Gallery**，用于：

- 展示现有控件；
- 手动验证真实交互；
- 验证不同控件在同一应用中的集成；
- 验证 Theme 运行时切换；
- 验证自定义窗口、Popup、Picking、拖动、Resize 等机制不会互相干扰；
- 反过来检查控件库的对外 API 是否合理。

最终 Gallery 使用的核心控件仍然保持精简：

- `StyledButton`
- `StyledComboBox`

并通过自定义窗口承载。

---

## 2. Gallery 应用定位

Gallery 不是单纯的截图展示页，而是一个可运行、可点击、可交互的集成验证应用。

当前结构大致为：

```text
Widget Gallery
├── 自定义标题栏
│   ├── 标题
│   ├── Theme ComboBox
│   └── 最小化 / 最大化 / 关闭
└── Window Content
    ├── StyledButton
    └── StyledComboBox
```

这一阶段不再为了“丰富 Gallery”继续增加 CheckBox、Slider、TextInput 等新控件。

Gallery 的主要任务是验证：

```text
现有控件
+ Theme
+ Window
+ Picking
+ Popup
+ Layout
```

能否在真实应用里稳定协作。

---

## 3. 自定义窗口能力

Stage 7 中补充并完成了自定义窗口支持。

### 3.1 无系统边框窗口

Gallery 使用：

```rust
decorations: false
```

关闭系统标题栏和边框，由控件库自己提供窗口 UI。

---

### 3.2 WindowRoot

内部使用 `WindowRoot` 保存目标 Bevy `Window` Entity：

```text
WindowRoot { target_window }
```

其他行为不再重复保存 `target_window`，而是通过 ECS 层级向上查找 `WindowRoot`。

这样避免了：

```text
TitleBar 一份 window
ResizeArea 一份 window
Button 再一份 window
```

这种状态复制。

最终原则是：

> 窗口归属由 ECS hierarchy 派生，而不是在多个组件中重复保存。

---

### 3.3 标题栏控制按钮

实现了：

- Minimize
- Maximize
- Restore
- Close

这些按钮使用 Bevy 0.19 的 headless `Button + Activate` 行为。

最大化 / 还原状态通过底层窗口状态判断，而不是在 UI 中额外维护一份“是否最大化”的状态。

---

### 3.4 标题栏拖动

一开始拖动逻辑直接挂在 `TitleBarContent` 上。

但真实 Gallery 很快暴露出问题：

- Text 会参与 Picking；
- ComboBox Field 会参与 Picking；
- Popup Option 的 Pointer 事件可能向上冒泡；
- 透明布局 Node 也可能阻挡底层 Picking；
- “视觉上的空白”并不等于 ECS Picking 上的空白。

最终改成显式的拖动层：

```text
TitleBar
├── TitleBarDragArea       ← 最底层，铺满标题栏
├── TitleBarContent        ← 上层内容
└── WindowControls         ← 上层窗口按钮
```

其中：

```text
TitleBarDragArea
```

只负责接住真正没有被上层内容命中的区域。

纯布局容器使用：

```rust
Pickable::IGNORE
```

这样 Text、Button、ComboBox、自定义控件等正常可交互内容会自然挡住拖动区域，而真正的空白位置会穿透到底层 `TitleBarDragArea`。

最终形成一个很稳定的原则：

> 布局容器负责排版并穿透；真实内容负责挡住；最底层 DragArea 接住剩余空白。

---

## 4. Resize

实现了 8 个方向的窗口 Resize：

```text
North
NorthEast
East
SouthEast
South
SouthWest
West
NorthWest
```

Bevy 0.19 使用：

```rust
window.start_drag_resize(direction)
```

启动原生窗口 Resize。

Resize Handle 覆盖在窗口边缘和四角。

---

### 4.1 Resize Cursor

鼠标移动到不同方向的 Resize Handle 上时，会更新对应光标。

为了避免原生 Resize 过程中 Pointer `Out` 提前清理状态，引入了内部 `Resizing` 状态。

Resize 结束不依赖 Handle 自己收到 `Pointer<Release>`，而是通过全局：

```rust
ButtonInput<MouseButton>
```

监听左键释放。

这一点避免了原生窗口 Resize 抢走鼠标后，Pointer Release 不再回到 UI Handle 的问题。

---

### 4.2 Windows 下快速 Resize 黑边问题

Gallery 测试中发现：

- Vulkan backend：快速拉伸窗口时，新扩展区域可能短暂出现黑色；
- DX12 backend：该现象基本消失。

进一步确认：

- 即使设置明显的 `ClearColor`，新扩展区域仍可能是黑色；
- 因此这不是 Bevy UI Layout 没填满；
- 更接近 Windows + Vulkan + wgpu / surface resize 路径的问题。

最终只在 Gallery 应用里强制：

```rust
Backends::DX12
```

而不把这种环境策略写进 `window` crate。

原则是：

> 控件库不替应用决定 GPU backend。

---

## 5. Picking / Popup 集成问题

Stage 7 最有价值的一部分，是 Gallery 真正测出了多个只在集成场景里才会出现的问题。

---

### 5.1 WindowContent 阻挡 ComboBox Popup

最初 ComboBox Popup 展开后，点击 Option 无法选择。

问题并不在 ComboBox，而在：

```text
WindowContent
```

本身是一个覆盖窗口大面积区域的 UI Node。

虽然视觉上透明，但 Bevy UI Picking 中：

> 没有 `Pickable` 的 Node 默认仍然会参与 Picking，并阻挡下面的元素。

因此 Popup 虽然画出来了，但点击实际上被 `WindowContent` 拦截。

最终给 `WindowContent` 固定：

```rust
Pickable::IGNORE
```

解决问题。

---

### 5.2 Pointer 事件冒泡到 TitleBar

修复 WindowContent 后，ComboBox Option 能收到 Pointer，但点击 Option 时又触发了标题栏拖动。

原因是 Pointer 事件会沿 ECS hierarchy 向上传播。

这里进一步确认了 Bevy 0.19 中：

```text
event.entity
```

表示当前传播到的 Entity，而：

```text
event.original_event_target()
```

表示最初命中的 Entity。

同时也确认：

- Button 会在 Press 阶段主动停止传播；
- ListBox 的 selection 主要工作在 Click；
- ComboBox Option 的 Press 可能继续向上冒泡。

最终不再尝试用“排除所有交互控件”的方式阻止拖动，而是采用前面提到的 `TitleBarDragArea` 正向设计。

这比维护：

```text
NoDrag
InteractiveControl
WindowDragExcluded
```

之类的 marker 更稳，也不要求外部用户了解窗口内部拖动规则。

---

## 6. UI Stack 与 Spawn 顺序

这一阶段也验证了 Bevy UI 的 sibling 默认层级顺序。

在没有额外 `ZIndex / GlobalZIndex` 时：

```text
同父节点
+ 同 ZIndex
→ 后插入 / 后 spawn 的 UI 在更上层
```

因此：

```rust
title_bar.spawn(TitleBarDragArea);
title_bar.spawn(TitleBarContent);
title_bar.spawn(WindowControls);
```

对应：

```text
最底层
TitleBarDragArea
TitleBarContent
WindowControls
最上层
```

这正好满足：

```text
DragArea 在底部
内容和按钮覆盖在上面
```

的需求。

---

## 7. Theme 运行时切换

Gallery 标题栏加入了 Theme ComboBox：

```text
[ Dark ▼ ]
[ Light  ]
```

Theme 仍然保持 Stage 5 确立的原则：

> Theme 只负责颜色，不扩展成通用 Style 系统。

---

### 7.1 ThemeMode 作为唯一真相源

Theme 切换链路为：

```text
Theme ComboBox
    ↓
ValueChange<usize>
    ↓
ThemeMode
    ↓
ThemeChanged
    ↓
StyledButton / StyledComboBox 刷新颜色
```

索引映射：

```text
0 → Dark
1 → Light
```

Gallery 不直接修改 Button / ComboBox 的颜色，而是只更新：

```rust
ThemeMode
```

然后触发：

```rust
ThemeChanged
```

控件自行刷新样式。

---

### 7.2 初始状态同步

Theme ComboBox 的初始选择不再假设一定是 Dark，而是从：

```rust
ThemeMode
```

派生。

例如应用预先：

```rust
app.insert_resource(ThemeMode::Light);
```

那么 Theme ComboBox 也应该显示 Light。

因此：

```text
ThemeMode
```

始终是唯一真相源，ComboBox 只是它的 UI 控制器。

---

## 8. spawn_window 高层 API

Stage 7 后期进一步收缩了 Window API。

最初 Gallery 需要显式理解并组装：

```text
WindowRoot
TitleBar
WindowContent
WindowResizeArea
```

这说明内部结构泄露到了使用侧。

最终新增高层入口：

```rust
spawn_window(...)
```

用户只需要提供：

- target Bevy Window；
- title bar 内容；
- window content 内容。

概念上：

```rust
spawn_window(
    &mut commands,
    &asset_server,
    window,
    |commands, title_bar| {
        // 用户自己的 title bar 内容
    },
    |commands, content| {
        // 用户自己的 window content
    },
);
```

内部自动创建：

```text
WindowRoot
├── TitleBar
│   ├── TitleBarDragArea
│   ├── TitleBarContent
│   └── WindowControls
├── WindowContent
└── WindowResizeArea
```

外部用户不再需要理解这些实现细节。

---

### 8.1 为什么 closure 使用 Commands + parent Entity

最终没有继续使用：

```rust
&mut ChildSpawnerCommands
```

作为高层 API 的唯一入口。

原因是部分控件，例如当前的：

```rust
spawn_styled_combo_box(&mut Commands, ...)
```

需要真正的 `Commands`。

因此 `spawn_window` 给用户：

```text
&mut Commands
+
parent Entity
```

用户可以先创建任意控件，再：

```rust
ChildOf(parent)
```

挂到对应区域。

这比强制所有控件适配 `ChildSpawnerCommands` 更灵活。

---

### 8.2 用户布局仍然保持自由

`WindowContent` 可以给一个稳定的默认布局，例如：

```text
Column
```

但用户如果需要完全不同的布局，只要在 content closure 里先创建自己的容器：

```text
WindowContent
└── UserContainer
    ├── ...
    └── ...
```

然后由 `UserContainer` 自己设置：

```text
Row
Grid
Absolute
复杂嵌套布局
```

即可。

因此没有必要为了所有布局需求，把大量 `Node` 参数塞进 `spawn_window()`。

最终职责边界是：

> Window 提供结构和行为；用户负责业务布局。

---

## 9. Window API 收缩

完成 `spawn_window()` 后，进一步把内部类型从 public API 中移除。

最终用户侧只需要看到类似：

```rust
use bevy_widgetry::window::{
    spawn_window,
    TitleBarPlugin,
};
```

而这些都退回内部实现：

```text
WindowRoot
WindowContent
TitleBar
TitleBarContent
TitleBarDragArea
WindowControls
WindowResizeArea
```

这样：

```text
window crate
```

真正形成了一个高层窗口控件，而不是要求调用者自己拼内部零件。

---

## 10. Stage 7 的主要收获

Stage 7 最重要的结果，并不是“Gallery 页面做出来了”，而是通过真实集成把库的结构验证了一遍。

这一阶段实际暴露并解决了：

```text
透明 Node 仍会挡 Picking
Popup 可以被其他 UI Node 截获输入
Pointer 事件会沿 hierarchy 传播
Button 和 ListBox 在不同事件阶段处理行为
视觉空白不等于 Picking 空白
标题栏拖动应该使用显式 DragArea
Window target 不应该在多个组件中重复保存
原生窗口 Resize 与 UI Pointer Release 不是完全对称的
Windows Vulkan Resize 黑边属于 backend / surface 路径问题
高层 API 不应该暴露内部 Window 组件结构
```

这些问题如果只写单个控件测试，很难全部发现。

这也证明了 Widget Gallery 的定位是有效的：

> Gallery 是手动集成测试场，而不仅仅是控件陈列页。

---

## 11. Stage 7 最终状态

Stage 7 当前可以正式判定完成。

已经具备：

```text
Widget Gallery
├── 独立 Gallery 应用
├── 自定义窗口
├── 自定义标题栏
├── 最小化 / 最大化 / 还原 / 关闭
├── 标题栏空白拖动
├── 8 方向 Resize
├── Resize Cursor
├── StyledButton
├── StyledComboBox
├── Theme ComboBox
├── Dark / Light Runtime Theme
├── Popup / Picking 集成验证
├── DX12 Gallery Backend 配置
└── spawn_window 高层 API
```

并且没有为了完整性继续扩充大量控件。

这与整个项目一直坚持的原则一致：

> 只实现当前真实需要的能力，不为了“看起来完整”提前增加功能。

---

## 12. 最终结论

Stage 7 完成后，`bevy_widgetry` 已经从“几个能单独工作的控件”进入了：

```text
可以在真实 Bevy 桌面应用中共同使用
```

的阶段。

Gallery 证明了：

```text
Styled Widget
+ Theme
+ Icon
+ Popup
+ Picking
+ Custom Window
```

这几层已经可以一起工作。

同时 `window` crate 也完成了从内部零件暴露，到高层 `spawn_window()` API 的收缩。

因此 Stage 7 可以结束，不需要继续为了 Gallery 增加更多控件。

后续新控件应继续遵循整个项目已经形成的开发方式：

```text
真实需求
→ Headless / Existing Primitive
→ Style
→ Theme
→ Test
→ Gallery 集成验证
→ API 收缩
```

而不是一次性追求大而全。
