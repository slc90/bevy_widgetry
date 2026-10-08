# 01 建立独立 Widgetry Theme 与完整 AntD 参考配色

所属总方案：Widgetry 颜色、Theme 与有效 Disabled 重构方案

版本：1.0 · 方案基线：2026-10-08  
目标仓库：`slc90/bevy_widgetry`，核对提交 `5d33dd561a3c9bb171d7dd3238a41903a0724e33`。  
目标环境：Windows 64 位、`x86_64-pc-windows-msvc`、Bevy **0.19.1**、现有 DX12 配置。

这是一份实施方案，不是已经完成的代码改动。颜色组织、传播边界、公开接口、Disabled 的输入适配和 Gallery 展示均在本文确定；实施者不需要再选择架构方案。实际编译、测试、性能和 GUI 验证属于实施验收，本文不把源码核对当作运行验证。

## 目标

让颜色数据、Light/Dark 常量与主题选择拥有唯一的 theme crate 入口，并把当前控件的全部有效颜色槽位明确为完整数据结构。

## 范围

负责 theme crate、公开命名、完整 schema、固定配色、主题通知和必须同步的现有字段引用。此时不引入有效 Disabled 的层级计算，也不提前替换前景色通道。下文最终规则是后续接入必须遵循的边界，不表示本方案承担全部控件行为重构。

## 预期产出

可独立编译的新 theme crate；Light/Dark 完整 const；新的 facade 入口与 mode setter；现有消费者已改读新字段，不再引用旧 Theme 类型。局部数据、切换通知和非颜色行为回归通过。

## 与前后方案的关系

这是唯一执行链的第一份。完成后执行 [有效 Disabled 方案](02-effective-disabled.md)。Disabled 的 OR 计算并不依赖色值算法，这个先后是整体迁移顺序；后面的颜色接入方案同时依赖本方案的 Theme 数据和 Disabled 的实际状态。

## 最终行为与改造边界

### 颜色解析规则

每个控件保留自己的状态集合和状态优先级。先确定某个颜色属性当前适用的状态，再查找**该状态**的显式覆盖；未设置时取对应 Theme 字段。

例如 Button 背景遵循 `Disabled > Pressed > Hovered > Normal`。只设置 `normal.background = Some(红色)` 时，Hovered 仍取 `hovered.background` 的覆盖或 Theme，不回退到显式 Normal。透明颜色是一个有效颜色，`Some(Color::NONE)` 不等于未设置。

前景色分清两个角色：

| 角色 | 规则 |
|---|---|
| Button、CheckBox、TextField、列表行等样式主体 | 自身有效状态 → 同状态显式覆盖 > 自身或所属组合部件的 Theme 配色 |
| `WidgetryText`、`WidgetryIcon` 等内容 | 自身适用状态的显式覆盖 > 最近前景色作用域的已解析颜色 > Text/Icon 自身 Theme 配色 |
| 普通未标记的 Bevy Text | 不纳入 Widgetry 颜色管理，保留原生 `TextColor` 用法 |

继承的前景色是**一个已解析的颜色结果**，不是父控件的全部状态颜色表。内容不会重新把父级的 Disabled 色拿去和自己的 Normal Theme 作状态竞争。背景、边框、光标、文字选区和波形 palette 都不沿父子关系继承。

### 两种向下影响，不能混用

**有效 Disabled** 表示实际是否禁用：本地禁用请求、控件自身模型/原生能力限制与父级有效禁用做逻辑或。它穿过普通布局节点，也穿过独立控件边界，并同时约束输入和样式。

**前景色作用域** 表示一片内容使用哪个控件解析出的前景色。普通布局节点不建立新作用域；独立控件、列表行、单元格和弹层内容根可以建立新作用域。新的作用域覆盖外层颜色，而不是覆盖外层 Disabled。

因此，禁用 Button 内嵌 CheckBox 时：CheckBox 有效禁用为真，用 **CheckBox 的 Disabled 配色**；CheckBox 标签继承 CheckBox 的结果。Button 的其他普通标签则继承 Button 的 Disabled 配色。用户不需要逐个 Children 添加 Disabled。

