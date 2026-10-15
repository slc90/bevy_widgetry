# 为 Facade 控件建立真正的可选编译边界

## 目标与范围

调整 `crates/bevy_widgetry/Cargo.toml`、`src/lib.rs` 和受影响的 Facade 测试、Benchmark 目标，为公开控件建立 optional dependency、Cargo Feature 和条件导出的一一对应关系。

## 修改设计

为 `button`、`check_box`、`radio_group`、`text_field`、`combo_box`、`tooltip`、`scroll_area`、`list_view`、`tree`、`table`、`waveform`、`window`、`message_box`、`file_dialog` 提供独立选择入口。默认 Feature 集合保持现有全部控件 API 可用，避免现有默认依赖突然缺失模块；需要精简的应用显式关闭默认集合，再选择所需控件。

保留真实的控件依赖链。例如 ComboBox 自身依赖 ListView、Button 等，并不因为 Facade 没有单独启用这些模块就取消其内部编译依赖。Feature 控制 Facade 的直接依赖和公开入口，不人为声明每个上层 Feature 都开启所有下层 Facade 模块。外部应用需要直接使用某个模块时，显式启用该模块的 Feature。

基础 `core`、`theme`、`cursor` 按当前真实依赖关系保留：`core` 已依赖 Cursor、Theme 和内建 Asset。不要给仍然强制编译的基础 crate 加一个只有名称意义的关闭开关。Scene、Text、Icon、Disabled、Style 等已有基础入口保持原有职责。内建 Asset API 不通过 Facade 新增公开模块；`internal_log`、`test_utils` 不成为对外控件 Feature。`publish = false` 与编译开关没有关系，也不为本项引入 crates.io 发布体系。

现有依赖特定控件的集成测试需要相应 cfg；Benchmark 目标需要按实际使用的模块声明 `required-features` 或采用等价的合法条件结构。不能让库本身在 no-default 下通过，却让 `--all-targets` 因测试和 Benchmark 的无条件导入失败。也不能用 Workspace 其他包启用的 Feature 掩盖单独依赖 Facade 时的错误。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

分别验证默认集合、无默认集合、每个单独控件 Feature 和有代表性的组合。检查普通依赖图，确认关闭的非必要控件不被 Facade 自身强行引入；区分必要的传递依赖与测试依赖造成的额外编译。针对宏导出、泛型注册、测试目标和 Benchmark 目标覆盖编译边界，保持 deny warnings 下的导入清洁。

更新 `docs/crates/bevy_widgetry.md` 的默认策略、按需使用方法和基础依赖限制。`app_icon_build` 保持独立 Build Dependency，不进入 Feature 表或公共 API。

## 预期产出

默认使用方式不变、关闭默认集合后可实际减少非必要控件依赖的 Facade，并有覆盖各编译边界的验证依据。

## 与前后方案的关系

顺序上接 [方案 03：将内部日志 crate 重命名为 internal_log](03-将内部日志-crate-重命名为-internal_log.md)。

完成后进入 [方案 05：通过可选 app_logging Feature 公开宿主日志能力](05-通过可选-app_logging-Feature-公开宿主日志能力.md)。

使用已经统一的内部依赖名称。本项建立控件 Feature 边界，后续 app_logging 入口沿用这套边界；它本身不依赖后续入口才能验收。
