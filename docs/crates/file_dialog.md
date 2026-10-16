# File Dialog

## 功能概述

提供可独立使用的文件与目录选择组件，支持嵌入式内容以及自有 Native Window。它把目录浏览、选择、确认、后台文件系统工作和可视 UI 组合在同一套业务契约下。

## 当前设计思路

业务 Authority、文件系统请求和 UI 投影分离。每次构造产生独立 Session，后台 Worker 提交准备好的 Snapshot / Validation Reply；UI 根据当前业务状态展示可见行和编辑器，不把物理 Row 当作文件身份。可选 Storage Scope 在 Session 间共享偏好。

## 主要功能

- 模式：PickFile、PickFiles、PickDirectory、PickDirectories、SaveFile。
- 导航与目录历史、搜索、Filter、Sort、隐藏项控制、常用位置与 Pin、新建文件夹。
- 按可见范围生成文件列表，可显示名称、类型、修改时间和可读大小；Selection 与 Active 分离。
- 单选、多选、Ctrl / Shift Range、Keyboard Navigation、Filename / Path Editor 和 IME 守卫。
- 可作为 Embedded 内容或独立 Owned Window，并支持 Modal / NonModal。
- SaveFile 的 Overwrite 确认复用 MessageBox。
- 后台目录流式读取、路径验证、创建目录、位置查询，及可选的偏好持久化和手工 Backend Reply 接入。
- 提供主题、颜色覆盖和 Style 配置，包括行高、Overscan、Sidebar 宽度与字体间距。

## 行为与状态契约

每个 Session 最多产生一次 `WidgetryFileDialogResultEvent`，通知发生在对应结果 Authority 提交后。`World` API 即时提交；`Commands` 在实际执行时提交；后台回复只有被当前 Session / Token 接受后才生效。程序更新和交互对应的 ChangeEvent 报告真实已提交业务变化，同值不通知。Selection / Active 是业务状态而非复用 Row State；确认操作必须等待必要的 Pending Selection / Projection / Validation。取消不等待后台 I/O。SaveFile 仅返回目标 PathBuf，不替应用写文件。

### 输入与候选身份

PickFile 与 SaveFile 提供 Filename Editor，多选与目录模式提供选择摘要，目录模式隐藏文件类型栏。PickFile 的临时文本只解析当前可见文件名，确认时使用真实 EntryId 并进入后台校验；公开 Filename Action 仅用于 SaveFile。地址栏隐藏常规 Windows 扩展路径前缀，未编辑的显示文本仍对应原始 PathBuf。详细行的修改时间使用 UTC。

方向键、Home / End、Page Navigation 改变 Active。多选模式 Space 切换 Selection，Ctrl+A 冻结当时已到达条目的范围；单选模式 Space 使用 Replace，SaveFile 或不匹配的 Kind 不执行 Selection，Enter 按模式 Activate。Selection Job 保留 Ctrl+A、Ctrl 点击和 Shift Range 的输入顺序，Filter / Search / Sort 可基于缓存重新投影而不改变原始路径与 Stable ID。Pending Selection / Projection 阻止 Confirm，Cancel 继续可用。

Path Enter 导航，Filename Enter 进入对应模式的确认校验；IME Composition 不触发业务提交或 Escape。待处理的非空预编辑与 IME Commit 仍受该守卫保护，空预编辑清理操作不视为正在合成，焦点恢复后可正常按 Escape 取消。隐藏 Panel 与 Disabled Control 不进入 Tab 顺序，单个输入失败不丢弃同帧后续输入。Style 更新不重置 Session、编辑器 Cursor 或 Selection。

字体、间距、Padding 和 Sidebar 宽度按首次构造及有效 Style 变化应用；动态生成或重挂的内容补齐当前配置。稳定 Dialog 不为外观配置递归全部字体，也不重复写回同值 Shell。地址栏仅在实际路径变化时同步，滚动、可见行、Header 几何、控件可用性、Tab 与后台回复继续按当前状态推进。无效 Style 在写入外观前拒绝，不部分应用配置。

