# Core

## 功能概述

提供 Widgetry 多个控件共同需要的 UI 基础设施，包括内容配色、默认字体、SVG Icon、有效 Disabled、Pointer State、Scene 错误处理、公共离屏渲染和 UI 准备阶段能力。

## 公共离屏渲染

`WidgetryOffscreenPlugin` 安装一个 App 共享的离屏层号池和 Image Asset 支持。`OffscreenTarget::allocate(world, owner)` 为有效 Owner 创建独立目标并租用唯一层号；消费者通过 BSN 创建相机与绘图 Scene，自行指定 Projection、ClearColor 和 MSAA，并将所有相机与几何绑定到该层号。

`OffscreenTarget::bind` 接收 UI Viewport、Camera 和专属 Scene，将目标交给 `OffscreenViewport`；消费者在 Owner 上挂载此 Component。绑定失败会释放目标、层号及本次接管的专属 Scene；已租用的 Scene、Camera、Viewport 和其他 Owner 不被改写或删除。相机必须属于传入 Scene；同一 Owner 可以用新 Scene 替换原 Viewport 绑定。绑定前的 Scene 构造失败由消费者调用返回 `Result` 的 `OffscreenTarget::release` 回滚。消费者专属或共享 Mesh/Material 不由公共层删除。

消费者在 `OffscreenUpdate` Set 内调用 `OffscreenViewport::update`，该 Set 位于 UI Layout、Visibility 传播之后和原生 Viewport 纹理尺寸更新之前。返回当前物理尺寸，隐藏或零尺寸时返回 None 并停用相机；原生 Viewport 系统维护目标 Image 尺寸。移除 Owner 上的 `OffscreenViewport` 会回收专属 Image、层号和 Scene，且仅清除仍指向该相机的 Viewport 关联。删除一个目标不影响其他实例或宿主 Asset。

Offscreen API 返回 `OffscreenFailures`，成员保留 `OffscreenError` 分类、Owner、目标 image/layer、绑定快照与检测阶段。资源缺失、层号耗尽、非法绑定、关联破坏和必要 Component 缺失分别可识别；主失败和清理失败按成员保留独立 ID/source/origin，不拼接为字符串。分配按 Owner 的显式原因键、更新按绑定的显式结构原因键复用持续诊断；确认结构恢复后记录一次 INFO。隐藏、零尺寸及正常删除不创建错误或 handled DEBUG。显式 release 尽量完成仍可执行的回收并返回所有清理失败；移除 Component 的回收在实际 queued Command 中以 Error Severity 传播给宿主，不配置 Handler。


## 当前设计思路

Core 尽量把跨 Widget 的机制集中管理，同时让各 Widget 保有自己的业务 State。共享能力通过专门 Plugin、Component 或公开适配方法装配，不要求应用接受一套取代 Bevy 的 UI Runtime。

## 主要功能

- 默认字体设置：只对适用的新增 `TextFont` 使用缺省字体，保留调用方的显式字体。
- `WidgetryText` 和内容 Foreground：按最近颜色作用域、Theme 与覆盖解析 Text / TextSpan。
- `WidgetryIcon`：通过 SVG 生成图像，可设置、替换或清除显式颜色与 SVG 输入。
- 有效 Disabled：组合祖先继承、本地请求和控件原生限制，并投影到官方输入状态。
- Pointer：把有效 Mouse、Custom、Touch 的 Hover 命中投影到官方 Hovered / DirectlyHovered；管理所标记控件的 Pressed Ownership 清理。
- Scene：同步返回 `SceneFailure`，Deferred 将保留原 origin/source 的诊断交给宿主；只清理本次失败预约 Entity。
- 提供动态 UI 同帧准备与共享 Z-index 标识等能力。
- `exclusive_selection` 是 RadioGroup / Segmented 共用的内部选择能力，维护结构回退历史、首次选择策略及互斥 Checked 提交；不建立第二份 Selection，不通过 Facade 公开。
- `WidgetryInputTarget` 提供 Workspace 内组合控件根到实际输入实体的路由元数据；Window 的 Tab 与初始焦点解析使用同一目标，不承载控件的编辑或业务状态。

