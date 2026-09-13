# MessageBox 控件最终设计方案

## 1. 目标

为 `bevy_widgetry` 增加一个基于 Bevy 0.19 的 `MessageBox` 控件，并补齐其依赖的通用窗口能力。

本次实现包含两部分：

1. `window` crate 增加通用的 owned window 与 parent-modal 支持。
2. 新增独立 `message_box` crate，在上述窗口能力之上实现 MessageBox。

MessageBox 保持非阻塞：不会暂停 Bevy 主循环，结果通过 ECS EntityEvent 异步返回。

---

## 2. crate 与目录组织

新增独立 crate：

```text
crates/
├─ window/
│  └─ src/
│     ├─ lib.rs
│     ├─ scene.rs
│     ├─ window_root.rs
│     ├─ modal.rs
│     └─ title_bar/
│
├─ message_box/
│  ├─ Cargo.toml
│  └─ src/
│     ├─ lib.rs
│     ├─ scene.rs
│     └─ lifecycle.rs
│
├─ button/
├─ combo_box/
└─ ...
```

依赖方向：

```text
message_box
    ├──> window
    └──> button

window
    ✕ 不依赖 message_box
```

`window` 只承载任何窗口都可能复用的基础能力；所有 MessageBox 专属语义均留在 `message_box` crate。

workspace 根 `Cargo.toml` 增加：

```text
crates/message_box
```

聚合 crate `crates/bevy_widgetry` 增加 `bevy_widgetry_message_box` 依赖，并公开：

```rust
pub mod message_box {
    pub use bevy_widgetry_message_box::*;
}
```

---

## 3. WindowControlsConfig 扩展

将现有配置扩展为：

```rust
pub struct WindowControlsConfig {
    pub minimize_visible: bool,
    pub maximize_visible: bool,
    pub close_visible: bool,
    pub resizable: bool,
}
```

普通窗口默认值：

```text
minimize_visible = true
maximize_visible = true
close_visible    = true
resizable        = true
```

行为要求：

- `close_visible == false` 时，不生成标题栏关闭按钮实体。
- `resizable == false` 时，不生成/启用 Widgetry resize 交互区域。
- 原生 Bevy `Window.resizable` 仍是 native window 的最终缩放约束。
- MessageBox 同时设置：
  - `minimize_visible = false`
  - `maximize_visible = false`
  - `close_visible = false`
  - `resizable = false`
  - native `Window.resizable = false`

不额外拦截 Alt+F4、任务栏关闭等系统级关闭行为。

---

## 4. owned_window：通用窗口所有权能力

### 4.1 两种窗口构造语义

保留现有：

```rust
window(target_window, target_camera, controls, title_bar_content, content)
```

语义：

```text
window(...)
= 外部拥有 Native Window + Camera
```

新增：

```rust
owned_window(/* native Window config, controls, title, content */) -> impl Scene
```

语义：

```text
owned_window(...)
= 内部创建并拥有 Native Window + Camera
```

两者最终组合相同的 Widgetry WindowRoot UI shell。

### 4.2 owned_window 不是 SceneComponent

`owned_window(...)` 是普通 Scene 组合函数，不建立新的业务 ECS identity。

其 root 上保留现有绑定信息：

```text
WindowRoot { target_window }
UiTargetCamera(camera)
OwnedWindow
```

其中：

```rust
#[derive(Component)]
struct OwnedWindow;
```

`OwnedWindow` 为 `window` crate 私有 marker，不重复保存 Window/Camera Entity ID。

Native Window ID 继续只由 `WindowRoot.target_window` 保存；Camera ID 继续由 `UiTargetCamera` 保存。

### 4.3 内部资源创建

`owned_window(...)` 内部可以通过私有 BSN Template，在场景展开时创建：

```text
Native Window Entity
Camera2d Entity
Widgetry WindowRoot/UI root
```

这是 Window crate 的内部实现细节，不暴露 Window/Camera Entity ID 给公共 API。

### 4.4 生命周期清理

owned root 整体被 despawn 时，通过：

```text
On<Despawn, OwnedWindow>
```

执行 owned resource cleanup：

1. 从同一 root 的 `WindowRoot` 取得 native Window Entity。
2. 从同一 root 的 `UiTargetCamera` 取得 Camera Entity。
3. 如果对应实体仍存在，则 despawn Camera。
4. 如果对应实体仍存在，则 despawn Native Window。

