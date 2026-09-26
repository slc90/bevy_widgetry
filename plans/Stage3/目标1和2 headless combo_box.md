# Stage 3：Select-only ComboBox —— Headless 设计与测试总结

## 1. 本阶段范围

Stage 3 原规划中：

- 目标 1：设计 Headless ComboBox
- 目标 2：Headless ComboBox 测试
- 目标 3：实现 ComboBox Style
- 目标 4：Style / Visual Test

实际学习过程中，目标 1 和目标 2 没有严格串行，而是采用：

```text
设计一小块
    ↓
实现
    ↓
立即测试
    ↓
发现问题
    ↓
调整设计
    ↓
继续下一块
```

因此本阶段实际上同时完成了：

```text
Headless ComboBox 设计
+
Headless ComboBox 单元测试
+
一个最小多实例集成测试
```

Style 尚未开始。

---

## 2. 第一版 ComboBox 的范围缩减

原规划中的 Select-only ComboBox 包括较完整的：

```text
open / closed
selected option
active / highlighted option
keyboard navigation
Enter
Escape
disabled
focus
selection change
```

实际学习时，为避免第一次设计高层控件就引入太多状态机，第一版主动缩小为：

```text
鼠标点击 Field
→ 打开 / 关闭 Popup

鼠标选择 Option
→ 更新 selection
→ 关闭 Popup
→ 对外发 ValueChange

点击 ComboBox 外部
→ 关闭 Popup

disabled
→ 用户不能操作

程序
→ 可以主动修改 selection
```

暂时不做：

```text
keyboard navigation
Escape
Enter
active/highlighted option
ComboBox 自己的 focus 状态机
完整 accessibility 行为
```

这些以后如果真实需求出现，再继续扩展。

---

## 3. 核心设计原则

### 3.1 优先组合官方 Headless Widgets

ComboBox 不重新实现底层 Button、ListBox、ListItem。

当前主要复用：

```text
Button
ListBox
ListItem
Selected
InteractionDisabled
Activate
ValueChange<T>
Visibility
ChildOf / Children
Pointer<Click>
```

自己的 ComboBox 负责的是：

```text
如何把这些 primitives 组合成 ComboBox 语义
```

而不是重新实现这些 primitives。

### 3.2 ComboBox 是一个多 Entity 控件

ComboBox 不能只由一个 Entity 表达。

最终的 Headless 结构为：

```text
ComboBox Root Entity
│
├── ComboBox
│
├── [InteractionDisabled]
│
├── Field Entity
│   ├── ComboBoxField
│   └── Button
│
└── Popup Entity
    ├── ComboBoxPopup
    ├── ListBox
    ├── Visibility
    │
    ├── Option Entity 0
    │   ├── ComboBoxOption { index: 0 }
    │   ├── ListItem
    │   └── [Selected]
    │
    ├── Option Entity 1
    │   ├── ComboBoxOption { index: 1 }
    │   ├── ListItem
    │   └── [Selected]
    │
    └── ...
```

这里一个重要认识是：

```text
Entity ≠ Component
```

Popup 并不是一个 `ListBox Entity` 外面再套一个 Popup Entity。

当前设计中：

```text
同一个 Entity
同时具有：

ComboBoxPopup
ListBox
Visibility
```

因此：

```rust
root.spawn((
    ComboBoxPopup,
    ListBox,
    Visibility::Hidden,
));
```

就是 Popup Entity 本身。

---

## 4. Root Entity 代表整个 ComboBox

公共控件身份放在 root：

```rust
#[derive(Component, Debug, Default)]
pub struct ComboBox;
```

内部角色则保持 private，例如：

```rust
#[derive(Component, Debug, Default)]
struct ComboBoxField;

#[derive(Component, Debug, Default)]
struct ComboBoxPopup;

#[derive(Component, Debug)]
struct ComboBoxOption {
    index: usize,
}
```

这样外部 API 面向：

```text
ComboBox root Entity
```

而不是要求调用者理解内部：

```text
Field Entity
Popup Entity
Option Entity
```

这成为后面 disabled 和 programmatic update 设计的重要基础。

---

## 5. Headless ComboBox 不存文本

Headless 层只知道：

```text
Option 0
Option 1
Option 2
...
```

而不知道：

```text
"Apple"
"Banana"
"Orange"
```

所以构造函数第一版采用：

