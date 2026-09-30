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

Gallery 通过外部 `bevy_brp_runtime` 提供完整 BRP runtime。

## BRP 验证的职责与场景选择

BRP GUI 验证是当前任务完成后的真实用户场景运行时验收，不是长期 exhaustive regression suite。

验证场景根据本次新增行为、修改行为及变更直接影响的 GUI 行为选择，不要求每次重新验证 Widget 的全部历史功能。长期 regression 保护由 unit test / integration test 承担。

## 标准流程

需要 GUI 调试或验证时，按以下流程执行：

1. 使用 `brp_list_bevy` 发现项目中的 Bevy target，并确认 `widget_gallery`。
2. 使用 `brp_launch` 启动 `widget_gallery`。
3. 使用 `brp_extras_screenshot` 获取当前 Gallery 界面作为基线。
4. 根据当前验证目标使用必要的 BRP 输入工具。
5. 使用截图、ECS 查询、Component / Resource 状态、日志或诊断信息检查结果。
6. 验证结束后通过 BRP 正常关闭 Gallery。

不要为了“完整”机械调用全部工具，只执行当前行为所需要的步骤。

Gallery 默认安装 BRP runtime，不需要环境变量启用。App 仍使用
`WinitSettings::desktop_app()`，请求入队与真实在途操作会按需 wake 后续 update。
不得为了提高 BRP 响应速度，把 Gallery 全局改为高频 Reactive 或
Continuous update mode。

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
- 验证 keyboard 行为时应真实取得 focus 后发送 keyboard input；pointer 行为应真实执行 pointer movement / click；scroll 行为应真实执行 wheel / drag。不得直接修改 ECS state 后把结果视为等价的用户行为验证。
- 视觉变化应使用截图确认。
- 优先验证实际视觉结果、交互效果、focus、scrolling、popup、selection 与 layout；需要精确判断内部语义时，可以结合 ECS / Component / Resource state，不仅依赖截图猜测。
- 截图与结构化 state 可以组合使用：截图验证视觉结果，ECS state 验证内部语义；不要求重复 integration test 中所有内部 invariant。
- 如果目标 Entity 已有稳定的 `Name`，应优先使用 Name 进行实体发现、截图选择和状态查询；输入模拟仍根据具体工具使用必要坐标。
- 不要仅为了 BRP 调试给大量生产 Widget 添加无业务意义的标记或 API。

## Temporal behavior

动态 GUI 行为不得默认采用“执行输入 → 等待完全稳定 → 只检查最终 state”的验证方式。对于容易产生瞬时错误的变化，应关注操作后的中间过程和收敛过程。

典型问题包括：

- 一帧或短暂空白、短暂消失、flicker。
- 旧内容残留，或数百毫秒后才刷新。
- popup 先出现在错误位置后再跳正。
- text / icon 晚一帧出现。
- background 已变化但 icon / foreground 尚未同步。
- dynamic subtree 创建后直到下一帧才真正可见。

重点关注 dynamic subtree、text、icon、popup、focus transition、theme change、visibility、layout、virtualized row，以及 asset / text / icon materialization。

当前行为存在 temporal risk 时，可以使用以下验证流程：

```text
baseline
↓
execute input
↓
尽快观察操作后 state
↓
必要时观察 intermediate state
↓
确认 final stable state
```

不要求所有 GUI 验证机械增加多次截图；根据当前行为的 temporal risk 选择必要观察。

## 与自动化测试的关系

BRP GUI 验证不属于长期自动化测试覆盖体系，不用于判断模块自动化测试是否全面，也不替代 rules/testing.md 要求的 unit test / integration test / regression test。

如果 BRP 发现可稳定复现的 GUI / temporal regression，并且该问题能够合理通过 headless unit / integration test 表达，应提炼可自动化 contract，并增加永久 regression test。

BRP 负责发现真实运行问题，不承担以后持续重复执行同一场景的责任。

对于 `gallery/` 自身，继续遵循 `rules/testing.md` 的 Gallery 例外。

## BRP 不可用时

如果任务需要 GUI 验证，但出现以下情况之一：

- `bevy_brp_mcp` 未安装或 MCP server 未连接；
- Gallery 没有可用的 BRP support；
- 必要 BRP tool 调用失败且无法在当前任务范围内恢复；

应明确报告 GUI 验证未完成以及缺失条件。

不得静默退回 PowerShell / Win32 GUI 自动化并把其结果视为等价的标准 BRP 验证。

如果当前行为本质上属于 BRP 无法忠实模拟的 OS-level 行为，例如真实系统窗口移动 / resize、跨应用焦点或 native OS 对话框，应明确说明 BRP 的边界并要求对应的人工验证；除非任务明确要求，否则不要自行引入另一套桌面自动化方案。