不使用 `On<Remove, OwnedWindow>` 作为所有权清理语义；单独移除 marker 不应等价于销毁 owned window。

现有 `WindowClosed` → WindowRoot cleanup 路径需保持幂等，使系统关闭 native window 和程序主动销毁 root 两种路径均安全。

---

## 5. 通用 parent-modal 能力

### 5.1 公共组件

`window` crate 新增：

```rust
#[derive(Component)]
pub struct ModalWindow {
    pub parent: Entity,
}
```

`parent` 必须指向父级 **Native Bevy Window Entity**。

有效 parent 必须已经绑定一个 Widgetry WindowRoot。

若找不到对应 Widgetry WindowRoot：

```text
ModalWindow 创建失败
→ 不静默退化为 modeless
→ 清理该 modal child / owned window
```

语义是：

> 要么真正建立 modal，要么创建失败。

### 5.2 ModalWindow 的挂载位置

`ModalWindow` 挂在 modal child 的 **Widgetry UI root** 上，而不是 child 的 native Window Entity。

例如 MessageBox：

```text
MessageBox / WindowRoot UI root
├─ MessageBox
├─ ModalWindow { parent }
├─ OwnedWindow
├─ WindowRoot { target_window: own native window }
└─ UiTargetCamera

Own Native Window Entity
├─ Window
└─ ModalState
```

这样 modal 与 owned window 保持正交：

```text
window(...)         = 外部资源所有权
owned_window(...)   = 内部资源所有权
ModalWindow         = 可选 parent-modal 关系
MessageBox          = owned_window + ModalWindow + MessageBox 业务语义
```

### 5.3 ModalState

每个已经绑定 Widgetry WindowRoot 的 Native Window Entity 都应具有私有：

```rust
#[derive(Component, Default)]
struct ModalState {
    blocker: Option<Entity>,
}
```

`ModalState` 不应只存在于 owned window；外部所有权 `window(...)` 同样必须具备 modal parent 能力。

建议在 WindowRoot 初始化成功、完成 native Window 与 Widgetry root 绑定时统一确保 Native Window 上存在 `ModalState`。

### 5.4 blocker 行为

第一个 modal child 出现时：

1. 找到 `ModalWindow.parent` 的 Native Window。
2. 取得其 `ModalState`。
3. 找到该 native window 对应的 Widgetry WindowRoot。
4. 创建一个 `ModalBlocker`，作为父 WindowRoot 的直属子节点。
5. 立即在 `ModalState.blocker` 中记录 blocker Entity。

多个 modal child 可以同时存在。

若 `ModalState.blocker` 已经为 `Some`，新增 modal child 不再创建第二个 blocker。

最后一个引用该 parent 的 `ModalWindow` 消失后：

```text
despawn blocker
→ ModalState.blocker = None
```

“是否还有 modal child”从 ECS 中查询 `ModalWindow { parent }` 关系推导，不维护额外计数 Resource。

### 5.5 blocker UI

层级：

```text
Parent WindowRoot
├─ TitleBar
├─ WindowContent
├─ ResizeArea
└─ ModalBlocker
```

blocker 使用绝对定位覆盖整个父 WindowRoot。

示意：

```rust
Node {
    position_type: PositionType::Absolute,
    left: px(0),
    right: px(0),
    top: px(0),
    bottom: px(0),
    ..
}
```

blocker 使用足够高、但非 `i32::MAX` 的固定 `GlobalZIndex`，为未来更高层的调试/系统 overlay 保留空间。

blocker 应参与 hit test 并阻断下层 picking，但自身不需要成为 hoverable 控件：

```rust
Pickable {
    should_block_lower: true,
    is_hoverable: false,
}
```

其半透明遮罩颜色/透明度在 Gallery 中实际运行后调优。

### 5.6 modal 范围

本次 modal 只保证：

- 阻挡父 Widgetry Window 的 pointer/mouse UI 交互。
- 显示半透明遮罩。

不处理：

- InputFocus。
- 键盘快捷键。
- Alt+F4。
- OS/application-wide modal。
- modal child 的显式 stack/order 管理。

多个 modal child 同时存在时，只保证父窗口 blocker 生命周期正确。

---

## 6. MessageBox crate

### 6.1 ECS identity

`MessageBox` 使用 `SceneComponent`，因为它拥有长期存在的业务语义和 ECS identity。

MessageBox 的 identity 是：

```text
Widgetry WindowRoot UI root Entity
```

不是 native Window Entity。

