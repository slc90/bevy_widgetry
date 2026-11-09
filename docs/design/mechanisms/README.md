# 跨模块机制

这些文档按共同协作问题组织。它们解释多个模块之间稳定的身份、状态和生命周期关系，不逐个复述每个控件的接入代码。

| 机制 | 关注的问题 |
| --- | --- |
| [控件构造与组合](ui-composition.md) | Scene、Props、持久状态、实际输入部件与外部内容怎样衔接 |
| [输入目标与手势](input-interaction.md) | 命中、焦点、捕获和窗口作用域怎样避免相互争夺输入 |
| [Disabled](disabled.md) | 独立禁用来源、父级继承与原生交互投影怎样保持一致 |
| [主题与样式归属](appearance.md) | Theme、局部覆盖、组合 Owner 和前景传播怎样共同决定外观 |
| [诊断传播](diagnostics.md) | 失败跨模块与线程时怎样保留身份、原因和终结责任 |
| [后台任务与结果提交](job-management.md) | 真实任务退出和业务结果有效性为什么是两层协议 |
| [离屏渲染](offscreen-rendering.md) | 目标、层号、相机、Scene 与消费者资源怎样分工 |
| [排他选择](exclusive-selection.md) | 唯一 Checked 与动态成员变化怎样形成可靠的恢复语义 |

对应模块的内部结构见 [模块设计](../modules/README.md)，整体组织见 [架构](../architecture.md)。
