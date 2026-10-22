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

当前只有无主题的基础横向标签栏与页面布局；Disabled、键盘导航、经典主题外观、标签栏滚动与 Gallery 展示尚未实现。
