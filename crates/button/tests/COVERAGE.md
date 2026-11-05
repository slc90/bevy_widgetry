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
- `diagnostics.rs`：无效目标与托管 Owner 的领域分类，Core 颜色真实 source/origin 及 root/state/field，真实 Command 的共享源头与 Error Severity；托管输出缺失保留 Query source 和已有输出，非法初始颜色经 Scene 保留身份并清理根，正常 loading/disabled/同值颜色请求静默。Loading Unit Test 覆盖内部 Button/state/content/spinner/必需组件损坏的原子拒绝，以及 Core disabled 运行资源不可用时保留表现。
- `link_diagnostics.rs`：URL 拒绝分类、原始 parser source 与输入原文，失败后 URL/文字不变；Core 颜色 source/origin/字段及转换保真；真实 URL/颜色 Command 的 Error Severity、源头 ID 和纯传播无 handled；初始 URL/颜色失败的 Scene 同源身份与根清理。
- `loading.rs`：同步 / Commands 提交、同值、变化通知与无效目标的宿主错误传播；本地 / 祖先 / 其他内部理由的恢复顺序；真实 Picking 鼠标与 Keyboard Dispatch 下的 Activate、Focus 和 Wait / Pointer；显式 Activate 边界；Content 子元素 Visibility 与布局占位；四主题 / 颜色覆盖、遮罩共享与 Spinner 动画停止。
- `link_button.rs`：URL 原文与文字同步、合法同值、非法输入（含百分号转义）的原子拒绝、World / Commands 时机、无效目标、Scene 失败清理与宿主错误传播；四套主题、Hovered / Pressed 下划线、父级 Disabled 继承、覆盖与清除；使用内建字体的真实 UI Layout 保护长 URL 自动换行与非零宽度。
- `link.rs` Unit Test 在私有系统调用边界注入真实 Win32 错误类型，验证官方 Pointer Activation 请求当前 URL、程序化修改不打开、继承禁用及显式 Activate Guard、恢复激活；每次打开失败只产生同 ID 的一次源头 ERROR 和实际放弃后的 DEBUG，不泄漏为未处理、不重试。成功请求与排队在前的禁用/删除取消保持静默；内部 URL/文字/颜色部件缺失分类并拒绝部分提交。不验证浏览器网页加载。
- 源码 Unit Test 保护更局部的样式优先级规则。

