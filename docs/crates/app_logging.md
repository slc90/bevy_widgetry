# app_logging

## 功能与装配

供宿主选择使用的 Terminal / File 日志基础设施。`AppLogging::new` 准备本地时间、目录和带毫秒时间戳的文件；`install` 安装输出资源并返回 `LoggingGuard`。库不安装全局 Subscriber，也不配置 ECS Error Handler。

宿主先 install，再由自身调用 `app.add_plugins(log_plugin())`。该函数返回官方 Bevy LogPlugin 的完整配置：外层允许 DEBUG，文件层保留 INFO 及以上和带 error_id/phase 的诊断 DEBUG；普通模块 DEBUG/TRACE 仍过滤。终端只展示 INFO 及以上。环境过滤是宿主主动覆盖项，RUST_LOG 的显式过滤仍生效。若自行使用 `file_layer` / `terminal_layer`，须保证外层过滤不裁掉必要诊断。Facade 独立非默认 app_logging Feature 重导出这些 API 与错误类型。

两层保持本地时间、毫秒格式；文件无 ANSI 并保存源码位置。正式诊断的多行 Backtrace 按 Display 写入。

## 写入和关闭

异步 writer 明确使用 non-lossy 模式及有界队列（1024条）。容量满时生产者等待，错误风暴下可能阻塞；不在持有 writer 依赖的锁时发日志。队列背压等待不持有内部健康状态锁。

所有日志生产者结束后才能释放 Guard。`LoggingGuard::finish` 等待已接受字节写入并 flush，返回 `Result<(), Arc<LoggingFailure>>`；默认最多等待5秒，然后执行底层 WorkerGuard 的固定关闭握手。Drop 使用相同收尾，未显式处理的输出失败直接报告到 stderr。正常关闭先完成可观察 flush，不仅依赖 WorkerGuard 的有限等待。后端阻塞、超时、断连或磁盘错误会明确返回失败，不能保证强杀、断电或失效设备上的绝对持久化。

Write、Flush、Queue、FlushTimeout 保存实际路径、原始 io::Error 或待刷字节及源头栈。后端出现失败后保持不健康状态；其源头与 Drop 终结只走独立 stderr，应避免递归写同一个失效后端。显式 finish 返回的失败由宿主继续处理或应急终结。

## 初始化失败

`AppLogging::new` 返回统一 `LoggingFailure`，LoggingError/LoggingStage 区分 LocalOffset、Timestamp、Directory、File 并保留原 source 与 path。准备失败时正式 Subscriber 可能尚未存在；源头事件发出不证明已保存。宿主应通过独立应急输出补齐完整原因、同一 ID、现场及原始栈，再决定退出或采用其他日志方案。

文件直接使用 create_new，时间戳碰撞等文件创建错误明确返回；不覆盖同名日志。
