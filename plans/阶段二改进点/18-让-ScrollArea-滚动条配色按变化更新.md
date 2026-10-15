# 让 ScrollArea 滚动条配色按变化更新

## 目标与范围

调整 `crates/scroll_area/src/style.rs::update_thumb_style` 及相应调度，只减少 Track／Thumb 的重复配色工作，不修改滚动、测量或几何机制。

## 修改设计

当前系统每帧枚举全部独立 ScrollArea，再遍历 Bar 和 Thumb 解析颜色。将更新范围限定到首次构造、新建或替换 Bar／Thumb、有效 Disabled 变化、Hover、拖动状态、Theme 与颜色覆盖变化。组件移除必须和添加一样得到处理，不能只添加 `Changed` 查询。

用控件内部的相关变化信号定位实际 ScrollArea 或 Thumb。拖动位置变化不天然意味着配色变化；只把现有颜色规则真正消费的状态作为样式输入。主题切换可刷新相应全部实例，这是必要更新，不应被当作要消除的全量工作。

被上层 StyleOwner 接管的 ScrollArea 继续接受所属组合控件提供的颜色；内部交互变化不能因跳过独立样式系统而无人处理。保留 owned 配色接口和当前样式优先级，不把覆盖颜色和主题解析另存为业务 Authority。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖横向、纵向、双轴，Hover 进入／离开、开始／结束拖动、Disabled 添加／移除、主题切换、Override 清除和动态创建。验证 Hidden 只隐藏 Bar，不关闭滚动；Disabled 不阻止合法程序化 ScrollPosition 和必要 Thumb Geometry。

静止区域不重复进行完整样式遍历；单个 Thumb 的状态变化不使不相关区域重新配色。通过相关测试和定向 Benchmark 检查实际工作量，保留现有滚动、IntoView、布局和虚拟化使用者的回归。GUI 证据按真实 Windows／BRP 能力取得，不能用样式断言代替滚动交互验证。

## 预期产出

保持现有滚动契约的变化驱动滚动条配色路径。

## 与前后方案的关系

顺序上接 [方案 17：让 RadioGroup 固定结构诊断响应实际变化](17-让-RadioGroup-固定结构诊断响应实际变化.md)。

完成后进入 [方案 19：让 ListView 容器和可见行配色按变化更新](19-让-ListView-容器和可见行配色按变化更新.md)。

与相邻增量的联系主要是整体实施顺序，不虚构额外的技术依赖；本项可以在自身范围内完成和验收。
