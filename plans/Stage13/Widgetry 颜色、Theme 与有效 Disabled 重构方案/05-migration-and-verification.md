# 05 完成 workspace 迁移收尾与全链行为验收

所属总方案：Widgetry 颜色、Theme 与有效 Disabled 重构方案

版本：1.0 · 方案基线：2026-10-08  
目标仓库：`slc90/bevy_widgetry`，核对提交 `5d33dd561a3c9bb171d7dd3238a41903a0724e33`。  
目标环境：Windows 64 位、`x86_64-pc-windows-msvc`、Bevy **0.19.1**、现有 DX12 配置。

这是一份实施方案，不是已经完成的代码改动。颜色组织、传播边界、公开接口、Disabled 的输入适配和 Gallery 展示均在本文确定；实施者不需要再选择架构方案。实际编译、测试、性能和 GUI 验证属于实施验收，本文不把源码核对当作运行验证。

## 目标

确认主题、有效禁用、覆盖、前景色、控件组合和 Gallery 已连成一条可工作的链路，没有旧 API、旧 writer 或只变色不禁用的遗漏。

## 范围

负责跨 workspace 残留核查、架构/使用说明同步、全量行为测试、性能回归、Windows/BRP 可视验证和独立 Code Review。前面各方案的局部测试仍须各自完成；本份不以最终验收替代局部正确性，也不增加新的产品功能。

## 预期产出

统一的新 API 与依赖图；无旧颜色/禁用镜像残留；规定的行为矩阵、性能检查、workspace 构建测试、实际 Gallery 交互与审查证据齐全。失败必须修复实现，不能改动已确定契约降低门槛。

## 与前后方案的关系

这是唯一执行链的最后一份，承接 [Gallery 方案](04-gallery-color-showcase.md) 及前三份的实现结果。这里列出的验收尚未执行；它是实施交付条件，不是本次文档生成的测试报告。

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

### 迁移兼容边界

这是 v0.1 项目的有意 API 迁移，不同时维护两套 Theme 数据。引入新 Theme 时，现有 resolver 先机械改读新字段并迁移 imports，使这一阶段可以独立编译；显式覆盖和新的继承消费语义在后续方案接入。旧 Foreground 实现可以在该阶段暂时继续运行，但最终交付没有旧公开名称的兼容层。新的覆盖配置、内容继承和全部控件消费端作为同一个颜色接入方案交付，避免先删除旧 writer，却把依赖它的子内容留到另一个方案才修复。

Table、FileDialog、Waveform 的非颜色 Style 配置保留原有职责；不要为了迁移颜色顺手重新设计高度、overscan、字体大小、padding 或 line width。涉及 workspace 角色和依赖变化，同步 `docs/architecture.md`。

## AntD 参考配色与完整颜色清单

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

## 有效 Disabled 的继承与 Bevy 输入适配

### 数据和含义

core 新增 `disabled.rs`，对外提供只读 `WidgetryEffectiveDisabled` 查询。内部保存两个本地原因：用户明确请求的 `explicit_disabled`，以及当前 Widget 模型/原生能力给出的 `intrinsic_disabled`。内部原因不包含父级结果。

```text
local_disabled(e) = explicit_disabled(e) OR intrinsic_disabled(e)
effective_disabled(e) = local_disabled(e) OR effective_disabled(parent(e))
```

没有父节点时右侧父值为 false。普通布局节点同样传递这个值；独立 Widget 的前景色作用域**不能**截断 Disabled。子级本地 false 无法覆盖祖先 true。不增加“强制启用”的第三态。

计算范围是安装 Widgetry UI 能力后的实际 UI hierarchy。遍历可以穿过没有 Widgetry 标记的中间节点；只为 UI 节点及必要的桥接祖先维护记录，不给十万条离屏数据模型逐条附加组件。原生 Window 的 owner 关系不等于 ChildOf，不通过它传播。

### 保留原生 BSN 用法，并精确记录本地意图

用户仍可在 BSN 写 `InteractionDisabled`，也可通过正常 World/Commands 插入或移除这个官方组件。不要求替换成新 WidgetryDisabled 控件，不要求逐个 Children 操作。

但是在受管理的 UI 节点上，`InteractionDisabled` 的稳定态存在性将表示**实际禁用**，供 Bevy 官方输入系统和 accessibility 消费；用户本地意图单独保存在内部记录中。读取 `WidgetryEffectiveDisabled` 得到同一实际结果。

具体识别规则：监听 `Insert<InteractionDisabled>` 和 `Remove<InteractionDisabled>`，不能仅监听 Add。外部 Insert（包括组件已经因继承存在时的再次 Insert）记录 explicit=true；外部 Remove 记录 explicit=false。core 自己投影组件时使用一个仅覆盖同步结构写入的内部 guard，忽略这些投影产生的 lifecycle 通知，避免把继承结果误记成本地请求。

