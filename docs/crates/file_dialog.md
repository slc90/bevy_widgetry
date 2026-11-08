# File Dialog

## 功能概述

提供可独立使用的文件与目录选择组件，支持嵌入式内容以及自有 Native Window。它把目录浏览、选择、确认、后台文件系统工作和可视 UI 组合在同一套业务契约下。

## 当前设计思路

业务 Authority、文件系统请求和 UI 投影分离。每次构造产生独立 Session，目录读取、缓存重投影、选择意图准备、完整候选确认校验、创建文件夹、位置查询及偏好读写全部通过 JobManager 提交业务结果；UI 根据当前业务状态展示可见行和编辑器，不把物理 Row 当作文件身份。可选 Storage Scope 在 Session 间共享偏好。

## 主要功能

- 模式：PickFile、PickFiles、PickDirectory、PickDirectories、SaveFile。
- 导航与目录历史、搜索、Filter、Sort、隐藏项控制、常用位置与 Pin、新建文件夹。
- 按可见范围生成文件列表，可显示名称、类型、修改时间和可读大小；Selection 与 Active 分离。
- 单选、多选、Ctrl / Shift Range、Keyboard Navigation、Filename / Path Editor 和 IME 守卫。
- 可作为 Embedded 内容或独立 Owned Window，并支持 Modal / NonModal。
- SaveFile 的 Overwrite 确认复用 MessageBox。
- 后台目录流式读取、路径验证、创建目录、位置查询，及可选的偏好持久化和手工 Backend Reply 接入。
- 提供主题、颜色覆盖和 Style 配置，包括行高、Overscan、Sidebar 宽度与字体间距。
- 窄布局允许导航与底部操作换行，Sidebar 宽度不超过主体的 28%；文件名列优先保留至少 40% 宽度，其余详情列可收缩并裁剪，文件区域独立滚动。

## 行为与状态契约

每个 Session 最多产生一次 `WidgetryFileDialogResultEvent`，通知发生在对应结果 Authority 提交后。`World` API 即时提交；`Commands` 在实际执行时提交；后台回复只有被当前 Session / Token 接受后才生效。程序更新和交互对应的 ChangeEvent 报告真实已提交业务变化，同值不通知。Selection / Active 是业务状态而非复用 Row State；确认操作必须等待必要的 Pending Selection / Projection / Validation。取消不等待后台 I/O。SaveFile 仅返回目标 PathBuf，不替应用写文件。

### 输入与候选身份

可见界面保留顶部后退、前进、上一级、刷新、路径和搜索输入；导航与搜索装饰使用 Fluent Regular 图标。搜索图标位于 TextField 内部右侧 Suffix，不接受 Picking 或独立输入焦点。侧栏仅按 Home、Desktop、Documents、Downloads 的顺序投影 Backend 提供的同名位置；缺失位置不显示，不自行推导路径。侧栏与文件列表之间使用随主题变化的 1 逻辑像素静态分隔线，不提供拖动调宽。

界面不提供固定目录、新建目录、排序、隐藏文件和系统文件选项的操作入口，也不显示分组、磁盘或固定目录列表；对应 Action、偏好、后台处理及新建目录内部面板继续保留，UI 构造与样式同步不重置这些状态。文件详情表头保持 Name、Type、Modified、Size，不显示排序箭头或 UTC 标签；实际排序和 UTC 时间计算保持原有语义。底部状态、文件名或选择摘要、类型过滤、确认和取消按模式提供。

PickFile 与 SaveFile 提供 Filename Editor，多选与目录模式提供选择摘要，目录模式隐藏文件类型栏。PickFile 的临时文本只解析当前可见文件名，确认时使用真实 EntryId 并进入后台校验；公开 Filename Action 仅用于 SaveFile。地址栏隐藏常规 Windows 扩展路径前缀，未编辑的显示文本仍对应原始 PathBuf。详细行的修改时间使用 UTC。

