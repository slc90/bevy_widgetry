# 分离 FileDialog 低频配置与持续 UI 同步

## 目标与范围

整理 `crates/file_dialog/src/style.rs::reconcile_root` 与 `view.rs`、`controls.rs` 的调用边界，把首次构造、Style 配置及新节点初始化从必要的滚动、视图和输入同步中分开。

## 现状与修改设计

当前每帧克隆 State／Shell，重复插入 Shell，扫描后代 TextFont，同步间距、Sidebar 宽度和控件状态。这些工作并不都随滚动或输入每帧变化。

字体、间距、Padding、Sidebar 宽度等配置只在初次构造、对应 Style 真实变化或新节点出现时应用。对动态控制和复用后的行补齐当前配置，不能只监听 Root 的 Changed<Style> 就漏掉后续创建的节点。Header 宽度与实际 Viewport 几何仍按布局需要同步。

地址栏只有实际显示路径改变时才更新，不能因为抽出低频路径而反复重设 EditableText。Shell 派生信息只在真实变化时写回，避免同值插入制造新的 Changed。State 的读取／克隆按实际需要缩小，但不把所有状态压缩成一个会漏掉异步回复的脏标志。

滚动、可见 Row 更新、输入队列、确认是否可用、Tab stop、Selection／Active 以及当前 Session 的后台回复仍按必要节奏推进。将不同职责明确分开，不引入第二个 Session Authority，也不重新构造整个 Shell 作为快捷修复。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

测量使用相同真实负载和既有 Benchmark 规范；内部工作量计数不成为公共 API。涉及可见行为时增加对应 Gallery 的 BRP 输入、ECS 状态与截图验证，原生 Window／Mouse 行为仍需 Windows 原生证据。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖静止、持续滚动、Style 更新、动态文件行和控制生成、目录／搜索／Filter／Sort 变化、读取中取消、Path／Filename 编辑、Selection 修复和异步结果。Caret、Selection、正在编辑的内容、现有 Session、可见行复用和 Tab 顺序不被重置。

非法 Style 尺寸仍按既有契约拒绝，不让无效配置部分写入 UI。通过定向测试确认静止时不再扫描全部后代字体或同值写回 Shell；用现有 FileDialog Benchmark 对比打开、稳定与滚动路径，并沿正式显示验收模式区分通过、失败和证据不足。

## 预期产出

低频外观配置与必要持续状态推进边界清楚的 FileDialog UI 协调逻辑。

## 与前后方案的关系

顺序上接 [方案 36：让 MessageBox 的组合配色按变化更新](36-让-MessageBox-的组合配色按变化更新.md)。

完成后进入 [方案 38：让 FileDialog 样式归属响应部件生命周期](38-让-FileDialog-样式归属响应部件生命周期.md)。

先分离 FileDialog 的配置与状态同步，为后续 Owner 和颜色更新提供清楚的初始化与变化边界。