## 行为与状态契约

通过 `@WidgetryIcon` 构造 Scene 时必须提供非空的 SVG `path`；未指定或显式传入空路径形成 `IconError::EmptyPath`，由 Scene 保留领域 source 和诊断身份；Template 与 queued 边界显式使用 Error Severity。`WidgetryIconProps::default()` 仍可用于补齐其他属性，但默认空路径不能直接构造图标。构造阶段不检查文件是否存在或资源是否已加载，非空路径继续异步加载，加载、解析和栅格化失败按下述领域诊断契约传递。`max_size`、颜色覆盖及默认样式保持原有行为。

`WidgetryIcon::set_svg_in_world()` 与 `WidgetryIcon::set_svg()` 用于切换已构造图标的 SVG，不用于先创建空图标再补充初始路径。直接构造底层 `WidgetryIcon` 组件的内部接口不经过 Props 校验。

图标的 `max_size` 与未指定时的 SVG 固有尺寸用于逻辑布局；实际栅格化使用所属 UI 相机目标与 `UiScale` 合成后的倍率。图像子节点显式保留逻辑尺寸，增加栅格像素不会撑大控件。倍率或目标相机变化后，在 UI 目标信息传播完成、内容布局前更新既有图像节点；相同资产与栅格规格复用缓存，颜色仍独立投影到 ImageNode，不因配色变化重新栅格化。

`IconFailure = WidgetryFailure<IconError>` 覆盖空初始路径、无效目标/颜色状态、托管颜色 Owner、无效颜色、缺失 AssetServer/内部 ImageNode、SVG 读取/解析、资产加载及像素分配失败。同步颜色与 SVG setter、颜色查询返回领域失败；无效 Icon 颜色通过共享 Core 校验映射分类，保留原 CoreError 原因链及同一 origin；领域字段和首个 context 保留目标、路径、缺失结构及颜色状态。读取与解析保留原生 source；像素失败保留实际 width/height、请求 max_size 和 scale。调用方只在真正 ECS 边界使用 `failure.into_bevy_error()`，不能使用裸 `?` 依赖 Bevy 默认转换。

SVG Loader 在读取/解析源头创建一次诊断，显式转换到 Bevy 的 Loader 返回接口。Owner 从 `LoadState::Failed` 取回 Loader 的共享诊断，保留同一个 origin/source/首次 Backtrace；Loader 之前的 reader/source/metadata 失败由 Owner 首次惰性纳入 Icon 领域。已经失败的加载保留现有图像或初始空态，实际采取此降级后记录一次 handled DEBUG。正常等待、尚未物化、零尺寸不创建错误或假恢复。

每个图标以资产身份、请求栅格规格及真实失败尺寸/scale 识别持续栅格异常，以共享 LoadState 失败对象区分加载失败与重试，缺失 ImageNode 的状态独立保留。重复观察不新捕栈，等待保留身份，确认成功生成或取回目标图像后记录一次恢复 INFO；恢复后再失败或独立 Owner/原因使用新的源头身份。多图标无法处理的失败以 batch 逐项到达宿主，不丢弃循环中后续诊断。内容 Foreground、覆盖色、disabled 色与旧图像降级策略保持原约定。


`WidgetryPointerPressQuery::started_this_frame(entity)` 只读查询共享 Pointer 层在本帧确立的新按压会话，供 `Update` 等后续阶段区分真实重按与 `Pressed` 恢复。结果每个 `PreUpdate` 重建；无关 Pointer 或鼠标键导致的 `Pressed` 恢复不计为新会话。

原生 Mouse 所属窗口失焦时终止受管按压，即使外部松手未产生 Pointer Cancel/Release，或失焦与重新聚焦合并在同一帧，也不会恢复旧会话。Custom/Touch Pointer 不以原生窗口焦点作为此取消条件。