结构：

```text
[Native Window Entity]
    Window

[MessageBox / WindowRoot UI root]
    MessageBox
    ModalWindow
    OwnedWindow
    WindowRoot
    UiTargetCamera
    MessageBoxState
    UI children

[Camera Entity]
    Camera2d
```

MessageBox root 与 WindowRoot root 为同一个 Entity，不增加无意义的中间 wrapper。

### 6.2 公共 API

公开：

```rust
pub struct MessageBoxPlugin;

pub struct MessageBox;

pub fn message_box(
    parent: Entity,
    title: impl Into<String>,
    buttons: MessageBoxButtons,
    content: impl SceneList,
) -> impl Scene;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageBoxButtons {
    Ok,
    YesNo,
    YesNoCancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageBoxResult {
    Ok,
    Yes,
    No,
    Cancel,
}

#[derive(EntityEvent)]
pub struct MessageBoxResultEvent {
    pub entity: Entity,
    pub result: MessageBoxResult,
}
```

`MessageBoxResultEvent.entity` 指向 MessageBox / WindowRoot UI root Entity。

### 6.3 推荐构造方式

公共推荐入口为：

```rust
message_box(
    parent,
    "Confirm",
    MessageBoxButtons::YesNo,
    bsn_list![
        Text("Are you sure?")
    ],
)
```

不要求调用方书写：

```rust
Box<dyn SceneList>
```

内部 SceneComponent props 可以进行类型擦除，但这一实现细节不暴露给日常调用者。

### 6.4 MessageBox SceneComponent 数据边界

持久语义：

```rust
MessageBox
ModalWindow { parent }
MessageBoxState
```

一次性场景构造 props：

```text
title
buttons
content
```

其中：

```text
content = 任意 SceneList
```

不限制为 `message: String`。

SceneComponent props 在场景展开后不作为持久运行期业务状态使用。

### 6.5 按钮组合

仅提供：

```rust
MessageBoxButtons::Ok
MessageBoxButtons::YesNo
MessageBoxButtons::YesNoCancel
```

对应结果：

```rust
MessageBoxResult::Ok
MessageBoxResult::Yes
MessageBoxResult::No
MessageBoxResult::Cancel
```

按钮固定居中排列：

```text
Ok            → [ OK ]
YesNo         → [ Yes ] [ No ]
YesNoCancel   → [ Yes ] [ No ] [ Cancel ]
```

按钮之间使用固定 gap。

### 6.6 私有 action 语义

每个 MessageBox 自己生成的底部结果按钮挂：

```rust
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct MessageBoxAction(MessageBoxResult);
```

`MessageBoxAction` 为 `message_box` crate 私有。

这样任意 `content` 中的普通按钮不会误触发 MessageBox result。

MessageBox root 通过 BSN 注册 entity-scoped observer：

```rust
on(handle_message_box_click)
```

Pointer/Activate 事件沿 ChildOf 层级冒泡后，handler 根据原始 target 查询 `MessageBoxAction`：

```text
target 有 MessageBoxAction
→ 是 MessageBox 结果按钮

target 没有 MessageBoxAction
→ 忽略
```

不增加冗余 `MessageBoxOwner` 组件。

---

## 7. Result 与关闭生命周期

### 7.1 at-most-once

MessageBox root 挂私有：

```rust
#[derive(Component, Default)]
struct MessageBoxState {
    resolved: bool,
}
```

有效结果按钮触发时：

1. 立即通过 `Query<&mut MessageBoxState>` 检查状态。
2. 若已经 `resolved`，忽略。
3. 否则立即设置：

```rust
resolved = true;
```

4. 再触发：

```rust
MessageBoxResultEvent
```

由于对 Component 的直接可变访问立即生效，同一帧内不会产生两个有效 result。

### 7.2 Result observer 执行期间实体必须仍存在

触发 `MessageBoxResultEvent` 后，不立即 despawn MessageBox。

关闭链：

```text
结果按钮触发
    ↓
MessageBoxState.resolved = true
    ↓
trigger MessageBoxResultEvent
    ↓
所有 MessageBoxResultEvent observers 执行
    ↓
内部 result observer 仅 queue 插入 MessageBoxClosing
    ↓
observer commands 统一应用
    ↓
On<Add, MessageBoxClosing>
    ↓
despawn MessageBox root
```

私有 marker：

```rust
#[derive(Component)]
struct MessageBoxClosing;
```