不向下复制 `Hovered`、`Pressed`、`Focused`、`Checked`、`Selected`、`Open`。父 Button 的 Hovered 可以通过其最终前景色影响标签，但这不表示标签或内部 CheckBox 获得了 Hovered 交互状态。

### 明确保留和移除的内容

新增 `crates/theme`；Theme 按控件组织，并提供固定 Light/Dark 常量。采用 Ant Design 配色作为参考，不引入 React、CSS、运行时 Token 推导或 AntD 依赖。

保留有限的前景色自动继承，替换当前没有显式覆盖边界的颜色同步方式。不扩展为所有颜色的 Propagation，也不传播整套状态配色。前面讨论中的“彻底删除颜色继承”已被这个最终规则替代。

仅新增有效 Disabled 的状态继承，不做通用状态传播框架。保留普通 Text 的原生能力，增加零数据的 `WidgetryText` 标记作为明确的接管入口。

控件自有颜色覆盖配置和 Bevy 渲染颜色组件分开。构造时通过 BSN Props 初始化，运行时通过控件 API 修改；`BackgroundColor`、`BorderColor`、被接管的 `TextColor`、Icon 的 `ImageNode.color` 是输出，不作为覆盖配置的反向输入。

不抽象只有 `Option::unwrap_or` 的公共 resolver。每个控件保留本地 `resolve_*_style()` 与 `apply_*_style()`，多个刷新入口复用同一实现。

### 不在本次改造中做的事

不改尺寸、padding、字体、布局、窗口渲染后端、数据模型 identity、虚拟化算法、选择事件语义或 BRP 协议。不增加 AntD 的按钮 variant、错误校验状态、动画、渐变、阴影或项目没有使用的装饰颜色。

颜色继承不承担禁用输入；Disabled 继承不替代 Window modal 的原生 owner 管理。不跨原生窗口 owner 关系推导 UI 父子状态。不会为了验证而给 Linux/macOS 增加项目兼容代码。

## 独立 Theme crate 与公开命名

### 文件和依赖

```text
crates/theme/
  Cargo.toml
  src/
    lib.rs
    theme.rs
    common.rs
    text.rs
    icon.rs
    button.rs
    check_box.rs
    radio_group.rs
    text_field.rs
    combo_box.rs
    scroll_area.rs
    list_view.rs
    tree.rs
    table.rs
    tooltip.rs
    window.rs
    message_box.rs
    file_dialog.rs
    waveform.rs
```

package 名为 `bevy_widgetry_theme`。只依赖 workspace 的 `bevy` 与底层 `bevy_widgetry_log`；不依赖 core、控件、asset、Gallery。颜色没有运行时 asset，也不放入 asset crate。新 crate 使用 workspace 的版本、edition、lint 和 Windows 64 位入口检查。

core、各消费 Theme 的控件 crate、test_utils 按实际使用添加指向 theme 的 path dependency。facade 导出正式主题 API；库的任何下层 crate 不反向依赖 facade。Theme 的单元测试用最小 App，不通过 test_utils 绕回 core。

`theme.rs` 放汇总数据、选择资源、变化事件和 Plugin；`common.rs` 放共享配色常量。各控件文件定义该控件自己的颜色结构和 `LIGHT`、`DARK` 两个 `const` 实例。相同初始色值通过 common 常量复用，但控件字段保持独立，不能因为当前色值相同就省掉状态字段。

### 命名与导出

| 旧公开名称 | 新名称 |
|---|---|
| `ColorTheme` | `WidgetryTheme` |
| `ThemeMode` | `WidgetryThemeMode` |
| `ThemeChanged` | `WidgetryThemeChanged` |
| `ThemePlugin` | `WidgetryThemePlugin` |
| `LIGHT_THEME` / `DARK_THEME` | `WIDGETRY_LIGHT_THEME` / `WIDGETRY_DARK_THEME` |

各颜色数据结构使用 `WidgetryButtonColors`、`WidgetryTextFieldColors` 等前缀。控件内公开状态配色类型同样加 `Widgetry`；私有类型、字段、文件名不用机械添加前缀。