Remove 通知时旧组件可能仍可查询，因此这里只记录请求和标脏，不能靠该时刻的 `Has<InteractionDisabled>` 再推断最终本地意图。记录失效时合并并排队一个 reconcile command，在当前结构写入返回后由 command queue / World::flush 执行；PreUpdate/PostUpdate 的显式同步 pass 作为确定的调度边界。不能在 Remove callback 内递归重插同一组件，也不把尚未 flush 的 World::trigger 或 Commands 调用说成投影已经完成。内部 guard 限定为当前投影的 Entity 与操作，不忽略同一回调链对其他实体的真实本地请求。插件晚装配时，对已有 UI 的现存组件做一次本地请求初始化；由插件投影的节点必须先建立内部记录，不允许后续初始化把投影误认成外部请求。

例如父级禁用期间，用户给子控件再次插入 InteractionDisabled，父级恢复后子控件仍禁用；用户在父级禁用期间移除子控件的本地请求，子控件当时仍实际禁用，父级恢复后才启用。这两种情况都必须测试。

这是新增继承语义所必需的来源记录，不把所有官方组件纳入自有 state API 治理。`rules/widget-api.md` 中增补这一个受管理 UI 的投影说明，明确原生插入/移除入口仍受支持，以及查询观察到的是实际结果；其他官方组件例外不变。

### 输入、编辑、焦点与运行期变化

PreUpdate 的禁用同步必须在 `PickingSystems::ProcessInput` 与 `InputFocusSystems::Dispatch` 之前完成，包含必要的 deferred flush。PostUpdate 在 Build/Materialize 产生新节点后再同步一次，先于样式和 `EditableTextSystems`。不保证尚未 apply 的 Commands 已改变世界；要求一次 World flush 和下一次正常输入派发前状态已一致。

禁用只阻止对应 Widget 的用户操作，不停止渲染、Theme 更新、模型修复或原来允许的程序化 setter。按既有控件契约保留只读、程序化选择、programmatic scroll 等能力。不要把 disabled 自动变成 Visibility::Hidden、Pickable::IGNORE 或清空 Focus；官方 InteractionDisabled 本就不要求禁止获取键盘焦点。

已经开始的 pointer press、scrollbar drag、table resize 和文字输入会话，在有效禁用转为 true 后不得继续提交操作。沿用现有 owner/cancel 路径清理库拥有的临时会话，不误清理项目已经约定保留的外部 Pressed 状态。TextField 在应用 pending edits、异步粘贴结果前重新检查有效禁用。ComboBox 禁用后关闭 Popup，保留既有隐藏弹层焦点处理；Tooltip 仍可说明禁用原因，不因为 anchor 禁用而被全局隐藏。

新增 Children、重新挂父级、脱离父级、祖先移除禁用、同帧 Remove 后 Insert、祖先 despawn 都触发重新计算。标脏合并同一棵子树，不对每个后代反复回溯整条祖先链。稳定帧不重写组件。生命周期 observer 只记录失效范围，实际结果以同步点的最终 hierarchy 和本地请求为准。

### 必须锁定的行为例子

父 disabled=true，子 explicit=false：父子都实际禁用。父恢复 false：子恢复启用。

父 disabled=true，子 explicit=true：父恢复后子仍实际禁用。即使子 explicit=true 是父禁用期间才设置的，也相同。

子本地未禁用，在两个容器之间 reparent：实际值立即按新 ancestry 在同步点重算。根本地状态不能从上一次投影结果反向推断。

Button 内的 CheckBox 因父级实际禁用后，用 CheckBox 自己的 Disabled 配色；用户不需重复给 CheckBox、标签、Icon 添加 Disabled。普通 Text 只是继承颜色或维持自己的显式颜色，变灰不构成输入禁用的证据。

跨原生窗口的 modal 对话框不继承被 OS 禁用的 owner 窗口的 UI Disabled，仍可以操作。该边界不修改已有 modal 系统。

## 显式颜色覆盖与公开 API

### 对应 Theme schema 的覆盖值

每个 Widget 提供 `WidgetryXxxColorOverrides` 公开值类型，与自己的 Colors 部件/状态结构对应，颜色叶子为 `Option<Color>`。Waveform 的 palette 叶子为 `Option<Vec<Color>>`。所有 None 表示没有覆盖，不提前把 Theme 默认值拷进去。

