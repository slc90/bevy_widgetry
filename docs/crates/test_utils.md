# test_utils

## 功能概述

提供跨 crate 复用的 Headless App、UI / Picking Fixture、输入构造、错误和日志捕获，以及 Benchmark 辅助能力。仅服务开发和测试。

## 当前设计思路

通过真实 Bevy ECS / UI / Picking 环境建立可控场景，允许测试显式推进 `App::update`。与直接 Trigger Observer 不同，Picking Fixture 可以把受控 PointerInput 送入实际输入管线。

默认仅使用 Dark 主题。测试基础设施提供 `light`、`pink_dream` 转发 Feature，以及二者的集合 `all_themes`；需要多主题样式回归的 crate 通过 dev-dependency 显式选择。`switch_theme(app, theme)` 接受完整 `WidgetryTheme`，通过 queued Setter 提交并 flush，既可切换已编译的预设，也可使用外部配色数据。基础诊断、非法配色原子拒绝和同值不通知测试在默认配置也运行，预设通知按实际开启的 Feature 覆盖。

## 主要功能

- `scene_app`、`text_input_app`、`text_edit_app`、`picking_app` 等不同能力的 App Fixture。
- `scene_app` 在设置默认等宽主字体前准备 Asset / Text Plugin；默认文本附加内建 Emoji 回退，后续 UI Fixture 复用这些资源，不再重复注册 Text / Widgetry Asset Plugin。
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

捕获器同时保留一个不安装的 Dispatch，避免当前 Tracing 的单 Dispatch 快速路径根据其他未订阅线程首次注册调用点时的状态缓存 `never`。该注册随捕获器及其 Dispatch 克隆释放，不改变线程的 Subscriber 或扩大捕获范围；其他线程先使用同一日志调用点后，作用域内的源头与处理日志仍须完整捕获。

`ErrorCapture::run()` 嵌套后恢复外层捕获，捕获状态按线程隔离。`take_records()` 取走 `CapturedError { error, context }`，`managed()` 通过统一非泛型载体读取单个或集合诊断，外来错误完整保留。`take()` 仍取走错误列表，适用于不检查 ECS context 的既有测试；两个入口消费同一记录队列。捕获 Handler 只保存数据，不替正式 Gallery 安装终结策略。无捕获作用域时仍交给 Bevy 原生 Severity 策略。

## 边界情况与错误处理

Camera / Asset / Layout 尚未就绪时，调用方需要满足相应 Fixture 前提。等待达到截止条件仍未完成时返回超时诊断，不能假定 UI 已准备就绪。

## 已知限制

- 不作为 Widgetry 生产依赖。
- Headless / Custom Pointer 不等于真实 OS Mouse 和 Native Window 行为。
- Fixture 只为调用方设置约定环境，具体业务断言仍由各 crate 测试负责。


Benchmark artifact 的 error 入口保留原始 source 和首次 origin，显式转换为 Error Severity；finish / report_failure 在独立工具的非 ECS 终结边界通过 stderr 补齐原因、ID、位置与原栈，适用于尚未安装 Subscriber 的 CLI。显示用业务状态字符串仍是独立的测量失败说明，不作为恢复原始库诊断的接口。
