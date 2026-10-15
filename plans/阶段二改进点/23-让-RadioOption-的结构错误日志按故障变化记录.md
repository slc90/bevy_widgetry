# 让 RadioOption 的结构错误日志按故障变化记录

## 目标与范围

修复 `crates/radio_group/src/option_style.rs::apply` 中父级 ColorState 缺失的日志路径，使它与已有 StyleDiagnostics 和 FailureState 的诊断语义一致。本项不改变样式更新频率、选项结构或选择 API。

## 现状与修改设计

当前父级颜色查找在进入 `FailureState::observe` 前就记录错误并提前返回。即使后面的 Indicator、Dot 校验已经使用去重设施，父级配置持续缺失时仍可能在每次调用中重复记录。问题是局部错误路径绕过了现有机制，不是需要新增日志框架。

把父级配置查找与必要的样式结构校验纳入同一个可观察失败结果，再交给当前 Option 的 FailureState 处理。第一次故障记录必要的中文上下文，相同持续故障不刷屏；真正恢复后记录恢复，再次发生故障仍能报告。日志字段保留 Option 与所属 Group 等实际定位信息，不使用包含变化调用栈的整段错误文本制造新的故障身份。

日志去重不等于错误去重。每次调用的真实失败仍按现有 Error 契约返回并交给宿主处理，不能把持续失败改为 Ok，也不在库内更换全局 Error Handler。多个 Option 独立记录自己的故障，不能用全局标记使一个实例压住另一个实例的错误。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

通过现有 LogCapture、ErrorCapture 和局部测试覆盖持续缺失父级 ColorState、不同故障、恢复及再次故障。重复调用仍返回错误，但同一故障不重复记录本地日志；有效实例继续更新，未曾失败的正常调用不输出恢复信息。

保持正常颜色、优先级、Checked 和 ValueChange 行为不变。依据 `rules/logging.md` 与 `docs/crates/radio_group.md` 更新相关说明，只同步实际改变的错误边界，不改变系统触发和公开状态契约。

## 预期产出

与现有 FailureState 一致、不会因父级配置持续缺失而重复刷屏的 RadioOption 诊断路径。

## 与前后方案的关系

顺序上接 [方案 22：让 ComboBox 的组合样式按变化更新](22-让-ComboBox-的组合样式按变化更新.md)。

完成后进入 [方案 24：让 RadioGroup 选项样式真正由变化驱动](24-让-RadioGroup-选项样式真正由变化驱动.md)。

这是独立的错误路径修复，先于 RadioOption 触发优化完成，避免把两种可分别验收的改动捆在一起。
