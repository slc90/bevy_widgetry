# Collapsible 测试覆盖

`state.rs` 从公开 API 与真实 BSN/ECS 组合验证：

- 状态：唯一 Expanded；默认展开、初始折叠，不发送初始化事件。
- 刺激：同步 setter、Commands 实际执行；重复同值为 no-op。
- Guards：失效 Entity、错误类型、必要内部 Node 缺失；Error 日志、Severity 与宿主 error handler。
- 不变量：提交后通知；切换只改 Content 容器的 Display/Visibility 及派生可见性，保留实体、Grid、编辑值、ScrollPosition 和显式隐藏子项。
- 布局：实际 UI Layout 中折叠 Content 高度为零。

`input.rs` 验证固定 WidgetryButton 的真实 Press/Click/Release 与 Tab/Enter/Space 路径：应用 HeaderContent 交互不折叠；Disabled、隐藏、重复键盘事件拒绝激活。同步 setter 的折叠将深层 Content 焦点移至可用按钮，否则清除；保留外部焦点，展开不恢复旧焦点。折叠后的 Tab 导航跳过 Content。

`disabled.rs` 验证 Header 局部请求初始化、查询、同步/排队更新、同值与无效结构错误；根/局部请求两种解除顺序、祖先请求与 Header 自身请求独立保留，禁用不通知展开事件。Header 局部禁用时真实 TextField 仍可编辑，根禁用阻止输入；禁用期间程序化折叠与焦点清理仍生效，包含初次 update 前的折叠。

`style.rs` 验证默认卡片尺寸与分隔线、两种初始展开状态及重复切换的 SVG 方向和实体保留；四套 Theme、局部覆盖及清除、应用 Children 样式保留；Header 禁用时箭头继承 Button 前景且方向不变；颜色 API 同值、无效颜色不提交、错误目标及 Commands 执行时错误传播。实际卡片渲染、SVG 像素与用户输入由 Gallery BRP 验收。
