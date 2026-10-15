# 让 ComboBox 的样式归属只在必要时建立

## 目标与范围

调整 `crates/combo_box/src/style.rs::establish_owners<T>`、`own_field<T>`、`own_list<T>` 与 Typed Plugin 注册，去掉每帧扫描全部 ComboBox 层级的永久兜底。

## 修改设计

现有 Add Observer 可能早于 BSN 父子关系完成，因此不能只删除兜底。使用实际构造与层级就绪信号记录待建立归属的实例，在 `WidgetryUiSystems::StyleOwners` 边界完成有界补偿。Field、Popup 内部 ListView 以及后续合法重建的节点分别绑定到正确 Root。

正常完成后不再逐帧检查已有所有者。动态重建、Owner 失效和实例销毁需清理对应记录；不能让陈旧 Entity 或旧 Owner 永久留在待处理集合。若缓存部件 Entity，只把它当作派生关系，不保存另一份业务或颜色 Authority。

保留每种 T 的独立 Typed Plugin 和注册幂等性，不增加全局泛型路由服务，不改变 Field／Popup 的公开结构契约和既有 owned 配色入口。内容的 `(id, index, revision)` 缓存也不属于此次修改。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

测量使用相同真实负载和既有 Benchmark 规范；内部工作量计数不成为公共 API。涉及可见行为时增加对应 Gallery 的 BRP 输入、ECS 状态与截图验证，原生 Window／Mouse 行为仍需 Windows 原生证据。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖完整和延后挂接的 BSN 构造、多个 T、多实例、Popup 首次显示、内部 ListView 重建和 Root 销毁。所有权在首次颜色消费前建立，不出现先用子控件默认色再被父控件覆盖的可见闪烁。

稳定 ComboBox 不再进入完整层级兜底；真实结构变化只影响相关实例。错误结构仍可诊断，合法等待不被错误记录为故障。通过初始化回归和定向测量验证，不以“少了一个 System”代替结果检查。

## 预期产出

有完整 BSN 构造兜底、但不永久轮询层级的 ComboBox StyleOwner 维护。

## 与前后方案的关系

顺序上接 [方案 20：补齐 ListView 共享 Model 的双 Renderer 展示](20-补齐-ListView-共享-Model-的双-Renderer-展示.md)。

完成后进入 [方案 22：让 ComboBox 的组合样式按变化更新](22-让-ComboBox-的组合样式按变化更新.md)。

本项先保证 ComboBox 的 Owner 生命周期，后续配色更新依赖这些可靠的构造与归属信号。