方向键、Home / End、Page Navigation 改变 Active。多选模式 Space 切换 Selection，Ctrl+A 冻结当时已到达条目的范围；单选模式 Space 使用 Replace，SaveFile 或不匹配的 Kind 不执行 Selection，Enter 按模式 Activate。Selection Job 保留 Ctrl+A、Ctrl 点击和 Shift Range 的输入顺序，Filter / Search / Sort 可基于缓存重新投影而不改变原始路径与 Stable ID。Pending Selection / Projection 阻止 Confirm，Cancel 继续可用。

Path Enter 导航，Filename Enter 进入对应模式的确认校验；IME Composition 不触发业务提交或 Escape。待处理的非空预编辑与 IME Commit 仍受该守卫保护，空预编辑清理操作不视为正在合成，焦点恢复后可正常按 Escape 取消。不提供跨控件 Tab / Shift+Tab 遍历；Pointer 与显式 WindowInitialFocus 选择实际 editor 或 Entries（Pointer 标记 -1），隐藏 Panel 与 Disabled Control 保留各自的可操作性守卫，单个输入失败不丢弃同帧后续输入。Style 更新不重置 Session、编辑器 Cursor 或 Selection。

Path、Filename、Search 等 TextField 的业务 Part 保留在控件根，真正编辑及 Focus 位于其内部输入实体。正文通过 TextField 根 API 读写，输入事件和编辑变化按实际输入目标映射到相应 Part；主题与 Style 分别更新输入 Frame 和编辑实体。

字体、间距、Padding 和 Sidebar 宽度按首次构造及有效 Style 变化应用；动态生成或重挂的内容补齐当前配置。稳定 Dialog 不为外观配置递归全部字体，也不重复写回同值 Shell。地址栏仅在实际路径变化时同步，滚动、可见行、Header 几何、控件可用性与后台回复继续按当前状态推进。无效 Style 在写入外观前拒绝，不部分应用配置。

样式归属由 Control、Part、文件行绑定和独立窗口 / 覆盖确认的构造与移除维护。归属更新直接使用部件保存的 Root；稳定帧不扫描全部部件确认 Owner。延后构造和复用绑定会进入有界补偿，实体回收后旧关系不能影响新 Dialog。

颜色按调色板、部件绑定和实际输入变化更新；局部 Hover、Focus、Disabled 与选择变化只使对应受管部件或受影响的可见行失效。滚动复用行按新的业务绑定初始化颜色；嵌套组合控件沿用 FileDialog 提供的调色板，稳定帧不重复计算全部控件和文件行配色。

组合配色失败按所有部件和所有 Dialog 汇总受管诊断，滚动层的 ID / origin 与原始原因保留，并附加所属 Dialog 和部件上下文。未迁移的原生 Bevy 失败各自通过实际 Command 边界交给宿主，保留 Severity / payload；失败部件继续保留待更新状态，恢复后重试。

### Windows 文件图标

自动服务开启时，文件行在等待期间显示内建文件 / 文件夹 SVG，占用约 16 逻辑像素；后台通过 Windows Shell 获取当前系统关联图标，转换为 32×32 straight RGBA，再由主线程注册到 `Assets<Image>` 并使用 `ImageNode` 显示。原生颜色和透明度不随行前景色重新着色。传统掩码图标与带 Alpha 的图标均通过黑白背景合成恢复透明度，不包含文件内容缩略图和云状态覆盖标记。

普通文件按 ASCII 大小写归一的扩展名共享系统关联图标，目录共享系统文件夹图标。EXE、ICO、LNK、URL、CPL、SCR 和符号链接文件按原始 `PathBuf` 区分，避免将某个文件的专属图标用于同类型其他文件；仅在 Shell 调用边界将 canonicalize 的扩展磁盘 / UNC 前缀转换为 Shell 路径表示，保持全部 UTF-16 单元及原始缓存键，不经过有损字符串往返。Shell 不支持的路径继续使用占位。不提供资源管理器的完整 Shell 扩展或预览能力。

