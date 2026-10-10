# 06 | 验证 Windows 渲染、Waveform 与 BRP 运行链路

[返回方案总览](00-overview.md)

## 目标

让升级后的真实桌面应用能正确显示透明多窗口和 Waveform，并能通过匹配版本的 BRP 驱动交互。

## 范围

window renderer/owned 生命周期、Waveform renderer、Gallery 启动及运行时验证；不修改 BRP 仓库。

## 预期产出

DX12 透明显示、多窗口创建关闭、Waveform 更新与资源回收、BRP 连接和响应式唤醒均满足现有场景。

## 与前后方案的关系

方案 01 已完成 renderer 类型和构造器迁移；本方案验证它们在真实 GPU 与窗口上的效果。之后只剩当前文档同步与全仓收口。

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

## DX12 renderer 的新资源类型与窗口身份

window/render.rs 当前手工创建 DX12 Instance、Adapter、Device 和 Queue，并选择 DxgiFromVisual 以支持透明窗口。这个路径直接跨越 wgpu 与 Bevy renderer 的类型边界，所以 wgpu 30 对齐与构造器迁移已属于第一份方案的硬性内容。[R10]

0.20 的 renderer 包装通过各自公开构造器建立，不能继续使用已移除的通用 WgpuWrapper 或套旧的 Arc 结构。已核对的形式是 RenderQueue::new(queue)、RenderAdapterInfo::new(adapter_info)、RenderAdapter::new(adapter)、RenderInstance::new(instance)；RenderCreation::manual 的完整参数以目标标签定义为准，Device 也必须来自同一套兼容 wgpu。[R25][R26]

ExtractedWindows 不再作为旧 resource 供遍历；使用渲染世界里的 ExtractedWindow 与对应 MainEntity。比对 SortedCameras 的 Window target 时，比较的是主世界 native Window 身份，不是渲染世界本地 Entity。多窗口下即使类型都能对上，身份混用也可能让“有无 camera”的判断作用到错误窗口。[R25][R27]

InstanceDescriptor、DeviceDescriptor、BackendOptions、Dx12BackendOptions 和功能掩码按 wgpu 30 的实际 API 调整。保留透明 renderer 的目标与错误传播；不能为了省事换成不透明默认 renderer，或重新要求用户手动设置一个外部环境变量才能透明。运行失败必须留下明确的 adapter/device 错误，而不是静默继续。

## 首帧呈现、窗口生命周期与透明背景

项目现在会在新窗口没有对应写出 camera 时推迟 initial present，并显式提交窗口渲染命令。这是为多 swap chain 与相机准备时序设置的保护。核对 0.20 的实际渲染 schedule、命令提交和窗口准备位置，让每个窗口的获取/绘制/提交/呈现对应正确的 camera；不能只把旧系统挂到一个同名 set 就认为时序相同。[R10][R25][R27]

若上游已经消除了某个保护原本针对的问题，可以移除对应 workaround，但必须用窗口运行场景证明首帧和多窗口均正确。也不能在没有证据时继续追加 flush，造成不必要的提交或把原本应共用的工作拆坏。检查窗口首次出现、尚未准备好 camera、最小化/恢复、连续开关多个窗口和最后一个窗口关闭。

保持 native Window 的 transparent、无系统 decorations 和 PreMultiplied 合成要求；自绘 title bar、边框和圆角正常，Image 背景的 Stretch/Cover/opacity 及未加载时透明行为不变。特别观察透明边缘、标题栏、最大化/还原以及图片 alpha，而不是只确认窗口“能出来”。[R28]

调用方提供的 Window/camera 在界面 root 销毁时保留；owned 窗口负责释放自己创建的 Window/camera。modal 使用 native Window 作为 parent，多个有效 child 共享阻挡、最后一个结束后释放，焦点恢复仍由既有输入逻辑管理。窗口 UI root、camera 与渲染世界窗口的删除不是同一时刻，要检查没有对已经失效实体继续操作。[R28]

没有 RenderApp 的 headless 场景继续跳过真正渲染安装，不能为了让 GPU 测试通过破坏纯 ECS/构造测试入口。

## Waveform 保留内建材质与增量绘制

已读取的 Waveform renderer 使用 Camera2d、Mesh2d、MeshMaterial2d<ColorMaterial>、离屏 Image、RenderLayers 和 ViewportNode，没有在这条路径实现自定义 UI shader。Bevy 的 shader 工具链和 UI renderer 变化不是重写 Waveform shader 的理由；只有发现实际自定义提取/材质接口命中变更时，才增加对应适配。[R24]

