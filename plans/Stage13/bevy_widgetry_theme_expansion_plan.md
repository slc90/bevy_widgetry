# bevy_widgetry：新增「梦幻粉」与「千石冠紫」Theme 修改方案

> 状态：方案文档，待实施；2026-10-10  
> 目标仓库：<https://github.com/slc90/bevy_widgetry>  
> 适用版本：以撰写时 `main` 分支已核对的 `crates/theme`、`gallery` 结构为基础。  
> 说明：本文是**实施方案**，目前**没有修改仓库代码，也没有运行 Bevy Gallery 验证新主题**。

## 1. 已确定的设计

在现有 `Light` / `Dark` 两套固定主题之外，新增两套完整的固定颜色主题：

| Theme 名称 | Rust 枚举建议 | 视觉定位 | 设计决定 |
| --- | --- | --- | --- |
| 梦幻粉 / Pink Dream | `WidgetryThemeMode::PinkDream` | 轻盈梦幻，奶油白底、柔玫瑰粉强调色 | **保留此前批准的粉色色表，26 项均不改动** |
| 千石冠紫 / Kamuri Violet | `WidgetryThemeMode::KamuriViolet` | 淡紫白底、柔紫强调色、安静清透 | **采用最新的「千石冠紫」色表，替代旧「冠之薰衣草紫」** |

新主题是 **Widgetry 的纯颜色配置**，不是壁纸、插画或自定义纹理主题。既有 Widget 布局、组件状态语义、交互协议、图标资源和窗口行为保持不变。浅紫配色是角色灵感设计，**不声称严格复刻《Slow Start》动画官方角色色卡**。

原有 `WIDGETRY_LIGHT_THEME`、`WIDGETRY_DARK_THEME` 及 `WidgetryThemeMode::default() == Dark` 保持不变。只增加两个可选值，不更改默认选项。

## 2. 最终确定的 26 个通用 Color Token

以下颜色值以此前确认的两张设计色表为准，保持 **sRGB、HEX、字母大小写不影响颜色语义**。Rust 中的非透明项使用 `Color::srgb_u8(r, g, b)`；透明项使用 `Color::NONE`。HEX 的 `#00000000` 表示全透明，不可误写成不透明黑色。

| Token（与现有 `common.rs` 同名） | 梦幻粉 Pink Dream | 千石冠紫 Kamuri Violet | 作用 |
| --- | --- | --- | --- |
| `WINDOW_BACKGROUND` | `#FFF8FC` | `#F8F6FB` | 窗口主背景 |
| `SURFACE` | `#FFFEFF` | `#FFFDFF` | 普通控件表面 |
| `ELEVATED_SURFACE` | `#FFFFFF` | `#FFFFFF` | 浮层 / 高层表面 |
| `HOVER_SURFACE` | `#FFF0F7` | `#F1ECF8` | 悬停表面 |
| `PRESSED_SURFACE` | `#F9DEEB` | `#E7DDF1` | 按下表面 |
| `DISABLED_SURFACE` | `#F7EFF3` | `#F4F0F7` | 禁用表面 |
| `BORDER` | `#E7CCD9` | `#D8CDE4` | 普通边框 |
| `SUBTLE_BORDER` | `#F3E3EC` | `#EEE7F5` | 弱边框 |
| `DISABLED_BORDER` | `#E5D9E0` | `#E7DFF0` | 禁用边框 |
| `TEXT` | `#422B3C` | `#3D314A` | 正文文字 |
| `SECONDARY_TEXT` | `#705566` | `#72617F` | 次要文字 |
| `TERTIARY_TEXT` | `#927A88` | `#9A8CA6` | 三级文字 |
| `DISABLED_TEXT` | `#AC98A3` | `#B5A9C1` | 禁用文字 |
| `INVERSE_TEXT` | `#FFFFFF` | `#FFFFFF` | 反色前景 / 勾选标记 |
| `PRIMARY` | `#B4437D` | `#8C6BC1` | 主题强调色 |
| `PRIMARY_HOVER` | `#CD6196` | `#A181D2` | 强调色悬停 |
| `PRIMARY_PRESSED` | `#913063` | `#6E4EA3` | 强调色按下 |
| `FOCUS_BORDER` | `#B4437D` | `#8C6BC1` | 键盘焦点边框 |
| `SELECTED_SURFACE` | `#FAE1EE` | `#EDE6F8` | 已选中表面 |
| `SELECTION_BACKGROUND` | `#F0BDD8` | `#D9CAEF` | 文本选区背景 |
| `UNFOCUSED_SELECTION_BACKGROUND` | `#EBDEE5` | `#E8E0F0` | 失焦文本选区背景 |
| `TOOLTIP_BACKGROUND` | `#493541` | `#4A3A5F` | Tooltip 背景 |
| `DANGER_HOVER` | `#D4495D` | `#9B5C86` | 危险操作悬停 |
| `DANGER_PRESSED` | `#AA263E` | `#7A456B` | 危险操作按下 |
| `TRANSPARENT` | `#00000000` | `#00000000` | 完全透明 |
| `IMAGE_TINT` | `#FFFFFF` | `#FFFFFF` | 图像默认染色 |