图标缓存按 Dialog Session / 目录 Generation 管理，最多 512 项，区分 Loading、Ready 和 Unavailable。Search、Sort 等同目录重投影复用缓存；导航、刷新和重开使旧缓存及请求失效。容量不足时按进入顺序淘汰不再可见且不在加载中的项，图像强引用随缓存和可见行的生命周期释放。Unavailable 不因稳定帧或往返滚动反复重试；新目录 Generation 可以重新请求。

每个 Dialog 的当前目录 Generation 复用一条 `JobManager` 受管线程同步调用 Shell，最多一个在途批次，每批最多 16 个去重后的可见请求；请求与回复均使用容量 1 的有界通道。慢速滚动出现单行新文件时也复用该线程，不在 UI 线程取图。批次取图完成后返回像素，所有 Dialog 每次更新最多消费 16 个图标结果，预算耗尽后请求后续帧；Job 的 Finished 表示整条服务线程真实退出。COM 初始化、Icon、DIB 和 DC 均限制在工作线程内。异步结果只写所属缓存，再按当前文件行绑定投影，不按发起请求时的 Entity 回写。文件行复用立即恢复新条目的占位图标。

无法提取图标时使用占位，不影响目录、选择、确认或结果。真实原生转换 / Job 错误保留类型、源头和原因链，在提交降级结果后记录处理；服务取图失败、Manager 拒绝提交或取消图标工作后，该图标服务停止继续提交请求，Dialog 仍可取消或选择。UI 资源不变量失效通过真实 ECS 边界报告一次，并停止当前目录 Generation 的图标更新；刷新或导航后可重新建立服务。关闭、销毁、Generation 变更和 App Drop 会取消服务，并复用 Runtime 的后台终态收尾能力回收记录，不等待 OS 调用，也不依赖 UI 再次更新。

`RuntimeOptions.automatic = false` 同时关闭自动 Shell 图标请求，手工 Backend / Headless 场景保留原有 SVG 占位。HeadlessPlugin 不创建图像或执行取图。

### Session 与后台服务

独立窗口的默认标题使用 Window 的 `WidgetryWindowTitle` 绑定，可对 Dialog Root 调用该类型的 `set_in_world` / `set` 更新原生标题与显示文字；不重建文件列表或重置 Session。嵌入式 Dialog 本身不提供 Window 标题。

Embedded Resolved Root 可通过 Reopen 创建新 Session，并使用 Scope 或实例最近访问的目录；旧 Session Reply 不再适用。Owned Dialog 的 Root 同时承载窗口与结果身份，独立窗口禁止用户 Resize 和原生最大化，隐藏最小化与最大化按钮；传入的 Native Window 配置也不会启用 Resize 或最大化，宿主仍可程序化设置窗口尺寸。Cancel、Escape、Title Bar X 最多产生一次 Cancelled，IME 与局部 Panel 优先处理 Escape。结果 Observer 和 Deferred Consumer 完成后，先通知 WindowClosing，跨过 Render Extraction 并应用 Cursor 更新的 Deferred Commands 后再回收 Owned Window / Camera，再次打开须创建新 Root。最终回收显式排在 ExitSystems 与 winit 原生窗口清理之前，不依赖宿主启用默认 close_when_requested。

HeadlessPlugin 自动调度目录读取、校验、创建目录和位置查询，可通过 RuntimeOptions 关闭自动服务以接入自定义 Reply Adapter。Backend 使用 Started 与已准备的 Snapshot 提交目录事实，Validated 表示 Path、Parent 与 Kind 已校验，Exists 只决定 SaveFile 是否需要 Overwrite。Overwrite Decision 必须携带固定候选 Token；No 回到当前 Session，文件名修改使旧候选失效。

HeadlessPlugin 自动确保唯一 JobManagerPlugin，复用宿主预先配置的 JobManager。目录枚举及其全部批次快照属于一个受管线程 Job，同步 Backend 在该线程执行，避免阻塞共享 I/O 任务池和关闭协调器；批次准备交给快照生命周期 Job 的有界收件箱，Job 等到最后批次处理完成并真实退出后才结束。默认每批最多128条目，或积累非空条目约8ms后发送；所有业务和偏好 Item / Finished 共享默认8消息、约2ms的消费预算。Job 缓冲按结果项数产生背压，不覆盖未读快照。

