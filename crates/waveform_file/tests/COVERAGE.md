# Waveform File Coverage

- `recording.rs`：真实落盘与再打开、元信息一致性、不同密度和跨桶随机读取、逐通道缺口、短尖峰、边界极值精确性、可见工作集与 I/O 有界、异常构建不发布文件、损坏文件拒绝。
- `source.rs`：后台 Pending、最新请求优先、reader 隔离、取消旧目标、真实文件错误。
- `source.rs` 还覆盖显式共享 Manager 的关闭拒收。Source 模块并发回归覆盖空闲全局取消、阻塞查询期间 Drop 返回且仍 Stopping、Timeout 后真实退出、过期失败不干扰其他 reader、线程 panic 后公开读取错误，以及 Source 销毁/关闭后迟到的 panic 诊断消费与失败记录释放。
- `source.rs` 同时检查真实读取的有界统计快照：查询计数、实际读取字节、非零文件 I/O 计时，以及包含 I/O 的完整查询耗时。不将机器耗时作为性能断言。
- `lttb.rs`：六个 LTTB 密度独立等价于直接原始样本降采样；跨块缺口、样本身份、时序、PolyLine 算法一致性及局部随机窗口。`source.rs` 还覆盖快速 MinMax/LTTB 交替后的最新算法结果。
- `diagnostics.rs`：真实不存在/权限拒绝、截断及打开后文件截短、错误标识、JSON 原因、单位与层表布局、两算法索引 offset、原始/producer 非有限样本、错误帧/通道范围与零密度；检查可匹配分类、path、offset/length 与通道/范围/算法语境。producer 原 origin 与块现场保持，失败清理临时资源、拒绝覆盖保留已有目标；源头一次 ERROR、纯传播无 handled、实际 Command 经宿主保留 Error Severity 和原 origin。

所有数据查询经正式文件格式执行，无模拟延迟。耗时由 Benchmark 测量，行为测试只断言确定性结果及工作量。

- 控制诊断：关闭拒收保留 SubmitError::ShuttingDown、共享 origin、源头一次 ERROR、无虚假 handled，并经真实 Bevy Command 保持 Error Severity；已移除记录的 Source Drop 无错误，panic 结束后移除再读取保留 NotFound 诊断。

- Source 并发回归同时检查已完成失败在取消、同 reader 替换、Source 销毁及阻塞查询迟到后被明确废弃时，只记录同源处理 DEBUG；错误构造于捕获作用域，真实执行线程显式携带测试 dispatcher。既有受管 Job 控制报告直接共享，不再匹配旧 enum 包装。
