# button Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Loading（与本地、祖先及其他内部 Disabled 独立）、Normal、Hover、Pressed、Disabled；当前 Theme 和覆盖颜色。
- 自定义 Node / Children 与 Foreground 内容。
- LinkButton 当前 URL 原文、内部文字与系统浏览器请求边界。

## Stimuli

- Scene 构造、Pointer 输入、添加 / 移除交互 Component、Theme 切换与 Override。

## Guards

- Disabled 阻止用户 Activation；样式不改调用方 Layout 或 Children。

## Transitions

- Normal ↔ Hover ↔ Pressed ↔ Disabled。
- 解除 Disabled 后正常恢复 Hover / Pressed 或 Normal。

## Invariants

- 视觉优先级为 Loading > Disabled > Pressed > Hover > Normal；Loading 使用 Normal 配色。
- 真实输入可触发官方 `Activate`，被禁用时不触发。
- Foreground 传到受管理内容，Theme 更新不改业务 Structure。

## Couplings

- 颜色 Override × 当前 Theme / 交互状态。
- Button 容器与 Text / Icon Children 的同帧配色传播。

## Coverage Map

- `headless.rs`：公开 BSN 根无默认视觉与 Loading；真实 Picking 主/副键、Pressed、取消/拖出、Focus、无障碍角色、显式/祖先 Disabled 与恢复，Enter/Space 非重复键盘激活；宿主同时安装原生 ButtonPlugin 时无重复激活；嵌套父级输入隔离涵盖取消后 Click、拖出并移回后的 Click/DragStart。

- `widgetry_button.rs`：基础状态、样式转换、Override、Theme、Scene Patch、内容传播与 Pointer Activation。
- `diagnostics.rs`：无效目标与托管 Owner 的领域分类，Core 颜色真实 source/origin 及 root/state/field，真实 Command 的共享源头与 Error Severity；托管输出缺失保留 Query source 和已有输出，非法初始颜色经 Scene 保留身份并清理根，正常 loading/disabled/同值颜色请求静默。Loading Unit Test 覆盖 Core disabled 运行资源不可用时保留行为状态及原始诊断。默认 Style Unit Test 覆盖视觉损坏独立报告且不阻止 Loading 提交，开始/结束仍更新 Wait 声明而不残留。
- `link_diagnostics.rs`：URL 拒绝分类、原始 parser source 与输入原文，失败后 URL/文字不变；Core 颜色 source/origin/字段及转换保真；真实 URL/颜色 Command 的 Error Severity、源头 ID 和纯传播无 handled；初始 URL/颜色失败的 Scene 同源身份与根清理。
- `loading.rs`：无 ComputedNode / 默认部件时的同步 / Commands 提交、外部 Cursor 声明独立性、同值、变化通知与无效目标的宿主错误传播；本地 / 祖先 / 其他内部理由的恢复顺序；默认 Button 和无默认 Style 的 Headless + Loading 在真实 Picking 鼠标与 Keyboard Dispatch 下的 Activate、Focus 和默认 Wait / 外部显式声明 / NotAllowed / Pointer；显式 Activate 边界；Content 子元素 Visibility 与布局占位；三套内置主题 / 颜色覆盖、遮罩共享与 Spinner 动画停止。
- `link_button.rs`：URL 原文提交与 Style 后续文字投影、无文字仍可提交、合法同值、非法输入（含百分号转义）的原子拒绝、World / Commands 时机、无效目标、Scene 失败清理与宿主错误传播；三套内置主题、Hovered / Pressed 下划线、父级 Disabled 继承、覆盖与清除；使用内建字体的真实 UI Layout 保护长 URL 自动换行与非零宽度。
- `open_url.rs` Integration Test：公开 Headless/OpenUrl 同根组合，缺 ComputedNode/默认子树仍可同步和排队更新，应用标签不被覆盖；Unicode/IPv6/端口原文保留，同值不改状态，setter 不产生 Activate；缺必要能力和非法 queued 更新拒绝并保留原 URL。
- `open_url.rs` Unit Test 在私有 opener 边界验证默认与外部链接的真实 Mouse Picking、主/副键、Focus、Enter/Space 各打开一次，继承禁用与显式 Activate Guard；连续 queued 更新使用执行时 URL，前置禁用/删除正常取消。原生 Win32 失败保留真实 source/origin，每次源头 ERROR 一次且放弃后关联 DEBUG 一次，无重试/宿主泄漏；缺运行资源与状态分类明确。不访问真实网页。
- `link.rs` Unit Test 覆盖默认文字损坏不阻断 URL 提交，多个损坏根的 Style 失败在 ECS 边界聚合，以及缺颜色状态仍为默认 Style 错误。
- 源码 Unit Test 保护更局部的样式优先级规则。
- `optional_loading.rs`：无默认 Style 的 Headless + Loading 通过公开 API 同步/排队提交，通知看到已提交状态和有效禁用；无 Loading、缺失 Headless/Loading 的明确拒绝及 Command Error Severity，无默认视觉树仍可阻断和恢复输入，并保持本地/祖先/其他内部理由独立。