Theme 类型定义在 theme crate；覆盖值、持久的覆盖状态和更新 API 定义在所属 Widget crate。Text/Icon 的覆盖与行为定义在 core。内部持久组件字段私有，对外只读。公开覆盖值是可构造的数据，不等于允许外部绕过 setter 直接改运行期 authority。

`Normal` 是一个具体状态，不是其他状态的 fallback。每个状态内逐颜色属性独立选择：只覆盖 Hovered.background，不影响 Hovered.border 和 Hovered.foreground，也不影响其他状态。

### 固定的更新接口

为所有覆盖值类型提供同一组入口，以 Button 为例：

```rust
WidgetryButtonColorOverrides::get(&World, Entity)
    -> Result<&WidgetryButtonColorOverrides, BevyError>;
WidgetryButtonColorOverrides::set_in_world(
    &mut World, Entity, WidgetryButtonColorOverrides
) -> Result<bool, BevyError>;
WidgetryButtonColorOverrides::set(
    &mut Commands, Entity, WidgetryButtonColorOverrides
);
WidgetryButtonColorOverrides::clear_in_world(&mut World, Entity)
    -> Result<bool, BevyError>;
WidgetryButtonColorOverrides::clear(&mut Commands, Entity);
```

这是类型签名约定，不是本次已编译的实现代码。接口挂在覆盖值类型上，使 Window、MessageBox 等使用 scene function 的控件也有统一入口，无需为了方法挂载再造一个窗口控件类。

`set` 是**完整替换覆盖配置**，不是含糊的 patch。局部更新时，调用方 clone 只读值、修改目标字段，再 set。清除一个字段就是在该副本中设为 None；clear 清空全部覆盖。避免让 None 同时表示“保持原值”和“清除此字段”。

更新先验证 Entity、Widget 类型和输入，再原子提交配置并标脏。同值返回 Ok(false)，变化返回 Ok(true)，无效目标/非法数值返回 BevyError 且不部分写入。排队接口在执行时校验并通过宿主 error handler 报错，不把成功入队当作已提交。颜色覆盖不是业务选择事件，不额外建立通用 ColorChanged 事件总线。

颜色转换为线性 RGBA 后，各分量必须有限；alpha 必须在 [0,1]。不按本方案的默认调色板限制用户色相，不将有限的 HDR RGB 强行裁剪成 sRGB [0,1]。Waveform palette 至少两个颜色，循环相邻颜色不能相同，沿用既有 palette 验证语义。普通 API 不检查或强改用户配色的对比度。

### 固定色便利接口

保留 Icon 的 `set_color / clear_color` 便利方法，其实现委托新的覆盖接口。`set_color(c)` 明确把 Icon 的 Normal 和 Disabled 两个 foreground 都设为 Some(c)，所以依旧表示“固定为这种颜色”；clear 清除这两个覆盖。Text 提供相同的固定色便利方法。

这不是解析时跨状态 fallback。需要只有 Normal 特殊、Disabled 仍按规则变灰时，使用完整 overrides，只填写 normal.foreground。其他有多个交互状态的 Widget 不新增含义模糊的单色 setter。

### 配置与输出的边界

控件状态变化、Theme 切换、内容重建不能改写用户覆盖值。每次只计算派生样式并更新真正的输出组件，使用相等检查或 `set_if_neq` 避免稳定帧重复写入。

对由 Widgetry 管理的实体，直接修改 BackgroundColor、BorderColor、TextColor 或 Icon 内部 ImageNode 不是持续覆盖入口，后续刷新可以重写。未标记的原生 Text 不受该规则约束，BSN 的 TextColor 仍是其原生颜色。

例如 Hovered 显式红色，Pressed 未设置：Hovered 显示红，Pressed 显示 Theme.pressed；切换 Light/Dark 后红色覆盖保留；清除 Hovered 覆盖后立即回到当前主题的 Hovered 配色，而不是保存过的旧主题颜色。

## 前景色作用域、WidgetryText 和 Icon

### 作用域边界

以下节点产生自己的 ResolvedForeground：独立 Widget 根、ListView/Tree 行、Table 各 region shell、ComboBox Field/Popup、TooltipPopup、FileDialog 自己的行与内容区、Window/MessageBox 内容根。

一个普通 Node、padding 容器或 Icon 的内部 image 节点不产生新作用域。遇到独立 CheckBox 时，旧作用域到此结束，CheckBox 用自己的有效状态和 Theme/显式配置产生新来源。

内容中的布局可以任意多层嵌套。用户不需要登记每个 Children；标记 Text 和 Icon 自动消费最近来源。存在本地禁用的纯布局节点时仍传递 Disabled，而不假造一个新的颜色来源。

### 内容解析规则与本地 Disabled 的例外

