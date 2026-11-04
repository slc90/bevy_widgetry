# Diagnostics Integration Test Coverage

`lifecycle.rs` 从公开 API 验证真实错误的产生、传播、共享及释放。`log_lifecycle.rs` 在独立测试进程中验证源头与处理日志，避免无 Subscriber 的并发源头测试干扰 Tracing callsite 的动态注册。

- 源头保存调用位置，并在移除回溯环境变量的子进程中强制捕栈；普通 Display/Debug 与源头日志不输出原栈。
- 初始现场、追加 context、领域包装及真实 source/downcast 链同时保留；跨线程共享不改变 origin/ID。
- 独立失败（包含相同文案）和并发失败有不同身份；最后一个拥有者释放后 origin 可销毁。
- 真实 Subscriber 组合观察到源头一次 ERROR、仅传播静默、显式实际处理一次 DEBUG；关联同一 ID，包含措施和结果。观察与 Drop 不推断已处理。

`ecs_bridge.rs` 仅在 `bevy` Feature 下运行，覆盖真实 System/Observer/Commands::queue 的 Error Severity、共享 origin/source、多层上下文、空批量和多失败逐项分发；原生 BevyError 保留原始对象、context、Severity 及 panic payload。

同一 origin 的重复与八线程并发终结分发只调用一次 managed 报告；此报告状态与 handled 独立。带受管载体但采用原生 Panic 策略的错误仍完整交给 native 回调，每次保留 payload，且不占用后续 Error Severity 的 managed 报告资格。

ID 计数器的跨字长进位由 `origin.rs` 的 Unit Test 覆盖。宿主最终日志/持久化及领域生产者迁移尚不属于此层。

`lifecycle.rs` 另覆盖宿主自定义源头reporter：接收已捕获的同一origin，传播后身份不变；普通格式化无完整栈。
