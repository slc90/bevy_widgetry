# Waveform File Coverage

- `recording.rs`：真实落盘与再打开、元信息一致性、不同密度和跨桶随机读取、逐通道缺口、短尖峰、边界极值精确性、可见工作集与 I/O 有界、异常构建不发布文件、损坏文件拒绝。
- `source.rs`：后台 Pending、最新请求优先、reader 隔离、取消旧目标、真实文件错误。
- `source.rs` 还覆盖显式共享 Manager 的关闭拒收。Source 模块并发回归覆盖空闲全局取消、阻塞查询期间 Drop 返回且仍 Stopping、Timeout 后真实退出、过期失败不干扰其他 reader，以及线程 panic 后公开读取错误。
- `source.rs` 同时检查真实读取的有界统计快照：查询计数、实际读取字节、非零文件 I/O 计时，以及包含 I/O 的完整查询耗时。不将机器耗时作为性能断言。
- `lttb.rs`：六个 LTTB 密度独立等价于直接原始样本降采样；跨块缺口、样本身份、时序、PolyLine 算法一致性及局部随机窗口。`source.rs` 还覆盖快速 MinMax/LTTB 交替后的最新算法结果。

所有数据查询经正式文件格式执行，无模拟延迟。耗时由 Benchmark 测量，行为测试只断言确定性结果及工作量。
