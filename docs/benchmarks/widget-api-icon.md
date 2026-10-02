# Icon entity API 的 CPU 更新验证

2026-10-02，比较 1dcbf85 的 Icon setter 与本次 entity API。Icon 的公开实例接口改为只读，立即入口写入私有输入，Commands 入口在执行时校验；SVG raster、cache 与调度保持原有实现。遵守 rules/benchmark.md，不新增 SLA，不声明性能改善。

## 负载与计时

可重复入口为 crates/core/benches/icon_update.rs，使用 test_utils 的 Criterion / artifact。Windows 10 19045、Ryzen 9 3900X、RTX 4070 Ti SUPER（driver 32.0.15.9579）、Rust 1.98.1，默认 features、bench opt-level=3、无 RUSTFLAGS。CPU 场景没有 native window、render device、GPU 或 tracing subscriber。

10/100/1000 个独立 Icon 使用 16×16 max_size；另保留两个 preload root，始终持有 WindowClose / WindowRestore SVG 的 strong handle。真实 BSN、loader、首 raster、image cache 建立与 fixture drop 排除在计时外。idle 为一次 App::update；color 连续交替 BLACK/WHITE；svg 连续交替两个预加载 SVG。计时包括所有 entity 输入提交与一次真实 App::update；每次操作的 image handle/color 与 entity 数量在计时外逐个核对。颜色或 SVG 成功请求不作为已显示证明。

Criterion flat sampling 使用60个 batch、0.5秒 warmup、1秒 measurement；独立 fixture 另预热20次、采样200次，报告逐次 median/P95/min/max，P95仅描述此次分布。排队场景使用真正 Commands entity API，入队、flush 和 projection 都包含在一次操作内，建立自己的初始 baseline。

## 修复后的比较

baseline 使用最终相同 fixture，仅临时恢复 HEAD 的 core Icon 与旧 setter 调用，其他 consumer 不参与此 headless core 场景。源文件先备份，再通过 finally 逐字节恢复并校验 SHA-256。测量均串行执行，没有并行构建、其他 benchmark 或 Gallery。

```text
cargo bench -p bevy_widgetry_core --bench icon_update -- --save-baseline stage12-17-fixed-before --sample-size 60 --warm-up-time 0.5 --measurement-time 1
cargo bench -p bevy_widgetry_core --bench icon_update -- 'icon/n[0-9]+/(idle|color|svg)$' --baseline stage12-17-fixed-before --sample-size 60 --warm-up-time 0.5 --measurement-time 1
cargo bench -p bevy_widgetry_core --bench icon_update -- queued --save-baseline stage12-17-queued-final --sample-size 60 --warm-up-time 0.5 --measurement-time 1
```

首条为记录实际取得 baseline 的命令，需要迁移前 setter 源码及同一 fixture；当前源码可直接执行后两条，比较需要保留原 named baseline。未保留 baseline 的新环境应使用新名称建立自身初始结果，不能假定本机 target artifact 存在。

| Icon 数量 / 操作 | baseline median / P95 μs | 最终 median / P95 μs |
| --- | ---: | ---: |
| 10 / idle | 121.500 / 150.700 | 123.850 / 156.600 |
| 10 / color | 122.550 / 163.700 | 125.050 / 152.600 |
| 10 / svg | 163.300 / 203.100 | 150.400 / 188.100 |
| 100 / idle | 125.750 / 156.700 | 115.400 / 134.000 |
| 100 / color | 129.800 / 184.800 | 119.900 / 152.700 |
| 100 / svg | 504.450 / 899.300 | 443.050 / 532.400 |
| 1000 / idle | 130.800 / 178.000 | 124.000 / 143.100 |
| 1000 / color | 180.600 / 218.100 | 175.250 / 194.800 |
| 1000 / svg | 3369.200 / 3777.100 | 3330.650 / 3592.000 |

9个场景实际图像/颜色核对全部通过，entity start/end/peak 不增长；Criterion 未检出回退。1000/color batch mean 为176.89 μs，baseline变化区间 −0.3852%..+2.8235%，p=0.12；1000/svg 为3335.3 μs，变化区间 −0.3471%..+1.9880%，p=0.18。无统一时间预算，因此结论仅为上述负载未检出 CPU regression，不代表任意规模、GPU、真实输入 latency、allocation 或长期 memory 已验收。

原始有效 artifact 位于 target/benchmark/icon-update-criterion-1790955971167-8572（baseline）、icon-update-criterion-1790956045433-10476（最终比较）、icon-update-criterion-1790956085439-17616（排队初始 baseline）。每个目录保留 environment/source snapshot/executable hash、samples.csv 的 min/max、latencies.jsonl 与 status.json；Criterion named baseline 位于 target/criterion/icon-update-criterion/。执行日志为 target/stage12-17-icon-fixed-before.log、stage12-17-icon-fixed-after.log、stage12-17-icon-queued-final.log。

| 排队输入的初始 baseline | median / P95 μs |
| --- | ---: |
| 10 / color | 137.600 / 175.700 |
| 10 / svg | 172.600 / 213.700 |
| 100 / color | 122.900 / 149.200 |
| 100 / svg | 434.050 / 510.600 |
| 1000 / color | 233.750 / 274.100 |
| 1000 / svg | 3385.450 / 3838.700 |

6个排队场景全部完成实际 projection 核对，entity 数量保持不变；这些结果是新入口的初始基线，不将它们与立即入口视为相同的计时边界。

## 无效测量与调查

最初 fixture 移除了 preload Icon，释放 source SVG 后可能重新加载，未证明 replacement workload；增加实际 projection 核对后明确失败。因此此前所有 SVG 样本弃用，artifact icon-update-criterion-1790955767050-20976 为 failed；这一失败轮还与 workspace build 重叠，整轮不用于结论。上表全部来自保留两个 preload root 后重新取得的独立 baseline / 最终比较。

初版 immutable Component 的整块 clone/reinsert 在1000/color 出现明显回退，已移除该实现。现行规则要求公开只读查询，不要求整个 Component immutable；库内写入私有 field，并保留 entity API 校验，同值不标记 changed。小型颜色提交路径 inline，错误诊断 cold。最终比较没有保留该回退；不将不同轮次的耗时下降直接归因于 API 改动。
