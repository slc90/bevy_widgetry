# 01 | 全工作区切换到 Bevy 0.20 并恢复完整构建

[返回方案总览](00-overview.md)

## 目标

把整个工作区、测试工具和 Gallery 一次性接到同一套 Bevy 0.20 依赖图上，形成可以完整编译且既有测试通过的起点。

## 范围

根依赖与锁文件，以及所有会因新依赖立刻失效的 Rust API、BSN、observer、文本输入和 renderer 接口。不是只改 Cargo.toml，也不是只编译 facade。

## 预期产出

使用 Bevy/ bevy_remote 0.20.0、BRP v0.4.0 与兼容 wgpu 30 的工作区；全部成员和 Cargo 目标构建通过，已知测试失败不遗留。

## 与前后方案的关系

这是唯一的引擎版本切换方案。它没有独立的基线准备前置方案；完成后进入 Scene 契约核对。依赖与所有硬性编译修复共同构成一个交付边界，不能把它拆成几个编译失败的提交成果。

## 执行边界

只迁移 slc90/bevy_widgetry，不修改 Bevy 或 bevy_brp 仓库。保持现有 crate 依赖方向、headless/style 分层、facade 导出、控件公共行为和 Windows 64 位定位；不借升级重做控件架构、不引入双版本 Bevy 兼容层、不扩大到其他平台或新增 CI。BRP 使用已有的兼容标签，不另外安排 BRP 升级项目。[R1][R2][R3]

不增加迁移前状态记录、快照、历史提交登记或独立性能对照任务。已有测试与 benchmark 入口必须保留并能编译，但不要求先收集旧版本测试、截图或性能数据。修复只服务于本次升级；不能靠删除测试、忽略失败、移出 workspace、关闭原有功能、临时空实现或放宽 unsafe/panic 约束换取通过。

## 每份方案的编译出口