`MessageBoxClosing` 保留为明确的关闭阶段边界。

不依赖 `PostUpdate` 来保证这个正确性。

### 7.3 root despawn 后

MessageBox root 被 despawn 后：

```text
On<Despawn, OwnedWindow>
→ 清理 owned Camera + Native Window
```

同时 `ModalWindow` 生命周期变化负责重新计算父窗口是否仍需要 blocker。

如果这是该 parent 的最后一个 modal child：

```text
移除 ModalBlocker
```

### 7.4 系统关闭

如果用户通过 Alt+F4、任务栏或其他 native close 路径关闭 MessageBox：

```text
Native WindowClosed
→ WindowRoot cleanup
→ MessageBox root despawn
→ owned/modal 生命周期清理
```

该路径：

- 不生成 `MessageBoxResultEvent`。
- 不伪造 `Cancel`。
- 用户 result observer 不执行。

即：

```text
系统关闭 = no-result destruction
```

---

## 8. MessageBoxPlugin

`MessageBoxPlugin` 自动确保内部依赖已注册：

```rust
impl Plugin for MessageBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WindowPlugin>() {
            app.add_plugins(WindowPlugin);
        }

        if !app.is_plugin_added::<StyledButtonPlugin>() {
            app.add_plugins(StyledButtonPlugin);
        }

        // 注册 MessageBox 自身 observers / lifecycle。
    }
}
```

调用者只需：

```rust
app.add_plugins(MessageBoxPlugin);
```

不要求记住 MessageBox 内部由 Window + StyledButton 组合实现。

---

## 9. MessageBox 默认视觉/窗口行为

第一版采用固定 MessageBox 尺寸。

不增加：

```text
size prop
auto-measure arbitrary SceneList
```

标题栏：

```text
minimize = hidden
maximize = hidden
close    = hidden
```

resize：

```text
Widgetry resize = disabled
native resizable = false
```

正文区域接受任意 SceneList。

具体窗口宽高、正文 padding、按钮 gap、遮罩透明度等视觉参数，在 Gallery 实际运行后微调，不扩大公共 API。

---

## 10. Gallery：Window 页面

MessageBox 示例继续放在：

```text
gallery/src/pages/window.rs
```

不新增独立 MessageBox 导航页。

### 10.1 页面布局

Window 页面调整为纵向多行布局，页面整体增加 padding，按钮不能紧贴页面边缘。

结构：

```text
Window Page

第一行：
[ Open Window ]

第二行：
[ OK ]  [ Yes / No ]  [ Yes / No / Cancel ]
```

建议布局语义：

```text
root:
    flex-direction: column
    padding: ~24px
    row-gap: ~16px

each row:
    flex-direction: row
    column-gap: ~12px
```

具体数值可按 Gallery 视觉效果微调。

### 10.2 Open Window 示例

现有 `Open Window` 示例改为使用：

```rust
owned_window(...)
```

不再由 Gallery 手动创建并维护专用 Camera 生命周期。

它用于展示：

```text
普通独立 owned Widgetry window
```

### 10.3 MessageBox 示例

第二行提供三种按钮：

```text
OK
Yes / No
Yes / No / Cancel
```

都以当前 Gallery 主 Native Window 为 `parent`。

分别创建：

```rust
MessageBoxButtons::Ok
MessageBoxButtons::YesNo
MessageBoxButtons::YesNoCancel
```

至少一个示例的 `content` 使用比单独 `Text` 更复杂的 SceneList，例如：

```text
Text + 普通 StyledButton
```

用来验证：

```text
content 中普通按钮事件冒泡
≠
MessageBox result
```

因为只有私有 `MessageBoxAction` 才能产生结果。

Gallery 还应能观察/记录 `MessageBoxResultEvent`，便于人工验证 OK / Yes / No / Cancel 结果。

---

## 11. Window crate 直接改动点

### `scene.rs`

负责：

- `WindowControlsConfig` 新字段。
- `window(...)` 根据配置组合 title-bar / resize。
- `owned_window(...)`。
- owned window 内部 Template。

### `title_bar/bar.rs`

负责：

- `minimize_visible`
- `maximize_visible`
- `close_visible`

其中关闭按钮从“始终生成”改成受配置控制。

### `window_root.rs`

负责：

- 现有 WindowRoot / native Window / Camera 绑定。
- owned window 生命周期清理所需的内部访问。
- Widgetry root 初始化成功时确保 Native Window 获得 `ModalState`。
- 保持 `WindowClosed` cleanup 幂等。

