# 让 ListView 数据与选择请求使用领域失败

先稳定列表Model、数据适配和程序化选择的错误契约，供后续虚拟化及组合控件继承。合法索引查询缺失仍保持Option/false。

## 修改落点与前置能力

涉及：`crates/list_view/src/model.rs`、`crates/list_view/src/source.rs`、`crates/list_view/src/selection.rs`、`crates/list_view/src/projection.rs`、`crates/list_view/src/view.rs`、`crates/list_view/src/registration.rs`。

实际前置为[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)、[03 让测试能观察完整错误生命周期](03-diagnostic-test-support.md)、[20 区分 ScrollArea 的布局等待与结构失败](20-scroll-area-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 稳定ID与请求字段

ListModel revision/item ID耗尽和插入越界保留index、len、item_id、revision/next_id；不再仅创建字符串BevyError。get_mut的Ok(None)、get/remove的Option、既有合法move失败false等维持契约，不把所有缺失统一改成Err。

source配置、模型类型不匹配、非法选择ID/索引、重复投影身份等实际请求失败保留view、source、key类型及相关项。自定义WidgetryListDataSource的Option契约保持其正常缺失语义，由确认Owner的上层判断是否违反结构约束。

## 公共签名与调用者

普通Model和选择API返回领域诊断；deferred方法只在Commands返回处转换。ListKey保持Copy/Eq/Hash等要求，不能把诊断数据塞进key。Tree、ComboBox、FileDialog、Waveform与Gallery的直接类型调用同次更新，不能以临时to_string兼容旧签名。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- ID/revision耗尽与插入越界都有结构化字段，并保持失败后Model/索引映射不变。
- 正常缺失、空Model、重复/清空选择和合法guard没有伪错误。
- 自定义source适配和不同key类型编译，真实API错误通过queued入口保留origin。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_list_view
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 ListModel、DataSource 和选择API说明及对应覆盖模型。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[29 保留 Tooltip 构造与 Popup 失败来源](29-tooltip-errors.md)；后一项：[31 保留 ListView 虚拟化与渲染的完整失败集](31-list-runtime-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
