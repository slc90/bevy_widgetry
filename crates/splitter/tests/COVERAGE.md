# Splitter coverage

`layout.rs` 使用真实 Bevy UI Layout 与 BSN Scene，覆盖主轴方向、Pane 本体 Patch、默认及初始化 Clamp、双侧 min/max 与根范围、单侧无上限、父级溢出、被动 Resize 目标保持、嵌套独立性和错误通路。状态维度为方向、保存比例、初始化阶段与原生约束；输入包括构造、父级尺寸及 Pane Node 变化。初始化保持通知静默。

`constraints.rs` 覆盖百分比约束与逻辑像素/Scale Factor 的耦合，以及外部 ScrollArea 包装最小尺寸溢出的真实布局；ScrollPosition 与 Overflow 保持其原生职责。动态给两个百分比最小尺寸 Pane 设置像素最大值时，先应用根范围再校验收敛后的单侧限制，合法更新不进入 error handler。

持续约束错误时，等待根重排保留诊断状态，不虚发恢复日志；只有实际有效布局恢复后才记录一次恢复。

`ratio_api.rs` 覆盖横/纵向 World Setter 与 Commands 的时机、[0.3,0.8] 可达区间 Clamp、同值静默、初始化/被动变化静默、回调读取已提交比例、主动替换被动不可达目标。错误路径包括非法数值、失效/错误/不完整Root、内部Handle丢失、未布局/隐藏后的无效尺寸及临时约束冲突；失败不改比例或通知，Commands实际错误由宿主捕获。分数端点保护初始化与Setter比例始终属于[0,1]。

`input.rs` 使用真实Picking覆盖Mouse/Custom Pointer、横/纵向按住即更新、移出区域Capture、两端Clamp后立即反向、同值/无移动静默、Release/Cancel/失焦/Capture丢失/指针丢失/隐藏/禁用/resizable中断清理与保留最后比例。覆盖Capture丢失后排队Move、不同Pointer/Target/Button终止事件隔离、拖动期间程序Setter及约束更新、DPI/UiScale、嵌套实例隔离、键盘不控制手势，以及Despawn即时释放Capture。

`resizable` Props、查询与World/Commands API保护错误结构及执行时机；真实Pane Button在resizable=false时保持可用，祖先InteractionDisabled时继承禁用。程序比例Setter在两种禁用下仍正常工作。

同一update内排队Move→Release时，Mouse/Custom在命中区内、区域外及超出约束的最后位移均先提交再清理；不把Bevy提前清除Capture误判为外部Capture丢失。

GUI 视觉、外部 ScrollArea 滚动由任务级 BRP 验证；不将合成 ComputedNode 或测试通过当作 GUI 证据。

`box_layout.rs` 保护分数 min 尺寸及 1.0/1.25/1.5 Scale Factor 下的首次比例修正，横/纵向 ContentBox Pane 与根的 padding/border 只计入一次，并验证 Bevy 原生 border 像素吸附后的双侧最大范围。百分比 padding 保留主轴比例关系，根范围稳定后才提交初始化比例，避免临时尺寸永久改变初值修正结果。
