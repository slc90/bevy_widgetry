# Tabs

`WidgetryTabs` 与 `WidgetryTab` 使用 BSN 构造固定数量的标签页。Root Props 的 `children: SceneList` 接收 Tab Scene；每个 Tab Props 的 `header: Scene` 和 `content: Scene` 成对声明任意内容。可预先从 Rust 集合生成 SceneList；实例建立后不支持增删或重排。

安装 `WidgetryTabsPlugin` 自动安装 Core UI；Facade 默认启用的 `tabs` Feature 通过 `bevy_widgetry::tabs` 导出。

## 选择与生命周期

Root Props 的 `selected_index` 默认为 0，集合必须非空且初值合法。正常输入前完成配对，将 Header 保留在标签栏，Content wrapper 移入独立 PageContainer；初始化无业务通知。

`WidgetryTabs::selected_index`、`header`、`page` 只读查询状态和固定实体。`set_selected_in_world` 同步返回 `Result<bool, BevyError>`；`set_selected` 入队，在执行时校验。选择是唯一的 usize 状态，只能通过公开 API 更新。Header 的 Checked 与页面 wrapper 的 Node.display、Visibility 由 Tabs 托管，不直接改写。

用户主键点击和程序化切换先提交选择、Header Checked 和页面 Display，再从 Root 发送 `ValueChange<usize>`（is_final=true）。合法同值不改写、不通知。页面全部常驻，仅活动页面 wrapper 为 Display::Flex，其他为 Display::None；不重建实体，不暂停隐藏页面的业务 System，页面内部滚动、数据、布局归应用管理。

切换隐藏当前焦点所在的页面时清除焦点，不自动聚焦新页面；其他区域焦点保持不变。页面 wrapper 同时投影 Visibility::Hidden / Inherited，并同步页面子树的继承可见性，避免下一次 PreUpdate 的 Tab 导航先于 Bevy PostUpdate 传播而进入旧页面。Tab / Shift+Tab 跳过隐藏页面，活动页面仍按内容自己的 TabIndex 导航；子树已有 Visibility 语义保持 Bevy 原生规则。

## 错误与边界

空集合、越界、失效 Root、缺失 Header/Content、固定配对或容器损坏均明确报告 Error Severity，不部分提交、不通知。Scene 构造错误遵循 Core Scene 错误通道；命令交给宿主 Error Handler。稳定帧不重复扫描结构，公开操作重新校验固定配对。

## Disabled

Root 或任意祖先上的 InteractionDisabled 通过 Core 的有效继承同时禁用标签栏和所有页面；切换后新显示的页面仍继承同一禁用约束。单个 Header 可单独禁用，只阻止用户选择该项，不影响其配对页面内部控件。禁用已选项不会清空或移动选择。

用户点击按当前有效 Disabled 判断，执行排队选择时再次检查。同步及排队的公开选择 API 仍允许 Root / 目标 Header 禁用期间的合法程序化切换；实际改变才通知。解除 Root 禁用不会清除 Header 的本地请求，Disabled 投影本身不发 Selection Change。

## 方向与焦点

Root Props 的 `orientation: TabsOrientation` 默认为 Horizontal（上方单行标签），Vertical 将标签列置于页面左侧。方向只在构造时初始化，同时决定布局与键盘轴，不提供运行时 Setter；单独改变 Node 不改变键盘轴。

Header 设置 TabIndex(-1)，点击可聚焦，Tab / Shift+Tab 顺序遍历跳过标签头，仍经过活动页面内显式配置 TabIndex 的控件。仅焦点位于 Header 时响应方向键：横向左右、纵向上下，从当前焦点 Index 寻找下一个非禁用项并立即切页，同时将焦点移到目标 Header。边界停止不循环，无目标、非对应轴、Home / End 不改选、不发通知；Root 有效禁用时不响应。

程序化选择不抢焦点；焦点仍在旧 Header 时，下一次方向键从该焦点位置开始导航，而非从程序设置的 Selection 开始。页面焦点隐藏清除规则继续适用。

经典主题外观、标签栏滚动与 Gallery 展示尚未实现。
