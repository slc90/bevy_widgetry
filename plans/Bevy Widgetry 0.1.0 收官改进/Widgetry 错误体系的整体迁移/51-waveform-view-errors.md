# 保留 Waveform 视图和控制栏的组合失败

迁移Waveform渲染/离屏及控制栏错误，保持数据源、播放时钟和宿主控制职责不变。

## 修改落点与前置能力

涉及：`crates/waveform/src/renderer.rs`、`crates/waveform/src/geometry.rs`、`crates/waveform/src/view.rs`、`crates/waveform/src/controls.rs`、`crates/waveform/src/style.rs`、`crates/waveform/src/time_axis.rs`、`crates/waveform/src/amplitude.rs`、`crates/waveform/src/channels.rs`、`crates/waveform/src/measurement.rs`、`crates/waveform/src/colors.rs`。

实际前置为[08 使离屏目标失败保留资源归属与清理结果](08-offscreen-errors.md)、[13 统一 Button 状态与结构失败的诊断](13-button-loading-errors.md)、[15 使 ToggleButton 的目标与样式失败可识别](15-toggle-button-errors.md)、[25 为 Slider 数值契约建立领域诊断](25-slider-errors.md)、[32 让 ComboBox 组合失败继承下层诊断](32-combo-box-errors.md)、[50 让 Waveform 后台降采样失败保持原任务现场](50-waveform-runtime-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 自己的错误与组合来源

Root、离屏target、camera、channel/grid/mesh和控制部件的实际结构或配置失败保留实体、通道/范围、尺寸和操作阶段。Offscreen/Core、ComboBox/ListView、Slider、ToggleButton等错误只补Waveform上下文，不重建origin。

播放/倍率/定位意图仍由宿主接管；不能为了日志改变时钟或拖动恢复策略。正常无source、隐藏、零viewport、Pending、停用或不适用测量输入保持原状态，不人工制造异常。

## 渲染失败与资源清理

Scene/渲染工作循环保留所有未处理诊断。清理只属于本次资源，不能将回收新mesh/实体误记为整个数据操作已处理。样式Owner路径同时迁移，防止剩余字符串BevyError绕开公共桥。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 损坏render target/控制栏部件/实际配置时source链和对象现场完整。
- 无source/未布局/停用等正常状态没有ERROR。
- Live、Replay、通道/幅值/时间切换、测量与播放请求行为回归通过；GUI/BRP验证真实组合。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_waveform
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步Waveform视图错误及跨控件组合覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[50 让 Waveform 后台降采样失败保持原任务现场](50-waveform-runtime-errors.md)；后一项：[52 为波形文件读写与索引错误保留文件现场](52-waveform-recording-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