facade 新增 `bevy_widgetry::theme`，导出上述类型和各控件 Colors 类型。`style` 继续保留原有字体、z-index 等不相关入口，但不再导出旧 Theme 名称；不保留废弃别名。core 移除旧主题定义与旧导出。迁移包括 Gallery、integration tests、benchmark 和 test_utils 的 imports。

### 数据结构

`WidgetryTheme` 按 `text / icon / button / check_box / radio_group / text_field / combo_box / scroll_area / list_view / tree / table / tooltip / window / message_box / file_dialog / waveform` 分组。`common` 不成为需要运行时查找的颜色来源。

以 Button 为例，固定结构是：

```rust
// 数据结构示意；没有在本次文档任务中修改仓库。
pub struct WidgetryButtonStateColors {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

pub struct WidgetryButtonColors {
    pub normal: WidgetryButtonStateColors,
    pub hovered: WidgetryButtonStateColors,
    pub pressed: WidgetryButtonStateColors,
    pub disabled: WidgetryButtonStateColors,
}
```

其他控件不强行套用四状态。CheckBox 是选中形态与交互状态的组合；Table 的焦点边框独立于背景状态；Waveform 保留 palette 序列。Theme 所有已定义颜色字段都是确定值，不用 `Option<Color>` 表示 Theme 缺省。Waveform 的固定 palette 使用 `&'static [Color]`，运行期覆盖仍可使用拥有数据的 `Vec<Color>`。

汇总 Theme 和所有固定颜色结构支持 Clone、Debug、PartialEq；可复制的数据支持 Copy。Light 与 Dark 必须构造相同 schema 的完整实例，遗漏字段由类型检查阻止。

### 主题选择与通知

`WidgetryThemeMode` 保持 `Light / Dark`，默认 Dark；`colors()` 返回对应静态 `&'static WidgetryTheme`。`WidgetryThemeChanged { mode }` 仍是全局 Event，不改为层级 EntityEvent。

提供 `WidgetryThemeMode::set_in_world(world, mode) -> Result<bool, BevyError>` 和 `set(commands, mode)`。实际执行时先写 mode，再触发变化通知；同值不通知。保留官方 Resource 可直接访问的能力，但“只改资源不通知”不属于完整的主题切换操作。Gallery 和库内切换统一走这两个入口，消除 mode 与 event payload 不一致的内部路径。

`WidgetryThemePlugin` 仅初始化主题管理，不读取控件状态、不写 UI 颜色，也不依赖 core。`WidgetryUiPlugin` 负责装配 core 的 Disabled、前景色和标记文字支持，并保证 ThemePlugin 已注册。所有独立 Widget Plugin 按需装配 `WidgetryUiPlugin`，包括此前只直接装配 PointerPlugin 的 TextField/Tooltip。不要求使用者手工补内部 Plugin，不重复注册。

`set_in_world` 提交主题资源不等于整个 UI 已完成渲染。已有控件的 Theme observer 可立即调用自己的 apply；前景色传递、新内容 materialize 和最终文字/图标颜色在本帧明确的颜色阶段收敛，不依赖多个 observer 的碰巧执行顺序。

### 迁移兼容边界

这是 v0.1 项目的有意 API 迁移，不同时维护两套 Theme 数据。引入新 Theme 时，现有 resolver 先机械改读新字段并迁移 imports，使这一阶段可以独立编译；显式覆盖和新的继承消费语义在后续方案接入。旧 Foreground 实现可以在该阶段暂时继续运行，但最终交付没有旧公开名称的兼容层。新的覆盖配置、内容继承和全部控件消费端作为同一个颜色接入方案交付，避免先删除旧 writer，却把依赖它的子内容留到另一个方案才修复。

Table、FileDialog、Waveform 的非颜色 Style 配置保留原有职责；不要为了迁移颜色顺手重新设计高度、overscan、字体大小、padding 或 line width。涉及 workspace 角色和依赖变化，同步 `docs/architecture.md`。

## AntD 参考配色与完整颜色清单

### 配色基线和取舍

参考 Ant Design **6.6.5** 的颜色规范、主题 Token 文档、Button/Input/Table 的状态用色，以及该版本 dark neutral palette 源码。它是视觉依据，不是运行依赖，也不承诺和浏览器中的 AntD 逐像素相同。文档末尾列出固定版本源码和官方页面。

