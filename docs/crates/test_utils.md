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
- `LogCapture`、`ErrorCapture` 捕获日志字段和 `BevyError`。
- Benchmark Fixture、Criterion Harness、可见文字准备检查与 Artifact 输出。

## 行为与状态契约

Headless 输入与 App Update 由测试主动调度；Queue 不代表已经执行。实际 Picking 测试应通过 Backend 计算命中，而不是直接填入被测业务结果。日志和错误捕获的作用域限于当前 Thread，相关 Schedule 应按需求采用 Single-threaded 执行。

## 边界情况与错误处理

Camera / Asset / Layout 尚未就绪时，调用方需要满足相应 Fixture 前提。等待达到截止条件仍未完成时返回超时诊断，不能假定 UI 已准备就绪。

## 已知限制

- 不作为 Widgetry 生产依赖。
- Headless / Custom Pointer 不等于真实 OS Mouse 和 Native Window 行为。
- Fixture 只为调用方设置约定环境，具体业务断言仍由各 crate 测试负责。