目录流还共享有限的后台快照保留预算，并为其他业务结果的后台保留留出空间。消费暂停、预算不足时，目录保留一个未处理的有界批次，等待容量释放后再推进，不将合法背压报告为目录读取失败；取消和服务停止也能打断这项等待。宿主配置的 Job 结果缓冲容量继续生效。

目录的最终快照先保留到该订阅消费 Finished，之后才提交最终 Ready / Partial 投影；执行状态 Completed 不代表 UI 已排空结果。根、Session、Generation 与投影版本仍决定回复是否适用。导航、取消和会话回收请求旧目录 Job 协作取消，不等待阻塞的 OS 调用返回；已取消 Job 的记录在其真实退出后移除。大快照由独立受管的长期快照生命周期 Job 保留最后引用并在后台释放，包含会话销毁和 App Drop 路径。所有 Job 结果使用 JobManager 的合并 Reactive 唤醒，公开动作和显式 Flush 使用 Winit WakeUp。

会话取消先向 JobManager 登记请求，再发布旧服务的取消标记，避免正常协作取消被误判为业务失败。过渡服务发生故障时，Runtime 先取消所有目录 Job，再提交 Failed；满结果缓冲不会使这些 Job 永久等待消费。

每次缓存重投影是独立的 AsyncComputeTaskPool CPU Job，包含 Query 重投影与对应 Selection 修复，不重新读取目录。新 Query / Snapshot Version 取消过期计算；结果消费重新核实 Root、Open Session、完整 Token 与快照版本。Pending 期间保留上次快照，结果 Item 暂存到该订阅消费 Finished 后再提交，执行 Completed 不提前更新画面。投影与其他 Job 共享每帧预算及合并唤醒。大投影结果发布前由快照生命周期 Job 保留引用，容量不足时协作等待，避免将最终释放移到 UI 或 App Drop 线程。

仅选择意图变化时，在已提交的目录 / Query / Snapshot Version 上启动独立 CPU Selection Job，按顺序准备整份意图队列；选择版本不会触发冗余目录重投影。新目录快照需要重投影时，由该投影整体修复选择；每个选中 ID 或集合操作不会单独注册 Job。Selection Item 在消费 Finished 后提交，消费前核实 Open Session、Generation、Query / Selection Revision 与当前意图身份；仅 Filename 编辑版本变化不会取消仍有效的选择意图。新目标、Query、导航、取消和关闭使旧结果失效，并请求协作取消；已经正常完成的旧计算保持 Completed。大选择集合及其快照在发布前取得后台保留引用，Pending 仍阻止 Confirm，Cancel 不等待计算退出。

候选准备与同步 Backend 路径校验属于同一个受管线程 Job，完整等待校验返回后才结束，不占用共享 I/O 任务池。多选大 Paths 在执行校验前取得后台保留引用。消费结果重新核实 Open Session、完整固定 Token 和 Validating 阶段；Item 先暂存，到 Finished 后才提交 Validated / ValidationFailed，保留 Overwrite 和最多一次 Result 语义。候选名字等可处理的准备错误作为业务 ValidationFailed 返回；I/O 错误同时使 Job Failed，backend panic 仅使该 Job Failed，UI 保持可恢复的确认错误。取消不等待 OS 返回；校验前真正停止可确认取消，校验已完成后的迟到取消仅抑制结果发送，正常返回仍为 Completed。Filename / Selection / Query / Session 改变后的旧结果不会提交；SaveFile 仍只返回目标路径。

创建文件夹是一个独立的受管线程 Job，在专属线程调用同步 Backend，不阻塞 ECS，也不占用共享 I/O 池，避免慢 OS 调用拖住关闭协调器。每个固定 Folder Request 只登记一次；Item 与 Finished 均消费后，才按原 Session / Generation / 请求身份提交 FolderCreated 并在成功时刷新目录。关闭、导航和取消会请求旧 Job 停止，但不撤销已经发生的 OS 副作用；OS 调用后的迟到取消不确认 Canceled，成功仍为 Completed，错误及 panic 为 Failed。错误清除 Pending 并保留可恢复的原始诊断，手工 Headless Reply 仍可使用。

