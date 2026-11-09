# waveform 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Source 未填满／稳定／滚动，Cursor Boundary、显示 Duration／Density 与各通道幅值范围。
- Live／Replay／Playback、Session、MinMax／LTTB 目标算法与已提交 Reduced Output。
- 后台 Reducer Pending／Ready／Stale、Runtime Job／独占槽、Renderer／Mesh／Time Axis／Measurement 状态。

## Stimuli
- Source 采样到达、Cursor 跳转／回退、窗口时间范围或物理宽度改变、同步／排队算法／会话 API。
- Playback 控制、通道／幅值／网格设置、真实 Pointer 测量、Theme／Override、Renderer 生命周期与异步计算完成。

## Guards
- Sample Rate、Duration、幅值上限、Source 范围及数据有限性必须合法；零尺寸不创建错误几何。
- 过期 Session／算法／密度结果或失效请求不得覆盖当前已提交波形；Manager 关闭与失败保持旧画面。

## Transitions
- 初始填充 → 增量滚动；改变 Duration／Density／算法／Session 后重建输出，Pending → 提交或过期废弃。
- Live ↔ Replay，播放／暂停／定位／停止／重播，波形 Renderer 可见／隐藏／销毁。
- 通道布局、时间轴、幅值缩放、测量框与鼠标交互的更新。

## Invariants
- Channel Data、Cursor、时间窗口和 Reduced Output 必须作为一致快照提交；源读取或后台重建失败不留下半完成画面。
- 相同目标处于稳定状态时为 No-op，不额外读取或重算；增量扫描与 Ring／Reduction 的容量和工作量有界。
- MinMax／LTTB 的有效值、缺口、单元转换及物理密度投影一致；超大窗口由后台 Job 重建，不阻塞主线程。
- 后台计算的旧结果、取消后迟到的 Outcome 或未交还的 Reducer 不能修改当前 Session；任务与独占资源完成交接后才更新 UI。
- Replay 暂停时由实际在途工作驱动 RequestRedraw，最终完整画面提交后停止无谓唤醒；失败／重试不丢失已提交快照。
- 多实例分别持有 Viewport／Camera／Mesh／离屏 Layer，资源创建／销毁有界；共享层号池回收不影响其他 View。
- Playback、幅值、Duration、网格、通道布局和测量状态按公开 Authority 先提交再通知；禁用、Theme／Override 不重置业务时序。
- Renderer 的几何、时间轴、通道标签与当前投影同帧一致；隐藏／零视口的故障恢复遵循真实就绪条件。
- Runtime／Source／Style 错误保留请求目标、单位、原始 I/O source 和共享 origin，合法稳定帧不制造新的故障身份。

## Couplings
- Source Read × Cursor／Transaction；Duration／Density／算法 × 物理 Layout／Reducer。
- Session／Playback × Pending 请求；Channel／Measurement × Renderer；多实例 × 资源归属／Layer 回收。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `algorithm.rs` | MinMax／LTTB 切换、后台重建、算法 API 和旧结果隔离。 |
| `amplitude.rs` | 幅值单位／范围、重投影、公开 API 与通知。 |
| `axis.rs` | 时间轴、网格、布局、窗口同步和可见性。 |
| `channels.rs` | 多通道布局、标签、滚动、密度与资源投影。 |
| `diagnostics.rs` | Source／Runtime／Model／Render 失败的诊断身份与宿主传播。 |
| `duration.rs` | 时间窗口长度、累计时间、Replay／Live 和更新原子性。 |
| `gaps.rs` | 逐通道数据缺口、MinMax／环形缓冲的一致性。 |
| `headless.rs` | 无渲染环境的采样、Cursor、Ring／Reducer 与错误恢复。 |
| `measurement.rs` | 真实 Pointer 测量、选区／拖动、时间幅值换算与生命周期。 |
| `playback.rs` | 回放开始／暂停／定位／停止、通知、时钟和结果同步。 |
| `reduced_source.rs` | 预降采样源、有限窗口、Pending 和旧快照保护。 |
| `renderer.rs` | BSN／Layout／Mesh、样式、离屏资源和多 View。 |
| `requests.rs` | 后台请求身份、过期结果、共享 Source 读者和重绘。 |
| `session.rs` | Live／Replay Session 切换、配置保留和公开状态。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/geometry.rs` | Renderer 几何边界与无效 Mesh 更新保护。 |
| `src/controls.rs` | 播放／幅值／通道控制状态和局部投影。 |
| `src/runtime.rs` | 受管 Reducer、Job Outcome、取消与独占槽的交接。 |
| `src/source.rs` | Source 读取、缓存与有效窗口的局部契约。 |
