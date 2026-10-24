# Collapsible

`bevy_widgetry_collapsible` 提供 `WidgetryCollapsible` SceneComponent 与 `WidgetryCollapsiblePlugin`，通过 Facade 的默认 `collapsible` Feature 与同名模块公开。

BSN `@header` 接收 `Option<Box<dyn Scene>>`，`@children` 接收 `Option<Box<dyn SceneList>>`。HeaderContent 占据 HeaderRow 剩余空间，末端保留固定 WidgetryButton；Content 为纵向 Flex 容器。默认展开，`@expanded: false` 指定一次性初始折叠。Header 和 Content 都可以为空。

根实体私有 Expanded 是唯一运行时展开状态。`WidgetryCollapsible::expanded(world, root)` 查询；`set_expanded_in_world(world, root, bool)` 同步更新，返回 `Result<bool, BevyError>`，实际改变为 true，同值为 false。`set_expanded(commands, root, bool)` 在执行时校验，通过宿主 error handler 反馈错误。无效根或必要结构缺失返回 Error，不提交、不通知。

`WidgetryCollapsibleChanged { entity, expanded }` 是 EntityEvent，真实改变时先提交、再通知；初始化与同值操作不通知，通知不保证布局已重算。折叠设置 Content 的 `Display::None` 和 `Visibility::Hidden`；展开恢复 Flex 与 Inherited，并即时同步必要的派生可见性。子树常驻，应用的布局、显式 Visibility、数据不被改写；第一版无折叠动画。

固定 WidgetryButton 使用原生 Activate，鼠标或 Enter/Space 激活均通过相同状态 API 翻转 Expanded；HeaderContent 不是额外热区。按钮使用 TabIndex(0)，插件自动安装 TabNavigation。隐藏或有效禁用的按钮拒绝激活。

用户和程序化折叠都检查当前 Content 子树焦点：转移至可聚焦按钮，按钮禁用或隐藏时清除；Content 外焦点保留，展开不自动恢复旧焦点。

根官方 `InteractionDisabled` 禁用整个控件。`@header_disabled` 默认 false，`header_disabled(world, root)` 查询局部请求，`set_header_disabled_in_world(world, root, bool)` 返回请求是否改变，`set_header_disabled(commands, root, bool)` 在执行时提交。局部请求通过 Core 独立 intrinsic reason 施加于 HeaderRow，禁用 HeaderContent 和固定按钮，Content 输入仍可使用。它与根/祖先请求 OR 组合，解除任一层不会误清除另一层；禁用本身不产生展开事件，禁用期间程序化展开 API 仍然可用。

卡片内建 1px 外边框、6px 圆角，Header 左右 12px / 上下 8px 内边距，Content 四周 12px 内边距。Header 高度随应用内容变化，末端按钮保持 32×32px，内置 16×16px ChevronUp / ChevronDown SVG；切换只更换 SVG 来源，不重建 Icon Entity。Content 顶部 1px 分隔线随折叠退出布局。尺寸来自内部 Style 常量，无额外尺寸 Props。

四套 Theme 均提供 `WidgetryCollapsibleColors`：background、header_background、content_background、border、separator。Header 与 Content 默认同色。`@colors` 初始化 `WidgetryCollapsibleColorOverrides`，运行期使用该类型的 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 查询、替换或清除覆盖。颜色必须有限且 alpha 位于 [0,1]，无效请求返回 Error，不修改已有覆盖。主题切换保留覆盖，未覆盖字段采用当前主题；只写入自身卡片、Header 和 Content 输出，不改写应用 Children。固定按钮继续使用 Button 交互配色，Icon 继承其前景色及禁用状态。插件自动安装 Asset 与 Icon 能力。