保留图像的用途与 alpha/颜色含义、mesh/material 资产归属、独立 layer 分配，以及 viewport 尺寸驱动的 image/camera 更新。尺寸为零或不可见时的 camera 活跃状态、恢复显示时的尺寸同步、多个 Waveform 同时存在时的 layer 隔离都必须正确。renderer 的 PostUpdate 顺序仍要满足布局尺寸已确定，而 viewport render target 更新不会再覆盖一个过期结果。[R24]

样式 tick、颜色 tick、数据 revision 和尺寸变化继续分别驱动必要工作。数据/几何变化更新 mesh，纯颜色变化走既有 recolor 路径，完全不变不重复构建；不能用每帧重建全部资产换取新版画面更新。UI 变化被保留式渲染器正确提取，与保留这套增量决策必须同时成立。

初始化 Scene 失败需要释放已分配的 Image/Mesh/Material 并归还 layer；正常移除 Renderer 后同样回收资产和自己持有的 Scene。生命周期 observer 的签名变化不能让清理失效、重复或误回收其他实例。主要自动化入口是 waveform/tests/renderer.rs、headless.rs 与 renderer 内部测试；render/update benchmarks 继续能编译。[R24]

## BRP 与 Gallery 的真实运行验收

Gallery 的 package 名是 widget_gallery。它安装 BrpRuntimePlugin::default()，并通过透明 render_creation、WinitSettings::desktop_app() 和各 Widget 插件启动。保持这条实际使用路径，不换成专为测试拼出的简化应用，也不再加一套重复 RemotePlugin/runtime。[R29][R30]

```powershell
cargo run -p widget_gallery --target x86_64-pc-windows-msvc --locked
```

BRP runtime 与 MCP 都使用 v0.4.0 配套来源。外部工具需要更新时，安装命令应与根 metadata/注释一致；安装工具不属于修改 BRP 仓库。[R3][R31]

```powershell
cargo install bevy_brp_mcp --git https://github.com/slc90/bevy_brp --tag v0.4.0 --locked --force
```

通过 Gallery 的实际配置连接，不猜固定端口或私自改变项目的连接约定。验证能够读取场景状态、执行输入、观察控件变化并取得有效截图。窗口处于 desktop_app 的响应式空闲状态时，外部请求仍应触发所需工作；不能把事件循环改成永久连续刷新来掩盖 BRP 唤醒失败。启动应用成功不等于 MCP 已经更新，读取实体成功也不等于输入、截图和窗口定位都正常。[R29]

运行场景覆盖基础控件、滚动/列表/Tree/Table、文本与 IME、tooltip/popup、主题与颜色组合、独立窗口、modal MessageBox/FileDialog 以及多个 Waveform。用真实 state 变化结合截图判断，不以一张主界面截图替代全部交互。

Gallery 按项目规则只要求正常编译和 GUI 运行期验证，不新增 Gallery 单元/集成测试义务。现有测量入口与脚本只检查与本次 API 变化相关的契约，保留其目标和输入/输出格式；不安排新的旧版截图、性能基线或全套 benchmark 对照收集。[R29][R30][R32]

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

[R10] Widgetry window renderer

`https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/render.rs`

[R24] Widgetry Waveform renderer 与资源回收

`https://github.com/slc90/bevy_widgetry/blob/main/crates/waveform/src/renderer.rs`

[R25] Bevy v0.20.0 renderer 资源与渲染入口

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/renderer/mod.rs`

[R26] Bevy v0.20.0 renderer wrapper 构造器

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/renderer/wgpu_wrapper.rs`

[R27] Bevy v0.20.0 渲染世界窗口组件

`https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/view/window/mod.rs`

[R28] Widgetry 窗口公开契约

`https://github.com/slc90/bevy_widgetry/blob/main/crates/window/src/lib.rs`

[R29] Gallery 入口与 BRP runtime 安装

`https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs`

[R30] Gallery package 名、依赖和 benchmark 入口

`https://github.com/slc90/bevy_widgetry/blob/main/gallery/Cargo.toml`

[R31] BRP v0.4.0 runtime 的依赖继承

`https://github.com/slc90/bevy_brp/blob/v0.4.0/crates/runtime/Cargo.toml`

[R32] Widgetry 测试规则及 Gallery 例外

`https://github.com/slc90/bevy_widgetry/blob/main/rules/testing.md`

