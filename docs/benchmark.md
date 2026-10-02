# Benchmark 用例

用例遵守 rules/benchmark.md。Table、ListView、Tree 使用 Criterion，Gallery startup 使用 gallery/benches/startup.rs。共享 fixture、逐次 latency 采样与 Rust 环境 snapshot 使用已有 test_utils，没有新增 Widget public API。Criterion、WMI、SHA-256 与 JSON 仅服务于测试基础设施；Gallery harness 通过 dev-dependency 使用 test_utils 与 ureq，不进入应用生产依赖。

Icon 的展示输入 CPU benchmark 位于 crates/core/benches/icon_update.rs，同样复用 Criterion / artifact；负载、异步预加载边界与 entity API 迁移比较见 [Icon CPU 验证](benchmarks/widget-api-icon.md)。该场景包含 Icon 的 headless scheduling / image projection，不包含下文 Table/ListView/Tree 的完整 UI fixture 或 GPU。

## 执行

在仓库根目录使用 PowerShell 7，串行执行，测量期间不要同时 build 或运行其他重负载。

```text
cargo bench --locked -p bevy_widgetry_table -p bevy_widgetry_list_view -p bevy_widgetry_tree --bench update -- --save-baseline criterion-20261002
cargo bench --locked -p bevy_widgetry_table -p bevy_widgetry_list_view -p bevy_widgetry_tree --bench update -- visible_mutation --baseline criterion-20261002
cargo bench --locked -p widget_gallery --bench startup -- --samples 5
```

Cargo 先构建全部选定的 Widget bench executable，再串行运行；Gallery 的 Rust harness 另行完成 release executable build 后测量。每个 executable 自动记录独占 target/benchmark/ 目录：environment.json、tracked.patch、新增源码 snapshot 与 executable SHA-256 描述实际版本、working tree、环境、profile 与参数。Widget 保存 samples.csv、latencies.jsonl、status.json；Gallery 保存 build.jsonl、build-stderr.log、options.json、samples.json 与各样本的 stdout/stderr/readiness。build 失败仍保留 diagnostics。完整 artifact 不进入版本控制，baseline 摘要保存在本文末尾。

Criterion HTML 与统计位于 target/criterion/table-criterion/、list_view-criterion/、tree-criterion/；各目录的 report/index.html 为当前汇总。--save-baseline 必须使用新名称，Rust harness 按 Windows 不区分大小写的名称语义拒绝覆盖同 package 的已有 named baseline；--baseline 比较已有结果并保留该 baseline。未显式指定 baseline 时使用本次 artifact 的唯一名称，避免自动更新 base。当前 new/report 文件会由 Criterion 更新，named baseline 保留原始统计。baseline 名称只允许 ASCII 字母、数字、下划线与连字符；保存、比较及加载均拒绝 Criterion 的 new、change、report、profile 及其大小写变体。