普通内容的顺序是：自身当前状态的覆盖 → 可用的最近前景色 → 自身 Theme 当前状态。

可用性要求：来源和接收方的有效 Disabled 一致。通常它们自然一致，例如禁用 Button 和里面的标签。如果一个普通内容分支单独被禁用，而外部 Button 仍启用，该分支不能继续拿外部 Normal 颜色压过自己的 Disabled Theme；这时继承项不可用，内容退到自身 Disabled 配色。不会跳过这个最近作用域去寻找更远祖先。

独立交互控件不把外层颜色当作自己的 Theme fallback。它的 Disabled 样式来自自己的 schema；它的内容再继承这个结果。因此 Disabled 可以跨越作用域，而前景色不能无条件跨越独立 Widget。

Tooltip 是非交互说明内容。它虽然处在禁用 anchor 的 hierarchy 中，仍建立正常可读的 Tooltip 前景色来源；其来源记录同样带实际 Disabled=true，于是内容正常消费 TI，而不是被通用文字系统洗成灰字。它不因此获得交互能力。

### WidgetryText 只做接管标记

在 core 的 `text.rs` 定义零数据 `WidgetryText` Marker。文字内容、排版、字体和渲染仍由 Bevy Text 负责，不新增文字缓冲区或替代 Text API。facade 在 `text` 模块导出该标记、覆盖值与便利 API。

规则固定如下：

| 写法 | 行为 |
|---|---|
| `Text("Hello") WidgetryText` | 接管颜色；有作用域则继承，没有则取 Text Theme |
| `Text("Red") TextColor(red)` | 不标记，不参与 Theme/继承，颜色原样保留 |
| 单独 `Text("Hello")` | 保留 Bevy 原生默认，不自动插入标记 |
| 带 WidgetryText 并通过 colors Props/API 覆盖 | 覆盖优先，再按继承/Theme 解析 |
| 同时 WidgetryText 与直接 TextColor | TextColor 被视为当前渲染输出，不能据此推断固定覆盖 |

WidgetryText 定义为零数据 SceneComponent（同时也是 Component），通过 required component 安装默认全 None 的私有文字覆盖状态。因此直接写 `Text("Hello") WidgetryText` 也具有完整默认状态；`Text("...") @WidgetryText { @colors: ... }` 则通过一次性 Props 初始化覆盖。该 Scene 不重建已有 Text，不自动制造 Text 缓冲区。运行期覆盖只由私有组件保存。

只处理 UI Text；带标记的 TextSpan 也按同一内容规则处理，用于富文本。TextSpan 不单独自动建立前景色作用域，父 Text 的显式颜色不会被本通道再次当作祖先主体颜色传播；需要固定某个 span 时仍可使用未标记 TextSpan 的原生 TextColor。未标记 TextSpan 的显式颜色不被扫描覆盖。不扩展到 Text2d，也不接管 UnderlineColor、StrikethroughColor 等本项目尚未使用的装饰能力。

EditableText/TextField 的文字颜色由它自己的 Style writer 管理，不能再加入通用 WidgetryText writer；迁移时发现同一 entity 同时带两种管理身份，视为内部构造错误并修正，不允许两个 system 争写。

### Icon 与内部标记颜色

WidgetryIcon 作为内容自动消费该通道；默认 fallback 从硬编码白色改为 Icon Theme。显式固定色、清除色、Theme 切换以及 SVG 异步加载/替换都使用最新配置重新解析 tint。

CheckBox 勾/横线、Radio dot、ComboBox 箭头、Tree 展开符、窗口控制图标等是具体部件：它们的颜色由拥有者的专属字段决定，不把“owner 算出的 Theme 颜色”调用公共 set_color 写成用户显式覆盖。

使用 WidgetryIcon 的内部部件统一采用派生来源：拥有者在该图标实体设置 ResolvedForeground，由 Icon 的唯一 tint writer 消费。解析 pass 先处理本节点来源，再计算该节点内容的 InheritedForeground，使源与 Icon 同实体也能生效。Radio dot 等本来由 BackgroundColor 绘制的几何部件，则由拥有者 apply 直接写它自己的 BackgroundColor，不绕经 Icon API。CheckBox 标签来源与 mark 来源分开，避免勾选符的白色染到整行标签。

Icon 改 tint 不重新栅格化 SVG，不破坏现有 image cache；加载等待中发生 Theme/覆盖变化，asset 就绪时应用最新结果。完整多彩图标需要保持原色时，显式固定白色 tint，而不是依赖旧默认白色。

### 该层的更新与不变量