**Danger 的既定处理：**

- 梦幻粉：严格保留已批准色表中的红莓色 `#D4495D` / `#AA263E`；**不擅自更改粉色色表**。
- 千石冠紫：使用新版色表中的梅紫色 `#9B5C86` / `#7A456B`，**不沿用旧紫色表的鲜红 Danger**。
- `DANGER_*` 仍表示危险操作，不能当成普通强调色复用。危险含义不能只依靠颜色区别，应延续现有控件的状态、动作语义与标识。

**可读性边界：** Token 已选定，不为了测试方便私自调整它们。需要检查实际各 Widget 的前景/背景组合。特别是千石冠紫 `INVERSE_TEXT`（白色）在 `PRIMARY` `#8C6BC1` 上的对比度约为 **4.21:1**，适合图标/勾选标记的非文字对比目标，但**不满足普通小字 4.5:1**；不要假定任意白色小字均能直接铺在主题主色上。粉色的 `DANGER_HOVER` 配白字约为 **4.28:1**，同样需区分窗口关闭图标与普通文字的用途。若真实控件使用文字，应调整该控件的颜色搭配，而非悄悄改动已选定的 26 个值。

## 3. Theme 模型与 API 修改

### 3.1 全局主题值

在 `crates/theme/src/theme.rs` 增加：

- `WidgetryThemeMode::PinkDream`、`WidgetryThemeMode::KamuriViolet`；
- `WIDGETRY_PINK_DREAM_THEME: WidgetryTheme`、`WIDGETRY_KAMURI_VIOLET_THEME: WidgetryTheme`；
- 在 `WidgetryThemeMode::colors()` 中新增两个匹配分支；
- 在 `crates/theme/src/lib.rs` 中公开新常量，保留现有 `WidgetryTheme`、`WidgetryThemeChanged`、`WidgetryThemePlugin` 的接口与语义。

继续维持：**提交 mode 后再触发通知；重复设置相同 mode 是 no-op；`Commands` 路径在实际应用队列时提交；组件自行响应 theme change。** 不增加新的一套主题事件或并行状态资源。

增加 enum 变体会影响外部调用方对 `WidgetryThemeMode` 的**穷举匹配**：内部所有 `match` 应完整扩充；如果第三方应用使用无通配分支的 `match`，也需要同步增加分支。这是 API 扩展带来的已知源码兼容影响。

### 3.2 通用 token 与完整组件颜色

在 `crates/theme/src/common.rs` 按现有 `light` / `dark` 的组织形式增加 `pink_dream` / `kamuri_violet` 两个模块，**逐项提供上表 26 个同名常量**。保留原 light/dark 常量不变。

`WidgetryTheme` 当前不是单纯 26 个 token，而是包含 16 个子配色字段（`text`、`icon`、`button`、`check_box`、`radio_group`、`text_field`、`combo_box`、`scroll_area`、`list_view`、`tree`、`table`、`tooltip`、`window`、`message_box`、`file_dialog`、`waveform`）。必须在对应模块补齐两个新主题的**全部状态字段**，不能只增两个 `common.rs` 色板就结束。