存在 Bevy 默认 Mouse writer 时，Core 使用保持其移动/按键/滚轮语义的有序适配替换它，在 Winit 的失焦事件位置产生 `PointerAction::Cancel`。Cancel 后发送三键 Release 同步官方 `PointerPressState`，已清除的 click/drag 状态不会产生松手 Activate。同批次的后续新按下能建立独立会话，外部丢失 Release 也不会吞掉重新进入后的第一次按下。失焦帧使用本帧真实 Backend Hit 按相同 Layer/Depth/Pickable 规则保留 Mouse 命中，避免 Bevy 的整帧 Cancel 过滤丢弃前后按下。Custom/Touch 输入源不替换；宿主自行替换的 Mouse writer 不接管。原生 Button 接受的鼠标键均保留其原有 owner 语义。

内部限制可通过 `set_intrinsic_disabled_for<R>` 按理由类型独立设置和撤销；已有 `set_intrinsic_disabled` 使用独立默认理由。所有内部理由按 OR 组合，撤销其中一项不会解除其他理由。

`validate_intrinsic_disabled_target(&World, entity)` 提供不修改状态的目标与运行资源预检，返回同一 CoreFailure 分类与真实实体访问 source；复合控件可在提交自身表现前确认 disabled 投影可执行。实际 setter 复用该校验，仍在原执行时机组合理由并刷新投影。

UI Hierarchy 的 Disabled 有效值采用 OR 继承，本地请求即使在继承期间修改也必须保留。官方 Hovered 包含子代命中，DirectlyHovered 限于直接命中；共享 Pointer 投影拥有相应写入权。Widgetry 内容颜色只影响使用标记的 Text，原生 Text 仍保留自身定义。Scene Props 在构造后不充当运行时状态。

Disabled 不通过 Native Window Owner 关系跨窗口传播。单独禁用的内容使用对应 Theme Fallback。`WidgetryPointerPressed` 只清理标记控件所拥有的真实 Pointer Press 会话，不接管预先存在的程序化 Pressed 或官方正常 Activation。默认字体可通过 `WidgetryFontPlugin` 或 `WidgetryAppExt::set_default_font` 启用，诊断状态与 Z-index 标识供自定义控件复用。

## 边界情况与错误处理

- 无效 Scene 展开返回 `SceneFailure`；Icon 同步更新返回 `IconFailure`，queued 操作及无法处理的 System 错误显式进入 Bevy 宿主 Handler。
- 构造失败仅清理本次新建的预约空 Entity，不回滚已经提交的业务副作用。
- SVG 加载失败、等待或过时更新需要按 Icon 的当前请求身份判断，不应破坏此前有效内容。
- 有效 Disabled 在 Deferred 操作实际 Apply / Flush 前不代表已生效。

## Scene 诊断与边界

`scene::spawn_scene`、`scene::apply_scene` 同步返回 `Result<_, SceneFailure>`，不自动调用宿主 Handler。调用方在真正的 ECS 边界显式使用 `SceneFailure::into_bevy_error()`；该类型不实现标准 Error，从类型上避免通用 BevyError 转换再次隐藏统一载体或改变原生 Panic 策略。

`SceneFailure::from_native()` 也可接收直接 Bevy Scene API 的 `SpawnSceneError`，附加 root/操作。适配逐层识别 CachedSceneApplyError、RelatedSceneError、TemplateBuildError；已受管的单个或多个诊断共享原 origin、ID、首次栈和真实 source，只增加上下文，不重新发源头日志。完整原生包装由 Arc 保留，`original()` 和 `original_scene_error(diagnostic)` 可访问缓存路径、关系索引及 Bevy 自身的上下文，而不假设标准 source 链可以穿透 BevyError。