每份方案完成后，验收对象都是整个 workspace，而不是本方案主要修改的几个 crate。使用 Windows 64 位的 x86_64-pc-windows-msvc，Rust 不低于 1.98.1，并具备现有 rust-lld.exe 配置所需的 MSVC/Windows SDK 环境。Bevy 自身的最低 Rust 要求是 1.97.1，但选用的 BRP v0.4.0 把本项目实际下限提高到 1.98.1。[R3][R4][R5]

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --target x86_64-pc-windows-msvc --locked
cargo build --workspace --all-targets --target x86_64-pc-windows-msvc --locked
```

check 用来尽早发现类型和宏展开问题；build 用来确认实际代码生成、链接及测试/benchmark 目标也能构建。只执行 check 不能声称完整构建通过。这里的 all-targets 指 Cargo 目标，不是扩大到其他操作系统；也不添加会改变原有 feature 范围的 all-features。

本方案触及的既有测试和新增回归测试必须通过。第一份方案还要让全部既有库/集成测试通过，不能把已知失败转交给后续方案；后续方案负责更深入的契约核对和补充覆盖，而不是补交前面的编译结果。编译失败时就在当前方案范围内闭合修复，不进入下一份方案。

## 证据与不确定性的处理

“确定必改”表示在已读取的项目代码中找到了与 0.20 不兼容的用法；“命中后修改”表示上游有变更，但还不能断言仓库每个候选文件都使用了它。实现时要对 src、tests、benches、Gallery 和可编译文档示例一起搜索。未命中的迁移条目不制造修改；契约已经满足的模块不为了让方案看起来有工作量而重写。

文中的编译命令和运行期检查是实施者的验收要求，不是本次已经取得的通过结果。任何具体 API 字段、构造器或调度点仍有疑问时，以锁定的 v0.20.0 源码和真实编译诊断为准，不能用默认值吞掉错误或猜一个 API 名称补洞。

## 依赖必须作为一组切换

根 Cargo.toml 目前把 bevy 和 bevy_remote 固定为 =0.19.1，bevy_brp_runtime 使用 v0.3.1，直接依赖的 wgpu 为 =29.0.4。升级时，这些不能各自停在不同引擎世代。目标组合如下。[R1]

| 项目 | 本次目标 | 处理理由 |
| --- | --- | --- |
| bevy | =0.20.0 | 用户指定的引擎版本，不使用浮动 main。 |
| bevy_remote | =0.20.0 | Gallery 直接使用它，必须与引擎一致。 |
| bevy_brp_runtime | 同一 git 仓库的 tag = "v0.4.0" | 该标签已经依赖 Bevy 0.20.0，无须修改 BRP 仓库。 |
| bevy_brp_mcp | metadata 为 0.4.0，安装说明指向 v0.4.0 | runtime 与外部工具配套，避免仍启动旧工具。 |
| wgpu | 30 系列，default-features = false，保留 dx12 | Bevy 0.20 的 bevy_render 使用 wgpu 30；直接传递 GPU 对象必须属于兼容的同一类型版本。 |
| accesskit | 保持 0.24 | Bevy 0.20 仍使用这一版本，不需要顺手升级。 |
| Rust | 至少 1.98.1 | 由 BRP v0.4.0 的实际要求决定。 |

上面的 BRP、wgpu、AccessKit 和 Rust 组合都有对应标签源码依据。[R3][R4][R7][R8][R31]

wgpu 可以用 version = "30" 与 Cargo.lock 固定实际解析结果；若继续采用现有精确锁版本风格，应填写 Cargo 实际解析并验证过的 30.x.y，而不是在没有解析结果时猜一个补丁版本。不能保留一个独立的 wgpu 29，再尝试把它的 Device、Queue 或 Instance 传给 Bevy 0.20。

edition = 2024、resolver = 3、Widgetry 自己的 0.1.0 包版本和不相关第三方库不因此改动。外部依赖版本仍只在 workspace.dependencies 定义，子 crate 继承；不在每个 crate 复制 Bevy 版本。[R1][R9]

## feature、锁文件和工具边界

保留 Bevy 的 default-features = false，以及根工作区已有的显式 feature 选择：std、async_executor、multi_threaded、bevy_asset、bevy_log、reflect_auto_register、bevy_scene、bevy_window、bevy_winit、bevy_render、bevy_core_pipeline、bevy_ui、bevy_ui_render、bevy_ui_widgets、bevy_input_focus、ui_picking、system_clipboard。Gallery 的 png/jpeg 仍由它自己的 manifest 贡献；其他成员按原有职责贡献所需 feature，不用打开默认 feature 或完整 UI profile 掩盖缺失依赖。[R1][R4][R30]

Cargo.lock 由 Cargo 根据新 manifest 重新解析，不手工替换锁文件中的版本和校验值。初次解析不能带 --locked；形成新的锁文件之后，所有方案的编译出口均使用 --locked。只接受本次版本切换及其必要传递依赖变化，不把无关库更新混进来。用依赖树确认 Widgetry、Gallery、BRP runtime 实际共享 Bevy 0.20；重复依赖需要逐项解释，重点排除仍在运行链路中的 Bevy 0.19，以及跨入 renderer 的 wgpu 29 对象。

```powershell
cargo tree --workspace --duplicates --target x86_64-pc-windows-msvc --locked
cargo tree --workspace -i bevy_ecs@0.20.0 --target x86_64-pc-windows-msvc --locked
```

这里只检查迁移后的依赖结果，不建立旧依赖图的保存任务。保留 .cargo/config.toml 中现有 Windows linker/rustdoc 配置，除非新工具链给出具体不兼容证据。也不为了宣布 Rust 下限而额外引入一套工具链管理机制。[R5]

根文件中的 MCP 安装注释与 workspace.metadata.tools 必须跟着 runtime 标签一起改到 0.4.0。外部 MCP 二进制是否已更新，与 Cargo 是否编译成功是两个问题：Gallery 依赖升级不会自动替用户替换已安装的 MCP。运行期验收必须实际使用匹配工具，不能只看 metadata。[R1][R3][R29]

## 22 个工作区成员的覆盖范围

以下是修改范围图，不是声称每个文件都存在确定错误。依赖清单覆盖 22 个成员；对受影响的成员同时覆盖 src、tests 和 benches。只读过关键调用点的模块，保留“命中后修改”判断，不把目录名当成已经证明的编译错误。[R1]

| 成员目录 | 本次应覆盖的内容 | 修改判定 |
| --- | --- | --- |
| crates/log | 宏及宏测试、Bevy 日志接口 | 依赖重编译；仅在实际 API 变化时修改。 |
| crates/app_logging | 宿主日志安装、错误处理和测试 | 核对插件/错误类型，保持日志语义。 |
| crates/asset | embedded 资源注册、字体和图标加载测试 | 不替换资源文件；检查加载接口即可。 |
| crates/theme | 配色数据、主题事件及应用入口 | 保持主题值；生命周期或反射变更命中才改。 |
| crates/core | scene、pointer、pointer/pressed、ui、font、disabled、text、icon、颜色与诊断 | 确定有硬性 API 迁移，也是多个运行期契约的共同边界。 |
| crates/button | BSN、官方 Button 集成、样式与交互测试 | 跟随构造/observer/Pointer 实际用法改。 |
| crates/check_box | checkbox、tri_state、indicator、样式与测试 | 保持二态/三态行为及变化通知。 |
| crates/radio_group | group/option 构造、选择与样式 | 保持组选择及禁用语义。 |
| crates/text_field | style 构造、只读与禁用、输入及支持测试 | 确定需要 TextInput 接入；不能仅让类型通过。 |
| crates/combo_box | field、popup、registration、样式及组合测试 | 核对 BSN、焦点、选择和 popup 关闭。 |
| crates/scroll_area | layout、pointer、disabled、headless/style | 确定有 Pointer 事件迁移；核对 scrollbar/viewport 类型与收敛。 |
| crates/list_view | model、behavior、view、virtualization、renderer 注册 | 保持稳定 ID、选择修复及虚拟化。 |
| crates/tree | model、behavior、renderer、view | 保持展开/折叠、可见项投影和选择。 |
| crates/table | interaction、resize、layout、projection、viewport | 核对指针拖动、布局裁剪与虚拟化边界。 |
| crates/waveform | renderer、view、geometry、runtime 及 render/update benches | 确定有 BSN/生命周期 observer；核对图像、相机和资源释放。 |
| crates/tooltip | headless/style、Pointer 生命周期及 benchmark | 保持延时、目标失效和 popup 收尾。 |
| crates/test_utils | pointer、scene、error、logging、benchmark 支持 | 确定需要新版事件构造器；不能漏掉所有调用方。 |
| crates/window | render、input、modal、scene、title_bar 各 controls、background | 确定有 renderer、Pointer、BSN、TextScroll 和生命周期接口迁移。 |
| crates/message_box | scene、lifecycle、结果与窗口组合测试 | 保持模态关系与只发一次结果。 |
| crates/file_dialog | view/controls/input/window、confirmation、runtime/worker/storage 及 tests/benches | UI 集成跟随升级；不改无关文件系统和业务协议。 |
| crates/bevy_widgetry | facade 公共导出、public_api/focus/pointer/color_composition tests、benchmark | 保持应用入口；公共 API 测试也在同一编译闭环中。 |
| gallery | main、pages、color_showcase、窗口页面、测量入口与 benches | 完整构建并进行实际 GUI 验收；不强加新的 Gallery 单元测试。 |

这些目录承担的迁移责任可以不同，但不能把“只改 core 就能通过”当成默认假设。例如公共测试工具返回旧 Pointer 类型时，消费者的测试、benchmark 和 facade 测试都会一起受影响；renderer 的底层版本不兼容也会直接阻断 Gallery 链接。[R16][R24][R29][R32][R33]

## 必须与版本切换同批交付的硬性修复

版本号改动与下列修复属于同一个完成边界。第一份方案可以覆盖很多文件，但它仍只有一个目标：整个工作区在新的引擎依赖上重新形成完整构建。不能把各 crate 的编译错误留给后面的行为方案。

| 接口交汇处 | 已确认的项目入口 | 同批修复要求 |
| --- | --- | --- |
| BSN 的 Scene 表达 | text_field/src/style.rs、waveform/src/renderer.rs、Gallery 等 | Scene 函数/值按 0.20 语法表达；不要误改模板、组件构造函数与普通 Rust 花括号。 |
| 生命周期 observer | core/src/pointer/pressed.rs、waveform/src/renderer.rs、window/src/input.rs | 把 On<Insert, T>、On<Remove, T>、On<Add, T> 等已命中的旧签名迁到相应新版事件类型，如 On<Remove<T>>。 |
| 高层 Pointer 事件 | core 与 ScrollArea Pointer、test_utils | 迁到 PointerPress、PointerRelease、PointerCancel、PointerDragStart/End 等具体类型；公共指针信息访问改用 event.pointer。 |
| 字体接口 | core/src/font.rs 的测试 | FontSource::Monospace 改成对应构造函数；所有同类用法一起搜索。 |
| 文本输入组件 | text_field/src/style.rs | TextInput 加入输入根实体，并保留 EditableText 数据及只读/禁用守卫。 |
| 文本滚动与 IME | window/src/input.rs | 移除 TextScroll 依赖，按 EditableText 的编辑视口计算 IME 位置；替换已失效的旧滚动调度依赖。 |
| renderer 包装类型 | window/src/render.rs | WgpuWrapper/Arc 包装改用 0.20 的 RenderQueue、RenderAdapter 等构造器。 |
| 渲染窗口集合 | window/src/render.rs | ExtractedWindows resource 改成渲染世界的窗口组件查询，使用 MainEntity 对应主世界窗口。 |

项目调用点见 Pointer、TextField、窗口输入、renderer、字体及 Waveform 源码；目标接口分别对照上游事件、文本、Scene 与 renderer 定义。[R10][R11][R12][R15][R16][R17][R18][R19][R20][R22][R24][R25][R26][R27]

Pointer 改动必须同时覆盖事件观察者、测试工厂、直接 trigger 的构造表达式和它们的调用方；不能只替换 import。低层 PointerInput 不是同一类型，它的 pointer_id/location 字段不能跟着高层事件做全局替换。ValueChange<T>、FocusedInput<T> 等其他泛型事件也不能被一起“扁平化”。[R11][R15][R16]

文本部分至少要保证新增 TextInput 真正进入原来的输入实体，原来的只读和禁用保护仍有效；renderer 部分至少要完成新类型的真实构造和窗口查询。不能以删除插件安装、跳过 renderer、移除输入组件或注释测试来缩小第一份方案。

## 只能按实际命中决定的兼容项

还要搜索新旧接口的交汇点，但以下内容不是已经确认全仓都需要改：Val 的单位解析及穷举 match、CalculatedClip 的读取、单角 BorderRadius 构造、TextReader、反射类型数据注册和动态转换、Mesh 包围盒方法、ExtractComponent/ExtractResource、自定义渲染提取、调度内部图查询和新的错误包装。命中时对照官方迁移索引与对应的 v0.20.0 源码决定修改，不命中就不扩展工作。[R6]

搜索范围要包含手工实现的 trait、泛型 helper、宏生成代码、文档示例和 benchmark。编译器会发现多数签名错误，但无法证明被写进字符串中的反射字段名、调度顺序和 GUI 行为仍正确。例如 window/input.rs 的 focused_event 通过 DynamicStruct 写入 focused_entity、input 和 window，再 FromReflect 构造 FocusedInput；即使这一段仍能编译，也必须在后面的输入方案验证运行期构造成功。[R20]

有些变化只是推荐写法，不要和移除接口混为一谈。旧的 BSN 列表分隔形式若在 0.20 仍只是弃用，可在 Scene 方案统一；若实际展开失败，则它就是第一份方案必须修复的编译项。shader 工具链变化也不自动意味着 Waveform 要改写 shader：已读取的 renderer 使用的是内建 ColorMaterial、Mesh2d 和 ViewportNode，没有在该路径实现自己的 shader。[R12][R24]

## 版本切换完成的判定

除公共编译出口外，第一份方案必须运行全部既有库和集成测试；目的是不把已经暴露的问题包装成“后续行为适配”。

```powershell
cargo test --workspace --lib --tests --target x86_64-pc-windows-msvc --locked
```

BSN、Pointer、文本和 renderer 改动可能来自不同 crate，但它们共同使用同一份新 Cargo.lock。所有这类编译修复应当在同一方案的交付内容里出现。后续方案可以发现并修复新的隐蔽回归，也可以补齐此前缺少的测试；它们不能被用来证明一个当前已经失败的工作区“暂时算完成”。

完整构建通过仍不等于升级结束。真实窗口透明合成、IME 候选框位置、指针消失后的 pressed 状态、只读编辑和响应式 BRP 唤醒需要后续方案的运行期证据。这里不把它们省略，也不把 Windows 上实际运行的结论写成已经验证。

## 事件外形改变，交互身份不能改变

高层事件从 Pointer<Press> 这类包装形式改为 PointerPress 等具体类型。公共指针信息位于 event.pointer 中：id、target 和 position 对应指针身份、渲染目标与坐标；需要 Location 时使用目标版本提供的转换接口。被事件触发/传播到的实体仍要和指针指向的渲染目标分开理解，不把 window/camera target 当成控件实体。[R11]

新事件构造必须使用公开 API。公共 Pointer 信息包含内部传播状态，不能为了拼结构体而假定所有字段都公开，更不能引入 unsafe 或重新在 Widgetry 定义一个旧 Pointer<T> 来绕开上游类型。press、release、cancel、click、drag 等测试工厂应输出对应的新事件，保留原来用于验证的 button、hit、count、duration 和 distance。[R11][R16]

低层 PointerInput 仍承担原始输入队列，继续使用自己的 pointer_id、location、action。直接 world.trigger(PointerPress ...) 和向消息队列写 PointerInput 是两种不同刺激：前者验证 observer 局部行为，后者才能验证 picking/hover/pressed 的真实调度组合。不能用前者替换所有后者，然后宣称输入管线已经覆盖。[R15][R16]

## TextInput 是输入能力，EditableText 是编辑数据

WidgetryTextField 和 WidgetryReadOnlyTextField 的基础 Scene 当前只插入 EditableText。0.20 的交互处理位于 TextInput，且 TextInput 会要求 EditableText；仅保留旧组件可能仍能显示文字，却不再进入官方键盘和指针输入查询。因此普通和只读输入根都要接入 TextInput，不能通过让只读框缺少 TextInput 来阻止修改，否则选择、导航和复制也会一起丢失。[R18][R19]

只读与禁用不是同一种状态。只读保留允许的选择、移动与复制，同时拒绝会改变文本的输入；禁用继续按 Widgetry 已有契约阻止交互。上游 TextReadWriteMode 与 TextEdit::is_destructive 可以作为对齐入口，但是否取代现有过滤器，要看它是否覆盖本项目的所有输入通道，不能仅凭枚举名称判定完全等价。[R18][R19]

当前 Widgetry 还会清理 pending_edits 和 pending_paste；只读过滤包括 Cut/Paste/Insert、各种删除以及 IME compose/commit。迁移时既检查真实键盘输入，也检查已经排队的编辑和异步粘贴，防止输入先排队、随后状态变为只读或禁用时，旧任务仍写入文本。新的破坏性编辑类型不能因为旧的黑名单没有列到就穿过保护。[R18]

这些守卫仍必须处于禁用状态确定之后、EditableText 实际消费编辑之前。不要只改查询组件而丢掉原有调度先后。TextField 的只读配色、焦点配色、光标和选区颜色继续通过既有样式 owner 管理，不借 TextInput 接入重新建立一套状态。[R18][R21]

## DX12 renderer 的新资源类型与窗口身份

window/render.rs 当前手工创建 DX12 Instance、Adapter、Device 和 Queue，并选择 DxgiFromVisual 以支持透明窗口。这个路径直接跨越 wgpu 与 Bevy renderer 的类型边界，所以 wgpu 30 对齐与构造器迁移已属于第一份方案的硬性内容。[R10]

0.20 的 renderer 包装通过各自公开构造器建立，不能继续使用已移除的通用 WgpuWrapper 或套旧的 Arc 结构。已核对的形式是 RenderQueue::new(queue)、RenderAdapterInfo::new(adapter_info)、RenderAdapter::new(adapter)、RenderInstance::new(instance)；RenderCreation::manual 的完整参数以目标标签定义为准，Device 也必须来自同一套兼容 wgpu。[R25][R26]

ExtractedWindows 不再作为旧 resource 供遍历；使用渲染世界里的 ExtractedWindow 与对应 MainEntity。比对 SortedCameras 的 Window target 时，比较的是主世界 native Window 身份，不是渲染世界本地 Entity。多窗口下即使类型都能对上，身份混用也可能让“有无 camera”的判断作用到错误窗口。[R25][R27]

InstanceDescriptor、DeviceDescriptor、BackendOptions、Dx12BackendOptions 和功能掩码按 wgpu 30 的实际 API 调整。保留透明 renderer 的目标与错误传播；不能为了省事换成不透明默认 renderer，或重新要求用户手动设置一个外部环境变量才能透明。运行失败必须留下明确的 adapter/device 错误，而不是静默继续。

## 参考来源

[R1] Widgetry 工作区、成员与依赖声明

`https://github.com/slc90/bevy_widgetry/blob/main/Cargo.toml`