系统位置查询同样由独立受管线程执行同步 Backend，每次目录会话初始化只提交一次，结果最多 128 项。Locations Item 到 Finished 后才提交；Runtime 核实 Open Root、Session 与 Generation，导航、关闭和重开使旧 Job 失效并请求协作取消。Query / Filename 变化不改变仍适用的位置查询身份。空列表合法，查询失败、超容量与 panic 保留现有侧栏列表并更新独立 location_error，不改变主目录状态；迟到取消不改写已返回的成功或失败结果。自动服务关闭时继续支持手工 Reply。

快照生命周期 Job 使用一条 Manager 受管线程，承接目录有界批次准备和 Snapshot / Selection / Paths 的后台保留。目录保留额度为 192，总保留额度为 256，收件箱为 64 项，继续为其他结果预留空间并在容量不足时背压。取消后拒收新工作并排空待回复请求；外部 UI / App 仍持有大对象时保持 Stopping，最后引用真正后台释放后才确认取消并退出。Runtime 观察到快照生命周期停止或终结时，使用同目录身份的幂等 OperationsStopped 回复结束未提交的 Selection / Projection / Validation / Folder 意图；已提交的数据仍保留，目录和错误状态明确表示后台不可用，Dialog 保持 Open 以允许用户 Cancel，不产生伪造的选择结果。原三线程 Worker、I/O 请求槽和专有 Reply / Wake 路由已退役；Crossbeam 仅用于该生命周期 Job 的有界收件箱。

### 偏好与持久化

默认只保存在 App 内存，显式配置 Persistence Path 才读写磁盘。一次 Load 或 Save 是一个完整受管线程 Job，等待同步 Backend 返回，Item 到 Finished 后才合并加载结果或确认保存 Revision。单个 Dialog 关闭不会取消共享偏好操作，Runtime 销毁或全局 Shutdown 会请求取消；已经完成的写盘不因迟到取消改写成功事实，失败仍同时进入 Job 和公开 Persistence 状态。RuntimeStatus 区分后台错误、Memory Committed 和 Disk Committed，`flush_preferences` 请求异步保存。Path 使用 Windows UTF-16 无损编码，未知路径编码会拒绝加载偏好。

同 Scope 偏好按字段 Delta 合并，Cancel 不改变 Last Picked Directory，晚到的偏好只影响后续 Session。Memory Snapshot 支持 Import / Export，Saved Revision Acknowledgement 不将更新的 Revision 误标为已保存。New Folder 只提交明确请求；关闭 Session 只拒绝后续旧 Reply，不保证撤销已经执行的 OS 副作用。

## 边界情况与错误处理

状态构造、Selection 准备和 Storage API 返回 `FileDialogFailure = WidgetryFailure<FileDialogError>`。领域类别区分 scope 字节长度、容量、重复 FilterId、Filter 配置、Session / Revision 耗尽、状态和存储契约，保留实际参数、Token、模式、EntryId 与 Revision。`apply`、`deliver`、`flush_preferences` 返回 `FileDialogOperationFailure`，分别承接领域诊断和原有 Job 失败；Job 不改写为契约拒绝，原生唤醒失败单独保留真实 source。

Snapshot 构造、追加及缓存重投影也返回 `FileDialogFailure`，分类保留目录、Token、条目路径 / 名字、容量和代际冲突字段。目录 I/O 与 CPU 批次通过 JobFailure 传递原诊断，不以摘要字符串替代失败对象；原生文件系统错误保留实际操作阶段、已知条目路径 / 序号、ErrorKind 和原始 OS source。文件系统 Trait 继续返回 I/O 结果，由目录 Owner 结合实际取消状态分类后建立诊断。