```rust
pub fn spawn_headless_combo_box(
    commands: &mut Commands,
    option_count: usize,
    selected: Option<usize>,
) -> Entity
```

而不是：

```rust
Vec<String>
```

文本、字体、图标、颜色等属于后面的 Styled ComboBox。

---

## 6. 没有额外保存 selected 状态

最开始考虑过类似：

```rust
struct ComboBoxState {
    selected: Option<usize>,
}
```

后来删除。

因为 Option Entity 上已经存在：

```rust
Selected
```

它本身就能表达真实 selection：

```text
Option 0       无 Selected
Option 1       Selected
Option 2       无 Selected
```

如果 root 再保存：

```text
selected = 1
```

就会形成两份状态：

```text
ComboBoxState.selected
+
Option 上 Selected 的存在情况
```

将来可能出现：

```text
root.selected = 1

但：

Option 2 + Selected
```

于是出现两个真相。

所以最终采用：

> `Selected` component 的存在与否，就是 ComboBox selection 的唯一事实来源。

这是这一阶段非常重要的一次 **Single Source of Truth** 实践。

---

## 7. Popup 是否打开也不额外存 bool

同样，没有设计：

```rust
open: bool
```

因为 Popup 已经有：

```rust
Visibility
```

于是：

```text
Visibility::Hidden
→ closed

Visibility::Visible
→ open
```

继续保持：

```text
一件事实
→ 一份状态
```

---

## 8. `ComboBoxOption.index` 不是重复状态

虽然我们避免保存：

```rust
selected: Option<usize>
```

但每个 Option 仍然保存：

```rust
ComboBoxOption {
    index: usize,
}
```

原因是它不是 selection 状态，而是：

> Option Entity 与业务 index 之间的映射元数据。

例如：

```text
Entity(52)
    ↓ ComboBoxOption
index = 2
```

因此：

```text
Selected
→ 当前状态

ComboBoxOption.index
→ Entity 的身份 / 映射信息
```

两者职责不同，不构成重复状态。

---

## 9. 构造函数负责建立结构

当前 Headless 构造大致为：

```rust
pub fn spawn_headless_combo_box(
    commands: &mut Commands,
    option_count: usize,
    selected: Option<usize>,
) -> Entity {
    if let Some(selected) = selected {
        assert!(selected < option_count);
    }

    let mut combo_box = commands.spawn(ComboBox);
    let combo_box_entity = combo_box.id();

    combo_box.with_children(|root| {
        root.spawn((
            ComboBoxField,
            Button,
        ));

        root.spawn((
            ComboBoxPopup,
            ListBox,
            Visibility::Hidden,
        ))
        .with_children(|popup| {
            for index in 0..option_count {
                let mut option = popup.spawn((
                    ComboBoxOption { index },
                    ListItem,
                ));

                if Some(index) == selected {
                    option.insert(Selected);
                }
            }
        });
    });

    combo_box_entity
}
```

这里第一次深入使用了：

```text
with_children
ChildOf
Children
```

理解为：

```text
ChildOf
→ child 向上找 parent

Children
→ parent 向下找 direct children
```

同时注意：

```text
Children
只包含直接 child
```

如果未来 Styled ComboBox 在中间增加容器：

```text
Popup
└── ScrollContainer
    └── Option
```

现有某些 direct-child 查询就可能需要调整为 descendant 查询。

当前结构简单，所以暂时保持 direct children。

---

## 10. 第一条行为：Field Activate 切换 Popup

Field 自己复用官方：

```text
Button
```

因此 ComboBox 不直接处理：

```text
Pointer<Click> → 打开
```

而是复用 Button 已经提供的语义：

```text
Pointer
→ Button
→ Activate
→ ComboBox
```

ComboBox observer 只关心：

```rust
On<Activate>
```

然后：

```text
Hidden
→ Visible

Visible
→ Hidden
```

这使 ComboBox 依赖的是 Button 的**语义事件**，而不是 Button 的底层鼠标实现。

---

## 11. 测试也直接从语义事件开始

单元测试没有反复模拟：

```text
Pointer Press
Pointer Release
Pointer Click
```

而是直接：

```rust
world.trigger(Activate {
    entity: field,
});
```

因为：

```text
Pointer → Activate
```

是官方 Button 的责任。

ComboBox 单元测试真正要验证的是：

```text
Activate(Field)
→ Popup Visibility 改变
```

这是本阶段形成的重要测试原则：

