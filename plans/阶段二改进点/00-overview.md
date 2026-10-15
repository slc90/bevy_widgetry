# 修改方案总览

围绕现行规则与行为契约，修复已确认的接口问题、清理重复维护点、减少无变化状态下的工作，并补齐共享 Model 展示与 Gallery 使用边界。下列 45 个增量保留原待办中本轮实施的 29 项内容及新增审查发现。

按 01 → 02 → … → 45 的唯一顺序执行。每篇正文包含自身的设计、边界和完成条件；测试与必要文档同步归入对应改动，不设置只有最终验收的阶段。

| 顺序 | 小方案 | 本项交付 |
| --- | --- | --- |
| 01 | [让 RadioGroup 非法选择请求正确报错](01-让-RadioGroup-非法选择请求正确报错.md) | 修正 `crates/radio_group/src/group.rs::WidgetryRadioGroup::set_selected` 的排队操作，使无效 Root、错误类型、缺失必要层级和越界 Index 按既有文档返回执行期错误。 |
| 02 | [为 FileDialog Runner 建立正式显示预算判定](02-为-FileDialog-Runner-建立正式显示预算判定.md) | 在 `gallery/benches/file_dialog_run.py` 增加明确的正式验收模式，并同步 `file_dialog.py` 的结果读取契约和 `file_dialog.md`。 |
| 03 | [将内部日志 crate 重命名为 internal_log](03-将内部日志-crate-重命名为-internal_log.md) | 把 `crates/log/` 和 `bevy_widgetry_log` 统一改为 `crates/internal_log/`、`bevy_widgetry_internal_log`。 |
| 04 | [为 Facade 控件建立真正的可选编译边界](04-为-Facade-控件建立真正的可选编译边界.md) | 调整 `crates/bevy_widgetry/Cargo.toml`、`src/lib.rs` 和受影响的 Facade 测试、Benchmark 目标，为公开控件建立 optional dependency、Cargo Feature 和条件导出的一一对应关系。 |
| 05 | [通过可选 app_logging Feature 公开宿主日志能力](05-通过可选-app_logging-Feature-公开宿主日志能力.md) | 在 Facade 中增加独立 `app_logging` Feature 和对应模块，导出已有 `AppLogging`、`file_layer`、`terminal_layer`、`WorkerGuard`。 |
| 06 | [移除三个 Plugin 内重复的 UI 基础注册](06-移除三个-Plugin-内重复的-UI-基础注册.md) | 删除 CheckBox、RadioGroup、Tooltip 的同一个 `build()` 中重复出现的 `WidgetryUiPlugin` 条件注册。 |
| 07 | [按职责拆分 TextField 的 Scene、输入与样式](07-按职责拆分-TextField-的-Scene、输入与样式.md) | 重组 `crates/text_field/src/style.rs`：TextField／ReadOnlyTextField Scene 放入 `text_field.rs`，输入过滤与 Focus Observer 放入 `input.rs`，样式解析与输出留在 `style.rs`，现有 `colors.rs` 保留。 |
| 08 | [把 Table 样式职责从 projection 迁回 style](08-把-Table-样式职责从-projection-迁回-style.md) | 整理 `crates/table/src/projection.rs` 与 `style.rs`，迁移 `update_colors`、`style_shell`、主题刷新及其样式辅助逻辑。 |
| 09 | [清理库入口和 Gallery 入口的重复功能说明](09-清理库入口和-Gallery-入口的重复功能说明.md) | 清理各库 `src/lib.rs` 和 `gallery/src/main.rs` 顶部已经由 `docs/crates/`、`docs/gallery.md` 承载的功能概述、职责列表、使用边界和重复介绍。 |
| 10 | [清理集成测试文件头的重复覆盖模型](10-清理集成测试文件头的重复覆盖模型.md) | 清理现有 Integration Test `.rs` 文件头的 Coverage Model、Coverage Map、States／Stimuli／Guards 等说明，统一由所属 crate 的 `tests/COVERAGE.md` 维护。 |
| 11 | [规范公共基础设施和工程配置的注释](11-规范公共基础设施和工程配置的注释.md) | 处理 Core、Theme、Asset、Cursor、internal_log、app_logging、app_icon_build、test_utils、Facade 及根级工程配置中的现存注释，范围只限注释规则适配。 |
| 12 | [规范基础交互控件的注释](12-规范基础交互控件的注释.md) | 处理 Button、CheckBox、RadioGroup、TextField、ComboBox、Tooltip 和 ScrollArea 的生产代码、单元测试及其局部辅助代码中的现存注释。 |
| 13 | [规范数据视图的注释](13-规范数据视图的注释.md) | 处理 ListView、Tree、Table 的 Model、投影、渲染、交互、样式以及相应测试和 Benchmark 源码中的注释。 |
| 14 | [规范窗口、对话框和波形运行时的注释](14-规范窗口、对话框和波形运行时的注释.md) | 处理 Window、MessageBox、FileDialog、Waveform 的生产代码、测试与 Benchmark 注释，不改变其业务或运行机制。 |
| 15 | [规范 Gallery 和测量脚本的注释](15-规范-Gallery-和测量脚本的注释.md) | 处理 Gallery 页面、国际化、素材、启动与 Benchmark 支持代码，以及相关 Python／配置说明中的代码注释。 |
| 16 | [删除只复述常量字面值的测试](16-删除只复述常量字面值的测试.md) | 以 `crates/theme/src/common.rs::new_tokens_match_approved_rgba` 和 `crates/theme/src/button.rs` 的固定色板比较为明确入口，清理全仓同类“常量等于另一份硬编码字面值”的测试。 |
| 17 | [让 RadioGroup 固定结构诊断响应实际变化](17-让-RadioGroup-固定结构诊断响应实际变化.md) | 调整 `crates/radio_group/src/group.rs::initialize` 中已初始化 Group／Option 的结构诊断触发方式。 |
| 18 | [让 ScrollArea 滚动条配色按变化更新](18-让-ScrollArea-滚动条配色按变化更新.md) | 调整 `crates/scroll_area/src/style.rs::update_thumb_style` 及相应调度，只减少 Track／Thumb 的重复配色工作，不修改滚动、测量或几何机制。 |
| 19 | [让 ListView 容器和可见行配色按变化更新](19-让-ListView-容器和可见行配色按变化更新.md) | 调整 `crates/list_view/src/style.rs::update<T>` 和必要的投影／状态变化连接，消除静止 View 对全部可见 Row 的重复颜色解析。 |
| 20 | [补齐 ListView 共享 Model 的双 Renderer 展示](20-补齐-ListView-共享-Model-的双-Renderer-展示.md) | 扩展 `crates/list_view/tests/behavior.rs` 等相关回归，在 `gallery/src/pages/list_view.rs` 的当前模块组织中增加并排 Shared Model 场景。 |
| 21 | [让 ComboBox 的样式归属只在必要时建立](21-让-ComboBox-的样式归属只在必要时建立.md) | 调整 `crates/combo_box/src/style.rs::establish_owners<T>`、`own_field<T>`、`own_list<T>` 与 Typed Plugin 注册，去掉每帧扫描全部 ComboBox 层级的永久兜底。 |
| 22 | [让 ComboBox 的组合样式按变化更新](22-让-ComboBox-的组合样式按变化更新.md) | 优化 `crates/combo_box/src/style.rs::update<T>` 的触发和作用范围。 |
| 23 | [让 RadioOption 的结构错误日志按故障变化记录](23-让-RadioOption-的结构错误日志按故障变化记录.md) | 修复 `crates/radio_group/src/option_style.rs::apply` 中父级 ColorState 缺失的日志路径，使它与已有 StyleDiagnostics 和 FailureState 的诊断语义一致。 |
| 24 | [让 RadioGroup 选项样式真正由变化驱动](24-让-RadioGroup-选项样式真正由变化驱动.md) | 调整 `crates/radio_group/src/option_style.rs::update_changed` 和相关失效传播，处理选项样式的重复计算。 |
| 25 | [让 Tooltip 只在配色变化时刷新外观](25-让-Tooltip-只在配色变化时刷新外观.md) | 调整 `crates/tooltip/src/style.rs::update_tooltip_colors`，只优化 Popup 的 Background、Border 和 Foreground 同步。 |
| 26 | [让 Tree 的样式归属随构造和层级变化维护](26-让-Tree-的样式归属随构造和层级变化维护.md) | 调整 `crates/tree/src/view.rs::establish_style_owners`、`own_list`、`own_expander` 和相关注册，消除静止 Tree 对全部内部 ListView 与 Expander 的永久所有权扫描。 |
| 27 | [合并 Tree View 的重复结构诊断](27-合并-Tree-View-的重复结构诊断.md) | 调整 `crates/tree/src/registration.rs` 中专门的 Source／Hierarchy 检查，以及 `view.rs::validate_sources`、`validate_hierarchy` 的触发。 |
| 28 | [让 Tree 列表和展开按钮配色按变化更新](28-让-Tree-列表和展开按钮配色按变化更新.md) | 调整 `crates/tree/src/view.rs::update_colors` 和必要的 Model／投影失效连接，停止每帧重新解析所有 TreeView 与可见 Expander 的颜色。 |
| 29 | [补齐共享 TreeModel 的双 View 展示](29-补齐共享-TreeModel-的双-View-展示.md) | 扩展 `crates/tree/tests/view.rs` 等相关回归，并在 Gallery Tree 页面增加并排 Shared TreeModel 示例。 |
| 30 | [让 Table 可见区域样式按变化更新](30-让-Table-可见区域样式按变化更新.md) | 在样式职责迁移后的 `crates/table/src/style.rs` 中优化 `update_colors<T>` 及相关区域更新。 |
| 31 | [补齐 Table 共享 Model 的独立布局展示](31-补齐-Table-共享-Model-的独立布局展示.md) | 在 Table 相关测试和 `gallery/src/pages/table.rs` 的当前组织中增加 Shared Model 双 View 场景，展示同一个 `WidgetryTableModel<DemoRow>` 下的不同 `WidgetryTableLayout`、列宽和样式。 |
| 32 | [让 Window 标题栏与系统按钮配色按变化更新](32-让-Window-标题栏与系统按钮配色按变化更新.md) | 调整 `crates/window/src/style.rs::update_colors` 和 `apply_owned_window_colors` 的调度与失效来源。 |
| 33 | [为 Window 提供稳定的标题文字绑定与更新入口](33-为-Window-提供稳定的标题文字绑定与更新入口.md) | 在 `crates/window/` 建立由控件负责的标题文字语义绑定和更新操作，并让 MessageBox、FileDialog 的库内默认标题构造使用这一绑定。 |
| 34 | [让 Gallery 标题国际化使用 Window 语义接口](34-让-Gallery-标题国际化使用-Window-语义接口.md) | 替换 `gallery/src/i18n.rs::refresh_window_titles` 的内部层级和旧文本匹配逻辑，使用 Window 的标题语义能力。 |
| 35 | [让 MessageBox 的样式归属按生命周期建立](35-让-MessageBox-的样式归属按生命周期建立.md) | 调整 `crates/message_box/src/style.rs::establish_owners`、`own_window`、`own_action` 和构造连接，去除稳定 MessageBox 每帧递归扫描全部后代的兜底。 |
| 36 | [让 MessageBox 的组合配色按变化更新](36-让-MessageBox-的组合配色按变化更新.md) | 调整 `crates/message_box/src/style.rs::update_colors` 与 owned 配色路径，减少 Window、Body 和结果按钮的重复颜色解析。 |
| 37 | [分离 FileDialog 低频配置与持续 UI 同步](37-分离-FileDialog-低频配置与持续-UI-同步.md) | 整理 `crates/file_dialog/src/style.rs::reconcile_root` 与 `view.rs`、`controls.rs` 的调用边界，把首次构造、Style 配置及新节点初始化从必要的滚动、视图和输入同步中分开。 |
| 38 | [让 FileDialog 样式归属响应部件生命周期](38-让-FileDialog-样式归属响应部件生命周期.md) | 调整 `crates/file_dialog/src/style.rs::establish_color_owners` 及 `controls.rs`、`window.rs`、`view.rs` 的对应归属逻辑，避免每帧重扫 Dialog、Controls、Parts 和嵌套组合控件。 |
| 39 | [让 FileDialog 颜色按实际状态和可见行变化更新](39-让-FileDialog-颜色按实际状态和可见行变化更新.md) | 调整 `crates/file_dialog/src/style.rs::update_colors` 及 controls、window、view 的颜色调用边界，只刷新实际受影响的 Dialog 部件与可见文件行。 |
| 40 | [规范 Gallery 的结果路径显示](40-规范-Gallery-的结果路径显示.md) | 修改 `gallery/src/pages/window/file_dialog.rs` 的结果摘要，使 Windows 扩展长度路径在展示时使用易读形式。 |
| 41 | [建立 TextKey 的公共语义分层](41-建立-TextKey-的公共语义分层.md) | 调整 `gallery/src/i18n/key.rs`、`en_us.rs`、`zh_cn.rs` 及公共界面的使用点，建立 Common、Header、Navigation、ColorShowcase 等明确的公共语义类别。 |
| 42 | [将基础控件文案归入各页面的 Key 类型](42-将基础控件文案归入各页面的-Key-类型.md) | 迁移 Button 页面及其中的 RadioGroup 演示、CheckBox、ComboBox、TextField、Tooltip、ScrollArea 的页面专属 Key。 |
| 43 | [将数据视图文案归入各页面的 Key 类型](43-将数据视图文案归入各页面的-Key-类型.md) | 迁移 ListView、Tree、Table 的页面专属 Key，包括新增 Shared Model 示例中的说明与状态文本。 |
| 44 | [完成窗口与显示页面的 Key 分层](44-完成窗口与显示页面的-Key-分层.md) | 迁移 Window 页面及其 MessageBox、FileDialog 演示，以及 Waveform、Cursor 的专属 Key，完成顶层扁平页面变体的清理。 |
| 45 | [测量并收敛 Waveform 的稳态更新开销](45-测量并收敛-Waveform-的稳态更新开销.md) | 围绕 `crates/waveform/src/runtime.rs::update_waveforms` 与 `WaveformRuntime::update`，补足能够评价稳态调用成本的 Benchmark；只有收益得到证明时才合入提前跳过更新的生产改动。 |

Waveform 优化收益需要实测验证；没有证据时不把候选优化写成已实现或已证实的结果。