子节点新增、重建、换父级、最近作用域删除、内容覆盖清除、有效 Disabled 变化及 Theme 切换，都需要刷新。内部缓存必须包含来源 identity 或等价的失效依据，不能只看颜色是否相同。

只有真正参与 Widgetry 配色的内容被写入。原生固定色 Text 永远不因为 ancestor 状态变化、Theme observer 或扫描后代而被覆盖。对内容标记的移除表示交回原生管理，此后不再刷新，保留移除时已经显示的颜色，调用方可自行设置 TextColor。

“有状态的 Widget 产生颜色，内容消费颜色”这个归属固定下来，不再恢复传播整套状态颜色表或向内容复制 Hovered/Pressed 的方案。

## 各控件的颜色消费端迁移

### 组合控件复用的颜色归属

当前 ComboBox Field 复用了 WidgetryButton，Tree 内部复用了 ListView 和 Button，MessageBox/FileDialog 也组合已有控件。如果这些内部控件仍无条件读取自己的全局 Theme，新增 `theme.combo_box` 或 `theme.file_dialog` 就会变成不生效的数据。

因此新增 workspace 内部 `WidgetryStyleOwner<T>` 组件，保存拥有者 Entity，T 是被接管的底层样式主体类型，**只标记组合控件自己创建的可复用样式部件**。例如 ComboBox Field 使用 `WidgetryStyleOwner<WidgetryButton>`，FileDialog 的过滤器使用对应 ComboBox 类型的 owner 组件。普通底层 Widget 的默认颜色 writer 只跳过属于自己 T 的托管部件；拥有者使用自己的 Theme 子记录和覆盖，调用底层 crate 提供的 workspace 内部 resolve/apply 入口完成绘制。T 只用于 ECS 身份过滤，不建立通用 resolver trait 或注册表。底层的 headless 行为、输入、selection、layout 仍照常工作。

这些入口对跨 crate 调用需要 pub，但不从 facade 导出，不形成面向应用的新通用样式 API。ListView 的内部入口仍负责它自己的行状态判断和行遍历，Tree/ComboBox 只传入它们所拥有的 ListView 配色及覆盖；不复制虚拟化实现。

这不是背景色/边框色的层级继承：只有明确的内部部件关联才传配置。用户放在 Button Children 中的独立 CheckBox 不设置 StyleOwner，依然使用 CheckBox 的 Theme。内部部件的 Theme 快照不是用户显式覆盖，主题切换不能因此失效。公开颜色 API 拒绝针对这个已托管的底层角色直接 set/clear，返回错误，调用方应设置外层拥有者的对应颜色字段；不能报告成功却没有可见效果。外层拥有者自己的覆盖 API 不受影响。

层级内的自有部件随拥有者按现有 hierarchy 生命周期清理；FileDialog 的独立确认 MessageBox 不具有 ChildOf，因此沿用 OverwriteOwner/现有窗口关闭路径管理生命周期，不能假定 despawn 会跨原生窗口自动级联。新生成部件在第一次颜色 pass 前建立归属，失效 owner 不得继续提供旧配色或复活窗口。Theme observer 的先后顺序不能产生两个 writer：普通底层 observer 也必须按相同归属过滤。

## 调度、刷新与生命周期的一致性

### 三种变化入口，以及初始化

状态变化使用现有 Added/Changed/RemovedComponents；颜色覆盖组件变化进入同一个刷新路径；Theme 切换沿用 `On<WidgetryThemeChanged>`。新增 Widget、新增内部部件、内容重新绑定都视为初始化刷新，不等待第一次 Hover 或下一次 Theme 切换。

Pressed/InteractionDisabled 等组件的移除不能仅靠 Changed 检测。覆盖清空仍保留一个默认空覆盖组件，通过正常 setter 提交，不用移除组件模拟“重置”，从而避免遗漏刷新。

### 明确的执行约束

core 为颜色相关共享阶段定义小范围 SystemSet，只约束顺序，不提供统一 Widget resolver。阶段关系如下：

```text
PreUpdate:
  本地禁用请求与 hierarchy 的 deferred 写入完成
  → 同步有效 Disabled 与官方组件投影
  → PickingSystems::ProcessInput / InputFocusSystems::Dispatch

PostUpdate:
  WidgetryUiSystems::Build
  → UiSystems::Prepare
  → WidgetryUiSystems::Materialize
  → 同步新 hierarchy 的有效 Disabled
  → 各控件解析并应用自身/内部部件颜色
  → 前景色作用域向内容解析
  → WidgetryText / Icon 应用最终颜色
  → detect_text_needs_rerender
  → UiSystems::Content（含 EditableTextSystems）
```