> 自己只测试自己负责的语义边界，不重复测试依赖库已经负责的内部实现。

---

## 12. 第二条行为：ListBox selection 转换为 ComboBox selection

官方 `ListBox` 点击 Option 后会产生：

```rust
ValueChange<Entity>
```

其语义为：

```text
source = ListBox Entity
value  = 新选中的 ListItem Entity
```

由于当前：

```text
Popup Entity == ListBox Entity
```

因此内部事件大致是：

```rust
ValueChange::<Entity> {
    source: popup,
    value: option_entity,
    is_final: true,
}
```

ComboBox 接收到后进行一次语义转换：

```text
内部：
ValueChange<Entity>

        ↓

外部：
ValueChange<usize>
```

例如：

```text
source = Popup Entity
value  = Option Entity(42)

        ↓ ComboBoxOption.index

source = ComboBox root
value  = 2
```

这样外部调用者不需要知道 ComboBox 内部 Option Entity。

---

## 13. ComboBox 自己维护 `Selected`

调研官方 `ListBox` 后发现：

```text
ListBoxPlugin
并不会自动注册 listbox_update_selection
```

官方虽然提供：

```text
listbox_update_selection
```

但它是可选状态更新策略。

因此没有选择：

```text
全局注册官方 listbox_update_selection
```

因为那会改变应用里**所有 ListBox** 的状态管理行为。

最终决定：

> ComboBox 自己只维护属于 ComboBox 的 ListBox selection。

逻辑大致：

```text
目标 Option
→ 确保拥有 Selected

其他 Option
→ 如果有 Selected，则 remove
```

从而维护：

```text
最多一个 Option 被 Selected
```

---

## 14. `Has<Selected>` 的使用

查询中使用：

```rust
Query<(Entity, Has<Selected>), With<ComboBoxOption>>
```

于是每个 Option 得到：

```text
Entity
+
bool
```

其中：

```text
true
→ 当前 Entity 有 Selected

false
→ 当前 Entity 没有 Selected
```

测试中也用了一个非常简洁的断言：

```rust
for (_, option, selected) in query.iter(world) {
    assert_eq!(selected, option.index == 2);
}
```

它一次验证了：

```text
index == 2
→ 必须 Selected

index != 2
→ 必须没有 Selected
```

因此不仅验证：

```text
新 Option 被选中
```

还验证：

```text
旧 Option 的 Selected 已移除
```

---

## 15. Option 必须属于对应 Popup

后面发现一个重要边界。

仅检查：

```text
event.source 是 ComboBoxPopup
event.value 是 ComboBoxOption
```

还不够。

因为可能出现：

```text
ComboBox A
└── Popup A

ComboBox B
└── Popup B
    └── Option B2
```

错误事件：

```rust
ValueChange::<Entity> {
    source: popup_a,
    value: option_b2,
    ...
}
```

两边类型都合法，但组合是非法的。

因此必须验证：

```text
Option 的 parent
==
event.source Popup
```

也就是维护一个新的不变量：

> 内部 ListBox 发来的 Option 必须真正属于该 ComboBox 的 Popup。

否则整个事件忽略：

```text
不改 Selected
不关闭 Popup
不发 ValueChange<usize>
```

并专门补了跨 ComboBox Option 的测试。

---

## 16. 第三条行为：Outside Click 关闭 Popup

ComboBox 需要：

```text
点击自身内部
→ 不关闭

点击自身外部
→ 关闭
```

这里第一次深入利用 Pointer EntityEvent 的：

```rust
original_event_target()
```

例如实际点中：

```text
Option
```

事件可能沿 hierarchy 冒泡：

```text
Option
↓
Popup
↓
ComboBox
↓
...
```

但：

```rust
event.original_event_target()
```

始终是最初真正点中的：

```text
Option
```

因此判断：

```rust
let inside =
    target == combo_box
    || q_parents
        .iter_ancestors(target)
        .any(|ancestor| ancestor == combo_box);
```

含义为：

```text
target 就是 ComboBox
    或
target 的祖先中存在 ComboBox

→ inside
```

否则：

```text
outside
```

然后关闭 Popup。

---

## 17. Outside Click 会遍历所有 Popup

当前 observer：

```rust
for (popup_parent, mut visibility) in &mut q_popups
```

实际上会检查所有 ComboBox 的 Popup。

但先过滤：

```rust
if *visibility != Visibility::Visible {
    continue;
}
```

