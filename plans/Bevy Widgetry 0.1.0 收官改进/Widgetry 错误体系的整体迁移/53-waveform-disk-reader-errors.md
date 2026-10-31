# 保留磁盘 Waveform Reader 跨线程失败

迁移DiskWaveformSource的Job、请求队列和完成结果，避免从JobOutcome或互斥锁错误重新生成WaveformReadError字符串。

## 修改落点与前置能力

涉及：`crates/waveform_file/src/source.rs`、`crates/waveform_file/src/lib.rs`、`Gallery大型Replay打开/重载与关闭路径`。

实际前置为[12 让后台 Job 的失败跨线程保持原诊断](12-job-outcome-diagnostics.md)、[50 让 Waveform 后台降采样失败保持原任务现场](50-waveform-runtime-errors.md)、[52 为波形文件读写与索引错误保留文件现场](52-waveform-recording-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 请求结果与源头

open/stats/read_reduced中的锁失败、提交失败、工作终止和查询失败保留source及path、reader/request ID、generation、range和job_id。typed Recording/Job失败沿同一origin传递，UI线程不替工作线程重新捕栈。

Slot.result、JobOutcome和Pending/Ready身份必须分开。请求被新代次替换后不再污染当前结果；已经产生的真实错误被明确废弃才记DEBUG，正常取消不能先生成ERROR。

## 线程停止与回收

Drop取消工作不自动进入Bevy Handler。需要观察结果的关闭放在显式拥有者路径；最后释放仍按当前线程/condvar协议完成，不panic、不只日志而遗留无限存活任务。

异常锁恢复与取消失败分别说明采取的实际措施；不能把请求已发送当作线程已经退出。与Job真实收割机制配合，不新建另一套线程管理。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 工作线程中的I/O错误到WaveformRuntime仍保持ID、source与origin位置。
- 过期错误不覆盖新请求；Pending和取消无伪ERROR。
- reader panic/线程失败不会永久卡Pending；打开重载/销毁后Job和队列生命周期回归通过。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_waveform_file
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步DiskWaveformSource请求/线程错误与覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[52 为波形文件读写与索引错误保留文件现场](52-waveform-recording-errors.md)；后一项：[54 为 Canvas 文档与几何操作建立领域诊断](54-canvas-model-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
