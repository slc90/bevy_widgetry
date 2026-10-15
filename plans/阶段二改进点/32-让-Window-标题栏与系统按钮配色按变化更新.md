# 让 Window 标题栏与系统按钮配色按变化更新

## 目标与范围

调整 `crates/window/src/style.rs::update_colors` 和 `apply_owned_window_colors` 的调度与失效来源。目标是减少 Root、TitleBar、WindowContent 和系统按钮的重复配色检查。

## 修改设计

根据窗口及受管节点创建、Theme、颜色覆盖、有效 Disabled、最小化／最大化／关闭按钮的 Hover／Pressed 添加与移除更新。保留独立窗口和被 MessageBox／FileDialog 接管的 owned 配色路径；系统按钮变化必须能触达最终样式 Owner。

保持 Theme 背景与 Image 背景的区别：Theme Window 才接受对应背景色，图片未加载时保持透明，Image Tint 与构造期 Opacity 的组合不变。Handle、Mode、Opacity 仍是当前构造期配置，不借优化增加运行时图片 API。

只管理 Window 负责的标题栏、正文承载区域和系统按钮，不递归改写调用方任意业务子树，也不越过嵌套窗口边界。复用派生部件关系时维护创建和销毁，不让缓存 Entity 成为新的生命周期 Authority。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖独立窗口与组合 Dialog，系统按钮 Hover／Pressed／Release、Disabled 切换、Theme／Override 改变、动态创建与关闭。验证 Image 和 Theme 两类背景保留既有行为，窗口内容不因配色更新重建。

Native Window／Camera 的 Owned／Borrowed 生命周期、拖动缩放、多窗口输入、IME、Modal 和状态同步不属于改动范围，相关回归不得退化。实际 Native 操作不能只凭 BRP Custom Pointer 声称已通过；样式性能分别检查静止多窗口与持续按钮输入。

## 预期产出

保持窗口基础设施和背景契约的变化驱动 Window 配色。

## 与前后方案的关系

顺序上接 [方案 31：补齐 Table 共享 Model 的独立布局展示](31-补齐-Table-共享-Model-的独立布局展示.md)。

完成后进入 [方案 33：为 Window 提供稳定的标题文字绑定与更新入口](33-为-Window-提供稳定的标题文字绑定与更新入口.md)。

先优化 Window 自身配色，供后续组合对话框沿用；标题语义 API 属于独立的下一项能力。