单个 crate 可直接运行 cargo bench --locked -p bevy_widgetry_table --bench update；在 -- 后传 Criterion regex filter、--sample-size、--warm-up-time、--measurement-time 等参数。默认 flat sampling、20 个 Criterion batch、100 ms warmup、500 ms measurement；Criterion 根据运行成本调整实际次数，适合作为初始 baseline，正式调查需按波动延长时间并复测。Criterion confidence interval 与 batch 分布不是逐次操作 P95，下面另行保留固定次数采样。参考 [Criterion timing API](https://docs.rs/criterion/0.8.2/criterion/struct.Bencher.html) 与 [baseline 参数](https://criterion-rs.github.io/book/user_guide/command_line_options.html)。

Gallery 不使用 Criterion 自动 warmup / adaptive sampling，以保持每次首次启动的新 App state；使用同一套 Rust artifact 记录，参数还有 --timeout-seconds（默认 60）与 --port（默认 15982）。所有入口均为 Rust / Cargo，不需要 PowerShell benchmark script。

## Widget 负载与计时边界

- Table：Row 1k/10k/100k 固定 20 Column；Column 20/200/2k 固定 10k Row；view 外尺寸 646×368 与 1246×704 分别用于比较 viewport。默认 Cell width 120、row height 28，Header/gutter 使用生产配置。
- ListView：1k/10k/100k String，view 640×384；另外比较 640×768。固定 item height 使用默认生产配置。
- Tree：1k/10k/100k 的 wide hierarchy，包含一个可展开 branch；另有 10k collapsed descendant、1k deep chain；view 640×384，另比较 640×768。hidden node 仍是业务 ECS entity，报告的 entity 总数包含业务 hierarchy。
- normal renderer 包含真实 Text 与 layout shell；rich 增加第二个 Text 和 layout 内容。结论限于这些 renderer，不代表任意自定义 renderer。

所有场景经过真实 BSN、plugin scheduling、App::update、UI layout/text/visibility/picking system，不写入 ComputedNode、私有 viewport 或 projection cache。使用 headless camera 1920×1080、scale factor 1、计时外预加载的内建 Smiley Sans font；没有 native window、render device、GPU 提交或 present。没有安装 tracing subscriber，计时内日志不输出。字体内容与 Gallery 默认字体一致，字体加载排除在 Widget 计时外；日志成本与 Gallery 不同。

first_scene 计时包含 BSN view 构造与三个 App::update，model/plugin/camera setup 排除；不是完整 App startup。idle 测量一次无变化 update。scroll 使用公开 ScrollPosition，在顶部与固定偏移之间往返；这是 CPU update 路径，不是 wheel 输入的端到端 latency。visible/offscreen mutation 每轮修改新内容并 update。resize 与 rebuild 包含三个 update，不能直接与一次 idle update 比较。hidden_idle 从初始 display:none 开始。destroy 包含 view/source（Tree 还包含业务 hierarchy）despawn 与一次 update；最后 App drop 不计入。

Criterion 通过 iter_custom 累加每次真实 operation 的 monotonic duration；setup、entity/Text glyph 观测与 fixture drop 排除。steady 使用一个连续 fixture，index 跨 warmup 与 batch 连续递增；first_scene/destroy 每次 operation 使用独立 fixture，Criterion warmup 只影响 CPU/cache，不将该 Widget lifecycle 当作 App 首次启动。

Criterion 执行后，用新的独立 fixture 再做固定次数观测：steady 预热 20 轮并连续采样 200 轮，报告 median/P95/min/max；P95 仅描述本次运行，不报告 P99。first_scene/destroy 每轮 setup 独立 fixture，20 样本，只报告 median/min/max。latencies.jsonl 保留每次原始 latency。连续 scroll/resize/toggle/rebuild 不增长输入负载，mutation 每轮改变内容，避免样本退化为重复 no-op。Tree collapsed/deep 的短 projection 不运行 scroll，用 wide 负载测量实际 viewport 往返。

entity start/end/peak 观测提供运行前后和采样期间的资源数量；不等于 allocation/heap memory 测量，也不证明无限持续运行有界。Tree 的 expand_collapse 是两个方向交替的组合分布，没有分别报告展开与收起 latency。

目前没有已确定的用户性能 SLA。新增用例建立初始 baseline，记录规模趋势与波动，不自动赋予统一 frame budget 或把采样结果标记为预算验收通过。后续性能修改必须在相同环境/负载/边界下比较，并按具体任务预算判断；GPU、真实输入 latency、allocation 和长期 memory 增长尚未测量。

## Gallery startup

使用 release executable、默认 Button 页与全部常驻 demo model、1920×1080、scale factor 1、DX12 DxgiFromVisual、WinitSettings::desktop_app()；BRP、正常日志配置与 plugin 装配保留。readiness instrumentation 仅在 harness 环境变量存在时启用。

起点为 Rust harness 调用 Command::spawn 前的 Instant monotonic timestamp，终点为观测到原子发布的 ready.txt；只使用 harness 的同一 clock。终点要求 GalleryRoot/ButtonPage/ButtonDemo 已 layout，ButtonNav 是 enabled 官方 Button；可见非空 Text 已有 glyph，Icon 已生成已加载 ImageNode；随后真实 GPU screenshot 的每个内容区域有像素变化。可见性在 VisibilityPropagate 后检查 InheritedVisibility，并排除 display:none 的 ancestor，避免把关闭的 ComboBox Popup 列为必需显示。ready.png 保存该帧作为证据。实际 pointer 交互 readiness 还通过任务的 BRP GUI 验证确认，静态 marker 不单独作为行为证明。

结果包含 screenshot 请求、GPU readback、内容像素检查、PNG 编码、文件发布与 polling 的 instrumentation 开销，是默认主要内容显示并具备交互结构后的启动耗时上界，不能当作精确首次 present 时间。polling 每轮请求 5 ms，实际调度 jitter 可能更大；记录 polling_ms，未校正该误差。在 screenshot 请求、GPU readback 在途和重试空 screenshot 时 request redraw；发布 readiness 后停止，不修改全局 update mode。

first 每个样本使用独立、尚未生成 logs 的 App state；subsequent 保留该 first 实际生成的 logs。Gallery 当前没有持久用户配置或 App 自有磁盘 asset cache，其默认 theme/page 固定；进程内 asset/cache 每次重建，内建 asset 的初始化成本包含在计时内。OS file cache、字体 cache、GPU/driver cache 不清空，不宣称完整 cold start；后续启动结果可能受 first 样本及前序 build 的 cache 影响。

失败、提前退出和 timeout 进入 samples.json，不剔除后只报告成功样本。readiness 后通过真实 BRP shutdown 正常关闭，shutdown 不计入 startup；失败时才强制清理，并把样本标为 failed。5 对样本只支持初始 median/range，不报告 startup tail latency；没有既定 startup SLA，不能宣称预算通过。

## 迁移前初始 baseline

2026-10-02，在 HEAD 64ec02d 加本任务 working tree 上建立初始 baseline；源码 patch、新文件 snapshot 与 executable SHA-256 均保存在 artifact。没有同一 harness 的历史 before 结果，不作性能改善声明，也没有已确定的用户 SLA，因此预算判断为尚未定义。

环境：Windows 10 Pro 19045、Ryzen 9 3900X（12 core / 24 thread）、RTX 4070 Ti SUPER（driver 32.0.15.9579）、Rust 1.98.1 x86_64-pc-windows-msvc、PowerShell 7.6.6。RUSTFLAGS、CARGO_ENCODED_RUSTFLAGS、RUST_LOG 均未设置。Widget 使用 bench profile，Gallery 使用 release profile；默认 features、Cargo.lock --locked。构建与测量串行，未并行执行其他本任务重负载；OS/background scheduling 与 boost 仍可能影响样本。

### Widget baseline

迁移前由 PowerShell harness 执行，保留历史结果。完整 artifact 位于 target/benchmark/widgets-20261002-103009-819/，三个 CSV 分别记录 Table 77、ListView 45、Tree 68 个场景，共 190 个场景、31,160 个计时样本。steady 200 样本并预热 20 轮；first_scene/destroy 各 20 个独立 fixture。下面耗时单位均为 ms，原始 CSV 使用 µs 并保留 min/max、P95 与 entity start/end/peak。

| idle 场景 | median | P95 | 存活 entity（start=end） |
| --- | ---: | ---: | ---: |
| Table 1k Row × 20 Column，646×368 | 0.548 | 0.620 | 380 |
| Table 10k Row × 20 Column，646×368 | 0.562 | 0.635 | 380 |
| Table 100k Row × 20 Column，646×368 | 0.557 | 0.630 | 380 |
| Table 10k Row × 200 Column，646×368 | 0.557 | 0.614 | 380 |
| Table 10k Row × 2k Column，646×368 | 0.582 | 0.644 | 380 |
| Table 10k Row × 20 Column，1246×704 | 0.820 | 0.926 | 959 |
| Table 10k Row × 20 Column，rich | 0.555 | 0.639 | 440 |
| ListView 1k item，640×384 | 0.486 | 0.587 | 202 |
| ListView 10k item，640×384 | 0.712 | 0.882 | 202 |
| ListView 100k item，640×384 | 0.516 | 0.621 | 202 |
| ListView 10k item，640×768 | 0.518 | 0.611 | 238 |
| ListView 10k item，rich | 0.493 | 0.577 | 214 |
| Tree 1k wide，640×384 | 0.787 | 0.895 | 1,291 |
| Tree 10k wide，640×384 | 1.516 | 1.788 | 10,291 |
| Tree 100k wide，640×384 | 12.221 | 12.747 | 100,291 |
| Tree 10k collapsed，640×384 | 1.140 | 1.281 | 10,192 |
| Tree 1k deep，640×384 | 0.691 | 0.785 | 1,192 |
| Tree 10k wide，640×768 | 1.661 | 1.903 | 10,399 |
| Tree 10k wide，rich | 1.558 | 1.823 | 10,303 |

各 workload 的完整规模/viewport/renderer 结果在同目录 CSV；下表概括跨场景的 median 范围，不能将不同负载的极值相减当作 regression。

| workload | 场景数 | median 最小–最大（ms） |
| --- | ---: | ---: |
| first_scene | 19 | 7.571–47.816 |
| idle | 19 | 0.486–12.221 |
| scroll（ListView / Tree） | 10 | 1.158–13.774 |
| scroll_x（Table） | 7 | 1.181–1.945 |
| scroll_y（Table） | 7 | 1.661–2.712 |
| scroll_xy（Table） | 7 | 1.993–3.560 |
| visible_mutation | 19 | 0.651–12.462 |
| offscreen_mutation | 19 | 0.499–12.298 |
| resize | 19 | 1.814–37.198 |
| expand_collapse（Tree） | 7 | 2.652–59.112 |
| hidden_idle | 19 | 0.342–12.047 |
| rebuild | 19 | 2.404–38.346 |
| destroy | 19 | 0.714–27.655 |

固定 viewport 的 Table idle 在分别增长 Row/Column 时约 0.55–0.58 ms，可见 projection 不增长；扩大 viewport 后约 0.82 ms。ListView 的 10k idle 本轮高于 1k/100k，结果不是单调趋势，不能凭单轮将这项波动解释为确定的数据规模成本。Tree wide 从 1k 到 100k 的 idle 由约 0.79 ms 增至 12.22 ms；collapsed/hidden 仍包含业务 hierarchy 成本，virtualized row 数量不代表每次 update 成本与数据规模无关。100k wide Tree 的 first_scene median 为 47.82 ms、destroy 为 27.66 ms，分别排除 model setup 与最终 App drop。

各 steady 场景末尾 entity 数量回到初始值；resize 使用偶数轮往返也回到初始尺寸。destroy 后保留 fixture 的 plugin/camera 等基础 entity。观测使用 Bevy count_spawned，而非 allocated entity index 数量；这是有限轮数的资源观测，未测 allocation、heap、GPU、真实输入 latency 或长期 memory 增长。以上结果建立 baseline，不表示已满足未定义的预算。

### Gallery baseline

迁移前由 PowerShell harness 执行 5 对样本，artifact 位于 target/benchmark/startup-20261002-105942-026/。5 对共 10 次启动全部达到 readiness，并通过 BRP 正常 shutdown，均保存 1920×1080 的 ready.png 与 46 个 Text/Icon region 的 readiness 通知。

| state | 样本数 | median（ms） | min–max（ms） |
| --- | ---: | ---: | ---: |
| first（每次独立的新日志 state） | 5 | 735.777 | 707.047–746.518 |
| subsequent（保留对应 first 的日志 state） | 5 | 733.398 | 705.611–815.233 |

这 5 对样本只建立 median/range，不支持 startup P95/P99，也没有既定 SLA 可作预算验收。结果包含截图请求、readback、像素检查、PNG 编码和 5 ms polling 的通知开销；不是精确首次 present。OS/GPU cache 未清空，且正式运行前已有构建、诊断和 smoke，不能宣称完整 cold start。单对最终 smoke 在 target/benchmark/startup-20261002-105825-859/，first 1305.917 ms、subsequent 719.409 ms；保留其差异，不将其与正式 first 分布混为一组。

旧版本 readiness 的失败及 build 诊断保留在 target/benchmark/startup-20261002-103247-954/、startup-20261002-104449-316/、startup-20261002-104814-307/、startup-20261002-105055-472/ 等目录，不属于最终版本成功 baseline。根因是隐藏的 theme ComboBox Popup 仍有 layout，旧判断将其 Text 误列为必需显示；最终版本按真实 InheritedVisibility 过滤，并为在途 screenshot/readback 按需推进。临时诊断未进入最终代码。

BRP 验收单独位于 target/benchmark/startup-brp-acceptance/：安装同一 instrumentation 的实例完成 readiness，BRP baseline screenshot 显示默认 Button 页；真实 pointer 点击 Text button 产生 Activate 日志，点击 ButtonNav 产生 page=Button 日志，随后正常关闭。该验收不计入上述 startup 结果。headless Widget benchmark 的 scroll/mutation 仍只是公开 Component 更新路径，没有测量这些 Widget 的真实输入 latency。

## Criterion / Rust baseline

2026-10-02 使用上述 Cargo 命令创建 criterion-20261002，环境与迁移前相同，代码仍为 HEAD 64ec02d 加 working tree。全部 190 scenarios 成功：ListView 45、Table 77、Tree 68。每个场景有 20 个 Criterion flat batch，另外有独立的固定次数采样，共 31,160 个逐次 latency 样本。部分较重 fixture 超过默认时间目标，Criterion 自动调整采样次数；计时仍只包含真实 operation。所有 steady 场景 entity start=end，CSV 与 latencies.jsonl 的场景数完全对应。

源码、环境、binary SHA-256 与固定次数采样分别保存于：

- target/benchmark/list_view-criterion-1790911974427-13120/
- target/benchmark/table-criterion-1790912037229-9912/
- target/benchmark/tree-criterion-1790912150304-10512/

Criterion 原始统计与不可覆盖的 criterion-20261002 baseline 位于上述 target/criterion/ 各 package 目录；三份 report/index.html 已生成。以下列出代表性 idle，耗时单位为 ms；Criterion mean 的 95% confidence interval 与逐次 P95 分属不同采样，不能混用。

| 场景 | Criterion mean 95% CI | 独立 median | 独立 P95 |
| --- | ---: | ---: | ---: |
| Table 10k Row × 20 Column，646×368 | 0.545–0.556 | 0.559 | 0.626 |
| ListView 10k item，640×384 | 0.511–0.528 | 0.499 | 0.572 |
| Tree 100k wide，640×384 | 12.379–12.919 | 12.667 | 13.517 |

这次迁移建立 Criterion 初始 baseline，没有修改 Widget 生产路径。保留迁移前数据作为历史观测，不将不同 harness / sampling 的差异作为性能改善或 regression 结论。首次 baseline 没有 Criterion 历史 before；后续通过 --baseline 在相同配置下比较。性能预算仍未定义，GPU、真实输入 latency、allocation 与长期 memory 增长仍未测量。

Rust startup 正式结果位于 target/benchmark/startup-rust-1790912635012-5680/：5 对共 10 次全部达到 46 regions readiness 并正常 shutdown，每次保存 ready.png。first median 705.545 ms（673.014–1335.764），subsequent median 688.302 ms（672.519–864.112）。first 第 1 次的 1335.764 ms 完整保留；OS/GPU cache 未重置，不宣称 cold start 或启动性能改善。另以占用 15982 port 验证失败路径，target/benchmark/startup-rust-1790912764389-21372/samples.json 保留 failed / 明确 port error，未启动或误关其他实例。

三个代表 idle 已验证 --baseline CLI，比较前后的 executable SHA-256 与全量初始 binary 相同；baseline 的 760 个文件 SHA-256 未变，真实 CLI 尝试同名 --save-baseline 明确 exit 1。比较运行 artifact 为 list_view-criterion-1790912765668-19248、table-criterion-1790912767380-2884、tree-criterion-1790912768997-9020。

相同 binary 的短采样仍被 Criterion 报告约 +11% 的变化；延长到 warmup 1 s / measurement 2 s 后，ListView / Table 约 +3.3% / +3.6%，Tree 约 +0.45% 且未检测显著变化。延长采样的 artifact 为 list_view-criterion-1790912892635-10208、table-criterion-1790912896797-15600、tree-criterion-1790912901045-14024，均位于 target/benchmark/。这些观测表明初始短采样对运行条件敏感；没有定位到具体机器噪声来源，也不构成源码修改前后的 regression 结论。保留全部结果与初始 baseline，不覆盖或宣称已消除波动。正式性能决策应对 before / after 使用相同且足够长的配置，并复测稳定性。

独立 Review 后补齐 baseline 大小写别名与内部保留目录防护；变更仅影响计时外的 CLI / artifact 参数检查。原始全量结果保留对应源码 snapshot，不覆盖；修复后另验证代表 steady / lifecycle 及真实非法 CLI，未重跑未受影响的全部负载。

守卫修复后 9 个代表 idle/first_scene/destroy 的原始结果位于 list_view-criterion-1790913607940-11660、table-criterion-1790913614896-20880、tree-criterion-1790913621866-21128。短采样仍有统计变化，属于运行验证，未作为新 baseline 或预算验收。6 个真实 CLI 的精确失败诊断位于 target/benchmark/baseline-guard-validation-1790913724491/，包括 uppercase alias、reserved save/short save/compare/lenient/load；均 exit 1，原 baseline 的 760 个文件 hash 再次确认未改变。

CLI 还覆盖 Clap 的 -s=NAME、-b=NAME、组合短 flags 与 -- 终止语义。真实 -s=syntax-equals-1790914465206 保存结果位于 list_view-criterion-1790914465359-13824；同一 binary、同一默认配置的 -vnb=syntax-equals-1790914465206 比较位于 list_view-criterion-1790914466781-19460，metadata 确认使用指定名称与比较模式，未被自动保存替换。这对控制运行未检测显著变化；原 criterion-20261002 的 760 个文件 hash 仍未变，-vns=NEW 明确拒绝。
