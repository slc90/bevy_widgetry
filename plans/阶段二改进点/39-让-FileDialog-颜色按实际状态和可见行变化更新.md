# 让 FileDialog 颜色按实际状态和可见行变化更新

## 目标与范围

调整 `crates/file_dialog/src/style.rs::update_colors` 及 controls、window、view 的颜色调用边界，只刷新实际受影响的 Dialog 部件与可见文件行。

## 修改设计

根级 Theme／覆盖色变化可以刷新该 Dialog 的受管范围；局部 Hover／Pressed、Focus、有效 Disabled、Selection／Active 以及控制状态变化应尽量定位到相关部件。Row 的 Session、EntryId、Generation 或绑定变化即使复用了同一 Entity，也必须触发正确的初始样式。

使用上一项建立的 Owner 链，把内部 Button、TextField、ComboBox、CheckBox、ScrollArea、Window 和覆盖确认 MessageBox 的交互变化传到最终负责者。不能只监听 FileDialog Root，漏掉内部交互；也不能让多个层级同时按不同主题覆盖同一输出。

异步 Snapshot、Partial／Ready 状态、空列表、搜索过滤和可见行变化都要正确反映当前业务状态。颜色缓存只保存派生值，不把选择、活动条目或业务文件身份移到物理 Row 上。保留当前 Renderer、布局和输入行为，不借配色重构后台服务。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖新建／复用行、滚动、Ctrl／Shift 选择、Active 导航、禁用／重新启用、主题和覆盖色清除、编辑器 Focus、目录异步刷新，以及覆盖确认的打开、交互和关闭。旧 Press 不能通过复用行提交新文件选择，旧 Session 的变化不能使当前 Dialog 重新配色。

静止 Dialog 不再每帧计算全部受管控件与可见行颜色；局部输入只产生必要工作。保持相同数据规模和工作负载进行 FileDialog Benchmark 比较，按正式显示证据判定 150 ms 目标；Headless 工作量正确不等于实际显示预算已通过。相关回归和 Coverage 随本项同步。

## 预期产出

不破坏异步 Session、嵌套控件和文件行身份的变化驱动 FileDialog 颜色同步。

## 与前后方案的关系

顺序上接 [方案 38：让 FileDialog 样式归属响应部件生命周期](38-让-FileDialog-样式归属响应部件生命周期.md)。

完成后进入 [方案 40：规范 Gallery 的结果路径显示](40-规范-Gallery-的结果路径显示.md)。

依赖 FileDialog 的配置／Owner 调整，沿用已经整理的下层控件受管配色接口。
