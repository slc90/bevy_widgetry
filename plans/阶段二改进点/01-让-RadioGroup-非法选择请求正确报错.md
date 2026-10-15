# 让 RadioGroup 非法选择请求正确报错

## 目标与范围

修正 `crates/radio_group/src/group.rs::WidgetryRadioGroup::set_selected` 的排队操作，使无效 Root、错误类型、缺失必要层级和越界 Index 按既有文档返回执行期错误。范围包含相关公开契约测试与 `docs/crates/radio_group.md`、`tests/COVERAGE.md` 的必要同步，不改变公开调用方式。

## 现状与修改设计

当前闭包在 Root 不匹配、取不到 Children 或 Index 越界时返回成功；并且会先调用 `initialize_group`，再检查目标 Index。因此，一个无效请求还可能先触发第一项选择。无效操作不是合法同值 No-op，两者不能共享成功分支。

在实际执行 Commands 时完成 Root、固定直接 Children 结构和 Index 的完整校验，然后才执行请求带来的状态修改。无效请求不新增 Initialized、不改动 Checked，也不触发业务通知。不要把排队时校验当作执行期校验：排队之后 Root 或结构仍可能失效。

错误使用 `Severity::Error`，在具有 Root、Index 等定位信息的边界记录中文 ERROR，再交给宿主 Handler；不要主动 panic，也不要由库替换 Handler。合法同值选择仍然成功且静默。RadioGroup 使用官方 Checked，合法程序化选择继续不发送用户选择的 `ValueChange<usize>`，不能借修错增加一套自有 Selection 通知。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

确定性回归覆盖错误类型 Root、已经销毁的 Root、缺失或不合法 Option 结构、越界 Index，以及未初始化 Group 上的越界请求。断言错误发生在队列执行时、严重级别正确，且请求前后 Checked 和初始化状态不被部分修改。

同时覆盖合法同值、合法不同值、Disabled 下合法程序化设置，以及同批队列中其他有效 Group 的操作。使用已有 ErrorCapture、LogCapture 与行为测试，不以 `should_panic` 或捕获 panic 代替错误契约。恢复正常路径后，既有真实输入和初始化选择测试仍应成立。

## 预期产出

一个不再把非法选择伪装成成功的程序化入口，以及保护“先校验、后提交、合法程序化选择静默”的回归用例。

## 与前后方案的关系

这是线性方案链的起点，先补齐有明确证据的非法请求契约。

完成后进入 [方案 02：为 FileDialog Runner 建立正式显示预算判定](02-为-FileDialog-Runner-建立正式显示预算判定.md)。

与相邻增量的联系主要是整体实施顺序，不虚构额外的技术依赖；本项可以在自身范围内完成和验收。
