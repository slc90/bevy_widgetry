# waveform_file 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 文件 Recording／Index、原始与预降采样数据、通道缺口和多层密度；后台 Source Reader Pending／Ready／Failed。
- 共享 JobManager、取消／关闭、多个 Reader 和统计快照。

## Stimuli
- 真实文件写入／读取、随机窗口与 LTTB／MinMax 查询、Reader 请求替换、Manager 关闭、文件损坏或原生 I/O 失败。

## Guards
- 非法文件、头／尾／偏移／通道／密度和非有限样本必须拒绝；失败不得覆盖已有合法目标文件。
- 旧请求、关闭后的提交和迟到后台结果不能污染当前 Reader。

## Transitions
- Recording 构建 → 发布文件 → 重新打开与查询；Source Pending → 完成／错误／取消；Request 替换旧目标。

## Invariants
- Wire Layout、原始有效性、MinMax／LTTB 分层索引及样本身份可由独立 Fixture 验证，不能只依赖自身读写 Round-trip。
- 不同密度／跨块缺口的随机窗口读取保留时间、极值和通道语义；可见工作集及实际 I/O 请求保持有界。
- 多 Reader 隔离，最新目标优先；Manager 关闭、Source Drop、线程 panic 或队列异常后，已完成错误被正确归属和消费。
- 真实 I/O 错误的 source、origin、path、offset、通道及算法上下文在后台和宿主 ECS 之间保持一致；持续故障不伪造新的根因。
- 查询统计可反映读取字节与耗时，但业务测试不将机器耗时当作性能阈值。

## Couplings
- 文件索引 × MinMax／LTTB；Reader 并发 × 取消／Manager；损坏文件 × 错误传播／输出原子性。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 文件内容、I/O、索引／单位和后台命令错误的原因链。 |
| `lttb.rs` | 各级 LTTB 与原始样本在不同窗口／缺口下的一致性。 |
| `recording.rs` | 真实录制文件构建、独立 Wire Fixture、再打开及非法数据拒绝。 |
| `source.rs` | 后台 Reader、请求替换、Manager 关闭、并发、统计与失败回收。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/source.rs` | 后台 Reader 的取消、替换、Drop 与真实退出；阻塞请求、线程 Panic／队列中毒及跨线程失败诊断的归属和消费。 |
