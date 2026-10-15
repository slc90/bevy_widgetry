# 让 RadioGroup 选项样式真正由变化驱动

## 目标与范围

调整 `crates/radio_group/src/option_style.rs::update_changed` 和相关失效传播，处理选项样式的重复计算。固定 Children 和对外 API 不变。

## 修改设计

当前 `update_changed` 的查询并没有变化过滤。根据现有样式实际使用的 Checked、Hover、Disabled、选项覆盖色、Group 覆盖色、主题和 Root Focus 等变化刷新相应对象。覆盖 Checked／Disabled 的移除、Focus 离开和父级颜色变化，不能只监听选项自身的 Added／Changed。

只保留当前定义的视觉优先级，不凭空增加未定义的 Pressed 配色或新的状态通知。Group 级变化可使其选项刷新，单个选项变化尽量只更新相关项。新建 Indicator、Dot 和完整 BSN 挂接仍须得到初始颜色。

保留已经修复的 FailureState 诊断路径。变化驱动不能使失效结构在必要的修复信号到来后仍不重新检查，也不能把一次失败误记为样式已成功应用；不增加专为诊断服务的逐帧全量扫描。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖选择／清除、Hover 进入／离开、Disabled 添加／移除、父／子覆盖更新与清除、主题、Focus 转移和初始节点准备。合法程序化选择不新增 ValueChange，结构异常不导致无关有效 Group 的样式失效。

覆盖缺失父级配置之后的真实恢复，确认变化触发不会丢失恢复机会。稳定状态不重复执行全部选项的配色解析；使用定向测量验证实际工作量，不把函数名字或没有写回当成变化驱动证明。

## 预期产出

真正响应样式输入变化、保留已有错误诊断语义的 RadioOption 更新路径。

## 与前后方案的关系

顺序上接 [方案 23：让 RadioOption 的结构错误日志按故障变化记录](23-让-RadioOption-的结构错误日志按故障变化记录.md)。

完成后进入 [方案 25：让 Tooltip 只在配色变化时刷新外观](25-让-Tooltip-只在配色变化时刷新外观.md)。

保留已完成的 RadioGroup 结构诊断和 RadioOption 日志修复，再改变样式触发；不恢复逐帧诊断兜底。