样式归属由 Control、Part、文件行绑定和独立窗口 / 覆盖确认的构造与移除维护。归属更新直接使用部件保存的 Root；稳定帧不扫描全部部件确认 Owner。延后构造和复用绑定会进入有界补偿，实体回收后旧关系不能影响新 Dialog。

### Session 与后台服务

独立窗口的默认标题使用 Window 的 `WidgetryWindowTitle` 绑定，可对 Dialog Root 调用该类型的 `set_in_world` / `set` 更新原生标题与显示文字；不重建文件列表或重置 Session。嵌入式 Dialog 本身不提供 Window 标题。

Embedded Resolved Root 可通过 Reopen 创建新 Session，并使用 Scope 或实例最近访问的目录；旧 Session Reply 不再适用。Owned Dialog 的 Root 同时承载窗口与结果身份，独立窗口隐藏最大化按钮并保留 Resize 配置。Cancel、Escape、Title Bar X 最多产生一次 Cancelled，IME 与局部 Panel 优先处理 Escape。结果 Observer 和 Deferred Consumer 完成后，先通知 WindowClosing，跨过 Render Extraction 并应用 Cursor 更新的 Deferred Commands 后再回收 Owned Window / Camera，再次打开须创建新 Root。

HeadlessPlugin 自动调度目录读取、校验、创建目录和位置查询，可通过 RuntimeOptions 关闭自动服务以接入自定义 Reply Adapter。Backend 使用 Started 与已准备的 Snapshot 提交目录事实，Validated 表示 Path、Parent 与 Kind 已校验，Exists 只决定 SaveFile 是否需要 Overwrite。Overwrite Decision 必须携带固定候选 Token；No 回到当前 Session，文件名修改使旧候选失效。

### 偏好与持久化

默认只保存在 App 内存，显式配置 Persistence Path 才读写磁盘。RuntimeStatus 区分后台错误、Memory Committed 和 Disk Committed，`flush_preferences` 请求异步保存。Path 使用 Windows UTF-16 无损编码，未知路径编码会拒绝加载偏好。

同 Scope 偏好按字段 Delta 合并，Cancel 不改变 Last Picked Directory，晚到的偏好只影响后续 Session。Memory Snapshot 支持 Import / Export，Saved Revision Acknowledgement 不将更新的 Revision 误标为已保存。New Folder 只提交明确请求；关闭 Session 只拒绝后续旧 Reply，不保证撤销已经执行的 OS 副作用。

## 边界情况与错误处理

- 过期 Session、EntryId、Generation、Validation Candidate 或失效 Root 不得修改当前 Session。
- 文件系统的 Partial / Failed 状态、超时及单项错误通过状态与诊断呈现；已到达条目可先显示，完成后统一排序。
- 扩展长度 Windows Path 在实际操作中保留原始 `PathBuf`；地址显示可以隐藏前缀，显示文本不能反向构造文件选择路径。
- Modal、IME、隐藏/禁用控件的 Tab 与 Escape 处理按当前输入归属执行；旧 Press 不能在滚动复用的新 Row 上提交选择。
- Owned Window 结束后清理其窗口和 Camera；外部销毁不冒充用户 Cancelled。Storage 写入失败不回滚已成立的结果。

## 已知限制

- 一个 Snapshot 最多 100,000 个 Entry；单次 Selection 最多排队 128 个 Intent；单个 Viewport 最多构造 256 行，Overscan 上限 8。
- 默认偏好只驻留 App 内存，除非宿主明确配置 Persistence Path。
- 后台任务取消不保证撤销已发生的 OS 文件系统副作用；退出 App 不无限等待阻塞 I/O。
- 同步读取依赖后台服务；Headless Contract 本身不直接创建 Native Window 或执行同步文件系统访问。
