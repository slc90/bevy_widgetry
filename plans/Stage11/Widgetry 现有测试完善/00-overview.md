# Widgetry 测试完善：总览与执行顺序

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

本套方案完善现有长期测试，而不是再修改测试规则。共 13 份：10 个控件各一份，WidgetryIcon 单列，另有共享测试基础和 facade 两份配套方案。各份均包含已有覆盖、实际缺口、修改边界、验证方式与证据链接，可以单独作为 Codex 的任务输入。总方案和全部分方案正文均保留在本包中。

## 唯一执行顺序

1. [共享测试基础：让输入、时间与渲染观察点可信](01-shared-test-foundation.md)

   收敛已经重复的 headless 装配，补齐 core 的跨阶段文字/前景色验证；不创建新的大测试框架。

2. [Icon：保护异步换图、颜色与首帧显示](02-icon.md)

   保留已有异步加载回归，补等待期间的连续显示、连续换图、继承色和独立 raster 合同。

3. [Button：补齐样式转换的完整输出与最小交互证明](03-button.md)

   加强已有样式转换断言，不重复有限状态优先级测试；补本模块的最小公开行为入口。

4. [CheckBox：补三态输入、取消与图标投影的状态转换](04-check-box.md)

   二态和三态留在一份方案中，重点补自有三态键盘/取消路径和图标实际输出。

5. [RadioGroup：保护固定选项的选择、队列与实例隔离](05-radio-group.md)

   保留已经较完整的输入和主题覆盖，补小状态边界、延迟操作与多组隔离。

6. [TextField：从输入过滤补到真实编辑消费与恢复](06-text-field.md)

   普通和只读文本框一起整理；保留完整过滤表，重点检查过滤后的文本结果、禁用恢复和主题不改内容。

7. [Tooltip：连接悬停计时与公开 popup 生命周期](07-tooltip.md)

   已有计时和样式测试保留，补公开 Tooltip 从 hover 到显示、隐藏、销毁及 picking 的组合证据。

8. [ScrollArea：补公开构造下的真实布局收敛与滚动恢复](08-scroll-area.md)

   保留纯算法和手工几何的局部测试，增强公开 Scene、真实 layout、回退状态和刷新终止的组合覆盖。

9. [ListView：补强状态转换、跨实例隔离与 ListModel 性质测试](09-list-view.md)

   保留现有 navigation、virtualization 和首帧文本回归，补公开状态转换缺口，并在 ListModel 首次实际使用 proptest。

10. [ComboBox：保留已有交互回归，补内容首帧与恢复路径](10-combo-box.md)

不重写已经充分覆盖的 popup/selection 逻辑，重点加强单帧删除、text+icon、真实 popup layout 和禁用后的恢复。

11. [Window：补 resize、modal 隔离与原生能力的验证边界](11-window.md)

保留外部/owned 资源与绑定测试，补精确 resize 请求和 modal 跨窗口协作，明确无法由 headless 覆盖的真实最大化。

12. [MessageBox：补结果重入、关闭顺序与真实内容投影](12-message-box.md)

围绕一次性结果和 observer 可见生命周期加强测试，补 reentrant 与 disabled 恢复，并把主题断言推进到实际文本。

13. [Facade：补消费者组合测试并完成全仓测试验收](13-facade-and-final-validation.md)

保留 API 可用性测试，补统一入口的行为证据和跨控件 focus 场景，完成所有小方案的全仓验证收尾。

按 01 → 13 执行。共享设施先于实际消费者；ScrollArea → ListView → ComboBox、Window → MessageBox 按现有组合关系排列。没有技术依赖的相邻方案只是本次工作的顺序，不代表新增依赖。详细约束与证据均在各分方案内，总览只做导航。