`WindowRoot` 继续私有，不因 MessageBox 暴露。

### `modal.rs`

负责：

- `pub ModalWindow`
- private `ModalState`
- private `ModalBlocker`
- modal add/remove/despawn 生命周期
- blocker 创建/移除
- parent 合法性验证
- parent WindowRoot 查找

### `lib.rs`

公开：

```rust
WindowControlsConfig
widgetry_window
window
owned_window
ModalWindow
WindowPlugin
```

---

## 12. MessageBox crate 内部分工

### `scene.rs`

负责：

```text
MessageBox
MessageBox props
MessageBoxButtons
MessageBoxResult
MessageBoxResultEvent
message_box(...)
MessageBoxAction
BSN UI 结构
结果按钮生成
```

### `lifecycle.rs`

负责：

```text
MessageBoxState
MessageBoxClosing
结果按钮处理
result event 触发
result → closing
closing → root despawn
```

### `lib.rs`

负责：

```text
mod scene
mod lifecycle
pub re-export
MessageBoxPlugin
```

---

## 13. 测试要求

### Window crate

至少覆盖：

1. `WindowControlsConfig::default()` 四项均为 true。
2. `close_visible = false` 时不生成 CloseButton。
3. `resizable = false` 时 Widgetry resize 交互不可用/不生成，行为与最终实现一致。
4. `owned_window` 创建 root、native Window、Camera，并建立正确绑定。
5. owned root despawn 会清理其 Camera 与 Native Window。
6. native Window 先被系统关闭时 cleanup 不产生二次销毁问题。
7. Widgetry window 初始化后 Native Window 具有 `ModalState`。
8. 第一个 `ModalWindow` 创建一个 blocker。
9. 同一 parent 多个 modal child 只存在一个 blocker。
10. 非最后一个 modal child 消失时 blocker 保留。
11. 最后一个 modal child 消失时 blocker 移除。
12. 非 Widgetry parent 被指定为 `ModalWindow.parent` 时，modal child 创建失败并清理。
13. blocker 覆盖父 WindowRoot 并阻断 lower picking。

现有测试中依赖旧 `WindowControlsConfig` 字段数量、关闭按钮始终存在、resize handle 始终生成等假设的部分应同步更新。

### MessageBox crate

至少覆盖：

1. 三种 `MessageBoxButtons` 生成正确按钮数量及顺序。
2. 结果按钮携带正确私有 `MessageBoxAction`。
3. content 内普通按钮不会生成 MessageBox result。
4. 有效点击产生正确 `MessageBoxResultEvent`。
5. 同一 MessageBox 最多产生一个 result。
6. result observer 执行期间 MessageBox root 仍存在。
7. result observer 完成后进入 `MessageBoxClosing` 并最终 despawn root。
8. root despawn 后 owned Window/Camera 被清理。
9. root despawn 后父 modal blocker 在最后一个 child 消失时被移除。
10. native/system close 不产生任何 `MessageBoxResultEvent`。

---

## 14. 非目标

本次不实现：

```text
MessageBoxKind
Info / Warning / Error 独立控件
任意按钮组合
键盘默认按钮
Escape → Cancel
焦点捕获/恢复
应用级 OS modal
modal stack/order 管理
自动 SceneList 内容测量
MessageBox size 公共配置
Alt+F4 拦截
系统关闭自动映射 Cancel
```

后续若有真实需求再独立扩展。

---

## 15. 最终调用示例

```rust
commands.spawn_scene(bsn! {
    message_box(
        parent_window,
        "Confirm",
        MessageBoxButtons::YesNoCancel,
        bsn_list![
            Text("Save changes before closing?")
        ],
    )
});
```

监听结果：

```rust
fn on_message_box_result(event: On<MessageBoxResultEvent>) {
    match event.result {
        MessageBoxResult::Ok => {}
        MessageBoxResult::Yes => {}
        MessageBoxResult::No => {}
        MessageBoxResult::Cancel => {}
    }
}
```

整体关系最终固定为：

```text
Window infrastructure
├─ external window(...)
├─ owned_window(...)
└─ optional ModalWindow

MessageBox
├─ MessageBox ECS identity
├─ owned_window
├─ ModalWindow { parent }
├─ arbitrary SceneList content
├─ fixed result button sets
└─ MessageBoxResultEvent
```

这份方案作为本次 MessageBox 实现的执行基准。