原有 Bevy `UiSystems::Propagate` 保留。在实际注册中，将 Widget 自身颜色解析放在 Materialize 之后、UiSystems::Propagate 之前；前景色解析加入 UiSystems::Propagate；内容颜色应用在其后，并显式先于文字变更检测和 UiSystems::Content。这与已核对的 Bevy 0.19.1 UI/text 调度衔接，不删除 camera/visibility 所需的传播。

TextField 的禁用编辑 guard 必须先于 EditableTextSystems；TextField Style 从原 Update 刷新迁入颜色阶段，避免颜色写入落在文字样式消费之后。Build/Materialize 之间所需的几何构造顺序不因颜色移动而改变。

Theme observer 立即调用拥有者的 apply 时，仍按 StyleOwner 过滤，不假设另一个 observer 已更新。它可以直接更新已经存在的主体输出；层级前景色和迟到的内部内容由上述阶段补齐。现有测试中根控件 Theme observer 的即时刷新继续覆盖，但后代最终颜色的断言在相关颜色阶段运行后进行。

### 失效传播与最小更新

来源颜色变化只影响它实际控制的内容区域；遇到下一个独立前景色来源，外部颜色失效到此停止。Disabled 变化则继续穿过该边界。

Theme 变化使所有已实例化相关 Widget 重新解析，但不会给每个控件重建 Scene 或重跑数据 renderer。颜色相等不写输出，仍更新必要的来源 identity 元数据。被 override 固定住的输出不强行标记 Changed。

可见行的来源、标记内容、有效禁用与主体颜色必须在同一正常 UI 准备帧取得一致结果。Icon/字体/渲染 target 的真实 asset 等待保持原行为，不能把“等资源”误记成颜色解析完成，也不能因此停掉 RequestRedraw。

### 不允许的实现捷径

不能把 Theme 颜色通过公共覆盖 setter 填入内部按钮，以此“复用 API”；那会把默认色升级成显式色并冻结主题。不能在最终 TextColor 与某个默认色相等时猜它是否来自用户。不能靠不同 system 每帧互相覆盖来维持表面正确。

不能在 parent enable 时无条件删除全部子孙的 InteractionDisabled。不能只让样式读取 EffectiveDisabled，却遗漏官方 headless Button 的输入。不能把祖先的 Hovered/Pressed 复制到普通内容或独立控件来换取相同颜色。

## Gallery 展示方案

### 显式覆盖示例

每个主要 Widget 放两个实例：一个纯 Theme，另一个仅覆盖下表确定的状态/部件。提供“应用示例覆盖”和“清除覆盖”按钮，实际调用新的公开 API，不直接写渲染颜色组件，不增加颜色选择器。演示紫色固定为 `#7C3AED`，青色固定为 `#0F766E`，它们只属于 Gallery 示例，不进入库的默认主题。