对应的文件位于 `crates/theme/src/`：

`text.rs`、`icon.rs`、`button.rs`、`check_box.rs`、`radio_group.rs`、`text_field.rs`、`combo_box.rs`、`scroll_area.rs`、`list_view.rs`、`tree.rs`、`table.rs`、`tooltip.rs`、`window.rs`、`message_box.rs`、`file_dialog.rs`、`waveform.rs`。

实现规则：

1. 以现有 Light 的**字段结构与状态组合**为基准复制配色关系，不变更结构体 / Widget API / 交互状态；使用新主题的同名 token 映射相应字段。
2. `normal`、`hovered`、`pressed`、`focused`、`selected`、`read_only`、`disabled` 等已有状态均按各模块现有结构保留；**不凭空为 Widget 新增状态**。
3. 组合配色要复用**本主题**的子常量：例如千石冠紫 FileDialog 应复用千石冠紫 Window、Button、TextField 等，避免混入 Light、Dark 或 Pink Dream 的组合成员。
4. 已经透明的区域仍透明；关闭按钮的 `DANGER_*` 保留危险语义；`IMAGE_TINT` 不做改变。
5. `WidgetryTheme` 的四套常量必须都是**完整**的静态配色，不采用运行时颜色过滤、全局 tint 或覆盖旧 Theme 的方式偷换颜色。

### 3.3 Waveform 特殊情况

`crates/theme/src/waveform.rs` 在通用 token 之外还有 `normal.palette` 和 `disabled.palette`，当前 Light/Dark 均有独立的 4-channel 起始色。**这 8 个色位不属于上述已批准的 26-token 色表**。

实施时给两套新主题各提供完整的正常/禁用 4-channel palette，而不是直接遗漏该字段或把四条波形画成同一种主色。为了让实施不依赖临时猜色，先采用以下**辅助候选配色**（不改变上表的 26-token 定稿）：

| 模式 | Channel 1 | Channel 2 | Channel 3 | Channel 4 |
| --- | --- | --- | --- | --- |
| Pink Dream / Normal | `#B4437D` | `#7766A7` | `#237A80` | `#9D6C28` |
| Pink Dream / Disabled | `#B6A2AE` | `#C6B7C0` | `#D4CBD0` | `#E4DEE1` |
| Kamuri Violet / Normal | `#6E4EA3` | `#A95387` | `#367584` | `#99712F` |
| Kamuri Violet / Disabled | `#AEA4BD` | `#BEB6C9` | `#D0CAD9` | `#DFDAE7` |

这些波形颜色属于**未在两张色表中展示过的实施补充方案**。其优先目标是让四个通道在浅背景下可辨；执行阶段需以真实 Waveform Gallery 截图验收，若需调整，仅调整这组辅助 palette，不修改已经批准的 26 个 token。

## 4. Gallery 与直接受影响的代码

`gallery/src/main.rs` 的 Theme ComboBox 当前只插入 `Dark`、`Light` 两项，其文字渲染也采用 `if Dark { ... } else { Light }`，初始选中同样是二选一。必须一起修改：

- Model 中加入 **Dark / Light / Pink Dream / Kamuri Violet** 四个真实 `WidgetryThemeMode` 值；
- Renderer 将四个模式一一对应正确的显示名，不能让新模式落入原来的 `else => Light`；
- 初始选择根据任意四种当前 mode 匹配正确的 `WidgetryListItemId`；
- 保留 `on_theme_combo_box_changed` 经 `WidgetryThemeMode::set_in_world` 切换，以及 `sync_theme_combo_box` 接收 `WidgetryThemeChanged` 后的同步机制，确保程序切换也能回显；
- 检查 Gallery 的 sidebar、页面、`color_showcase` 中使用 `mode.colors()` 的所有消费者，切换后能正确刷新，不留下上个主题的边框、文字、图标或弹出层。

