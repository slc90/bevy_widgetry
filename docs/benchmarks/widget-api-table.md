# Table Widget API 的 resize CPU 验证

2026-10-02，在 0ef0b39 的 Table 实现与本次 working-tree 修改间对照。长期场景在 crates/table/benches/update.rs 的 column_drag：真实公开 Header hierarchy 定位 strip，派发 Pointer DragStart/Drag/DragEnd，flush observer Commands，执行三次完整 headless UI update。每轮 width 往返 20 logical px，避免后续样本退化为 no-op；fixture/字体加载/预热/观测/drop 在计时外。包含 CPU/ECS/text/layout/projection，不包含 OS picking、GPU 或 input 到显示的 latency。

命令：cargo bench -p bevy_widgetry_table --bench update -- column_drag --save-baseline stage12-table-before；之后使用 --baseline stage12-table-before。bench profile、Criterion 0.8.2、20 samples、100 ms warmup、500 ms measurement；额外预热20次、逐次200 samples，CSV 的 P95 仅描述本轮，不声明可靠 P99。

环境：Windows 10 19045、Ryzen 9 3900X、RTX 4070 Ti SUPER、Rust 1.98.1、Workspace 默认 features。环境、source patch、executable hash、原始 latency 与 entity 采样均保存在各 artifact。

| Row / Column / viewport / 内容 | 初始 baseline median ms | 旧代码 control median ms | 最终 median / P95 ms | 最终相对 control |
| --- | ---: | ---: | ---: | ---: |
| 1k / 20 / 646×368 / 普通 | 1.729 | 1.814 | 1.868 / 2.041 | +2.9% |
| 10k / 20 / 646×368 / 普通 | 1.780 | 1.844 | 1.855 / 1.981 | +0.6% |
| 100k / 20 / 646×368 / 普通 | 1.762 | 1.908 | 1.836 / 1.970 | −3.8% |
| 10k / 200 / 646×368 / 普通 | 1.828 | 1.874 | 1.852 / 2.036 | −1.2% |
| 10k / 2000 / 646×368 / 普通 | 1.853 | 1.969 | 1.947 / 2.171 | −1.1% |
| 10k / 20 / 1246×704 / 普通 | 2.728 | 2.936 | 2.884 / 3.063 | −1.8% |
| 10k / 20 / 646×368 / 丰富 | 1.838 | 1.992 | 1.987 / 2.462 | −0.3% |

第一轮与复测的 Criterion 相对最早 baseline 在部分负载提示 regression，因此未直接接受该提示或覆盖 baseline：临时将五个 Table production 源文件换为 0ef0b39 的原实现，以相同新增 harness 运行 control，finally 按精确 bytes 恢复 working tree，再执行最终测量。旧代码也比最早 baseline 慢约5–10%，表明该轮对照存在环境漂移；不将原因进一步归为未经测量的温度或 CPU frequency。最终相对最近 control 的差异为 −3.8%..+2.9%，低于已观察到的同代码波动，未复现超出该波动的代码 regression。所有负载 start/end/peak entity 数一致：普通380、较大 viewport959、丰富440。

没有已确定的绝对 SLA；完成的是上述 CPU 路径的 baseline/对照验证，不代表 GPU、所有交互或其他负载的性能预算通过，也不声明性能改善。

artifact 位于 target/benchmark/，依次为 table-criterion-1790949027112-1688（before）、table-criterion-1790949493259-9128（after）、table-criterion-1790949518799-18116（repeat）、table-criterion-1790949649818-15880（旧代码 control）、table-criterion-1790949715280-8536（final）。命令日志为 target/stage12-12-benchmark-{before,after,repeat,control,final}.log。历史 named baseline 未覆盖。

独立 Review 后增加 DragEnd 最终 width observer Commands flush 和 gesture 有效性检查，再以相同命令执行两轮，artifact 为 table-criterion-1790951314968-18632 与 table-criterion-1790951376072-2828，日志为 target/stage12-12-benchmark-reviewed{,-repeat}.log。上表 final 是 Review 前版本；最终提交版本的数据如下。

| Row / Column / viewport / 内容 | Review 修复后 median / P95 ms | 同版本复测 median / P95 ms |
| --- | ---: | ---: |
| 1k / 20 / 646×368 / 普通 | 1.786 / 1.908 | 1.781 / 2.025 |
| 10k / 20 / 646×368 / 普通 | 1.932 / 2.160 | 1.725 / 1.922 |
| 100k / 20 / 646×368 / 普通 | 1.818 / 1.979 | 1.718 / 1.910 |
| 10k / 200 / 646×368 / 普通 | 1.989 / 2.288 | 1.768 / 2.007 |
| 10k / 2000 / 646×368 / 普通 | 1.960 / 2.114 | 1.843 / 2.029 |
| 10k / 20 / 1246×704 / 普通 | 2.914 / 3.286 | 2.934 / 3.331 |
| 10k / 20 / 646×368 / 丰富 | 1.980 / 2.137 | 1.864 / 2.132 |

第一轮 200 列 median 比旧代码 control 慢约6.1%，同版本复测则比 control 快约5.7%，未复现该退化；两轮 entity 数仍稳定。结果支持本场景未观察到可重复的代码 regression，但有明显环境波动，不将单次快慢解释为改善或预算通过。