本次采用中性色界面、蓝色强调、浅色白底和深色分层灰底。普通 Button 保留中性背景，不额外引入 AntD 的 primary/danger/ghost 等 variant。勾选标记、活动边框和选区承担强调，不让每种状态的正文都换一种鲜艳颜色。

为减少原生渲染与 CSS 透明混色差异，正文、主要背景和边框使用下表的**确定不透明 sRGB 色值**；这些值是 Widgetry 的 AntD-inspired 配色，其中正文灰度等有适配，不宣称全部等于 AntD 原始 Token。透明只用于没有背景/边框的槽位，以及已有图片 opacity。不同状态的字段可以赋同一个常量，字段完整不等于必须制造视觉差别。

### `common.rs` 的固定色值

下表符号仅用于本方案压缩配色矩阵；代码使用左列语义名称。非透明字段以 `Color::srgb_u8` 定义。Light/Dark 各有一组 common 常量，各控件的 const 实例直接引用。

| common 字段（符号） | Light | Dark | 用途 |
|---|---|---|---|
| `window_background`（W） | `#FFFFFF` | `#141414` | 窗口、常规内容底色 |
| `surface`（S） | `#FFFFFF` | `#141414` | 普通输入控件表面 |
| `elevated_surface`（E） | `#FFFFFF` | `#1F1F1F` | Popup、对话内容 |
| `hover_surface`（H） | `#F5F5F5` | `#262626` | 普通 Hovered 背景 |
| `pressed_surface`（P） | `#EBEBEB` | `#303030` | 普通 Pressed 背景 |
| `disabled_surface`（D） | `#F5F5F5` | `#1F1F1F` | 禁用表面 |
| `border`（B） | `#D9D9D9` | `#424242` | 普通边框 |
| `subtle_border`（B2） | `#F0F0F0` | `#303030` | 分隔和单元格边框 |
| `disabled_border`（BD） | `#D9D9D9` | `#424242` | 禁用边框 |
| `text`（T） | `#1F1F1F` | `#DCDCDC` | 正文、默认图标 |
| `secondary_text`（T2） | `#595959` | `#ADADAD` | 说明、状态文字 |
| `tertiary_text`（T3） | `#8C8C8C` | `#7E7E7E` | 弱强调图标 |
| `disabled_text`（TD） | `#BFBFBF` | `#595959` | 禁用文字/标记 |
| `inverse_text`（TI） | `#FFFFFF` | `#FFFFFF` | 有色标记、Tooltip 文字 |
| `primary`（A） | `#1677FF` | `#1668DC` | 选中标记填充 |
| `primary_hover`（AH） | `#4096FF` | `#3C89E8` | 交互边框、标记 Hovered |
| `primary_pressed`（AP） | `#0958D9` | `#1554AD` | 按下强调 |
| `focus_border`（AF） | `#1677FF` | `#3C89E8` | Focused/Active 边框 |
| `selected_surface`（AS） | `#E6F4FF` | `#111A2C` | Selected 背景 |
| `selection_background`（SE） | `#BAE0FF` | `#15395B` | 文本选区 |
| `unfocused_selection_background`（SU） | `#D9D9D9` | `#303030` | 失焦文本选区 |
| `tooltip_background`（TT） | `#262626` | `#424242` | Tooltip 高对比内容底色 |
| `danger_hover`（RH） | `#D9363E` | `#D9363E` | 窗口关闭按钮 Hovered |
| `danger_pressed`（RP） | `#A8071A` | `#A8071A` | 窗口关闭按钮 Pressed |
| `transparent`（Z） | `#00000000` | `#00000000` | 明确不绘制，不表示缺省 |
| `image_tint`（IT） | `#FFFFFF` | `#FFFFFF` | 保留原图片颜色，opacity 仍由已有配置控制 |

