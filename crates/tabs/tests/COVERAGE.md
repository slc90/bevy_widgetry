# Tabs Coverage

`selection.rs` 覆盖固定 BSN 配对与变量 SceneList、默认及指定初值、选择提交、同步与延后执行 API、PointerClick 从 Header 内容冒泡、同值静默、通知时已完成 Checked 与 Display 投影、实例隔离。

状态为唯一 usize Selection、Header Checked 投影、页面 Display 和输入焦点。真实改变后始终只有一个 Header 选中、一个页面显示；所有页面实体、正文和应用滚动偏移保留。仅隐藏页面子树中的焦点会被清除，不向新页面转移。

无效 Root、空集合、初始及运行时越界、缺失配对和执行时结构损坏必须返回 Error Severity；拒绝不改选、不发通知、不部分重排页面。构造期拒绝与健康实例共存，稳定帧不重复诊断。

有效 Disabled 覆盖本地 Header、Root 祖先继承、用户拒绝、已选禁用保留、公开同步 / 排队选择放行、切页后继承保持、解除祖先时本地请求保留；Header 与页面不共享禁用父子关系，Disabled 本身不发选择通知。

`text_input.rs` 使用真实 WidgetryTextField 与 Bevy 键盘派发：输入后隐藏页面，后续按键不写入隐藏编辑器，返回原页面后正文和页面实体保留，不自动抢回焦点。选择切换后不额外等待传播帧，真实 Tab / Shift+Tab 导航即只经过活动页面的可聚焦控件，跳过隐藏页面，两种选中状态均覆盖。

`keyboard.rs` 覆盖 PointerPress 内容冒泡聚焦 Header、两种方向轴的自动激活、首尾停止、禁用中间 / 尾部跳过与无目标、失焦及 Root Disabled guard、非对应箭头及 Home/End 静默、事件时已提交页面投影、Header TabIndex(-1) 与活动页面顺序导航。程序化选择不移焦，焦点和选择不同时从 Header 焦点位置继续导航。

`style.rs` 验证四主题的普通/选中、Hover、Pressed、Disabled、选中禁用及焦点样式；Root与Header覆盖优先级、同值/非法输入、配置清除；样式不改选、不发业务通知、不覆盖页面背景。真实UI布局覆盖两种方向及1/1.25/2尺度的自然/统一宽度、统一高度、贴合边界、长文字省略与原文保留、本地化式文本更新后取消省略。托管Header、面板与省略标签输出缺失去重报错，重新插入后恢复。
- Review回归：Header直接Text及子Text在透明/半透明背景下按字形截断；缩放、文字与宽度重排恢复完整派生排版，稳定帧不改写。
