# 02 建立有效 Disabled 继承并接通真实输入禁用

所属总方案：Widgetry 颜色、Theme 与有效 Disabled 重构方案

版本：1.0 · 方案基线：2026-10-08  
目标仓库：`slc90/bevy_widgetry`，核对提交 `5d33dd561a3c9bb171d7dd3238a41903a0724e33`。  
目标环境：Windows 64 位、`x86_64-pc-windows-msvc`、Bevy **0.19.1**、现有 DX12 配置。

这是一份实施方案，不是已经完成的代码改动。颜色组织、传播边界、公开接口、Disabled 的输入适配和 Gallery 展示均在本文确定；实施者不需要再选择架构方案。实际编译、测试、性能和 GUI 验证属于实施验收，本文不把源码核对当作运行验证。

## 目标

用户只需禁用一个 UI 祖先，其后代便正确禁用；恢复祖先时仍保留子控件自己的禁用请求，并让 Bevy 的真实输入消费者看到一致结果。

## 范围

负责 core 的本地原因记录、有效值计算、InteractionDisabled 投影，以及现有 ComboBox/Tree/ListView/RadioGroup/FileDialog/Window 的禁用镜像归并与会话取消。下文共享调度约束在本方案落实 Disabled 的同步点；颜色阶段由下一份方案落地，不在这里重构所有颜色 writer。

## 预期产出

只读有效禁用状态与可用的官方组件适配；既有父子禁用镜像退出；用户请求和模型/原生能力限制相互独立；真实 Pointer、键盘、编辑及祖先恢复用例通过。颜色数据仍由前一方案提供。

## 与前后方案的关系

承接 [Theme 方案](01-theme-and-palette.md)，完成后执行 [颜色接入方案](03-color-resolution-and-widget-integration.md)。本方案必须先保证实际输入禁用，不能让后续 Style 仅凭一个无人消费的 bool 变灰。

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

### 主题选择与通知

`WidgetryThemeMode` 保持 `Light / Dark`，默认 Dark；`colors()` 返回对应静态 `&'static WidgetryTheme`。`WidgetryThemeChanged { mode }` 仍是全局 Event，不改为层级 EntityEvent。

提供 `WidgetryThemeMode::set_in_world(world, mode) -> Result<bool, BevyError>` 和 `set(commands, mode)`。实际执行时先写 mode，再触发变化通知；同值不通知。保留官方 Resource 可直接访问的能力，但“只改资源不通知”不属于完整的主题切换操作。Gallery 和库内切换统一走这两个入口，消除 mode 与 event payload 不一致的内部路径。

`WidgetryThemePlugin` 仅初始化主题管理，不读取控件状态、不写 UI 颜色，也不依赖 core。`WidgetryUiPlugin` 负责装配 core 的 Disabled、前景色和标记文字支持，并保证 ThemePlugin 已注册。所有独立 Widget Plugin 按需装配 `WidgetryUiPlugin`，包括此前只直接装配 PointerPlugin 的 TextField/Tooltip。不要求使用者手工补内部 Plugin，不重复注册。

`set_in_world` 提交主题资源不等于整个 UI 已完成渲染。已有控件的 Theme observer 可立即调用自己的 apply；前景色传递、新内容 materialize 和最终文字/图标颜色在本帧明确的颜色阶段收敛，不依赖多个 observer 的碰巧执行顺序。

## 有效 Disabled 的继承与 Bevy 输入适配

### 为什么不能只增加一个新 bool

当前项目已锁定 Bevy 0.19.1。已核对的官方 `bevy_ui_widgets::Button` 在 pointer 和 keyboard observer 中直接检查 `Has<InteractionDisabled>`；它不会读取 Widgetry 新增的状态。项目 ComboBox、Tree、ListView 也已存在把 root disabled 写入内部节点的逻辑。

因此，只把 Style 查询改成 `WidgetryEffectiveDisabled` 会造成“颜色变灰、官方 Button 仍然可以触发 Activate”的错误。此次明确采用**本地请求保留 + 有效状态计算 + 官方组件投影**，不 fork Bevy、不重写整套官方 Button/Checkbox/Radio 行为，也不靠 observer 的执行顺序拦截已经产生的 Activate。

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

### 官方组件投影与既有局部镜像的迁移

父到子的稳定遍历计算有效值，并仅在结果不同的时候插入/移除 InteractionDisabled、更新只读有效值。不要直接用“最近的 Propagate<bool>”替代逻辑或，因为子节点的 false 不应取消父节点的 true。

ComboBox 的 `mirror_disabled_*`、`initialize_disabled`、List disabled 镜像，以及 Tree、ListView、RadioGroup 和 FileDialog 中把父级值写成子级本地值的路径，迁移到同一计算链。旧镜像必须在这个阶段移除，不能和新 bridge 并存，否则会产生永久禁用或提前解除禁用。