Waveform 的普通 palette 固定为：Light `[#0958D9, #08979C, #389E0D, #AD4E00]`；Dark `[#4096FF, #36CFC9, #95DE64, #FFC069]`。禁用 palette：Light `[#8C8C8C, #A6A6A6, #BFBFBF, #D9D9D9]`；Dark `[#595959, #737373, #8C8C8C, #A6A6A6]`。保留四通道起始配置和按序循环，不能把 palette 简化为一个 foreground。

未被实际控件消费的 common 字段不需要额外公开。颜色数值由此表固定，不在实施时另行选择“差不多”的色值。

### 状态表的统一记法

`(background, border, foreground)` 是表面三元组；只有列明的部件才有这些字段。若某部件只有 background，就只定义一个字段，不为了统一形式增加无用途字段。

通用矩阵 M 的完整四行是：Normal `(S,B,T)`，Hovered `(H,AH,T)`，Pressed `(P,AP,T)`，Disabled `(D,BD,TD)`。这是**const 值复用说明**，不是运行时公共 resolver，也不是强制统一所有控件状态。

列表项矩阵 R 的完整五行是：Normal `(Z,Z,T)`，Hovered `(H,Z,T)`，Pressed `(P,Z,T)`，Selected `(AS,Z,T)`，Disabled `(Z,Z,TD)`。活动边框作为正交字段 `active_border = AF`、`disabled_active_border = Z`。背景优先级维持 `Disabled > Pressed > Hovered > Selected > Normal`，不因为引入新配色改变原先的选择行为。

### 全部 Theme 分组、部件与字段

以下清单就是需要实现的颜色 schema。表中引用其他分组的颜色结构，只复用**数据类型与初始 const 值**；每个拥有者都有自己的字段，不反向依赖 Widget crate，不把宿主 Theme 值写进子 Widget 的用户覆盖配置。

