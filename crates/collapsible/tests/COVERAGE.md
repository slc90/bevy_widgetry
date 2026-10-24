# Collapsible 测试覆盖

`state.rs` 从公开 API 与真实 BSN/ECS 组合验证：

- 状态：唯一 Expanded；默认展开、初始折叠，不发送初始化事件。
- 刺激：同步 setter、Commands 实际执行；重复同值为 no-op。
- Guards：失效 Entity、错误类型、必要内部 Node 缺失；Error 日志、Severity 与宿主 error handler。
- 不变量：提交后通知；切换只改 Content 容器的 Display/Visibility 及派生可见性，保留实体、Grid、编辑值、ScrollPosition 和显式隐藏子项。
- 布局：实际 UI Layout 中折叠 Content 高度为零。

`input.rs` 验证固定 WidgetryButton 的真实 Press/Click/Release 与 Tab/Enter/Space 路径：应用 HeaderContent 交互不折叠；Disabled、隐藏、重复键盘事件拒绝激活。同步 setter 的折叠将深层 Content 焦点移至可用按钮，否则清除；保留外部焦点，展开不恢复旧焦点。折叠后的 Tab 导航跳过 Content。

Header 局部禁用和外观属于后续增量，尚未覆盖。
