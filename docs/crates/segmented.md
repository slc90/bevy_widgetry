# Segmented

## 功能与组合

`WidgetrySegmented` / `WidgetrySegmentedOption` 是独立的严格单选分段控件，通过 BSN 构造。Root 的直接 Children 必须全部为 Option 且至少一项；Option 内部可组合任意 Scene 内容，不使用字符串或图标专属内容 API。安装 `WidgetrySegmentedPlugin` 自动安装共享 UI、Pointer Focus 和 Tab 基础设施；Facade 通过默认开启的 `segmented` Feature 导出。

当前提供横向 / 竖向输入与浮块式主题外观、可选水平等宽模式，并在 Gallery 独立页面展示。

## 选择与输入契约

- Checked 是唯一选择权威。初始无 Checked 时选择首项；一个初始 Checked 时采用指定项；多个初始 Checked 报错，不覆盖冲突声明。修复冲突标记后可恢复。
- 初始化完成后通过 `WidgetrySegmented::set_selected(&mut Commands, root, index)` 更新，不直接插入或移除 Checked 绕过单选约束；没有清空选择 API。
- 用户点击非禁用项时提交选择，再发送来源为 Root 的 `ValueChange<usize>`；同项点击、程序化设置、初始化及结构修复静默。Observer 可以读取已经提交的 Checked。
- 点击 Option 或内容可使 Group 获得 Focus；有焦点时横向使用左右方向键、竖向使用上下方向键跳过禁用项，到首尾停止不循环，非对应轴 / Home / End 不改选。失焦及整体 Disabled 时不响应导航。Group 默认 `TabIndex(-1)`，Tab / Shift+Tab 跳过；程序设置不抢焦点。
- Disabled 沿用 Core 的继承 OR 语义。禁用不清除选中项，不限制合法程序选择或删除后的自动补选。

## 动态结构与错误

动态追加通过 BSN 创建带 `ChildOf(root)` 的 Option，移除可使用正常 ECS Despawn / 层级操作，不提供任意重排 API。添加不抢走合法已有选择；删除未选中项保留身份；删除选中项按上次合法结构优先选存活后继，无后继则选前驱。旧项全部消失或已确认的空集合恢复后选择首项。

程序命令按实际执行时的 Root、直接 Children 和 Index 校验。非法 Root、空集合、错误 Child 或越界 Index 记录上下文 ERROR，并以 `Severity::Error` 交给宿主 Error Handler；失败不部分改选或发送业务事件。同值保持 Checked change tick，失败组不阻止其他合法组更新。

结构维护在下一次正常输入派发前按变化执行，稳定结构不逐帧全量校验。每个组 / Option 通过 FailureState 去重持续诊断，实际恢复记录 INFO。初始 Checked 冲突的移除也触发恢复检查；构造期尚未挂接 Children 不立即报错。

## 实现边界

Segmented 与 RadioGroup 共用 Core 的结构历史和互斥 Checked 提交。RadioGroup 保留原生 RadioGroup / RadioButton 输入；Segmented 使用原生 Checked / Checkable、标准 Accessibility Role 及共享 Focus / Pointer 基础设施，自行处理点击和键盘，避免原生循环导航及 RadioButton 向最近 RadioGroup 祖先发事件时干扰外层组。两个 Widget 无相互生产依赖。

## 样式与主题

共享容器采用柔和背景、小内边距及圆角，参考最小高度 32px；Option 按任意 Children 的自然尺寸和水平内边距布局。Checked 项即时显示独立高亮块，无插值或动画。BSN Patch 可覆盖 Node 的尺寸、padding、圆角。

四套主题具有独立 Segmented Colors。Root Focus 使用容器边框；Option Hover 提供轻反馈，实际 Disabled 优先于 Hover，选中禁用仍保留淡化高亮。Style 不修改 Checked。

Root / Option Props 的 colors 仅初始化；运行时通过 WidgetrySegmentedColorOverrides / WidgetrySegmentedOptionColorOverrides 的 get、set[_in_world]、clear[_in_world] 更新。优先级为 Option 字段覆盖 → Group Option 字段覆盖 → Theme；非法颜色及托管目标拒绝写入。通过 Core 的 ResolvedForeground 向 WidgetryText / WidgetryIcon 传播默认前景，保留外部独立视觉配置。

样式按输入变化、主题通知、Focus 与托管输出生命周期维护，稳定帧不重复解析颜色。托管背景 / 前景 / 边框缺失会报告去重错误，恢复输出后重算。

`WidgetrySegmentedProps::equal_width` 默认 false，可在 BSN 构造时开启等宽列。使用 Node Grid 的水平 `1fr` 列均分可用宽度；未固定容器宽度时按内容的 intrinsic 尺寸形成等宽列。动态追加同样参与均分，Theme、Checked 与 Disabled 不改变模式。无需固定文本宽度，支持任意 Children。该配置只初始化 Node，不保留运行时配置副本或专用 Setter；调用方后续 Node Patch / ECS 布局设置遵循 Bevy 自身语义。

`SegmentedOrientation::{Horizontal, Vertical}` 通过 Root Props 的 `orientation` 构造配置，默认 Horizontal。Props 初始化私有有效方向，同时用于外层主轴布局和键盘映射；不提供运行时方向 Setter，不支持通过移除 / 替换内部方向状态切换。仅 Patch `Node::flex_direction` 不会改变键盘方向。Vertical 使用 Column 和交叉轴 Stretch，按内容保留各 Option 自然高度；equal_width 不映射为等高。所有方向按 Children 顺序维护索引、程序化选择及删除回退，保持相同主题、Disabled、Focus 和 Tab 规则。