同时检查 `crates/bevy_widgetry/src/lib.rs` 等对外说明以及 `docs/architecture.md` 中目前写成“Light/Dark 固定配色”的描述，将事实更新为四套固定主题；无需引入额外的 Theme 管理抽象。

## 5. 验证与完成标准

**自动化检查**

- 四种 mode 均映射到各自完整 `WidgetryTheme`，默认仍为 Dark；新常量所有字段均实际来自对应主题。
- 既有 Light/Dark 的原值测试继续通过；新增 Pink Dream、Kamuri Violet 的准确 RGBA/HEX 断言，覆盖所有组件及关键状态。
- 同值切换不通知；四模式间有效切换先提交后通知；World 与 Commands 路径行为一致。
- 各 Widget 直接使用新 `WidgetryThemeMode::colors()` 时，结构不缺字段、复合 Widget 不混用其他主题。
- 对可见文字使用目标前景/背景组合检查正常文字对比（通常至少 4.5:1）；图标、选中勾与边框按对应非文字视觉要求评估。特别检查危险悬停、禁用、选中、焦点和 Tooltip。对于无法达到普通文字要求的组合，应调整具体控件应用方式并记录例外，**不能静默修改批准的 token 数值**。
- 运行项目规定的格式、编译、Clippy、必要测试与独立 Code Review；遵循仓库 `AGENTS.md` 和 `rules/`，保持 Windows x64 / Bevy 0.20 的现有项目边界。

**真实 GUI 验收**

- 在 Windows x64 运行 `widget_gallery`，依 `rules/gui-debugging.md` 优先使用 `bevy_brp_mcp` 截图和交互；**不能用设计稿代替真实运行截图**。
- 从 Dark 切换到 Pink Dream，再切换到 Kamuri Violet，再切回 Light/Dark；确认四个选项显示正确、选择器同步正确、各页面即时刷新，无明显短暂旧色残留。
- 抽查 Button、TextField、CheckBox、RadioGroup、ComboBox、ListView、Tree、Table、Tooltip、Window，以及 MessageBox / FileDialog 等复合内容；检查 normal、hover、pressed、focus、selected、disabled 等实际存在的状态。
- Waveform 至少检查四通道的颜色区分、浅色背景可读性和 disabled 效果。
- Gallery 的 `Theme 颜色` 目录应显示**运行时真实 Color** 和 HEX，与上表 26 项一致。对照两张已批准设计图时以 token 和主要视觉关系为准，不要求设计稿中的装饰花纹出现在产品界面中。

## 6. 明确不做的内容

- 不重设 Light / Dark，不改默认 Dark，不改变既有主题切换事件语义。
- 不改 Widget 的 Layout、焦点/输入语义、交互 API 或底层渲染方式。
- 不加入角色图片、花纹背景、动态特效或新的资产加载依赖；这次是**颜色 Theme**。
- 不为了新增两种固定色板而引入运行时任意主题编辑器、JSON 配置、跨平台兼容代码。
- 不推送或修改远端 GitHub；真正施工阶段由本地工作树执行并按仓库规则验证。

## 7. 交付定义

实施完成后可通过公开的 `WidgetryThemeMode` 使用四种固定主题，其中 `PinkDream`、`KamuriViolet` 的 **26 个通用 Color Token 与本文件完全一致**；所有既有 Widget / 组合部件均能从相应的完整主题对象取得其状态配色，Gallery 可从下拉框正确选择与展示四套主题，并通过自动化测试及运行时截图验收。

---

### 参考依据

- 项目源码：`crates/theme/src/common.rs`、`theme.rs`、`lib.rs`、各 Widget 配色模块、`gallery/src/main.rs`、`gallery/src/color_showcase.rs`。
- 项目规则：`AGENTS.md`、`rules/project-context.md`、`rules/testing.md`、`rules/gui-debugging.md`、`docs/architecture.md`。
- 视觉定稿：本轮讨论批准的 **Pink Dream / 梦幻粉色表** 与 **Kamuri Violet / 千石冠紫色表**；本文件中的 26-token 表格即为其可独立执行的文字版本。
