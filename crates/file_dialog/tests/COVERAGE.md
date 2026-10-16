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
- 后台固定 3 Worker，I/O 不在 App Thread 执行；队列 / Snapshot 容量及 Cancel 不等待 I/O 分别验证。
- 根颜色 Owner 控制部件、动态 Row、Owned Window 和 Overwrite MessageBox；托管部件拒绝直接覆盖，Theme / Clear 不改确认状态。
- PickFile Draft 与 Selection 分离，Enter / Open 解析当前可见 EntryId 后异步校验；非法名字、目录、Query Repair 和陈旧文件不能提交旧 Selection。
- Owned 结果先由 Observer / Deferred Consumer 消费，再发送 WindowClosing，允许 Render Extraction 释放 Surface，应用 CursorSystems::Update 的 Deferred Commands 后才销毁 Native Window。

## Couplings

- Selection × Snapshot / Query / Entry Kind。
- Filename Draft × Overwrite Candidate × Backend Validation。
- Scope Prefs × 多 Session；Modal × Window Focus / IME。
- Virtualization × 动态 Theme / Row Press Identity。

## Coverage Map

- `contract.rs`：Headless API、Session / Scope、Navigation / Selection、确认与通知。
- `runtime.rs`：后台 Worker、固定容量、流式更新、取消、存储与 Session 释放。
- `style.rs`：真实 BSN / Input / Layout、编辑器、可见 Row、Theme 与 Color Owner，以及排队空 IME 清理后的真实键盘取消。
- `window.rs`：Owned Window、Modal / NonModal、Focus、IME、Overwrite MessageBox 和资源回收，包含嵌套 MessageBox 关闭后焦点恢复与 Escape 取消。
- `window.rs` 同时保护独立 Dialog 的默认标题绑定、原生 / 可见标题同步和更新时内容身份保持。
- `window.rs` 的覆盖确认配色回归验证最终 FileDialog Owner 的 Hover 覆盖色、Hover 移除及 Theme / Override 清除，不由中间 MessageBox 自行覆盖。
- `style.rs` 的配置回归保护 Style 更新时保留编辑内容及 Session；源码 Unit Test 保护稳定帧无字体遍历 / Shell 重写，以及延后字体与容器重挂后的配置补偿。

- 源码 Owner 生命周期测试保护稳定帧零归属处理、延后 Control、Owner 丢失 / 替换修复、跨 Root 重绑、Marker 移除与 Root 恢复；既有 Style / Window 回归保护嵌套控件、复用行与覆盖确认。

- 源码配色工作量回归保护静止零配色、局部 Hover / 移除只更新单个 Control、Selection 只更新受影响行及同 Entity 新绑定初始化；现有 Style / Window 回归继续保护 Focus、Disabled、滚动、主题与覆盖确认的配色。

## 覆盖边界

真实 OS 文件系统 / Native Window 完整行为仍需相应环境验证；测试用受控 Backend 和 ECS Runtime 验证已定义 Contract。
