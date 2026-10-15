# 让 ComboBox 的组合样式按变化更新

## 目标与范围

优化 `crates/combo_box/src/style.rs::update<T>` 的触发和作用范围。保留 Field 内容投影缓存及每种 T 的 Typed Plugin／System 设计。

## 修改设计

按实际样式输入收集变化：Field Hover／Pressed、Popup 开闭、根与内部有效 Disabled、ListView Selection／Active／Focus、颜色覆盖、Theme，以及新建 Field、箭头、Popup 和可见行。选择造成内容替换时，新的 Text／Icon 也要获得正确 Foreground。

由 ComboBox 作为组合样式 Owner 决定 Field、Popup、箭头与内部 ListView 的配色。内部 Row 或 Field 的变化必须能使所属 ComboBox 的相关样式失效，不能仅监听 Root 的 Changed，也不能要求被接管的子控件重新使用自己的主题。

复用上一项已建立的构造／归属信号，不再用每帧扫描找出缺失 Owner。保留 `(id, index, revision)` 内容缓存，不把颜色失效变成内容重建，也不调整 Popup 几何、隐藏 Focus 清理和选择初始化的必要调度。

程序化选择继续保留既有 Popup 状态；选择提交与通知、Disabled 下允许的 API 操作和关闭后键盘归属均不改变。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖打开、关闭、重选、程序化设置与清空、Model Edit／Move、Popup 中滚动复用行、Field 内容替换、Hover／Pressed 移除、Disabled 添加／移除、主题和 Override 清除。多个 T 和多个实例之间的脏状态不能串用。

稳定 ComboBox 不重复解析整个组合的颜色，交互后的首次有效帧样式正确。保留对应选择、Focus、渲染与错误路径测试，并在 Gallery 中用 BRP 验证实际弹出、选择和滚动；性能测量保持数据量和可见行数不变。

## 预期产出

不破坏内容缓存、类型注册和 Popup 行为的 ComboBox 变化驱动组合配色。

## 与前后方案的关系

顺序上接 [方案 21：让 ComboBox 的样式归属只在必要时建立](21-让-ComboBox-的样式归属只在必要时建立.md)。

完成后进入 [方案 23：让 RadioOption 的结构错误日志按故障变化记录](23-让-RadioOption-的结构错误日志按故障变化记录.md)。

依赖 ComboBox 的 Owner 生命周期调整，并复用已有 Button、ListView 配色入口；内容投影缓存不受本项改动。