于是只处理当前打开的 Popup。

这不是额外维护：

```text
CurrentOpenComboBox
```

而是直接以：

```text
Visibility
```

作为事实来源。

这样即使意外出现多个 Popup 同时 Visible：

```text
A Visible
B Visible
C Visible
```

点击 B 时：

```text
A：outside → Hidden
B：inside  → 保持
C：outside → Hidden
```

最终自然只剩 B。

因此没有为了优化遍历额外增加：

```text
当前打开 ComboBox Entity
```

这种第二份状态。

---

## 18. `propagate(false)` 的理解

这一阶段还澄清了一个 EntityEvent 概念：

```rust
event.propagate(false);
```

不是：

```text
停止当前 Entity 上剩余 observer
```

而是：

```text
当前 Entity 上匹配的 observer
仍然继续执行

但事件不再传播到下一个 parent Entity
```

即：

```text
Popup
├── Observer A
└── Observer B

Observer A:
propagate(false)

→ Observer B 仍执行
→ 不再冒泡到 ComboBox
```

---

## 19. `original_event_target` 与 `event target`

对于传播型 EntityEvent：

```text
original_event_target
→ 第一次触发时的 Entity

event_target
→ 当前传播到的 Entity
```

因此判断用户到底最初点击哪里时，应使用：

```rust
original_event_target()
```

---

## 20. 第四条行为：Disabled

最终确定：

```text
InteractionDisabled
```

应该放在：

```text
ComboBox root
```

而不是要求外部知道内部 Field。

结构：

```text
ComboBox
├── InteractionDisabled
├── Field
└── Popup
```

这表达：

> 整个 ComboBox 是 disabled 的。

---

## 21. Disabled 阻止的是用户交互

disabled 后：

```text
Activate(Field)
→ 忽略

ListBox ValueChange<Entity>
→ 忽略
```

因此：

```text
不能打开 Popup
不能通过用户选择修改 Selected
不能产生对外 ValueChange<usize>
```

但一个关键语义是：

```text
disabled
≠
控件状态不可被程序修改
```

也就是：

```text
用户不能改
程序仍然可以改
```

---

## 22. Disabled 时打开的 Popup 会立即关闭

还处理了：

```text
Popup Visible
    ↓
ComboBox 加 InteractionDisabled
    ↓
Popup Hidden
```

这里没有使用普通每帧 system，而是使用：

```rust
On<Add, InteractionDisabled>
```

即 lifecycle observer。

它表示：

```text
InteractionDisabled
刚被添加到某个 Entity
        ↓
如果该 Entity 是 ComboBox
        ↓
关闭它的 Popup
```

这也让我们进一步区分：

```text
普通 System
→ Schedule 到点运行

Observer
→ 对事件 / lifecycle 变化作出反应
```

---

## 23. World 直接修改与 Commands 延迟修改

disabled 测试中还明确了：

```rust
world.entity_mut(entity)
    .insert(InteractionDisabled);
```

是直接修改 World。

因此：

```text
Add<InteractionDisabled>
observer
```

会在这次操作过程中被触发，不需要：

```rust
app.update();
```

而如果使用：

```rust
commands
    .entity(entity)
    .insert(InteractionDisabled);
```

则是 deferred command，需要：

```text
flush / apply Commands
```

之后组件才真正进入 World。

---

## 24. Disabled 与内部 Button 的关系

当前 `InteractionDisabled` 只放在：

```text
ComboBox root
```

并没有同步到内部：

```text
Button
ListItem
```

因此理论上内部 Button 仍可能生成：

```text
Activate
```

但 ComboBox observer 会再次检查 root：

```text
InteractionDisabled？
→ 是
→ return
```

所以 ComboBox 自己仍能守住：

```text
disabled → 不打开
```

是否将 disabled 状态进一步同步给内部 Button / ListItem，暂时作为更完整的工程化设计留到以后，不属于本阶段必做范围。

---

## 25. 用户修改与程序修改必须是两条路径

这一阶段后半段形成了一个非常重要的设计：

```text
用户交互路径
≠
程序更新路径
```

用户路径：

```text
Pointer
↓
ListBox
↓
ValueChange<Entity>
↓
ComboBox
↓
Selected 更新
↓
ValueChange<usize>
```

程序路径：

```text
外部代码
↓
SetComboBoxSelected
↓
ComboBox
↓
Selected 更新
```

两者语义完全不同。

---

