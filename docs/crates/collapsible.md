# Collapsible

`bevy_widgetry_collapsible` 提供 `WidgetryCollapsible` SceneComponent 与 `WidgetryCollapsiblePlugin`，通过 Facade 的默认 `collapsible` Feature 与同名模块公开。

BSN `@header` 接收 `Option<Box<dyn Scene>>`，`@children` 接收 `Option<Box<dyn SceneList>>`。HeaderContent 占据 HeaderRow 剩余空间，末端保留固定 WidgetryButton；Content 为纵向 Flex 容器。默认展开，`@expanded: false` 指定一次性初始折叠。Header 和 Content 都可以为空。

根实体私有 Expanded 是唯一运行时展开状态。`WidgetryCollapsible::expanded(world, root)` 查询；`set_expanded_in_world(world, root, bool)` 同步更新，返回 `Result<bool, BevyError>`，实际改变为 true，同值为 false。`set_expanded(commands, root, bool)` 在执行时校验，通过宿主 error handler 反馈错误。无效根或必要结构缺失返回 Error，不提交、不通知。

`WidgetryCollapsibleChanged { entity, expanded }` 是 EntityEvent，真实改变时先提交、再通知；初始化与同值操作不通知，通知不保证布局已重算。折叠设置 Content 的 `Display::None` 和 `Visibility::Hidden`；展开恢复 Flex 与 Inherited，并即时同步必要的派生可见性。子树常驻，应用的布局、显式 Visibility、数据不被改写；第一版无折叠动画。

当前固定按钮仅建立结构，用户激活、焦点转移、Header 局部禁用及卡片主题外观尚未接入。
