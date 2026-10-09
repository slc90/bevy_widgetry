# Widget API 规则

本文件规定 Widgetry 自有 runtime state 的公开更新、查询与输入输出契约。具体 API、event type、target / source 与业务 identity 由对应 Widget 定义，不引入统一 set_value、binding、callback 或 event 总线。

## 自有 state 的更新与查询

公开支持的自有 runtime state 更新必须通过 Widget API；state Component 通过公开只读接口提供查询。适用范围包括自有 selection、active、focused_cell、三态 state 与展开 state。

UI 操作与程序化设置都必须在实际执行更新时先提交真实 state，再发对应变化通知。通知不能作为要求外部回写 authority 的更新请求。排队入口仍在 Commands 实际执行时提交，不承诺调用瞬间完成。

构造 Props 仅负责一次性初始化。自有业务数据 Model CRUD、展示配置和外部数据输入的适用边界由对应 Widget 的公共契约明确，不能仅因它们是 Component 就冻结所有输入。

外部替换或移除自有 state Component、破坏内部 hierarchy、despawn 等 ECS 结构操作不属于支持的 Widget state 更新 API，不承诺相同的变化通知。库内部维护与其他正常 ECS 操作仍遵守各自契约。

提交 authority 不代表高亮、renderer subtree、颜色或 layout 已同步。不得承诺同一 event 的多个 observer 的固定执行顺序。

## 官方 Component 例外

直接复用的 Bevy 官方 Component 保持原有使用方式和行为，不纳入自有 state 的只读化与更新入口治理。例如 EditableText、Checked、ScrollPosition、InteractionDisabled。

不得为了本规则新增官方 Component wrapper、写入限制、写入监测或 event 转接；不要求这些官方机制改为先提交再通知。

有效 Disabled 继承是 InteractionDisabled 的特定例外。安装 Widgetry UI 能力后，受管理 UI 的用户本地请求、Widget 模型或原生能力限制与祖先有效值做 OR，稳定态 InteractionDisabled 表示实际禁用结果。原生 BSN、World 与 Commands 插入或移除入口仍受支持，包括继承期间再次 Insert 以记录本地请求、Remove 以撤销本地请求。投影不把继承结果记为本地请求，移除组件时不允许取消尚未解除的祖先或内部限制。只读 WidgetryEffectiveDisabled 查询同一实际结果，在结构写入后的 World flush 与正常输入派发前同步。尚未 apply 的 Commands 不代表请求已经执行，投影 lifecycle 不转接 Widgetry 业务通知。其他官方 Component 例外不变。

## 变化、同值与清空

状态变化通知只在对应 state 实际改变时发出。合法同值设置成功但不发变化通知；确认、重选与 Activate 等操作语义不能借用状态变化 event。没有明确需求时不得新增独立确认或重选 event。

Widget API 显式清空使 state 实际变化时，先提交再通知，payload 必须能够表达未选中等清空结果。已为空时再次清空不通知；不要求本来必须保持选中的 Widget 新增空 state。

初始化与 Model repair / 自动修复保持既有行为，不因本轮规则新增通知。删除 Model 数据引起的自动 selection repair 不等于 API 显式清空。

统一后的变化通知不增加 User / Programmatic 等 origin 字段。事件语义、identity、target / source 不统一，consumer 按对应 Widget 契约消费。

## 附带行为与时机

保留 setter 对 active、cursor、focus、reveal / scroll 及其他既有附带行为，不统一为只修改目标 state，不为此拆分 API。

保留立即更新 World 与 Commands queue 两类入口及执行时机，不统一调用形式或重新安排执行阶段。disabled、read-only、modal 与 index / stable ID / Entity 继续遵守各 Widget 自己的契约。

## 无效目标与错误

程序化自有 state 更新遇到失效 node Entity、stale item / Row / Column ID、越界 index 或不满足 API 要求的 Widget / source entity 时必须返回错误，不再静默忽略或合并为合法未变化。失败不得执行请求的 state 修改，不发对应变化通知。

对于 Result<bool, BevyError>，Ok(true) 表示目标 state 实际改变，Ok(false) 表示合法但未改变，Err 表示无法执行的无效请求。其他入口按自身形态明确反馈方式；排队入口在实际执行时通过宿主 error handler 反馈错误，不把入队时视为校验完成。

错误诊断、Severity::Error 与传播遵守 [代码规则](code.md) 和 [日志规则](logging.md)。正常 observer 过滤、Option 缺失、asset 等待、UI guard、初始化、自动 repair 和官方 Component 行为不因此改成错误。
