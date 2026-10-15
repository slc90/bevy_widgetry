# 按职责拆分 TextField 的 Scene、输入与样式

## 目标与范围

重组 `crates/text_field/src/style.rs`：TextField／ReadOnlyTextField Scene 放入 `text_field.rs`，输入过滤与 Focus Observer 放入 `input.rs`，样式解析与输出留在 `style.rs`，现有 `colors.rs` 保留。

## 修改设计

拆分依据是职责混合，不是单纯超过某个行数。Plugin 装配放在明确的注册边界，避免拆出 Scene 和输入后仍把所有组装塞回样式模块。`lib.rs` 保持声明、重导出与必要装配；公开名称和使用方式不变，不新增笼统的 `widget.rs`。

TextFieldBase、ReadOnly 等内部标记只开放到实际需要的内部模块范围，不为移动测试而改成外部公开。Scene 构造仍使用 BSN 与一次性 Props，不另存一份文本状态，也不增加新的构造器体系。

保留 Disabled 过滤 pending edit／paste 的时机，ReadOnly 保留导航、Selection 与 Copy、阻止 Cut／Paste／Insert／删除／IME 写入的区别，以及 pointer focus 和 AcquireFocus 的传播处理。输入系统仍在正确的 EditableTextSystems 边界执行，样式仍处于 `WidgetryUiSystems::Colors`，不改变首次构造和主题更新时机。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

现有 Scene、ReadOnly、Disabled、Focus 和颜色回归在新模块组织下成立。重点确认禁用期间旧请求不会在重新启用后执行，样式变化不重置 EditableText、Caret 或 Selection。Native IME 与 Clipboard 的 OS 端行为不能仅凭 Headless 测试宣称已验证。

保留并用英文表达必要的 Focus 原因注释，删除纯职责说明；不借此次内部拆分改变外部 API 或功能文档的行为契约。

## 预期产出

模块名与职责一致、公开行为不变的 TextField 实现。

## 与前后方案的关系

顺序上接 [方案 06：移除三个 Plugin 内重复的 UI 基础注册](06-移除三个-Plugin-内重复的-UI-基础注册.md)。

完成后进入 [方案 08：把 Table 样式职责从 projection 迁回 style](08-把-Table-样式职责从-projection-迁回-style.md)。

模块整理先于 TextField 注释清理与后续 FileDialog 调整；先保持行为，再处理后续性能问题。