单条目录条目失败按既定行为跳过并继续，每条都记录源头 ERROR 与实际跳过 DEBUG；Partial 的可读摘要仍最多 8 条。没有条目路径时只记录目录与序号。正常取消、CPU Busy 和等待不创建错误；已形成的 CPU 失败及真实 I/O 失败不因迟到取消被改写，明确取消后的原始 Interrupted 则按协作取消处理。Runtime 在提交失败状态或明确放弃旧请求后记录处理。

候选准备与单次文件操作使用 `FileDialogRequestError`：`Rejected` 表示非法文件名 / 后缀等可恢复业务提示，`Canceled` 表示正常停止，`Failed(JobFailure)` 保存真实失败。`ValidationJob::prepare` 及 ValidationFailed、FolderCreated、Locations 回执均使用此分类；手工 Adapter 传入真实失败时保留原诊断对象。内部候选选择契约失效不是用户输入提示，返回包含 Token / 模式 / EntryId 的领域失败。

原生候选验证、创建目录与位置查询失败保留实际操作阶段、已知路径及原始 I/O source；多选校验还记录失败项在候选中的索引，不复制整个目录快照。SavePath 目标尚不存在仍是合法判断。真实后台失败在 Item 与 Finished 之间保持同一 origin，迟到取消不改写已返回的失败或已完成的副作用。

自定义 Backend 的单路径失败保存实际候选路径，多路径失败共享原候选 Paths；Backend 没有指出失败项时不编造路径或索引。诊断格式化只输出多路径候选数量，完整集合由错误对象保留。

Backend 递送已有 JobFailure / SendFailure 时，`RequestManaged` 传播包装保留类型化请求上下文、共享原 origin 与 source；不重新生成诊断 ID 或捕获栈。

请求上下文的 `path` 保留单个候选或请求路径，`resource` 单独保存原生失败涉及的实际资源。例如 SaveFile 的父目录检查失败，同时保留完整候选文件名与失败父目录；多选保留共享 `paths`，并由 `resource` / `index` 指出已知失败项。

偏好 Loaded / Saved 回执及 Runtime 最后消费层保留同一 JobFailure，显示字符串只在提交状态时生成。`FileDialogPreferenceContext` 保存目标文件、已知失败资源、读写 / 编解码 / 原子替换阶段、Scope 和完整的已提交 Revision 集合；原生 JSON 错误保留原始 `serde_json::Error`，格式、容量及存储路径故障保留 `FileDialogPreferenceFault`。自定义 Backend 的已有 JobFailure / SendFailure 只附加类型化偏好上下文，保留原 origin / source。

首次偏好文件缺失仍按空加载处理。加载失败后保留当前内存偏好并抑制当前 Revision 的自动保存；保存失败保留内存值与未确认 Revision，不回滚已经完成的磁盘写入。实际提交这些结局后才记录处理 DEBUG；I/O 与回执递送都失败时分别保留两份诊断。

偏好异常状态复用统一 FailureState 保存最近一次真实失败组；成功采用加载结果或完成保存确认后单独记录恢复 INFO。Loading / Saving、正常取消与资源销毁不算恢复，正常自动保存不增加诊断日志；独立的新 I/O 失败仍各自产生源头诊断。

公开递送在失败状态实际提交后记录处理 DEBUG；旧请求或目标销毁时明确记录放弃，保持当前状态。递送或状态提交本身失败时保留原请求诊断与新的失败并继续传播，不记录已处理。位置回执匹配 Session / Generation；同代 Query 改变仍可接受位置结果，导航后的旧位置失败不能覆盖新目录。

Runtime 的 Job 访问与接收失败保留原 Job 诊断，补充请求 Owner、Token、操作和 Job 身份；正常 Empty 不创建诊断或日志。Retention 启动失败由 Runtime 保留，在所有受影响会话完成安全降级后才记录处理 DEBUG；降级递送失败时，同时向宿主传播原启动错误和全部递送错误。服务运行故障同样在停止相关任务并提交可恢复失败状态后处理，清理仍失败则保留整组诊断。临时移出 World 的 Runtime 在返回错误前仍会放回。

