# test_utils

## 功能概述

提供跨 crate 复用的 Headless App、UI / Picking Fixture、输入构造、错误和日志捕获，以及 Benchmark 辅助能力。仅服务开发和测试。

## 当前设计思路

通过真实 Bevy ECS / UI / Picking 环境建立可控场景，允许测试显式推进 `App::update`。与直接 Trigger Observer 不同，Picking Fixture 可以把受控 PointerInput 送入实际输入管线。

## 主要功能

- `scene_app`、`text_input_app`、`text_edit_app`、`picking_app` 等不同能力的 App Fixture。
- UI Plugin、Camera、Viewport / Scale Factor 准备。
- Pointer Move / Press / Release / Cancel / Drag 等输入构造，以及确定性的 Custom Pointer Identity。
- Queue KeyboardInput、Focus Dispatch、Theme 切换及条件等待。
- `LogCapture`、`ErrorCapture` 捕获日志字段、真实 `BevyError` 和 `ErrorContext`，按原始诊断身份观察错误生命周期。
- Benchmark Fixture、Criterion Harness、可见文字准备检查与 Artifact 输出。

## 行为与状态契约

Headless 输入与 App Update 由测试主动调度；Queue 不代表已经执行。实际 Picking 测试应通过 Backend 计算命中，而不是直接填入被测业务结果。日志和错误捕获的作用域限于当前 Thread，相关 Schedule 应按需求采用 Single-threaded 执行。

`queue_key` 只排队，`press_key` 另推进一次 Update；`queue_pointer` 不推进 Update。`pointer_event` 可指定 PointerId、Location、Target 和 Payload，便捷 Event Helper 默认使用 Mouse，直接 Trigger 只证明 Observer 契约。Picking Fixture 保留 Mouse Entity、关闭硬件输入生成，由测试显式注入 PointerInput；原生窗口操作仍需桌面验收。

`LogCapture::default()` 只接受 `bevy_widgetry`；`with_targets()` 选择精确 target 集合（替换默认集合），可显式包含 Gallery/宿主的 target。`records_for_error()` 按 `ErrorId` 关联，`diagnostic_records()` 按 `DiagnosticPhase::{Source, Handled, Unhandled}` 筛选；记录保留 Level 和原始格式化字段，`matches_field()` 支持展示值或 Debug 字符串值匹配。未知/未迁移事件没有 phase 时返回 None，不按错误文案猜测生命周期。

`run()` 只设置当前线程 Subscriber，不安装全局 Subscriber。`dispatcher()` 可显式移到工作线程，或交给 future 的 `WithSubscriber::with_subscriber()`，确保每次 poll 都处于对应作用域；没有 dispatcher 的线程未捕获日志不代表生产未发事件。真实全局 Subscriber 组合在独立子进程验证。

`ErrorCapture::run()` 嵌套后恢复外层捕获，捕获状态按线程隔离。`take_records()` 取走 `CapturedError { error, context }`，`managed()` 通过统一非泛型载体读取单个或集合诊断，外来错误完整保留。`take()` 仍取走错误列表，适用于不检查 ECS context 的既有测试；两个入口消费同一记录队列。捕获 Handler 只保存数据，不替正式 Gallery 安装终结策略。无捕获作用域时仍交给 Bevy 原生 Severity 策略。

## 边界情况与错误处理

Camera / Asset / Layout 尚未就绪时，调用方需要满足相应 Fixture 前提。等待达到截止条件仍未完成时返回超时诊断，不能假定 UI 已准备就绪。

## 已知限制

- 不作为 Widgetry 生产依赖。
- Headless / Custom Pointer 不等于真实 OS Mouse 和 Native Window 行为。
- Fixture 只为调用方设置约定环境，具体业务断言仍由各 crate 测试负责。

