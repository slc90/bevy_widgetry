# waveform Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Source 未填满 / 稳定 / 滚动、Cursor Boundary、Reduction Density。
- Source 成功 / 失败、渲染可见 / 隐藏 / 销毁、Polyline / Envelope。
- 算法 MinMax / LTTB、目标算法与已提交显示、后台重建在途 / 过期 / 就绪。

## Stimuli

- Cursor 连续前进、跳转、倒退；Source Partial / Failed Read。
- BSN Scene、实际 Layout Width / Resize、Style / Theme、Despawn。

## Guards

- Config 的 Sample Rate、Duration、统一对称幅值上限有效。
- Source 返回请求范围的完整有限数据；零尺寸不创建有效可见几何。

## Transitions

- 初始填充 → 增量滚动；Density 改变重建 Reduced Output。
- 失败保留先前已提交 Display；相同 Boundary / Density 为 NoOp。
- 相同 Cursor / Density 在读取失败后重试并恢复，成功后的稳定帧清空读取和 Reduction Stats；密度改变经过零宽再恢复时重建输出。
- Render Scene 从准备转为可见 / 隐藏并最终释放资源。

## Invariants

- Channel Data、Viewport 与 Reduced Output 同步提交；失败不留下半完成画面。
- `algorithm.rs` 负责算法 API 的同步/排队、同值通知、Live/Replay、多实例隔离、暂停刷新与物理单位；Pending 请求身份随算法失效，旧画面保留到新结果完整就绪。大窗口后台重建不能提前提交原始缓存，旧算法或旧密度任务不能覆盖新目标。
- 暂停 Replay 的后台算法重建通过真实 App 调度验证：每个在途帧发出 RequestRedraw，直到完整折线快照提交，不依赖外部采集时钟或输入。
- Ring / Reduction 工作量与容量有界。
- Mesh、Camera 和 Renderer Layer 的资源归属隔离。
- Theme / Palette Override 不破坏已有几何。
- NoOp 不读取 Source、不执行 Reduction；增量原始 Sample 扫描只随新增 Sample 与边缘 Bucket 增长。LTTB 还按当前可见桶数进行代表点查询，凸包内使用二分，避免凸曲线候选数退化成每帧全窗口重扫。
- Renderer 保持单 Viewport / Camera、独立波形 Mesh 和主网格 Mesh，失败保留已提交波形 Mesh，Root 同时管理 Entity 和 Asset；Disabled / Override / Theme 的 Palette 与 Background 解析后，Clear 恢复当前状态 Theme。

## Couplings

- Source Read × Cursor / Transaction。
- Output Density × Physical Layout Width。
- Renderer Lifecycle × Asset 释放和 View Isolations。

## Coverage Map

- `reduced_source.rs`：预降采样源不分配 Raw Ring，完整结果、单位转换与窗口成对提交，Pending/过期身份/非法数值/读取失败保留旧画面，稳定目标 NoOp，Live 错误恢复。

- `headless.rs`：Source / Cursor / Ring / Reducer 的更新、失败与增量数据 Contract。
- `renderer.rs`：BSN、Layout、Mesh、资源生命周期、Style / Theme 和多 View 隔离。
- `session.rs`：同根实体 Live 32 → Replay 64 → Replay 8 → Live 切换、元信息验证、显示配置保留、V 单位换算、归零与暂停、停止保留画面、清空会话、排队更新及提交后通知、实例错误查询。
- 会话替换使所有旧 Raw / Reduced 表示失效；Renderer 的 Camera、Mesh、Image 和 Entity 身份保持稳定，下一帧清除旧几何（`renderer.rs`）。


- `gaps.rs`：逐通道显式缺口、真实零值、MinMax 有效段分离、跨桶边界/环回/缺口淘汰的增量与完整重建一致性、非法缺失区间拒绝。
- `requests.rs`：Pending 与过期 Ready 保留一致快照，Live 追到最新窗口；同目标 request 身份稳定，密度/会话变更使旧身份失效，共享 Source 的读取者互相隔离；Replay Pending 继续、真错误停止且不自动重试。
- 几何 Unit Test 覆盖 Polyline 显式断点、自定义 Reducer 稀疏代表点仍连线、独立有效样本可见及全缺失时透明空 Mesh。
- `duration.rs`：8s/30s 左起填充、35s/10s 累计时间滚动、同一时刻扩大/缩短窗口、缺口保留、候选/无效实体校验、提交后通知、Pending / 历史不足保留快照、暂停重算与新会话归零、空暂停 Replay 不读取；缓存随当前窗口缩小。
- 控制栏 Unit Test 使用真实 ComboBox / Layout，覆盖初始化选中、API 投影、选项更新同一状态、稳定 Model / Combo 实体及随根清理。
- 取消大窗口 Pending 后缩到小窗口，不将旧暂存容量转为小窗口的长期缓存（`duration.rs`）。
- `axis.rs`：主网格与累计秒数标尺消费同一已提交窗口、滚动连续移动、Pending 时保持旧窗口、网格公开 API 同值不通知及先提交后通知、开关不读取源且保留波形与时间、窄宽度和 UI 缩放下实际文字不重叠、根销毁清理网格和标尺。时间刻度 Unit Test 覆盖零点锚定、1–2–5 密度、次刻度最小距离、小数秒及长累计时间。
- 文字保护消费当前 NoWrap 字体测量后的 ComputedNode，不读取后续 PostLayout 才刷新的旧 TextLayoutInfo：Unit Test 覆盖真实 MenuPlugin / TextInputPlugin 的调度组合，`axis.rs` 验证累计时间位数、字号及缩放同时变化当帧的文字不越界或重叠。
- `amplitude.rs`：幅值 API 先提交后通知、同值不通知、排队入口、无会话可设、非法输入/失效实体保持状态；真实 Renderer 下 V/mV/µV 等价、暂停重投影不读取或降采样、跨会话保留。控制栏 Unit Test 覆盖幅值与时间选择器共存、候选校验、用户选择/API 投影同一状态和实体清理；几何 Unit Test 覆盖行高、±3A 越道和整体裁剪的真实重入交点。
- `channels.rs`：6/16 及 64通道 × 600px 的8/16/32/全部真实布局、24px最小行高、标签精确文本与间距；局部行滚动、密度/窗口高度变化的顶部锚点、真实波形Mesh与标签中心一致；独立多视图滚动、末尾钳制、换会话标签/密度保留、空会话与极窄根尺寸；垂直操作不增加源读取、不改变降采样 revision/时间窗口。Unit Test 覆盖每屏数量提交后通知、非法配置和三个 ComboBox 共存及用户选择。既有 Renderer / Axis 的负载按实际绘图区宽度保持。

- `measurement.rs`：Mouse / Custom Pointer 的真实 Picking 绘制、持续选中、内部平移、边角编辑、单像素窄框、跨通道换算、唯一框替换；Live / Replay 播放和暂停、时间/幅值/密度/尺寸重算、Pending 使用已提交时间尺度，不因手势读源或暂停；关闭/重开、换会话清框保留开关，取消/焦点/隐藏/Disabled/销毁释放捕获及提交后通知；测量开启时真实 Picking 可展开并选择控制栏下拉项，弹出层不受工具栏裁剪且不触发绘制。Gesture Unit Test 覆盖四边四角命中和编辑、反向绘制及平移边界尺寸保持。为保持已有绘图区负载，未带选择器的旧 fixture 总高度增加44px容纳新工具栏。