[R2] Widgetry 项目范围与架构约束

`https://github.com/slc90/bevy_widgetry/blob/main/rules/project-context.md`

[R3] BRP v0.4.0 工作区版本与 Rust 要求

`https://github.com/slc90/bevy_brp/blob/v0.4.0/Cargo.toml`

[R4] Bevy v0.20.0 工作区与 Rust 要求

`https://github.com/bevyengine/bevy/blob/v0.20.0/Cargo.toml`

[R5] Widgetry Windows 构建配置

`https://github.com/slc90/bevy_widgetry/blob/main/.cargo/config.toml`

[R6] Bevy 0.19 → 0.20 官方迁移索引

`https://bevy.org/learn/migration-guides/0-19-to-0-20/`

[R7] Bevy v0.20.0 renderer 的 wgpu 依赖

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/Cargo.toml`

[R8] Bevy v0.20.0 AccessKit 依赖

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_a11y/Cargo.toml`

[R9] Widgetry 依赖管理规则

`https://github.com/slc90/bevy_widgetry/blob/main/rules/dependencies.md`

[R10] Widgetry window renderer

`https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/render.rs`

[R11] Bevy v0.20.0 Pointer 事件定义

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_picking/src/events.rs`

[R12] Bevy v0.20.0 BSN 与 Scene 说明

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_scene/src/lib.rs`

