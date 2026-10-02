# Widget API 迁移的 Gallery startup 验证

## ListView

2026-10-02，比较 2b0817f 与其后的 ListView working-tree snapshot。此次变更包括只读 state、程序 selection / clear 通知，以及保留静默初始化的 ComboBox composition 适配、Gallery 清空按钮。测量使用已有 `gallery/benches/startup.rs`，不新增性能 SLA，不声明性能改善。

环境为 Windows 10 19045、Ryzen 9 3900X、RTX 4070 Ti SUPER（driver 32.0.15.9579）、Rust 1.98.1，workspace 默认 features、Gallery release / harness bench、未设置 RUSTFLAGS。命令为 `cargo bench -p widget_gallery --bench startup -- --samples 3 --timeout-seconds 60 --port 15983`；基线在独立 2b0817f worktree 运行，使用相同 target 缓存。切回当前源码时 Cargo 曾复用旧 ListView artifact，构建失败且未产生测量样本；清理本地包 release artifact 后完整重建再测量。编译与关闭均不计入 startup，采样时没有并行构建或其他本任务 benchmark。

起点为 harness 发起进程创建前的 monotonic timestamp。终点为默认 Button 页面首次完成 GPU screenshot readback，并确认 47 个可见 Text/Icon 区域具有像素、ButtonNav 可 picking 且 enabled；1920×1080、DX12、desktop_app。5 ms polling / readiness 文件传输与调度误差包含在时间内，GPU readback instrumentation 也包含在该定义内。首次每样本使用新的 App 自有日志 state，后续保留对应 state；OS file cache / GPU driver cache 未控制，不代表完整 cold start。

| 模式 | 基线 3 样本 ms | 当前 3 样本 ms | 基线 / 当前 median ms |
| --- | --- | --- | --- |
| 首次 | 1463.714、827.394、827.264 | 1543.504、815.845、865.740 | 827.394 / 865.740 |
| 后续 | 832.698、837.963、832.573 | 903.986、865.597、849.319 | 832.698 / 865.597 |

全部 12 次进程都 ready 并正常 shutdown，无失败样本。首次范围为基线 827–1464 ms / 当前 816–1544 ms；后续为基线 833–838 ms / 当前 849–904 ms。当前 median 增加约 38 / 33 ms，但仅各3样本且首次启动波动明显，不能判断为可重复、超出环境噪声的 regression，也不能据此声明可靠的 P95 / P99 或性能改善；本次验证证明已测负载下启动成功与成本量级，未证明其他初始页面或完整 cold start。

基线 artifact 保存在 `target/benchmark/stage12-list-view-before-1790944017662-13868`，当前 artifact 为 `target/benchmark/startup-rust-1790944212263-21028`。其中保留 options / environment、源码 snapshot、executable hash、全部 samples、readiness screenshot、stdout / stderr，便于复核；独立 worktree 已清理。

### 扩大采样复核

第一组出现小幅 median 增幅后，以相同配置分别执行 `--samples 8`，仍测量相同生产源码与输入负载；本轮切换版本前明确清理本地包 release artifact，避免混用源码。结果如下（没有丢弃每组较慢的首个进程）：

| 模式 | 基线 median / min–max ms | 当前 median / min–max ms |
| --- | ---: | ---: |
| 首次 | 824.628 / 788.899–1446.944 | 807.388 / 766.252–1439.501 |
| 后续 | 829.619 / 771.695–893.338 | 837.789 / 750.067–860.793 |

本轮全部 32 次启动 ready 且正常 shutdown。首次 median 比基线低约17 ms，后续高约8 ms；第一组约33–38 ms 的增幅未稳定复现，差异在重复测量观察到的波动内，未发现可重复、超出噪声范围的 startup regression。没有已有 startup SLA 可据此判定绝对预算，本次通过当前默认启动场景的对照验证，不声明性能改善或其他负载的性能。两轮共44次成功，仍不据少量样本声明可靠 tail latency。

复核 artifact 为 `target/benchmark/stage12-list-view-before-1790944610743-7192` 和 `target/benchmark/startup-rust-1790944440393-7284`。基线复核目录还保存实际 Gallery release executable；复核用独立 worktree 已清理。

## ComboBox

2026-10-02，比较上述 ListView 迁移后的8样本 snapshot 与 8f4ad7e 上的 ComboBox working-tree snapshot。本次增加程序 selection / clear 的 root Option 通知、执行时目标验证，并迁移 Gallery consumer 和清空按钮；不新增性能 SLA，不声明性能改善。