多个会话及 Job 的停止、过期回收与窗口同步各自继续处理其他独立项，保留已经产生的全部失败；未完成的清理仍由 Runtime 持有。界面 Scene、部件配色和输入错误保留原下层诊断并补充 Root / Token / 部件上下文；外来 Bevy 错误沿真实执行边界保留 Severity 和 payload。内部缺失、无效样式和窗口构造前置条件使用 `FileDialogInternalFault`，不再通过字符串契约错误重建源头。

颜色查询、同步更新与清空返回 `FileDialogFailure`。非法目标保留 Entity / 已知样式 Owner；颜色校验通过 `CoreColor` 保留原 Core 原因及 origin，失败不写入配置。排队操作与 Template 在执行边界显式转换为 Error Severity。

普通 API 不赋予 Bevy Severity。Template、排队请求及 System / Observer 边界显式使用 Error Severity，保留原 ID / origin / source；组合结果的 Bevy `From` 转换也使用该受管边界，不落入原生 Panic 默认转换。构造 Template 失败沿 Core Scene 清理本次新建 Root，不回滚既有 Entity 的全部 Patch。

合法同值、已关闭会话的 Cancel / Confirm、过期正常回执及已销毁目标的递送继续返回无变化，不记录错误。非法用户输入导致的候选业务提示仍由确认状态表达。偏好合并失败保留既有已提交偏好，并在实际提交 Failed 状态和保存抑制结果后按原 error_id 记录处理 DEBUG；Failed 文本只是显示投影。

- 过期 Session、EntryId、Generation、Validation Candidate 或失效 Root 不得修改当前 Session。
- 文件系统的 Partial / Failed 状态、超时及单项错误通过状态与诊断呈现；已到达条目可先显示，完成后统一排序。
- 扩展长度 Windows Path 在实际操作中保留原始 `PathBuf`；地址显示可以隐藏前缀，显示文本不能反向构造文件选择路径。
- Modal、IME、隐藏/禁用控件的输入与 Escape 处理按当前输入归属执行；旧 Press 不能在滚动复用的新 Row 上提交选择。
- Owned Window 结束后清理其窗口和 Camera；外部销毁不冒充用户 Cancelled。Storage 写入失败不回滚已成立的结果。

## 已知限制

- 一个 Snapshot 最多 100,000 个 Entry；单次 Selection 最多排队 128 个 Intent；单个 Viewport 最多构造 256 行，Overscan 上限 8。
- 默认偏好只驻留 App 内存，除非宿主明确配置 Persistence Path。
- 后台任务取消不保证撤销已发生的 OS 文件系统副作用；退出 App 不无限等待阻塞 I/O。
- 同步读取依赖后台服务；Headless Contract 本身不直接创建 Native Window 或执行同步文件系统访问。
- RuntimeStatus.retention_jobs 表示非终态快照生命周期 Job 数（0或1），queued_retention 只统计其有界收件箱中等待的批次/保留请求，retained_snapshots 统计保留的大对象数；这些指标不冒充全部 Job 数、OS 线程数或独立广播缓冲长度。

后台工作返回共享 JobFailure，保留原始IO/Bevy对象和受管错误身份。业务失败与结果递送失败一同保存在任务报告中，Preference内部回执保留同一报告；Runtime交付失败状态或放弃过期结果后才记录处理。后台服务失败进入宿主时继续携带原诊断，不以运行状态中的展示字符串替代传播。

业务Control停止或服务进入shutdown时，尚未观察到自身Job取消的工作也按正常停止结束；在途结果通道随关闭退出不产生新诊断。没有停止请求时的真实通道故障仍作为失败传播。

Runtime 或 App 销毁时，将偏好加载/保存、活动会话、已过期任务及快照生命周期服务的 Job 身份移交给已有 Io 池的终态 Owner。先请求全部 Job 取消，再设置业务停止标志并销毁接收器；后台继续驱动真实退出、明确放弃迟到的失败结果并移除记录。收尾不依赖 UI 再次更新，也不向已进入关闭阶段的 Manager 提交新 Job，不承诺强制中断 OS 调用。