未知原生 Scene 错误首次纳入时成为 `WidgetryFailure<SceneError>`。`SceneError` 保留原始对象，`native_template_error()` 可读取非标准 Error 的 BevyError；它的显示文本只是日志投影，不代替原对象。Bevy 0.20 没有公开的不含回溯的格式化入口，适配对其格式化文本剥离附加的栈帧，防止源头日志/普通格式化输出 Bevy 的边界栈；匹配原始对象和身份始终使用类型。原生 Severity 和 panic payload 在显式转换时保留，Widgetry 自身源头仍按 Error Severity 传播。

`source_error(error, ErrorFrame)` / `scene_source_error!(error; fields, message)` 是旧 Scene helper 消费者的源头适配入口，保留实际错误对象，并把已知字段放入拥有的 ErrorFrame。它不再表示字符串“已记录”标记；后续领域迁移可替换为各 crate 的具体错误类型。其他尚未迁移的运行时原生错误维持现有返回路径，不在 Scene 增量重写其异常状态机。

Deferred 继续使用 `Commands::queue`，预留根执行前删除属于正常取消。失败清理不是业务处理，不记录 handled DEBUG；既有 Root 的部分 Patch 和独立业务 Entity 不回滚。只有同步构造层清理其本次空预约，deferred spawn 另清理本次新根。

Core 内部的 `scene::attempt` 保留仅执行 BSN 与预约清理的 native 尝试，供已有持续失败 Owner 使用；它不由 Facade 导出，也不提前决定一次重试是否构成新的诊断。Owner 通过 `attempt::into_bevy_error()` 显式转交尝试结果：Template 内已有的受管诊断保持完整 origin/source/native 包装，纯 native 失败保持原有报告节奏。尚未进行领域迁移的 Owner 使用 `UnmigratedFailureState`，不为已受管源头重复记录 ERROR。ComboBox/ListView/Tree/Table 的内容重试仍由各自 Owner 负责报告和恢复，业务异常键及惰性诊断的迁移另行完成。公共 Scene API 和 Deferred 入口不绕过统一诊断。

native 尝试桥接也保留内层 Template 的 Severity 和 panic payload，与公共 Scene 共用提取规则；纯 native 分支不为一次重复观察构造新 origin。

## 共享状态与颜色诊断

`diagnostics::CoreFailure = WidgetryFailure<CoreError>` 承载共享颜色校验、Text 颜色 API、Disabled 请求、互斥选择辅助及 Native Mouse writer 安装失败。颜色错误保留实际 Color；目标错误保留 Entity 和缺失类型；托管颜色保留 Owner；Entity 访问与 Schedule 失败保留原生 source。源头只创建一次诊断，上层补操作、Root、consumer/field 等 context，不重建 ID 或重复源头 ERROR。

`color::validate_color()`、`WidgetryTextColorOverrides::{get,set_in_world,clear_in_world}`、`WidgetryText` 的同步颜色 setter 和 `SelectionHistory` 的可失败操作返回领域结果。Template 与 queued/System 出口显式转入 Bevy。直接消费者的颜色校验沿用自身尚未迁移的返回形状，但通过公共桥保留 Core 的 origin；同步 Widget setter 的 Root 通过 Bevy 原生 context 补充。

`set_intrinsic_disabled()` 和 `set_intrinsic_disabled_for<R>()` 返回 `Result<bool, CoreFailure>`：true 表示该原因请求实际改变，false 表示合法重复，不以最终 OR 结果是否变化判定。失效 Entity 或缺失 DisabledRuntime 在修改前返回失败；queued 入口在执行时校验并交宿主 Handler。多原因的 OR、显式请求与投影 Flush 时机保持原约定。

互斥选择先校验 Root 与选项 Entity，再按既定顺序提交 Checked；空集合、多个初始 Checked 和不属于集合的选择目标明确返回领域失败，初始校验拒绝时不先清除有效选择。提交 Checked 触发的生命周期 Observer 仍可改变后续结构；这种提交中的外部变更会保留原生访问失败，不承诺事务回滚，也不发虚假的成功通知。正常 repair、初始化及同值设置不增加状态通知。

