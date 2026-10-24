# Collapsible

`bevy_widgetry_collapsible` 提供 `WidgetryCollapsible` SceneComponent 与 `WidgetryCollapsiblePlugin`，通过 Facade 的默认 `collapsible` Feature 与同名模块公开。

BSN `@header` 接收 `Option<Box<dyn Scene>>`，`@children` 接收 `Option<Box<dyn SceneList>>`。HeaderContent 占据 HeaderRow 剩余空间，末端保留固定 WidgetryButton；Content 为纵向 Flex 容器。默认展开，`@expanded: false` 指定一次性初始折叠。Header 和 Content 都可以为空。

根实体私有 Expanded 是唯一运行时展开状态。`WidgetryCollapsible::expanded(world, root)` 查询；`set_expanded_in_world(world, root, bool)` 同步更新，返回 `Result<bool, BevyError>`，实际改变为 true，同值为 false。`set_expanded(commands, root, bool)` 在执行时校验，通过宿主 error handler 反馈错误。无效根或必要结构缺失返回 Error，不提交、不通知。

`WidgetryCollapsibleChanged { entity, expanded }` 是 EntityEvent，真实改变时先提交、再通知；初始化与同值操作不通知，通知不保证布局已重算。折叠设置 Content 的 `Display::None` 和 `Visibility::Hidden`；展开恢复 Flex 与 Inherited，并即时同步必要的派生可见性。子树常驻，应用的布局、显式 Visibility、数据不被改写；第一版无折叠动画。

固定 WidgetryButton 使用原生 Activate，鼠标或 Enter/Space 激活均通过相同状态 API 翻转 Expanded；HeaderContent 不是额外热区。按钮使用 TabIndex(0)，插件自动安装 TabNavigation。隐藏或有效禁用的按钮拒绝激活。

用户和程序化折叠都检查当前 Content 子树焦点：转移至可聚焦按钮，按钮禁用或隐藏时清除；Content 外焦点保留，展开不自动恢复旧焦点。Header 局部禁用及卡片主题外观尚未接入。
