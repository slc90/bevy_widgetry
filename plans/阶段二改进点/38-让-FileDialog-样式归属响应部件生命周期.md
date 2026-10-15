# 让 FileDialog 样式归属响应部件生命周期

## 目标与范围

调整 `crates/file_dialog/src/style.rs::establish_color_owners` 及 `controls.rs`、`window.rs`、`view.rs` 的对应归属逻辑，避免每帧重扫 Dialog、Controls、Parts 和嵌套组合控件。

## 修改设计

独立 Window、Button、TextField、ComboBox、CheckBox、ScrollArea、文件行及覆盖确认 MessageBox 的样式归属，在真实构造或重建时建立。优先利用已有 Control／Part 的 Root 信息，而不是反复查询所有实体和祖先来重建已知关系。

BSN 延迟挂接、异步生成控制、列表行复用和覆盖确认窗口创建都需要有界补偿。移除永久兜底前，必须有对应的生命周期信号。销毁或 Session 结束时清除关系与派生缓存，旧 Entity／旧 Session 不能影响后来的 Dialog。

多个 Dialog 的 Owner 链相互隔离。对嵌套组合控件建立完整链条，后续内部交互变化能够找到最终 FileDialog Owner；不能只把中间控件排除出独立样式系统，却没有负责者接收其变化。

本项只维护派生样式关系，不改变 Runtime／Worker 架构、请求 Token、View 的业务身份和对话框结果语义。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

测量使用相同真实负载和既有 Benchmark 规范；内部工作量计数不成为公共 API。涉及可见行为时增加对应 Gallery 的 BRP 输入、ECS 状态与截图验证，原生 Window／Mouse 行为仍需 Windows 原生证据。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖 Embedded／Owned、Modal／NonModal、多个并存 Session、动态行／控制、复用、覆盖确认打开关闭和根节点回收。首次颜色更新时所有必要 Owner 已就绪，旧关系不串到新 Session。

稳定 FileDialog 不再为了确认已有 Owner 扫描全部部件。相关构造、样式、异步和生命周期回归保持成立；定向测量区分首次建设与稳定开销，不通过删除必要异常检查取得结果。

## 预期产出

覆盖异步与嵌套组合、按生命周期维护的 FileDialog StyleOwner 关系。

## 与前后方案的关系

顺序上接 [方案 37：分离 FileDialog 低频配置与持续 UI 同步](37-分离-FileDialog-低频配置与持续-UI-同步.md)。

完成后进入 [方案 39：让 FileDialog 颜色按实际状态和可见行变化更新](39-让-FileDialog-颜色按实际状态和可见行变化更新.md)。

承接 FileDialog 的低频配置拆分；本项建立可靠归属，下一项再消除重复配色。
