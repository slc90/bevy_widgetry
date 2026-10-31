# 让测试能观察完整错误生命周期

扩展已有 LogCapture 和 ErrorCapture，验证的对象从一段错误文本变成源头、处理、兜底三个可关联事件。它是后续迁移共享的测试能力，不是单纯运行全仓测试。

## 修改落点与前置能力

涉及：`crates/test_utils/src/logging.rs`、`crates/test_utils/src/error.rs`、`crates/test_utils/src/lib.rs`、`crates/test_utils/tests/`、`rules/testing.md`。

实际前置为[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 捕获错误与事件

LogCapture 当前只接受 bevy_widgetry target，且 with_default 仅作用当前线程。保留默认行为，并允许测试显式选择 Gallery/宿主诊断 target；增加按 error_id、事件阶段、level、结构化字段查询的断言帮助。

ErrorCapture 保留真实 BevyError 与 ErrorContext，支持读取受管单错误/集合，不通过格式化错误文本决定来源。测试捕获 Handler 与正式 Gallery Handler 保持职责分开。

后台线程/异步任务需要在测试创建线程或轮询 future 时显式传递 dispatcher，不能以主线程没有捕获到日志断言后台没有记录。需要验证真实全局 Subscriber 的情形用独立子进程，避免测试间反复安装全局订阅。

## 可复用断言

支持断言：源头一条 ERROR 且没有栈；纯传播零条重复 ERROR；真实处理一条 DEBUG 且没有栈；最终兜底关联同一 ID 并输出源头栈。不要断言完整回溯字符串或固定栈帧序号，验证业务源头标识/捕获位置及诊断对象身份。

必要测试覆盖空集合、多个失败、外来错误、同文案不同错误、无环境变量捕获和显式处理后的释放。基础设施仍只通过 dev-dependencies 使用，不能泄漏到生产依赖图。

`rules/testing.md` 的「错误路径验证」仍沿用所有 API 一律检查 `Severity::Error` 的旧写法，需要随本增量调整：领域层按契约检查 `kind`、`source`、`origin`、`error_id` 和源头日志；在 Bevy 执行边界检查 `Severity::Error` 与宿主 Handler 调用；真实处理、仅传播和最终未处理的事件按已定义的生命周期验证。保留 Gallery 既有测试例外，以及正常 Guard、合法 No-op 不人为制造错误的规定。

## 完成条件

- `rules/testing.md` 不再要求所有领域 API 返回 `BevyError` 或检查 `Severity`，同时保留 ECS 边界检查与真实业务 Invariant。
- 自身单元/集成测试证明不会串扰不同测试线程，不丢掉 DEBUG 或未知 target 的显式捕获。
- 实际受管线程中产生的事件可被观察，未传 dispatcher 的行为不被误判为生产丢日志。
- 用最小 Bevy Scene/Commands fixture 与普通线程/future 证明捕获接口可复用，不要求尚未迁移的 Job 工作闭包已经保留受管诊断；Core Scene 和 Job 的真实组合验收分别归入其迁移增量，避免反向依赖。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_test_utils
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 docs/crates/test_utils.md 与 tests/COVERAGE.md。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 顺序衔接

前一项：[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)；后一项：[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