具有 `WidgetryText`、已确认 UI 文本 context 的 Text/TextSpan 缺少自有 ColorState 或 TextColor 时，持续状态按缺失类型保留原诊断；多个文本失败全部以 batch 到达宿主，恢复后一次 INFO。非 UI 文本、尚无前景 context、EditableText 等原有过滤保持静默。Pointer 普通 Query 未命中、无焦点、撤销/Cancel，Foreground 缺省继承，以及 Font/UI 的正常装配不创建伪错误。

## 持续异常状态

Core 的 `diagnostics::FailureState<K>` 重导出独立 diagnostics crate 的 Bevy 桥接实现，供不同依赖层的 Owner 共用；它保存一个 Owner 当前异常的业务键和共享诊断。`K: Eq` 由 Owner 根据对象、操作、失败分类及请求代次定义，不读取错误 Display，也没有默认文本键。Owner 必须为独立请求或不同原因提供不同键；不同 Owner 各自保存状态，同文案不会导致合并。

`failure(key, create)` 的工厂返回 `Arc<dyn Diagnostic>`，仅在无活动异常或键改变时调用。源头可在工厂内用 `WidgetryFailure::new()` 创建，也可以转交已经存在的共享诊断，不重建其 origin。相同键再次观察复用相同 ID、source、首次 Backtrace 和已有上下文；返回值始终是可显式转为 BevyError 的 `WidgetryEcsError`，不会因为已记录而伪装成成功。该类型只保存受管诊断，原生 BevyError 的 Severity/panic payload 由原生边界保留，不纳入该缓存。

正常等待时保留状态，不调用 `recover()`。只有 Owner 确认恢复后调用 `recover(context)`，清除活动诊断并记录一次关联 `error_id`、`phase = "recovered"` 的 INFO。重复恢复没有日志，context 工厂不在正常空闲状态运行；切换到不同失败键或销毁 Owner 也不伪造恢复。恢复后再次失败调用新工厂，得到新的身份。返回给调用方的诊断可继续独立存活，最后一个拥有者释放后正常销毁。

`active()` 只借用当前诊断供读取 source/origin 或实际处理。真正执行降级等处理后，调用 `Diagnostic::record_handled(action, outcome)` 记录一次 DEBUG；它不会自动清除持续状态。仅观察、重复返回错误、终结报告或恢复 INFO 都不会标记 handled。宿主通过 Diagnostics 的 `dispatch_error()` 对同一 origin 只执行一次最终未处理报告；System 仍可每次返回 Err，不新增每帧诊断扫描。

作为分项迁移边界，现有控件对旧辅助类型的引用已明确改为隐藏的 `UnmigratedFailureState`，它仍保留旧首行文本比较和报告节奏。它不属于新键式状态契约，也不是 `FailureState` 的兼容别名；各控件在所属领域迁移增量中替换该类型，定义具体业务键和等待/恢复策略。本公共能力增量不为所有尚未分类的原生错误猜测身份。

## 已知限制

- Font、Icon 和 Scene 各能力有各自的装配前提；Core 不是全局业务 Model。
- 官方 Hover State 的独立写入者需要与共享 Pointer Plugin 协调。
- Headless UI 证明的是 ECS / Layout 阶段行为，不等于原生窗口的完整显示验证。


WidgetryPointerPlugin 安装本帧命中缓存。`pointer::refresh_hover` 在 Backend 完成本帧 Picking、事件批次开始之前，为指定 Pointer 恢复当前路由：有效的新捕获 owner 优先，否则按本帧 Backend Hit 的 Layer/Depth/Pickable 重建命中。事件批次已经开始时保留当前 HoverMap，由下一帧生成完整的 Out/Leave 与 Over/Enter 迁移，避免无声跳过事件。共享层只缓存本帧消息 ID，不重放旧帧命中；原生 Mouse 的有序失焦恢复与 Canvas 捕获结束共用该机制。此入口属于 Workspace 内输入组合能力，不通过 Facade 导出。