| 分组 | 部件和状态 | 完整颜色字段及确定配色 |
|---|---|---|
| `text` | `normal / disabled` | `foreground = T / TD`；独立标记 Text 只支持这两种外观状态 |
| `icon` | `normal / disabled` | `foreground = T / TD`；资产内颜色仍乘 ImageNode tint |
| `button` | `normal / hovered / pressed / disabled` | 每行 `background, border, foreground`，使用 M |
| `check_box` | `unchecked / checked / indeterminate`，各自包含 `normal / hovered / pressed / disabled` | 每行 `background, border, foreground, mark`；background/border 指 indicator，foreground 指内容。Unchecked 普通三行为 M 的前三行并设 mark=Z；Unchecked Disabled 为 `(D,BD,TD,Z)`。Checked 与 Indeterminate：Normal `(A,A,T,TI)`，Hovered `(AH,AH,T,TI)`，Pressed `(AP,AP,T,TI)`，Disabled `(D,BD,TD,TD)`。不改变勾/横线的 visibility 规则 |
| `radio_group.container` | `normal / focused / disabled` | `(S,B,T)`、`(S,AF,T)`、`(D,BD,TD)` |
| `radio_group.option` | `unchecked / checked`，各含 `normal / hovered / disabled` | 每行 `background, border, foreground, dot`。Unchecked 为 `(S,B,T,Z)`、`(S,AH,T,Z)`、`(D,BD,TD,Z)`；Checked 为 `(S,A,T,A)`、`(S,AH,T,AH)`、`(D,BD,TD,TD)`。这里不新增 Pressed 状态 |
| `text_field.editable` 与 `text_field.read_only` | 各含 `normal / hovered / focused / disabled` | 每行 `background, border, foreground, caret, selection_background, unfocused_selection_background, selection_foreground`。表面为 `(S,B,T)`、`(S,AH,T)`、`(S,AF,T)`、`(D,BD,TD)`；普通三行 caret=T，禁用 caret=TD；选区 SE/SU，selection_foreground 普通=T、禁用=TD。ReadOnly 拥有独立字段但初始同色，不因此获得编辑能力 |
| `list_view.container` | `normal / focused / disabled` | `(S,B,T)`、`(S,AF,T)`、`(D,BD,TD)` |
| `list_view.item` | R 五状态和独立活动边框 | 使用 R；item 的 active 不覆盖背景状态 |
| `tree.container`、`tree.item` | 同 ListView 的容器和行 | 自己的字段，初始复制 ListView 配色 |
| `tree.expander` | `collapsed / expanded`，各含 M 四状态 | 两种形态都取 M，但默认 background=Z、border=Z；Hovered/Pressed 使用 H/P，foreground=T；Disabled 为 `(Z,Z,TD)`。展开形态只决定 SVG，不能继承 Checked |
| `table.table / column_header / row_header / corner / cell` | 每个 region 含 `normal / hovered / selected / disabled`，以及 `focused_border / disabled_focused_border` | `table` Normal `(S,B,T)`；表头与 corner Normal `(H,B2,T)`；cell Normal `(Z,B2,T)`。所有 region Hovered `(H,B2,T)`，Selected `(AS,B2,T)`，Disabled `(D,BD,TD)`。focused_border=AF，disabled_focused_border=BD。背景保持 Disabled > Hovered > Selected > Normal，焦点边框独立选择 |
| `combo_box.field` | `normal / hovered / pressed / open / disabled` | 每行 `background, border, foreground, indicator_foreground`。前面三行和禁用取 M；Open `(S,AF,T)`；indicator 普通=T3，Open=T，Disabled=TD。优先级 Disabled > Open > Pressed > Hovered > Normal |
| `combo_box.popup` | `normal / disabled` | `(E,B,T)`、`(E,BD,TD)`；禁用时 Popup 仍应关闭，Disabled 字段用于完整数据和过渡一致性 |
| `combo_box.list` | ListView 全部字段 | 初始复制 ListView，但 container.background=E；不套用普通 Button 的默认配色 |
| `scroll_area.horizontal` 与 `vertical` | 每轴 `track.normal / disabled` 和 `thumb.normal / hovered / dragged / disabled` | track 只有 background，均=Z；thumb 只有 background，依次 B、AH、AP、BD。两轴分别可覆盖；拖动是此部件已有状态，不向 Children 复制 |
| `tooltip.popup` | 仅 `normal` | `(TT,Z,TI)`；不新建交互 Disabled 外观，禁用 anchor 仍可显示说明 Tooltip |
| `window.frame` | `normal / disabled` | `background, border, foreground`：`(W,B,T)`、`(W,BD,TD)`；只有 Theme 背景模式绘制 background |
| `window.title_bar` | `normal / disabled` | `background, border, foreground`：`(Z,B,T)`、`(Z,BD,TD)`；border 用于原来的底部分隔 |
| `window.minimize / maximize` | 每个含 M 四状态，但只有 background/foreground | Normal `(Z,T)`，Hovered `(H,T)`，Pressed `(P,T)`，Disabled `(Z,TD)` |
| `window.close` | 四状态，只有 background/foreground | Normal `(Z,T)`，Hovered `(RH,TI)`，Pressed `(RP,TI)`，Disabled `(Z,TD)`；替换当前硬编码红色 |
| `window.image_tint` | 一个 Color | IT；已有图片透明度与该 tint 的 alpha 相乘，不由切换 Theme 覆盖图片/模式 |
| `message_box.window` | 完整 WindowColors | 自己的字段，初始复制 Window；不改变独立原生窗口与 modal 行为 |
| `message_box.body` | `normal / disabled` | 只有 `background, foreground`，分别 `(E,T)`、`(E,TD)`；不新增当前未绘制的 body 边框 |
| `message_box.action_button` | 完整 ButtonColors | 初始复制 M。现有 OK/Yes/No/Cancel 共用此配置，不新增未存在的 danger/primary 语义 |
| `file_dialog.window` | 完整 WindowColors | 自己的字段，初始复制 Window |
| `file_dialog.body` | `normal / disabled` | 只有 `background, foreground`，分别 `(W,T)`、`(W,TD)` |
| `file_dialog.sidebar_button` | 完整 ButtonColors | 当前位置/固定位置实际使用 WidgetryButton，采用 M；不伪造不存在的 Selected 行状态 |
| `file_dialog.entry` | `normal / selected / disabled` 和 `active_border / disabled_active_border` | Normal `(Z,Z,T)`，Selected `(AS,Z,T)`，Disabled `(Z,Z,TD)`；活动边框 AF/Z。背景优先级 Disabled > Selected > Normal；当前行没有 Hovered/Pressed writer，本轮不凭空添加 |
| `file_dialog.toolbar_button / accept_button / cancel_button` | 每个完整 ButtonColors | 初始 M，各自可覆盖；toolbar 包含 Back/Forward/Up/Refresh/Pin/NewFolder |
| `file_dialog.folder_create_button / folder_cancel_button / overwrite_accept_button / overwrite_cancel_button` | 每个完整 ButtonColors | 初始 M，分别用于内嵌 NewFolder/Overwrite 面板已有动作 |
| `file_dialog.path_field / search_field / filename_field / folder_name_field` | 每个完整 TextFieldColors | 初始复制 TextField，分别对应路径、搜索、文件名和新目录名编辑区域 |
| `file_dialog.filter / sort` | 各自完整 ComboBoxColors | 初始复制 ComboBox，保持过滤和排序业务语义 |
| `file_dialog.hidden_option / system_option` | 各自完整 CheckBoxColors | 初始复制 CheckBox，分别控制 Hidden/System 选项，不合并业务状态 |
| `file_dialog.sidebar_scroll / entries_scroll` | 各自完整 ScrollAreaColors | 初始复制 ScrollArea；两块已有滚动区域分别可覆盖 |
| `file_dialog.confirmation` | 完整 MessageBoxColors | 独立窗口模式现有覆盖确认 MessageBox 使用该记录；内嵌面板继续使用 body 和对应动作字段 |
| `file_dialog.status` | `normal / disabled` | `foreground = T2 / TD`；不新增错误/成功状态机器 |
| `waveform` | `normal / disabled` | 每行 `background` 与 `palette`，background 为 S/D，palette 见[固定配色表](01-theme-and-palette.md)；line_width 不进入 Theme |