| 示例 | 固定覆盖内容 | 观察动作 |
|---|---|---|
| Button | hovered.background=紫色 | 真实悬停；Pressed 仍用 Theme |
| Text / Icon | 固定色便利 API 设置青色 | 切换主题和父级 Disabled，固定覆盖仍保持 |
| CheckBox | checked.normal.mark=青色 | 勾选后观察标记，Hover/Disabled 仍取各自 Theme |
| RadioGroup | option.checked.normal.dot=青色 | 选择一个 Option，其他状态不被覆盖 |
| TextField / ReadOnly | 各自 focused.border=紫色 | 点击聚焦；只读仍不能编辑 |
| ComboBox | field.open.border=紫色 | 展开后观察 Field，Popup/list 使用 Theme |
| ScrollArea | vertical.thumb.hovered.background=紫色 | 悬停垂直滑块，水平滑块不变 |
| ListView | item.selected.background=青色 | 选中后移开鼠标，避免 Hovered 优先遮住示例 |
| Tree | item.selected.background=青色 | 选择节点后移开鼠标，展开符仍用 Theme |
| Table | cell.selected.background=青色 | 选择单元格后移开鼠标，焦点边框不变 |
| Tooltip | popup.background=紫色 | 真实 Hover 出 Tooltip，内容仍用 TI |
| Window | frame.normal.border=紫色 | 打开独立窗口，图片模式保持不变 |
| MessageBox | body.normal.background=青色 | 打开对话框并切换主题，动作按钮仍用 Theme |
| FileDialog | entry.selected.background=青色 | 选中文件，过滤/排序和侧栏按钮仍用 Theme |
| Waveform | normal.palette=[#7C3AED,#0F766E,#B45309,#BE185D] | 普通波形变色，Disabled palette 仍取 Theme |

只有 normal 槽位被覆盖的例子，在 Disabled 时回到 Disabled Theme，是状态优先规则的刻意演示，不是覆盖丢失。窗口类演示保存已打开实例的 Entity，通过现有生命周期判断有效性；不要求用户重复创建窗口才能清除覆盖。

Light/Dark 切换后，覆盖字段保持，其他字段跟随 Theme。清除覆盖后回到当前 Theme，而不是固定的 Dark 或初始颜色。示例文字清楚区分“Theme 色目录”和“控件最终效果”，不凭另一套 resolver 猜来源。

### 组合与 Disabled 的真实演示

Button 页增加一个多层 Node 内含 WidgetryText/Icon 的示例，改变根 Button Disabled，内容自动变色且按钮不能触发。另一个禁用容器包含 CheckBox/TextField 和一个本地单独禁用的 CheckBox：父级恢复后只有最后那个继续禁用。

这些示例不让用户逐个 Children 操作状态，也不把可交互 CheckBox 真正塞入 Button 当作常规 UI 推荐；嵌套独立控件语义在自动化测试中覆盖，Gallery 用普通禁用容器展示更清楚。

已有 Tooltip Disabled 示例继续工作，固定色的未标记 Text 显式展示不随 Theme/父级前景色变化。Gallery 标题、导航、说明、renderer 内应跟随主题的文字统一添加 WidgetryText；确实有意固定色的原生 Text 不添加标记。

### 运行配置不变

保留 `WinitSettings::desktop_app()`、BRP runtime、原窗口尺寸和现有 benchmark 入口。配色展示区的滚动使用已有 ScrollArea，不把 Gallery 设置成永久 continuous update 来掩盖刷新问题。

Library 不能反向依赖 Gallery，演示用色块、标签、折叠状态留在 gallery。真实 Widget 的覆盖调用是消费者用法，不为演示破坏私有 state 边界。

## 迁移收尾、验证与交付标准

### 跨 workspace 收尾

根 Cargo.toml 增加 theme member，各生产消费者使用 path dependency；锁定的 Bevy、wgpu、BRP 版本不变。facade、test_utils、Gallery、测试和 benchmark 全部迁移新名字。新增 crate 的 Windows 64 位检查与 workspace lint 一致。

更新 `docs/architecture.md` 的 crate 职责、依赖、Theme 归属、共享 Disabled/内容能力。`rules/widget-api.md` 只补充[有效 Disabled 方案](02-effective-disabled.md)明确的 Disabled 请求/投影语义，不把其他官方 Component 改成私有 state。必要的模块头部和使用说明同步，历史 plans 不重写。

删除旧的公开 Theme/Foreground 名称和旧的父子 disabled 镜像，不留下新旧 writer 并存。不能通过搜索删除所有 Propagate 标识：Bevy camera、visibility、transform 和事件冒泡与本次颜色通道不是同一机制。

每份小方案完成自己的局部编译和回归。全链最终检查不仅看 git diff 中新增模块，还检查各旧消费者是否仍用过时的 defaults、硬编码 tint、旧 Style 颜色或丢失的 Plugin 装配。非颜色字段、原有业务事件和数据 identity 不应发生无关变化。

### 行为测试矩阵

| 领域 | 必须覆盖的场景 | 通过条件 |
|---|---|---|
| Theme 数据 | 两主题、每个实际部件/状态字段、palette | 完整可构造、颜色有限、固定数值与配色表一致 |
| Theme 切换 | Dark→Light→Dark、同值、先更新 mode 再通知 | 配色正确，同值不产生多余通知，已有业务状态不被重置 |
| 状态优先级 | Button 的组合状态；TextField focus；Table active 与 selected | 按各控件既有优先级选状态，不被通用排序抹平 |
| 覆盖解析 | 只覆盖 Normal、只覆盖 Hovered、多个属性部分覆盖 | 不跨状态 fallback，未覆盖叶子仍取该状态 Theme |
| 清除与透明 | 单叶清空、整体 clear、显式 Color::NONE | 清空回当前 Theme，透明不当作未设置 |
| API | 构造 Props、World setter、Commands setter、同值、错误目标 | authority 单一；错误不部分写入；queue 在执行时提交 |
| 有效 Disabled | 祖先禁用/恢复，子本地禁用保留，多级祖先 | OR 语义，无 false 覆盖祖先 true |
| 禁用期间改本地请求 | 继承 true 时再 Insert 或 Remove 官方组件 | 父恢复后正确保留或解除子自身请求 |
| lifecycle | 同帧 Remove/Insert、late plugin、new child、reparent、detach、despawn | 不产生残留原因、误启用、复活实体或过期颜色 |
| 输入真实性 | pointer、键盘 Enter/Space、Checkbox/Radio、TextField 编辑/粘贴；父级写入并 flush 后马上派发下一输入 | 实际 disabled 时不产生用户激活/值修改，不只是变灰；不可依赖多跑一帧才阻止输入 |
| 会话取消 | press/drag/resize/粘贴期间祖先禁用 | 已有会话不能继续提交；不破坏既有外部状态 contract |
| 焦点/Tooltip | disabled 仍可按原规则聚焦、Tooltip 可显示 | 不全局禁用 picking 或清空 Focus；说明内容可读 |
| 前景色 | 多层布局、最近作用域、嵌套独立控件、局部内容禁用 | 颜色来源正确，Disabled 与颜色边界不混用 |
| Text 接管 | Marker、未标记 TextColor、未标记 Text、标记移除 | 原生显式色永不被覆盖；不靠颜色值猜来源 |
| Icon | 显式固定色、clear、Theme 切换、SVG loading/replacement | 最新 tint 生效，不重复栅格化，原色白 tint 可用 |
| 动态内容 | List/Tree/Table/Combo renderer 替换和新增行 | 首个正常准备帧正确，无须等待再次 hover/theme |
| 组合 Theme | 修改 ComboBox/Tree/FileDialog 专属字段 | 内部控件真正在用宿主字段，普通独立控件不受影响 |
| Window | Theme/图片模式、controls enabled_buttons、独立/modal 窗口 | 图片与 opacity 不被改写，原生 owner 不导致对话框全禁用 |
| Waveform | palette 覆盖、clear、Theme/Disabled 切换 | 颜色更新不重新读取数据或重算 envelope |
| Gallery | 所有页面、两主题、展开/折叠、覆盖/清除 | schema 全覆盖，色块与真实实例用途明确，折叠后不留旧主题 |

单元测试使用彼此不同的测试颜色标记每个槽位，验证 resolver 的来源和状态选择；不能只用真实配色里重复的 T 值断言“它大概选对了”。integration tests 必须断言最终 TextColor/ImageNode/BackgroundColor/BorderColor，不再只检查中间 Foreground 或 bool。

涉及真实输入的测试通过现有 test_utils/BRP 输入路径，不用直接 trigger 一个 Activate 冒充真实用户交互。新增 schedule graph 测试验证 Disabled 先于两类输入、内容颜色先于文字消费，以及组合 writer 排除条件。

### 性能与无多余写入

复用现有基准工具，在 100、1,000、10,000 个物化 UI 节点的浅树和深树上比较稳定帧、单个祖先禁用、根前景色变化、Theme 切换。记录访问节点数、发生 Changed 的输出数和耗时；使用迭代遍历，避免深层 hierarchy 递归栈问题。

稳定帧应没有因本系统引起的重复颜色/disabled 结构写入。子树变化的计算量与受影响物化节点线性相关，不为每个节点从根重复解析。列表有大量离屏数据时，Theme 与 Disabled 不能触发离屏 UI 构造或 model 重建。

保留现有 FileDialog 与 Waveform benchmark 验收，不用新增 Gallery 色块把其测量 fixture 扩大。对外不宣称性能提升，除非实际基准提供证据；本次要求是不引入全量 renderer 重建、重复栅格化和永久刷新循环。

### Windows 上的实施验收

在 PowerShell 7 中完成适用的验证：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo build --workspace`、`cargo test --workspace`。按 crate 运行局部测试不能代替全 workspace 最终检查。

`cargo run -p widget_gallery` 后，通过项目现有 BRP 流程实际操作各页 Light/Dark、状态、覆盖/清除和禁用容器，并保存截图。不得仅凭示意色块或静态源码宣称 GUI 通过。需要同时检查 Mouse 与项目支持的其他 Pointer 路径，保留 reactive desktop update mode。

实施者遵守仓库 development/testing/logging/documentation 等适用规则，创建并维护实际任务进度记录；方案生成阶段不在目标仓库创建进度文件。代码完成后的独立 Code Review 按 AGENTS 现行流程执行，review 与修复均不由本次文档交付冒充。

### 完成定义

新 Theme crate 和命名已成为唯一入口；schema 与两个主题的颜色值完整；显式配置不会被状态/Theme 刷新破坏；只有最终前景色对内容继承；只有 Disabled 产生层级有效状态，且真实输入与外观一致；组合控件自己的 Theme 字段确实被内部部件使用；普通未标记 Text 保持原生行为；Gallery 全量可检查且局部覆盖可操作；规定的回归、运行验证和审查通过。

某项实际测试失败属于实现未达标，应修复对应实现，不改动本文已经确定的语义来绕过测试。本文没有需要用户继续选择的设计分支，也没有把未执行的验证写成已通过。

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
