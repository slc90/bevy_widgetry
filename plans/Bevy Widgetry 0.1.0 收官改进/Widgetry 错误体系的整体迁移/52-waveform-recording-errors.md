# 为波形文件读写与索引错误保留文件现场

替换recording::error(Display)的字符串转换，保留I/O、格式、索引与生产者失败的原始原因。

## 修改落点与前置能力

涉及：`crates/waveform_file/src/recording.rs`、`crates/waveform_file/src/lttb_index.rs`、`crates/waveform_file/src/lib.rs`、`Gallery录制数据生成与文件打开调用`。

实际前置为[49 为 Waveform 数据契约保留可匹配错误](49-waveform-read-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 文件操作与格式分类

open、read_at、查询、索引读取/构建、write/sync/persist等错误保存path、offset、length、channel、frame/range、算法和操作阶段。解析/serde/转换失败保留实际source；截断、非法布局、数据非有限和输出密度/范围不合法用可匹配领域类别表达。

build_recording的producer失败继承原origin，不能把上层业务错误转成一段文本。Waveform数据契约错误只增加文件语境。

## 发布边界与临时文件

保持完整数据与索引写完再发布目标的规则，保留tempfile和persist_noclobber的不覆盖语义。失败后的临时资源按既有生命周期清理，不能宣布整个外部文件操作自动具有更强事务或崩溃恢复保证。

普通recording API允许返回领域错误，在Gallery/Job拥有者处消费或桥接，不把文件函数直接绑死到ECS。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 不存在/权限/截断/错误格式/索引offset/非法范围分别保留path和原source或具体字段。
- producer失败经过文件构建后ID不变，临时文件不被错误发布为完整目标。
- 完整Replay/MinMax/LTTB读取、边界和写后查询正确性保持原测试。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_waveform_file
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步WaveformFile记录/索引错误和覆盖模型。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[51 保留 Waveform 视图和控制栏的组合失败](51-waveform-view-errors.md)；后一项：[53 保留磁盘 Waveform Reader 跨线程失败](53-waveform-disk-reader-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