FileDialog 或 MessageBox 内部未来没有实例化的部件不凭空创建。上述字段绑定到当前的工具栏、路径/搜索/文件名/新目录名、过滤/排序、Hidden/System 选项、侧栏位置按钮、滚动区、确认面板与窗口结构；隐藏的文件名栏或面板仍使用同一配置，在显示时获取当前 Theme。

### 完整性与视觉验收

Light、Dark 都必须覆盖上表每个实际状态槽位。正常字段不能因为最终等于 T 而省略 Hovered/Pressed foreground。不要往所有控件里硬塞 Focused/Open 等不适用状态。

按正常正文的实际背景检查可读性；本方案将普通表面文字固定为 T，选区文字也使用 T，避免亮蓝色小字。可读性检查的目标是正常正文对比度至少 4.5:1；禁用内容和用户显式指定颜色不据此自动改色。这里只设验收条件，不声明已完成 Windows 最终渲染对比验证。AntD 参考不等于自动获得无障碍合规认证。

Gallery 的配色展示必须取这里的 const 数据，不维护第二份 HEX 表。只有窗口图片染色和 Color::NONE 保留透明，透明色块用棋盘底显示，不能误显示成黑色。

## 迁移收尾、验证与交付标准

### Windows 上的实施验收

在 PowerShell 7 中完成适用的验证：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo build --workspace`、`cargo test --workspace`。按 crate 运行局部测试不能代替全 workspace 最终检查。

`cargo run -p widget_gallery` 后，通过项目现有 BRP 流程实际操作各页 Light/Dark、状态、覆盖/清除和禁用容器，并保存截图。不得仅凭示意色块或静态源码宣称 GUI 通过。需要同时检查 Mouse 与项目支持的其他 Pointer 路径，保留 reactive desktop update mode。

实施者遵守仓库 development/testing/logging/documentation 等适用规则，创建并维护实际任务进度记录；方案生成阶段不在目标仓库创建进度文件。代码完成后的独立 Code Review 按 AGENTS 现行流程执行，review 与修复均不由本次文档交付冒充。

## 本方案局部验证

以下是本方案交付时必须通过的局部行为检查，不能留到全部方案结束后才验证。完整 workspace、GUI 和独立审查的最终门槛见收尾方案；这里的检查不代表已经执行。

| 领域 | 必须覆盖的场景 | 通过条件 |
|---|---|---|
| Theme 数据 | 两主题、每个实际部件/状态字段、palette | 完整可构造、颜色有限、固定数值与配色表一致 |
| Theme 切换 | Dark→Light→Dark、同值、先更新 mode 再通知 | 配色正确，同值不产生多余通知，已有业务状态不被重置 |

## 已核对依据与来源

以下资料用于核对当前实现和外部规范。本文的新增 schema、配色适配、API 与有效状态 bridge 是本次确定的设计，不声称来源仓库已经实现。

### 项目源码与约束

项目源码链接统一固定在核对提交，避免后续 main 移动改变本方案依据。

- [Workspace 依赖与目标版本](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/Cargo.toml)：Bevy 0.19.1、现有 crates 与 BRP 版本。
- [项目背景](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/rules/project-context.md)、[Widget API 约束](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/rules/widget-api.md)、[架构约束](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/rules/architecture.md)：Windows-only、BSN、官方组件例外与依赖边界。
- [原 Theme](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/core/src/theme.rs)、[原 Foreground](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/core/src/foreground.rs)、[Icon](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/core/src/icon.rs)：现有颜色来源、直接写 TextColor 与白色 tint fallback。
- [ComboBox Field](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/combo_box/src/field.rs)、[注册/输入调度](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/combo_box/src/registration.rs)：复用 Button 与既有 Disabled 镜像。
- [ListView 物化](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/list_view/src/virtualization.rs)、[TreeView](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/tree/src/view.rs)：模型到 UI 的禁用来源与动态内容。
- [Pointer press 生命周期](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/core/src/pointer/pressed.rs)：真实 Pointer 会话与禁用取消。
- [FileDialog 内部部件与可用性](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/file_dialog/src/controls.rs)、[条目行](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/file_dialog/src/view.rs)：Sidebar 实际为 Button，过滤/排序/选项/新目录面板，以及选中和活动边框的独立颜色。
- [Tooltip](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/tooltip/src/style.rs)、[MessageBox](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/message_box/src/scene.rs)、[Window Close](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/crates/window/src/title_bar/close.rs)：Popup 内容作用域、独立 modal 根、硬编码关闭色。
- [Gallery 页面](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/gallery/src/gallery.rs)、[Gallery 主程序](https://github.com/slc90/bevy_widgetry/blob/5d33dd561a3c9bb171d7dd3238a41903a0724e33/gallery/src/main.rs)：既有导航、主题切换与运行模式。

### Bevy 0.19.1

- [官方 Button 源码](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ui_widgets/src/button.rs)：输入 observer 直接消费 InteractionDisabled，不能只改自有 Style bool。
- [UI Plugin 调度](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ui/src/lib.rs)、[Text Plugin 调度](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_text/src/lib.rs)：UiSystems::Content、EditableTextSystems 和文字检测的顺序。
- [InteractionDisabled 官方说明](https://docs.rs/bevy/latest/bevy/ui/struct.InteractionDisabled.html)：禁用交互不等同于停止更新/渲染，也不自动排除键盘焦点；该文档页会随发布移动，本方案的实际输入适配以已固定的 0.19.1 源码为准。
- [Observer / World::trigger 源码](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/src/observer/mod.rs)：通知与 command flush 是不同操作，不把调用 trigger 本身视作完成整棵 UI 的同步。

### AntD 与可读性

- [Ant Design 颜色规范](https://ant.design/docs/spec/colors/) 与 [深色模式](https://ant.design/docs/spec/dark/)：颜色职责和明暗层级。
- [主题 Token](https://ant.design/docs/react/customize-theme/) 与 [6.6.5 dark neutral palette 源码](https://github.com/ant-design/ant-design/blob/6.6.5/components/theme/themes/dark/colors.ts)：基础/语义/控件配色关系与暗色中性色。
- [Button](https://ant.design/components/button/)、[Input](https://ant.design/components/input/)、[Table](https://ant.design/components/table/)：本次状态配色参考；不迁入它们的 React 实现。
- [W3C 文字对比度说明](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)：正常正文 4.5:1 的验收参考及非活动控件的区别。此项不是对 Widgetry 的合规认证。