沿用同一 Rust startup harness、release / bench profile、默认 features、Button 初始页面、1920×1080、DX12 与 desktop_app，以及47个可见 Text/Icon 区域的 GPU readback 和 ButtonNav readiness。命令为 `cargo bench -p widget_gallery --bench startup -- --samples 8 --timeout-seconds 60 --port 15983`。环境与上节相同，编译、工具请求和 shutdown 排除在计时之外，5 ms polling 与 readiness 传输误差包含在测量内。首次每样本使用独立 App 日志 state，后续保留该 state；OS file cache 与 GPU driver cache 未控制，不代表完整 cold start。采样时没有并行构建、另一份 Gallery 或本任务 benchmark。

| 模式 | 第09项 baseline median / min–max ms | ComboBox 第一轮 Review 前 median / min–max ms |
| --- | ---: | ---: |
| 首次 | 807.388 / 766.252–1439.501 | 821.055 / 697.885–1378.473 |
| 后续 | 837.789 / 750.067–860.793 | 779.666 / 684.849–979.972 |

Review 前16次测量均 ready 且正常 shutdown，没有失败样本，较慢的首个进程及全部波动均保留。首次 median 增加约14 ms，后续降低约58 ms；两组范围重叠，本次对照未发现超出已观察波动的明显 startup regression，不能据此声明性能改善或可靠 P95 / P99。没有已有绝对 startup SLA；本次结论限于默认启动场景，不覆盖其他初始页面或完整 cold start。

测量前的两次执行没有产生 startup 样本：首次编译复用了旧 ListView release artifact，缺少当前 set_active / clear_selection；清理相关本地包后，第二次 harness 的旧 test_utils artifact 仍嵌入已删除 baseline worktree 的 CARGO_MANIFEST_DIR，Artifact::new 在创建测量目录前失败。dep-info 证实该路径；清理 test_utils release artifact 后完成本次测量，未为缓存问题修改生产源码。原始失败分别保留在 `target/stage12-10-startup-build-failed.log` 与 `target/stage12-10-startup-harness-failed.log`。

baseline artifact 为 `target/benchmark/startup-rust-1790944440393-7284`，Review 前版本为 `target/benchmark/startup-rust-1790945962230-17484`。其中保存源码 patch、environment / options、executable hash、全部 samples、readiness screenshot 和进程日志；成功执行的完整输出为 `target/stage12-10-startup.log`。

### Review 修复后的最终版本

独立 Review 发现内部 event observer 的重入程序请求可能令 root 通知倒序。最终版本增加私有待转发 payload 队列，后续请求先派发旧通知并执行其 observer commands，再验证当前 source-local id。本次修复不新增 selection authority；按相同条件再次测量8组首次 / 后续启动，保留上述中间版本结果。

| 模式 | 第09项 baseline median / min–max ms | 最终版本 median / min–max ms |
| --- | ---: | ---: |
| 首次 | 807.388 / 766.252–1439.501 | 726.153 / 656.910–1227.949 |
| 后续 | 837.789 / 750.067–860.793 | 709.210 / 699.230–812.567 |

最终16次测量全部 ready 且正常 shutdown，无失败样本，本次重新构建也无失败。median 均低于 baseline，未观察到 startup regression；中间与最终两轮存在明显环境波动，不将耗时下降归因于此次通知修复，不声明性能改善、可靠 tail latency、其他初始页面或完整 cold start。最终 artifact 为 `target/benchmark/startup-rust-1790947216213-16452`，完整执行日志为 `target/stage12-10-startup-final.log`，计时、readiness、cache 控制和预算边界与上述场景相同。

## Table

2026-10-02，Table 的初始 width override consumer 从公开 Map mutation 迁移为 layout builder 后，执行同一默认 Button page startup harness：cargo bench -p widget_gallery --bench startup -- --samples 8 --timeout-seconds 60 --port 15983。仍为1920×1080、DX12、desktop_app，进程创建前开始计时，47个真实 Text/Icon region 与交互 readiness、5 ms polling；首次恢复每样本 App 日志 state，后续保留该 state，OS/GPU cache 未控制。

| 模式 | ComboBox 最终 baseline median / min–max ms | Table median / min–max ms |
| --- | --- | --- |
| 首次，8样本 | 726.153 / 656.910–1227.949 | 709.130 / 654.199–1223.827 |
| 后续，8样本 | 709.210 / 699.230–812.567 | 683.006 / 643.999–748.136 |

全部16次 ready 且正常 shutdown，无失败样本。此次差异在已观察到的运行波动范围内，未观察到默认场景的 startup regression；不声明性能改善、绝对 SLA、可靠 P95/P99、其他初始页面或完整 cold start。artifact 为 target/benchmark/startup-rust-1790949793369-10736，执行日志为 target/stage12-12-startup.log；Column gesture 的独立 CPU 场景见 [Table resize 验证](widget-api-table.md)。
