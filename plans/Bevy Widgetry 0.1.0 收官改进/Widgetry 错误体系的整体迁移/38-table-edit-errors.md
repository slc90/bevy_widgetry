# 区分 Table 编辑拒绝与编辑基础设施失败

把用户校验拒绝保留为业务结果，同时让编辑器映射、回调和提交协议的真正失败携带完整诊断。

## 修改落点与前置能力

涉及：`crates/table/src/editing.rs`、`crates/table/src/lib.rs`、`Gallery注册的Table编辑器与回执处理`。

实际前置为[26 为 TextField 内容与编辑器错误保留映射](26-text-field-errors.md)、[35 保留 Table 模型与投影请求的行列现场](35-table-model-errors.md)、[36 让 Table 虚拟化与单元格展开完整传播失败](36-table-render-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 两个结果维度

当前编辑read回调Result<V,String>同时容易承载普通输入拒绝和基础设施失败。改为明确区分有效值/业务拒绝结果与真正的失败载体：Rejected(String)用于可展示的业务校验信息，真实缺结构/类型冲突/回调故障通过外层领域错误传播。

保持WidgetryTableEditResult::Accepted/Rejected的业务语义；错误label可继续保存显示字符串，但不得把它当作唯一诊断保存处。request_id、view、cell、editor和当前提交阶段进入真正失败的context。

## 请求与回执

begin/submit/resolve的inspect_err不能重复api_error已经记录的源头。过期回执和合法取消按现有协议处理，不因统一错误而成为异常；真正协议破坏与不存在目标API请求分开。

继续保持pending时禁止重复提交、先通知再接受结果、editor与popup清理、输入焦点规则。单纯显示“提交失败”不自动意味着技术错误已处理；明确取消或恢复到可编辑状态后才记录DEBUG。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 普通无效输入产生Rejected显示而非ERROR；编辑器结构故障产生领域诊断和原source。
- 过期回执/正常取消无伪错误，非法请求和类型错配包含view/cell/request_id。
- 接受/拒绝/重复提交/关闭的状态与焦点不变量保留，跨用户回调source不丢失。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_table
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步编辑回调协议、拒绝状态和错误覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[37 保留 Table 复制回调和剪贴板失败的原因链](37-table-clipboard-errors.md)；后一项：[39 保留原生窗口与渲染初始化的失败现场](39-native-window-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