## 26. `SetComboBoxSelected`

为程序更新设计自己的 EntityEvent：

```rust
#[derive(EntityEvent, Debug)]
pub struct SetComboBoxSelected {
    #[event_target]
    pub entity: Entity,
    pub selected: Option<usize>,
}
```

例如：

```rust
SetComboBoxSelected {
    entity: combo_box,
    selected: Some(2),
}
```

表示：

```text
请把这个 ComboBox 的 selection 设置为 2
```

---

## 27. `EntityEvent` 与 `#[event_target]`

这一阶段还明确理解了：

```rust
#[event_target]
```

属于 **Bevy 事件路由层**，并不绑定业务语义。

例如：

```rust
SetComboBoxSelected {
    #[event_target]
    entity: Entity,
}
```

业务语义：

```text
这个 Entity 是我要修改的 ComboBox
```

而：

```rust
ValueChange<T> {
    #[event_target]
    source: Entity,
}
```

业务语义：

```text
这个 Entity 是产生变化的 Widget
```

虽然业务含义不同，但在 Bevy EntityEvent 层面，它们都代表：

```text
事件的 Entity 路由锚点
```

因此：

```text
event target
```

不要简单理解成：

```text
消息接收者
```

更准确地说是：

> 这个 EntityEvent 在 Entity hierarchy 中锚定在哪个 Entity 上。

---

## 28. Programmatic update 不受 disabled 限制

`SetComboBoxSelected` 不检查：

```rust
InteractionDisabled
```

因此：

```text
ComboBox Disabled
+
SetComboBoxSelected(Some(2))

→ selection 仍然改为 2
```

因为 disabled 的语义是：

```text
阻止用户交互
```

而不是：

```text
禁止程序更新控件
```

---

## 29. Programmatic update 不发 `ValueChange`

程序主动：

```text
set 2
```

以后，ComboBox 不再发：

```text
ValueChange(2)
```

否则容易形成：

```text
应用 set 2
↓
ComboBox 发 ValueChange(2)
↓
应用收到 ValueChange 又 set 2
↓
反馈环
```

因此确定契约：

```text
用户产生变化
→ ValueChange

程序同步状态
→ 不发 ValueChange
```

并为这一点单独写了测试。

---

## 30. Programmatic selection 的边界语义

最终定义：

```text
Some(valid_index)
→ 选中该 Option

None
→ 清除所有 selection

Some(invalid_index)
→ 整个请求忽略
→ 保持原 selection
```

例如：

```text
当前 Selected = 1

SetComboBoxSelected(Some(2))
→ 2

SetComboBoxSelected(None)
→ 无 selection

SetComboBoxSelected(Some(999))
→ 保持原值
```

---

## 31. 为什么非法 index 通过遍历 Option 判断

曾考虑在 root 保存：

```rust
ComboBoxOptionCount(usize)
```

然后：

```text
index < option_count
→ 合法
```

但最后没有采用。

原因是虽然当前 Option 是：

```text
0..option_count
```

连续生成的，但未来 ComboBox 很可能支持：

```text
动态增加 Option
动态删除 Option
```

如果保存 `option_count`，就又增加了一份需要持续同步的结构状态。

所以最终使用：

> 当前实际存在的 `ComboBoxOption` Entity 集合，就是合法 Option 集合。

程序 set 时先遍历获得实际 Option，再检查：

```text
Some(index)
是否真实存在
```

虽然多一次遍历，但 Option 数量通常很有限，换来了更低的状态同步复杂度。

---

## 32. `for &child` 与 Entity 引用

这一阶段还碰到一个 Rust / Bevy hierarchy 的实际类型问题。

例如：

```rust
children.iter()
```

得到：

```text
&Entity
```

而：

```rust
Query::get(...)
```

需要：

```text
Entity
```

所以：

```rust
for child in children.iter() {
    q_children.get(child); // &Entity，类型不对
}
```

改成：

```rust
for &child in children.iter() {
    q_children.get(child);
}
```

此时：

```text
&Entity
↓ pattern 解引用
Entity
```

也可以写：

```rust
for child in children.iter().copied()
```

但当前选择：

```rust
for &child
```

更直接。

---

## 33. 单元测试策略

Stage 3 的单元测试基本遵循：

```text
尽量从自己负责的语义边界开始
```

例如测试 ComboBox 的 Field 行为：

```text
直接 trigger Activate
```

而不是重复模拟完整 Button pointer 链。

