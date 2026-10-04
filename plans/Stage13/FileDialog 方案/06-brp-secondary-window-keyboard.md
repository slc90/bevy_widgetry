# 06 · GUI 前置：BRP 独立窗口 keyboard 支持

## 本阶段目标

解决当前 BRP v0.2.1 只能把 keyboard 送往 PrimaryWindow 的工具缺口。

## 范围与输出

**范围：**slc90/bevy_brp fork 中 keyboard 的 target 参数、MCP/runtime 贯通、timed release/typing 与回归验证，以及对应 Widgetry 工具/依赖版本对齐。外部仓库实施需单独执行或授权。

**完成时应有：**向后兼容的可选 window 参数、secondary 输入成功与 stale target 回归证据、runtime/MCP 实际匹配的 revision/tag。

## 承接关系

**输入：**05 的独立窗口场景，以及已核对的 BRP v0.2.1 mouse/screenshot/keyboard contract。

**前序：**[05 · Gallery：接入新 crate 并移除 rfd](05-gallery-migration-rfd-removal.md)。

**交付给后序：**只有目标窗口输入能力真实可用后，07 才能把独立弹窗 keyboard case 标为通过。未完成时保留明确阻塞。

**下一阶段：**[07 · GUI 测试：BRP 用户场景与 temporal 验收](07-brp-gui-acceptance.md)。

## 目标、来源与当前事实

本次新增 crates/file_dialog，package 为 bevy_widgetry_file_dialog，提供 Bevy 自绘、可复用、non-blocking 的文件选择 Widget。最终由 bevy_widgetry::file_dialog 导出，并替换 Gallery 的 native FileDialog 示例；不是在 rfd 外再包一层，也不引入 egui runtime。

本方案依据 2026-10-04 读取的 bevy_widgetry commit ea9378a2917672143f68b129343a340e0b27f3d2，以及 egui-file-dialog 0.15.0 commit 42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e。当前 workspace 使用 Bevy =0.19.1，BRP runtime 使用 slc90/bevy_brp 的 v0.2.1。开始实施时核对本地差异，不能用本方案覆盖用户已有修改。[R25][R30]

此前 rfd 与 light-file-dialog 的实验均未解决启动延迟，这是用户提供的实验结论；本方案没有重新运行那些实验，也不把“延迟来自 Show 内部”进一步写成已证明的 Windows 内部根因。当前远端源码仍是 rfd 实现。[R22]

用户提到的 WidgetTryXXX 与现有源码拼写不同。本方案按仓库已建立的 WidgetryXXX 前缀命名，例如 WidgetryFileDialog、WidgetryFileDialogPlugin；不另建 WidgetTryXXX 别名，也不重命名既有 Widget。[R12][R17][R26]

以下章节明确区分“已核对的现有能力”和“本次新增 contract”。新 type、调度策略和数值预算是本方案设计，不是声称仓库已经实现。150 ms 是待验证的交付目标，不是目前已经达到的性能。

## BRP 前置条件：补齐独立窗口 keyboard target

已核对当前 slc90/bevy_brp v0.2.1：mouse API 有可选 window Entity，screenshot 可以指定 camera/entity；send_keys 与 type_text 的实现及测试则把 KeyboardInput.window 固定为 PrimaryWindow。仅把 FileDialog UI focus 移到新窗口不足以修复这个输入 target。[R31][R32]

因此完整的独立弹窗 keyboard BRP 验收有一个明确前置工作：在 bevy_brp fork 增加可选 window 参数，并贯通 MCP schema/参数、运行时 handler、按键 press/release 以及逐字符 typing。省略 window 保持当前 PrimaryWindow 行为，显式无效或已销毁的 window 返回清晰错误，不默默转发给其他窗口。

一次输入序列确定 target 后，延迟 release 与后续字符沿用同一个 window。中途 focus 或主窗口身份变化不能把释放事件发去别处。销毁目标时正确终止 pending 输入和 modifier 状态，不能留下 Ctrl/Shift 卡住。为默认兼容、secondary target、无效 target、销毁中的 timed key/typing 增加 fork 自己的 unit/integration regression，并检查 MCP 到 runtime 的真实参数透传。