模型项是否禁用、Window.enabled_buttons 的原生能力限制，以及 FileDialog session/操作可用性限制，写入 intrinsic 原因。一个 Widget 的 intrinsic 由其拥有者统一汇总当前所有内部限制再提交，不能让多个 writer 用一个 bool 互相清除原因；例如 FileDialog 的 session 暂停与 NewFolder 请求未完成须先做 OR。用户请求和这些限制互不清除；模型或原生能力恢复只撤销自己的限制。列表回收/重新绑定行时，清除旧 item 的 intrinsic 原因并读入新 item 原因，不能把前一个条目的禁用粘到后一个条目。

正常调用方的同值操作不产生业务状态变化通知。投影产生的官方生命周期只服务其原有 Bevy 行为，不把它们再转成新的 Widgetry 业务事件。

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

## 调度、刷新与生命周期的一致性

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

### 不允许的实现捷径

不能把 Theme 颜色通过公共覆盖 setter 填入内部按钮，以此“复用 API”；那会把默认色升级成显式色并冻结主题。不能在最终 TextColor 与某个默认色相等时猜它是否来自用户。不能靠不同 system 每帧互相覆盖来维持表面正确。

不能在 parent enable 时无条件删除全部子孙的 InteractionDisabled。不能只让样式读取 EffectiveDisabled，却遗漏官方 headless Button 的输入。不能把祖先的 Hovered/Pressed 复制到普通内容或独立控件来换取相同颜色。

## 迁移收尾、验证与交付标准

### 性能与无多余写入

复用现有基准工具，在 100、1,000、10,000 个物化 UI 节点的浅树和深树上比较稳定帧、单个祖先禁用、根前景色变化、Theme 切换。记录访问节点数、发生 Changed 的输出数和耗时；使用迭代遍历，避免深层 hierarchy 递归栈问题。

稳定帧应没有因本系统引起的重复颜色/disabled 结构写入。子树变化的计算量与受影响物化节点线性相关，不为每个节点从根重复解析。列表有大量离屏数据时，Theme 与 Disabled 不能触发离屏 UI 构造或 model 重建。

保留现有 FileDialog 与 Waveform benchmark 验收，不用新增 Gallery 色块把其测量 fixture 扩大。对外不宣称性能提升，除非实际基准提供证据；本次要求是不引入全量 renderer 重建、重复栅格化和永久刷新循环。

### Windows 上的实施验收

在 PowerShell 7 中完成适用的验证：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo build --workspace`、`cargo test --workspace`。按 crate 运行局部测试不能代替全 workspace 最终检查。

`cargo run -p widget_gallery` 后，通过项目现有 BRP 流程实际操作各页 Light/Dark、状态、覆盖/清除和禁用容器，并保存截图。不得仅凭示意色块或静态源码宣称 GUI 通过。需要同时检查 Mouse 与项目支持的其他 Pointer 路径，保留 reactive desktop update mode。

实施者遵守仓库 development/testing/logging/documentation 等适用规则，创建并维护实际任务进度记录；方案生成阶段不在目标仓库创建进度文件。代码完成后的独立 Code Review 按 AGENTS 现行流程执行，review 与修复均不由本次文档交付冒充。

## 本方案局部验证

以下是本方案交付时必须通过的局部行为检查，不能留到全部方案结束后才验证。完整 workspace、GUI 和独立审查的最终门槛见收尾方案；这里的检查不代表已经执行。

| 领域 | 必须覆盖的场景 | 通过条件 |
|---|---|---|
| 有效 Disabled | 祖先禁用/恢复，子本地禁用保留，多级祖先 | OR 语义，无 false 覆盖祖先 true |
| 禁用期间改本地请求 | 继承 true 时再 Insert 或 Remove 官方组件 | 父恢复后正确保留或解除子自身请求 |
| lifecycle | 同帧 Remove/Insert、late plugin、new child、reparent、detach、despawn | 不产生残留原因、误启用、复活实体或过期颜色 |
| 输入真实性 | pointer、键盘 Enter/Space、Checkbox/Radio、TextField 编辑/粘贴；父级写入并 flush 后马上派发下一输入 | 实际 disabled 时不产生用户激活/值修改，不只是变灰；不可依赖多跑一帧才阻止输入 |
| 会话取消 | press/drag/resize/粘贴期间祖先禁用 | 已有会话不能继续提交；不破坏既有外部状态 contract |
| 焦点/Tooltip | disabled 仍可按原规则聚焦、Tooltip 可显示 | 不全局禁用 picking 或清空 Focus；说明内容可读 |

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