测试 selection：

```text
直接 trigger ValueChange<Entity>
```

而不是重新验证 ListBox 的 Pointer Click 逻辑。

因此测试划分为：

```text
Bevy Button
负责 Pointer → Activate

Bevy ListBox
负责 Pointer → ValueChange<Entity>

我们的 ComboBox
负责：
Activate → Popup
ValueChange<Entity> → ComboBox selection
```

---

## 34. 当前单元测试覆盖的主要行为

Headless ComboBox 目前已经覆盖了：

```text
结构
├── Root 存在
├── Field 属于 Root
├── Popup 属于 Root
├── Options 属于 Popup
└── 初始 Selected 正确

Field
├── Activate → open
└── 再 Activate → close

Selection
├── ListBox ValueChange → Selected 更新
├── 旧 Selected 被移除
├── Popup 关闭
└── 对外发 ValueChange<usize>

Outside Click
├── 点击外部 → close
├── 点击 descendant → 不 close
└── 点击 root → 不 close

Disabled
├── disabled → Activate 无效
├── disabled → ListBox ValueChange 无效
├── selection 不变化
├── 不发 ValueChange
└── open 时变 disabled → 立即 close

Programmatic Selection
├── disabled 时仍可 set
├── Some(valid) → 修改
├── None → clear
├── Some(invalid) → ignore
└── programmatic set 不发 ValueChange

Ownership
└── Popup A + Option B → 整个事件忽略
```

---

## 35. 为什么 `Popup A + Option B` 值得测试

虽然正常用户点击基本不可能构造：

```text
Popup A
+
Option B
```

但库代码可能面对：

```text
其他 system 手动 trigger
未来内部重构
动态 hierarchy
多个 ComboBox 共存
```

因此库测试不仅验证：

```text
happy path
```

还要验证：

```text
状态不变量
Entity ownership
异常输入
多实例隔离
事件契约
```

这也是从普通业务测试向控件库测试思维的一次升级。

---

## 36. Integration Test：两个 ComboBox 协作

此前测试都放在：

```text
src/combo_box.rs
#[cfg(test)]
```

属于 crate 内部单元测试。

最后新增：

```text
tests/
└── combo_box.rs
```

作为真正的 integration test。

测试场景：

```text
ComboBox A
Popup = Visible

ComboBox B
Popup = Hidden

用户点击 B Field

最终：

A Popup → Hidden
B Popup → Visible
```

---

## 37. Integration Test 不访问 private 实现

这是这次集成测试很重要的一点。

`tests/combo_box.rs` 是作为 crate 外部编译的，所以不能访问：

```text
ComboBoxField
ComboBoxPopup
ComboBoxOption
```

这反而是好事。

测试通过公开组件识别内部角色：

```text
Root child + Button
→ Field

Root child + ListBox
→ Popup
```

因此测试的是：

> 外部用户真正能观察到的控件组合行为。

而不是依赖 private marker。

---

## 38. Integration Test 真正走 Pointer → Button → ComboBox

单元测试里我们直接：

```text
Activate
```

但集成测试故意走完整链：

```text
Pointer<Press>
↓
ButtonPlugin
↓
Pressed

Pointer<Click>
↓
ButtonPlugin
↓
Activate

Activate
↓
ComboBoxPlugin
↓
Popup B 打开
```

与此同时：

```text
Pointer<Click>
↓
ComboBox outside-click observer
↓
Popup A 判断这是 outside
↓
Popup A 关闭
```

最终验证多个官方 / 自定义 observer 同时协作时，整体行为仍然正确。

---

## 39. 为什么 Click 前还要 Press

官方 Button 的 click observer 并不是：

```text
Click
→ 无条件 Activate
```

而需要 Button 当前处于：

```text
Pressed
```

所以集成测试必须：

```text
Press
→ Click
```

而不能只：

```text
Click
```

这也说明：

> 单元测试直接触发语义事件，集成测试再验证真正的依赖组合。

两种测试职责不同。

---

## 40. 当前事件模型

目前 ComboBox 的事件流可以总结为三种方向。

### 用户打开

```text
Pointer
↓
Button
↓
Activate
↓
ComboBox
↓
Visibility
```

### 用户选择

```text
Pointer
↓
ListBox
↓
ValueChange<Entity>
↓
ComboBox
↓
Selected
↓
ValueChange<usize>
↓
应用
```

### 程序设置