runtime 与 MCP 发布到同一个经过验证的兼容 revision/tag，Widgetry 再更新实际 dependency/tool 版本。这里不预写一个尚未存在的版本号，也不为了这项能力升级整套无关 BRP 功能。

这部分是另一个仓库的明确依赖任务，不属于本次只读规划已经修改的内容。实施需要用户在该 fork 中单独执行或授权；在它完成之前，允许继续完成 Widgetry 的代码与 mouse GUI 检查，但独立窗口 keyboard BRP 验收必须标为阻塞，不能将其删掉或假装已通过。

## BRP 能力验证与不能采用的替代方式

完成 fork 后先在包含 PrimaryWindow 与 secondary Window 的最小 app 验证：mouse window 参数、keyboard 新 window 参数和 screenshot camera 指向同一窗口。使用 live tool discovery 确认实际 schema，不能直接把方案里的参数强塞给旧工具。保留旧客户端省略 window 的回归结果。[R33]

不允许通过临时更换 PrimaryWindow 标记、把 secondary KeyboardInput 伪装成 primary、直接修改 FileDialog selection/EditableText，或修改 TextField 的 window guard 来“通过”GUI 测试。这些做法会掩盖真实多窗口输入问题。

BRP 不负责模拟 OS 输入法候选、跨应用 focus 或真实 native resize 的全部行为。新 window 参数只补输入路由，不把工具能力夸大为所有桌面行为都能自动验证。无法忠实模拟的场景在后续 case 中单列人工记录，不静默退回另一套 Win32/PowerShell 自动化。[R07]

本阶段产出是已验证的 BRP 多窗口 keyboard 能力及版本对应关系。它在执行链中位于 Gallery 接入之后、完整 GUI 验收之前；代码上可独立开发，但本方案只给这一条交付顺序，不把它变成可忽略的分支。

## 核对依据

- [R07 · BRP GUI 与 temporal 验证规则](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/gui-debugging.md)
- [R12 · 现有 Window 能力](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/lib.rs)
- [R17 · 现有单选 ListView contract](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs)
- [R22 · Gallery 现有 rfd 消费路径](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs)
- [R25 · 当前 Bevy 与 BRP 版本、workspace 配置](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml)
- [R26 · 现有 facade re-export](https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs)
- [R30 · egui-file-dialog 版本与依赖声明](https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml)
- [R31 · BRP mouse window 与 screenshot camera 参数](https://github.com/slc90/bevy_brp/blob/v0.2.1/crates/extras/src/lib.rs)
- [R32 · BRP v0.2.1 keyboard 主窗口绑定](https://github.com/slc90/bevy_brp/blob/v0.2.1/crates/extras/src/keyboard.rs)
- [R33 · BRP MCP 调用、发现与结果 contract](https://github.com/slc90/bevy_brp/blob/v0.2.1/docs/mcp.md)

[R07]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/rules/gui-debugging.md
[R12]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/window/src/lib.rs
[R17]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/list_view/src/lib.rs
[R22]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/gallery/src/pages/window/file_dialog.rs
[R25]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/Cargo.toml
[R26]: https://github.com/slc90/bevy_widgetry/blob/ea9378a2917672143f68b129343a340e0b27f3d2/crates/bevy_widgetry/src/lib.rs
[R30]: https://github.com/jannistpl/egui-file-dialog/blob/42b3ebca6e66dcbd2b368ea1b62698c5ecd3c83e/Cargo.toml
[R31]: https://github.com/slc90/bevy_brp/blob/v0.2.1/crates/extras/src/lib.rs
[R32]: https://github.com/slc90/bevy_brp/blob/v0.2.1/crates/extras/src/keyboard.rs
[R33]: https://github.com/slc90/bevy_brp/blob/v0.2.1/docs/mcp.md
