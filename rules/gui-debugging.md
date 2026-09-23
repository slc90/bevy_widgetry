# GUI 调试与验证规则

本文件定义 Codex CLI 对 Widgetry GUI 行为进行运行时调试与验证时必须遵循的流程。

## 适用范围

当任务涉及以下任一内容时，应读取并遵守本文件：

- Widget 或 Gallery 的可视表现；
- pointer / mouse 交互；
- keyboard / text input；
- focus；
- picking；
- layout；
- theme；
- drag / scroll；
- 其他必须通过运行中的 Gallery 才能可靠判断的 GUI 行为。

纯数据、纯算法或其他不影响 GUI 可观察行为的修改，不要求为了流程完整而启动 Gallery。

## 默认 GUI 调试通道

Codex 应优先通过 `bevy_brp_mcp` 操作 Widget Gallery。

不得把 PowerShell 截图脚本、Win32 鼠标控制、`SetCursorPos` 等桌面自动化方式作为默认 GUI 调试通道。

Gallery 通过 `bevy_brp_extras` 暴露 BRP 能力。

## 标准流程

需要 GUI 调试或验证时，按以下流程执行：

1. 使用 `brp_list_bevy` 发现项目中的 Bevy target，并确认 `widget_gallery`。
2. 使用 `brp_launch` 启动 `widget_gallery`，在 `env` 中设置 `WIDGETRY_BRP=1`。
3. 使用 `brp_extras_screenshot` 获取当前 Gallery 界面作为基线。
4. 根据当前验证目标使用必要的 BRP 输入工具。
5. 使用截图、ECS 查询、Component / Resource 状态、日志或诊断信息检查结果。
6. 验证结束后通过 BRP 正常关闭 Gallery。

不要为了“完整”机械调用全部工具，只执行当前行为所需要的步骤。

通过 BRP 启动 Gallery 时必须设置 `WIDGETRY_BRP=1`。该变量仅用于启用 BRP
自动化所需的低频 App polling。普通人工运行不应设置该变量，并继续使用
`WinitSettings::desktop_app()`。不得为了提高 BRP 响应速度，把 Gallery 全局改为
高频 Reactive 或 Continuous update mode。

## 常用 BRP GUI 工具

鼠标：

- `brp_extras_move_mouse`
- `brp_extras_click_mouse`
- `brp_extras_double_click_mouse`
- `brp_extras_send_mouse_button`
- `brp_extras_drag_mouse`
- `brp_extras_scroll_mouse`

键盘：

- `brp_extras_send_keys`
- `brp_extras_type_text`

视觉检查：

- `brp_extras_screenshot`

运行时检查可根据任务使用 BRP 的 Entity / Component / Resource query、watch、日志和 diagnostics 工具。

## 验证原则

- GUI 验证必须检查任务要求的可观察行为，不得只确认“应用没有崩溃”。
- 鼠标、键盘、drag、scroll 等交互行为应实际执行对应输入。
- 视觉变化应使用截图确认。
- 能通过 ECS / Component / Resource state 精确确认的状态，优先使用结构化状态，不仅依赖截图猜测。
- 截图与结构化状态可以组合使用：截图验证最终视觉结果，ECS 状态验证内部语义。
- 如果目标 Entity 已有稳定的 `Name`，应优先使用 Name 进行实体发现、截图选择和状态查询；输入模拟仍根据具体工具使用必要坐标。
- 不要仅为了 BRP 调试给大量生产 Widget 添加无业务意义的标记或 API。

## 与自动化测试的关系

BRP GUI 验证是运行时集成验证，不替代 `rules/testing.md` 要求的 unit test / integration test / regression test。

对于 `gallery/` 自身，继续遵循 `rules/testing.md` 的 Gallery 例外。

## BRP 不可用时

如果任务需要 GUI 验证，但出现以下情况之一：

- `bevy_brp_mcp` 未安装或 MCP server 未连接；
- Gallery 没有可用的 BRP support；
- 必要 BRP tool 调用失败且无法在当前任务范围内恢复；

应明确报告 GUI 验证未完成以及缺失条件。

不得静默退回 PowerShell / Win32 GUI 自动化并把其结果视为等价的标准 BRP 验证。

如果当前行为本质上属于 BRP 无法忠实模拟的 OS-level 行为，例如真实系统窗口移动 / resize、跨应用焦点或 native OS 对话框，应明确说明 BRP 的边界并要求对应的人工验证；除非任务明确要求，否则不要自行引入另一套桌面自动化方案。
