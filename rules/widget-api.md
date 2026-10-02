# Widget API 规则

本文件规定 Widgetry 自有 runtime state 的公开更新、查询与输入输出契约。具体 API、event type、target / source 与业务 identity 由对应 Widget 定义，不引入统一 set_value、binding、callback 或 event 总线。

## 自有 state 的更新与查询

公开支持的自有 runtime state 更新必须通过 Widget API；state Component 通过公开只读接口提供查询。适用范围包括自有 selection、active、focused_cell、三态 CheckBox state 与 Tree 展开 state。

UI 操作与程序化设置都必须在实际执行更新时先提交真实 state，再发对应变化通知。通知不能作为要求外部回写 authority 的更新请求。排队入口仍在 Commands 实际执行时提交，不承诺调用瞬间完成。

构造 Props 仅负责一次性初始化。自有业务数据 Model CRUD、展示配置和外部数据输入的适用边界由对应 Widget 的公共契约明确，不能仅因它们是 Component 就冻结所有输入。

外部替换或移除自有 state Component、破坏内部 hierarchy、despawn 等 ECS 结构操作不属于支持的 Widget state 更新 API，不承诺相同的变化通知。库内部维护与其他正常 ECS 操作仍遵守各自契约。

提交 authority 不代表高亮、renderer subtree、颜色或 layout 已同步。不得承诺同一 event 的多个 observer 的固定执行顺序。

## 官方 Component 例外

直接复用的 Bevy 官方 Component 保持原有使用方式和行为，不纳入自有 state 的只读化与更新入口治理。例如 EditableText、Checked、ScrollPosition、InteractionDisabled。

不得为了本规则新增官方 Component wrapper、写入限制、写入监测或 event 转接；不要求这些官方机制改为先提交再通知。

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

## Table resize 的结束与取消

自有 Table gesture 保留 ColumnResizeStart 与实际 width 变化的 ColumnResized。正常 DragEnd 先提交最终有效 width，再发 ColumnResizeEnd；没有新 width 时不重复发 ColumnResized。

Pointer Cancel、disabled 或 Column / handle 失效引起的中断发独立 ColumnResizeCancel，保留最后已提交 width，不默认回滚。End 与 Cancel 不承担 width 变化通知的职责。root 销毁不保证补发结束或取消。

程序 width 更新按实际 state 变化通知，不伪造用户 drag 的 Start / End。具体 payload 由 Table API 定义。

Window 的官方 WindowResized 是 native 尺寸变化 Message；ScrollArea 复用官方 ScrollPosition / scrollbar 行为，不包装统一结束协议。离散选择、展开与 MessageBox 一次性结果不套用 resize gesture 协议。

## 实施与验证边界

本规则落地不代表现有 Widget 已全部迁移。当前事实以源码、rustdoc 与 docs 为准；受影响 Widget 逐个完成 API、consumer 与说明同步，不以历史 plans 放宽本规则或提前宣称新行为。

具体只读 type、event payload、清空 API、是否携带 old value / 发起 view，以及各排队入口的错误反馈方式，应在对应任务中明确，不由本规则统一 signature。Tree loader state、Waveform 外部 cursor、Icon mutable method 的边界必须明确，不能无依据认作官方例外，也不能据此新增业务通知。

新增或改变行为按 [开发流程](development.md) 与 [测试规则](testing.md) 验证；通过真实 public API / UI stimulus，在 consumer observer 中读取已提交 authority，不能只检查最终 state 来证明通知顺序。GUI 与性能验证分别遵守 [GUI 规则](gui-debugging.md) 与 [benchmark 规则](benchmark.md)。公共语义附着 rustdoc，直接 consumer 同步迁移；实际 architecture 事实改变时同步 docs/architecture.md。
