# 完善 Gallery 设置操作的意图与结果记录

## 目标与设计思路

让主题、语言和窗口缩放设置都能够追踪用户**确实发起的选择**，即使目标值与当前状态相同；只有设置真实生效时才记录程序内部状态变化。这个增量针对 `gallery/src/header.rs` 的三个独立设置处理器，可作为一组同构小改动单独交付。

## 现有行为与缺口

`on_scale_factor_combo_box_changed` 在目标窗口的 `scale_factor_override` 与选定值不同时才写 INFO；`on_language_changed` 仅在 `language.set_if_neq(selected)` 为真时写 INFO；`on_theme_combo_box_changed` 则排队执行 `WidgetryThemeMode::set_in_world()`，也只有返回实际变化时记录 INFO。因此同值的明确用户选择在当前代码里完全无日志，而且现有 INFO 的文案更接近“设置已经生效”，混淆了请求与真实状态。

需要保持已有 `sync_external_settings`、`sync_theme_combo_box`、`initialize_*_selection` 的程序化同步路径：这些路径不能被误记为用户发起操作，程序内部普通同值同步仍保持静默。`ValueChange` 事件是否会包含选择原值，应根据当前 Widgetry ComboBox 的实际语义核实；不能假定它一定会为再次选择同一项触发事件。

## 记录方式

由三个设置的应用层事件处理器记录结构化 INFO：`setting`、`requested`、已知的 `previous`，以及同值时的 `unchanged`／`outcome`。如果实际事件表达的是“值确已变化”而非“用户点击选择”，就不要把没有收到的意图凭空补成日志。对于应用层明确能观察到的同值选择，记录操作但不重做状态变更或配置保存。

对真实生效的状态变化保留少量 DEBUG：Theme 的 `set_in_world` 成功、语言 Resource 更新、窗口 DPI Override 变化，按执行时机记录实际值；Theme 通过 `commands.queue` 延迟设置时，INFO 不得提前宣布成功，DEBUG 只有在 queued work 确实执行成功后发出。

已有配置保存失败执行 ERROR + 实际处理 DEBUG；新日志不得把“UI 已生效”误称为“配置已经持久保存”。若要表达配置保存成功，必须在确认写盘后记录，不能依靠 `AppConfig::save()` 的返回形式虚构结果。本增量不改变设置生效的公开 API、保存策略、Theme 事件传播或 ComboBox 的正常选择契约。

新增日志正文统一中文，动态上下文使用英文结构化字段；只记录必要的对象身份、状态和结果摘要，不打印完整用户输入、敏感内容或大体量业务数据，也不手动重复 Tracing 已提供的 `file`、`line`、`module_path`、`crate_name` 等元数据。

## 验证

验证用户更改三项设置时分别有明确 INFO 和实际变化 DEBUG；已能通过应用语义处理器观察到的同值操作有 INFO 而无虚假 DEBUG；初始化或程序同步不产生用户 INFO；Theme 延迟队列失败不产生成功日志；配置保存失败仍维持原有 ERROR/handled DEBUG，界面可保留已应用状态。必要时在现有 Gallery 设置测试或 BRP 中覆盖一次完整选择与重复选择。

同步 `docs/gallery.md` 的设置行为说明即可；原有 `rules/logging.md` 无需继续加特例，因其已规定用户语义操作、同值操作和程序同步的区分。
