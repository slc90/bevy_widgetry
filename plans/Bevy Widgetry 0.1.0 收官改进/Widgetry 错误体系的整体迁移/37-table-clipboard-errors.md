# 保留 Table 复制回调和剪贴板失败的原因链

移除TSV生成过程的字符串式逐格包装，保留回调失败、单元格位置和最终原生剪贴板错误。

## 修改落点与前置能力

涉及：`crates/table/src/clipboard.rs`、`crates/table/src/lib.rs`、`Gallery Table复制格式化回调`。

实际前置为[35 保留 Table 模型与投影请求的行列现场](35-table-model-errors.md)、[36 让 Table 虚拟化与单元格展开完整传播失败](36-table-render-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 格式化与提交两阶段

WidgetryTableCopyFormatter回调需要能传递受管领域失败或保留原类型的外来错误。类型不匹配、缺formatter、不可用row/column/value、容量溢出/分配失败分别保留cell和必要类型/长度。

现有BevyError::error(format!("Table copy cell ..."))改成增加结构化context；selection_tsv/copy_selection的inspect_err不再次记录下层已经发出的ERROR。原生Clipboard::set_text失败保留source及输出规模，不泄露整段剪贴板内容作为唯一现场。

## 不提前写剪贴板

全部字段成功格式化之后才执行原生写入，维持原有提交边界。disabled、编辑中或无有效选择仍Ok(None)/false。放弃复制是否视为实际处理由调用Owner决定，不能格式化失败后返回成功。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 回调返回自定义错误，穿过cell context后仍可downcast且ID不变。
- 一个字段失败不修改剪贴板；原生写入失败保留其原source。
- 正常无选择/disabled、引号换行制表符转义和大小边界回归通过。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_table
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步复制formatter返回契约与原子提交覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[36 让 Table 虚拟化与单元格展开完整传播失败](36-table-render-errors.md)；后一项：[38 区分 Table 编辑拒绝与编辑基础设施失败](38-table-edit-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
