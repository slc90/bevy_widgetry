# Diagnostics Integration Test Coverage

`lifecycle.rs` 从公开 API 验证真实错误的产生、传播、共享及释放。`log_lifecycle.rs` 在独立测试进程中验证源头与处理日志，避免无 Subscriber 的并发源头测试干扰 Tracing callsite 的动态注册。

- 源头保存调用位置，并在移除回溯环境变量的子进程中强制捕栈；普通 Display/Debug 与源头日志不输出原栈。
- 初始现场、追加 context、领域包装及真实 source/downcast 链同时保留；跨线程共享不改变 origin/ID。
- 独立失败（包含相同文案）和并发失败有不同身份；最后一个拥有者释放后 origin 可销毁。
- 真实 Subscriber 组合观察到源头一次 ERROR、仅传播静默、显式实际处理一次 DEBUG；关联同一 ID，包含措施和结果。观察与 Drop 不推断已处理。

ID 计数器的跨字长进位由 `origin.rs` 的 Unit Test 覆盖。此增量不包含 Bevy ECS 边界、宿主输出/持久化或领域生产者迁移。
