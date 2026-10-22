# Tabs Coverage

`selection.rs` 覆盖固定 BSN 配对与变量 SceneList、默认及指定初值、选择提交、同步与延后执行 API、PointerClick 从 Header 内容冒泡、同值静默、通知时已完成 Checked 与 Display 投影、实例隔离。

状态为唯一 usize Selection、Header Checked 投影、页面 Display 和输入焦点。真实改变后始终只有一个 Header 选中、一个页面显示；所有页面实体、正文和应用滚动偏移保留。仅隐藏页面子树中的焦点会被清除，不向新页面转移。

无效 Root、空集合、初始及运行时越界、缺失配对和执行时结构损坏必须返回 Error Severity；拒绝不改选、不发通知、不部分重排页面。构造期拒绝与健康实例共存，稳定帧不重复诊断。

`text_input.rs` 使用真实 WidgetryTextField 与 Bevy 键盘派发：输入后隐藏页面，后续按键不写入隐藏编辑器，返回原页面后正文和页面实体保留，不自动抢回焦点。选择切换后不额外等待传播帧，真实 Tab / Shift+Tab 导航即只经过活动页面的可聚焦控件，跳过隐藏页面，两种选中状态均覆盖。
