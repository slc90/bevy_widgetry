# 让 MessageBox 的样式归属按生命周期建立

## 目标与范围

调整 `crates/message_box/src/style.rs::establish_owners`、`own_window`、`own_action` 和构造连接，去除稳定 MessageBox 每帧递归扫描全部后代的兜底。

## 修改设计

Root 的 Window StyleOwner 和结果按钮的 Button StyleOwner 属于 MessageBox 已知部件。优先在构造与真实关系就绪时建立；对 Add Observer 早于 BSN 父子挂接的情况保留有界补偿，不能简单删除现有递归系统。

完成绑定后只处理合法创建、替换或层级变化影响的对象。可以维护内部部件关系，但必须在 Node／Root 销毁时清理，不能把失效 Entity 长期留在待处理状态。只接管 MessageBox 自己的结果按钮和 Window，不接管调用方正文中的普通业务 Button。

嵌套或被 FileDialog 用作覆盖确认的 MessageBox 仍有各自正确的 Owner 链。构造未完成、结构失效和资源销毁要区分；正常 BSN 等待不当作故障，真实托管结构缺失也不能被永久忽略。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

测量使用相同真实负载和既有 Benchmark 规范；内部工作量计数不成为公共 API。涉及可见行为时增加对应 Gallery 的 BRP 输入、ECS 状态与截图验证，原生 Window／Mouse 行为仍需 Windows 原生证据。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖各结果按钮组合、外部正文 Button、延后挂接、多个 MessageBox、FileDialog 覆盖确认和 Root 销毁。首次配色前归属正确；正文 Button 仍不会提交 MessageBox 结果。

稳定对话框不再每帧遍历整个子树建立已有关系。保留一次结果决议、先通知 Consumer 后关闭、Parent Blocker 和 Owned Window／Camera 回收测试；测量只针对归属维护，不宣称改变了这些生命周期行为。

## 预期产出

具有构造期兜底、但不永久递归扫描的 MessageBox StyleOwner 维护。

## 与前后方案的关系

顺序上接 [方案 34：让 Gallery 标题国际化使用 Window 语义接口](34-让-Gallery-标题国际化使用-Window-语义接口.md)。

完成后进入 [方案 36：让 MessageBox 的组合配色按变化更新](36-让-MessageBox-的组合配色按变化更新.md)。

先保证 MessageBox 的归属生命周期，后续组合配色依赖它；继续沿用已整理的 Window 颜色入口。
