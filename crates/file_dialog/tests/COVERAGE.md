# file_dialog Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Session / Generation、Directory / History、Selection / Active、Search / Filter / Sort。
- Confirmation / Overwrite、Storage Scope、Runtime Loading / Ready / Partial / Failed。
- Embedded / Owned、Modal / NonModal、Viewport / Editor / Theme。

## Stimuli

- BSN Spawn、`World` / `Commands` API、后台 Snapshot / Validation Reply。
- 目录导航、批量 Selection、Keyboard / Pointer / IME、FileDialog Result、Cancel。
- Streaming I/O、Prefs Load / Save、Window Closing、Theme 与 Color Override。

## Guards

- 有效 Root / Session / Token / Revision、Mode / Entry Kind。
- Confirm 不能穿透 Pending Selection / Projection / Validation。
- Disabled、Focus、IME Composition 和旧 Row Press 不得触发错误提交。
- 非空预编辑与待处理 IME Commit 保留输入保护；空预编辑清理不能阻断 Escape 取消。
- Snapshot / Queue / Worker 容量边界。

## Transitions

- Started / Partial / Ready / Failed 的目录请求生命周期。
- Navigate / Search / Select / Save / Overwrite / Cancel / Reopen。
- Scope Preferences Merge 与迟到 Reply 丢弃。
- Owned Window 在结果 Consumer 后结束，外部销毁不合成 Cancel。

## Invariants

- Session Result Once-only；通知读取已提交 Authority。
- Stable EntryId / PathBuf 原始身份不由 Lossy UI 文本重造。
- Disabled / Editor Cursor、可见 Row 和 UI Style 在 Theme / Scroll / Reuse 后一致。
- 后台 I/O 不阻塞 App Update；已完成业务结果不因 Storage 失败回滚。
- 新一轮 Session 不接受旧 Reply。
- 目录 I/O 在受管线程执行，过渡服务仍为3 Worker；I/O 不在 App Thread 执行且不会占满共享 I/O 池，队列 / Snapshot 容量及 Cancel 不等待 I/O 分别验证。
- 根颜色 Owner 控制部件、动态 Row、Owned Window 和 Overwrite MessageBox；托管部件拒绝直接覆盖，Theme / Clear 不改确认状态。
- PickFile Draft 与 Selection 分离，Enter / Open 解析当前可见 EntryId 后异步校验；非法名字、目录、Query Repair 和陈旧文件不能提交旧 Selection。
- Owned 结果先由 Observer / Deferred Consumer 消费，再发送 WindowClosing，允许 Render Extraction 释放 Surface，应用 CursorSystems::Update 的 Deferred Commands 后才销毁 Native Window；宿主禁用默认关闭系统时，同一轮 Last 的原生后端仍能消费到 Window 移除，不依赖后端的注册先后顺序。

## Couplings

- Selection × Snapshot / Query / Entry Kind。
- Filename Draft × Overwrite Candidate × Backend Validation。
- Scope Prefs × 多 Session；Modal × Window Focus / IME。
- Virtualization × 动态 Theme / Row Press Identity。

## Coverage Map

- `contract.rs`：Headless API、Session / Scope、Navigation / Selection、确认与通知。
- `runtime.rs`：过渡 Worker、JobManager 安装与宿主实例复用、手工模式、固定容量、流式更新、取消、存储与 Session 释放；目录读取失败、100,000条目边界与每帧消息预算。
- `style.rs`：真实 BSN / Input / Layout、编辑器、可见 Row、Theme 与 Color Owner，以及排队空 IME 清理后的真实键盘取消。
- `window.rs`：Owned Window、Modal / NonModal、Focus、IME、Overwrite MessageBox 和资源回收，包含嵌套 MessageBox 关闭后焦点恢复与 Escape 取消。
- `window.rs` 同时保护独立 Dialog 的默认标题绑定、原生 / 可见标题同步和更新时内容身份保持。
- `window.rs` 的覆盖确认配色回归验证最终 FileDialog Owner 的 Hover 覆盖色、Hover 移除及 Theme / Override 清除，不由中间 MessageBox 自行覆盖。
- `style.rs` 的配置回归保护 Style 更新时保留编辑内容及 Session；源码 Unit Test 保护稳定帧无字体遍历 / Shell 重写，以及延后字体与容器重挂后的配置补偿。
- TextField 复合结构的断言分别检查根 Part、内部编辑状态及 Frame / Editor 配色；既有 Tab → Filter/Sort、IME、Enter 与文本投影回归保护输入实体迁移后的真实业务路由。

- 源码 Owner 生命周期测试保护稳定帧零归属处理、延后 Control、Owner 丢失 / 替换修复、跨 Root 重绑、Marker 移除与 Root 恢复；既有 Style / Window 回归保护嵌套控件、复用行与覆盖确认。

- 源码配色工作量回归保护静止零配色、局部 Hover / 移除只更新单个 Control、Selection 只更新受影响行及同 Entity 新绑定初始化；现有 Style / Window 回归继续保护 Focus、Disabled、滚动、主题与覆盖确认的配色。

## 覆盖边界

目录源码测试保护128条目/8ms阈值、真实目录流式快照、容量1背压、无订阅者、取消确认、阻塞backend不占满共享I/O池、CPU末批应答先于Finished，以及执行Completed后必须等消费Finished才发布最终快照；旧服务耗尽共享预算后仍为目录积压续帧。旧Runtime场景继续保护双Session、快速导航、搜索/排序、Partial、旧Token过滤、关闭/重开和大型Snapshot后台持有。

共享保留容量测试保护八条32项缓冲目录流在消费暂停时保持Running，恢复消费后各自按序完成，以及容量等待中的取消退出。CPU预算单元测试保护压力下返回未处理批次、释放后重试和为八个Session旧投影保留空间；退出测试同时保护最后清空/Receiver析构窗口、Sender存活时释放排队应答，以及ECS admission不等待入队锁。

Runtime 源码回归在取消通知的同步唤醒点检查 JobManager 请求先于旧 Control 旗标发布；服务 fault 回归固定结果缓冲已满的交错，保护 Runtime 仍存活时取消目录 Job、确认真实退出并回收记录。

缓存重投影回归保护 CPU Job 身份、Manager 关闭后拒收、执行 Completed 与订阅 Finished 消费分离、新 Query / 快照版本 / Session 拒收旧结果及后台保留引用；Runtime Integration Test 保护 Pending 保留上次 Snapshot、同 Query no-op、EntryId 稳定及无需再次读取目录。既有 Filter / Sort / Selection、窗口关闭与手工 Headless 路径继续覆盖迁移后的行为。

真实 OS 文件系统 / Native Window 完整行为仍需相应环境验证；测试用受控 Backend 和 ECS Runtime 验证已定义 Contract。