```text
应用
↓
SetComboBoxSelected
↓
ComboBox
↓
Selected
```

其中：

```text
用户选择
→ 发 ValueChange

程序设置
→ 不发 ValueChange
```

这是当前状态管理模型最核心的边界之一。

---

## 41. 当前状态真相来源

本阶段最终形成的状态模型非常简单：

```text
是否打开
→ Popup.Visibility

当前选择
→ 哪个 Option 拥有 Selected

是否 disabled
→ ComboBox root 是否拥有 InteractionDisabled

Option 对应哪个 index
→ ComboBoxOption.index
```

没有额外保存：

```text
open: bool
selected: Option<usize>
current_popup: Entity
option_count
```

因此尽量减少状态同步问题。

---

## 42. Observer 与直接修改的职责

这一阶段也进一步明确了：

```text
已有 Component 内部值变化
→ &mut Component

Component 的添加 / 删除
→ Commands

对某种事件作出响应
→ Observer
```

例如：

```text
Visibility::Visible
→ Visibility::Hidden
```

是修改已有 Component：

```rust
&mut Visibility
```

而：

```text
插入 Selected
移除 Selected
```

是 Component presence 的变化：

```rust
commands.entity(entity).insert(Selected);
commands.entity(entity).remove::<Selected>();
```

---

## 43. Headless 与 Styled 的边界现在更加清楚

Headless ComboBox 当前负责：

```text
Entity structure
Button/ListBox/ListItem 组合
open / close
selection
outside click
disabled
programmatic set
semantic events
状态不变量
```

它不负责：

```text
Text
Font
BackgroundColor
BorderColor
hover visual
selected visual
popup geometry
padding
radius
icon
arrow
```

这些都属于下一阶段 Styled ComboBox。

---

## 44. Popover 暂未加入 Headless

虽然一开始考虑：

```text
Popup + Popover
```

但最终 Headless 构造阶段暂时没有加入 `Popover`。

原因是：

```text
Popup 相对 Field 如何定位
Popup 尺寸
偏移
层级展示
实际 UI layout
```

和 Styled / layout 层关系更紧。

所以当前 Headless 只表达：

```text
Popup 存在
Popup visible / hidden
```

下一阶段真正构建 Styled Popup 时，再引入 `Popover` 并研究实际定位行为。

---

## 45. 本阶段暂时没有解决的问题

这些不是遗漏，而是明确 defer：

```text
keyboard navigation
focus 行为
Escape close
Enter commit
active/highlighted option
完整 accessibility
动态增加 / 删除 options API
Popover 实际 positioning
公开 API 的最终形态
disabled 向内部 Button / ListItem 的同步
Styled UI
Theme
Visual Test
```

其中动态 Option 虽然本阶段没做，但 programmatic selection 的合法性检查已经避免强依赖固定 `option_count`，为以后留下了一点空间。

---

## 46. 这一阶段最重要的学习成果

如果压缩成几个核心认识，大概是：

```text
1. 高层 Headless Widget 往往不是一个 Entity，
   而是一组 Entity 的语义组合。

2. 能复用 Button / ListBox / ListItem，
   就不要重新实现它们。

3. State 尽量只保留一个事实来源：
   Visibility = open
   Selected = selection

4. Widget 内部 Entity 不应该泄露为外部 value，
   所以：
   Entity → semantic index

5. 用户修改和程序修改是两条不同的数据流：
   ValueChange
   vs
   SetComboBoxSelected

6. disabled 只禁止用户交互，
   不禁止程序状态同步。

7. 多 Entity Widget 必须维护 ownership invariant，
   不能只检查 Component 类型。

8. 单元测试从语义边界开始，
   集成测试再验证真实插件组合。

9. 控件库测试不仅测“能不能用”，
   还要测不变量、异常输入和多实例隔离。
```

---

## 47. Stage 3 目标 1 / 目标 2 当前状态

目前可以认为：

```text
Stage 3

目标 1：设计 Headless ComboBox
✅ 第一版完成

目标 2：Headless ComboBox 测试
✅ 第一版完成

目标 3：实现 ComboBox Style
⬜ 下一步

目标 4：Style / Visual Test
⬜ 后续
```

而且由于这次采用“设计一块、测试一块”的方式，目标 1 和目标 2 实际上已经形成一个比较完整的闭环。

下一步就可以正式离开纯 Headless 世界，开始做 **Styled ComboBox** 了。