[R15] Widgetry 按下状态所有权

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/pointer/pressed.rs`

[R16] Widgetry Pointer 测试构造器

`https://github.com/slc90/bevy_widgetry/blob/main/crates/test_utils/src/pointer.rs`

[R17] Widgetry 滚动条拖动所有权

`https://github.com/slc90/bevy_widgetry/blob/main/crates/scroll_area/src/pointer.rs`

[R18] Widgetry TextField 构造、只读和禁用处理

`https://github.com/slc90/bevy_widgetry/blob/main/crates/text_field/src/style.rs`

[R19] Bevy v0.20.0 TextInput 输入处理

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui_widgets/src/text_input.rs`

[R20] Widgetry 多窗口键盘、IME 与焦点路由

`https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/input.rs`

[R21] Widgetry UI 调度约束

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/ui.rs`

[R22] Widgetry 默认字体与调度测试

`https://github.com/slc90/bevy_widgetry/blob/main/crates/core/src/font.rs`

[R24] Widgetry Waveform renderer 与资源回收

`https://github.com/slc90/bevy_widgetry/blob/main/crates/waveform/src/renderer.rs`

[R25] Bevy v0.20.0 renderer 资源与渲染入口

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/renderer/mod.rs`

[R26] Bevy v0.20.0 renderer wrapper 构造器

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/renderer/wgpu_wrapper.rs`

[R27] Bevy v0.20.0 渲染世界窗口组件

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/view/window/mod.rs`

[R29] Gallery 入口与 BRP runtime 安装

`https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs`

[R30] Gallery package 名、依赖和 benchmark 入口

`https://github.com/slc90/bevy_widgetry/blob/main/gallery/Cargo.toml`

[R31] BRP v0.4.0 runtime 的依赖继承

`https://github.com/slc90/bevy_brp/blob/v0.4.0/crates/runtime/Cargo.toml`

[R32] Widgetry 测试规则及 Gallery 例外

`https://github.com/slc90/bevy_widgetry/blob/main/rules/testing.md`

[R33] Widgetry facade 与平台边界

`https://github.com/slc90/bevy_widgetry/blob/main/crates/bevy_widgetry/src/lib.rs`

