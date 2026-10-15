# 让 MessageBox 的组合配色按变化更新

## 目标与范围

调整 `crates/message_box/src/style.rs::update_colors` 与 owned 配色路径，减少 Window、Body 和结果按钮的重复颜色解析。

## 修改设计

根据初次构造、动态受管节点、Theme、颜色覆盖、有效 Disabled 及结果按钮 Hover／Pressed 的添加和移除刷新。使用已维护的部件归属定位对象，不能在收集 Dirty 之后仍无条件递归整个正文。

MessageBox 继续提供 Window 和 Button 的组合配色，正文承载区域的颜色与调用方业务内容的自有样式分开。内部按钮和窗口控制的变化需要传到正确 Owner；若 MessageBox 又被 FileDialog 接管，变化还须到达该最终 Owner，不能停在一个不会执行独立样式系统的中间节点。

保持现有颜色优先级和 Foreground 管理。`set_if_neq` 仍可避免冗余写入，但它本身不是避免解析与扫描的措施。Theme 变化时必要的全局更新保留，动态新建内容不能漏掉初始配色。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖 Ok／YesNo／YesNoCancel、结果按钮与正文普通按钮、Hover／Pressed 移除、Disabled、Theme／Override 清除、多个窗口和覆盖确认嵌套。稳定对话框不反复执行组合颜色计算，局部输入后颜色及时正确。

结果通知、回调、一次决议与关闭时序完全不变；外部销毁不伪造 Cancel。相应行为测试和 BRP 实际对话框操作随本项验收，性能比较保持相同正文和按钮数量。

## 预期产出

不改变 MessageBox 决议生命周期的变化驱动组合配色。

## 与前后方案的关系

顺序上接 [方案 35：让 MessageBox 的样式归属按生命周期建立](35-让-MessageBox-的样式归属按生命周期建立.md)。

完成后进入 [方案 37：分离 FileDialog 低频配置与持续 UI 同步](37-分离-FileDialog-低频配置与持续-UI-同步.md)。

依赖 MessageBox 的 Owner 调整，保持下层 Window／Button 的受管配色契约。
