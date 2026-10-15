# GUI 调试与验证规则

## 适用范围

涉及 Widget 或 Gallery 的可观察 GUI 行为变化时，必须进行针对性的自动 BRP 运行时验证。

包括视觉表现、Pointer / Keyboard 输入、Focus、Picking、Layout、Theme、Drag、Scroll 以及其他需要通过实际界面确认的行为。

验证范围由当前任务新增、修改及直接影响的行为决定，不要求重新遍历全部历史 GUI 场景。

纯数据、算法或不改变 GUI 可观察行为的修改，不需要启动 Gallery 验证。

## BRP 验证

Widgetry 的 GUI 调试与验证统一使用 `bevy_brp_mcp` 操作 Widget Gallery，不以 PowerShell 截图脚本、Win32 鼠标控制或其他桌面自动化方式替代标准 BRP 验证。

验证时应根据实际任务选择必要的 BRP 工具和操作：

1. 发现并启动 `widget_gallery`，获取初始 Screenshot 或必要的运行时 State。
2. 通过 BRP 执行与目标行为对应的真实输入。
3. 使用 Screenshot、ECS Query、Component / Resource State、日志或诊断信息检查结果。
4. 完成验证后释放 BRP Pointer 控制，并正常关闭 Gallery。

不得只通过应用成功启动或未发生崩溃判断 GUI 行为正确。

### 输入与状态检查

- Pointer 行为应实际执行移动、点击、Press / Release 或 Drag 等相关输入。
- Keyboard 行为应先获得正确 Focus，再执行对应输入。
- Scroll 行为应通过真实滚动输入验证。
- 不得直接修改 ECS State 后，将结果视为等价的用户交互验证。
- 视觉变化应通过 Screenshot 确认；涉及内部状态契约时，应结合 ECS State 检查。
- BRP 请求成功仅表示工具操作被接受，不代表目标 UI 行为已经正确完成。

如果 Entity 已有稳定的 `Name`，应优先使用它进行目标发现与状态查询，不为了调试添加无业务意义的生产标记或 API。

### Pointer 控制

BRP 普通 Pointer 输入使用独立的 Custom Pointer，不等同于原生 OS Mouse 输入。

验证期间应避免真实鼠标与 BRP Pointer 同时操作，防止输入来源相互干扰。

完成 Pointer 自动化后，必须通过 `brp_extras/pointer_control` 执行 `release`，再查询 `status`，确认 Pointer Phase 已恢复为 `inactive`。

`draining` 表示释放过程尚未完成；`busy = false` 仅表示没有在途操作，不代表 UI 验证成功。应同时检查 `last_error` 及实际界面状态。

不得将 Custom Pointer 的执行结果直接视为原生 Mouse、Native Window 操作或其他 OS-level 输入行为的验证结果。

## 时序行为验证

动态 GUI 不能只检查操作完成后的最终稳定状态。

当变更可能涉及短暂显示错误或跨 Frame 状态同步时，应根据风险观察输入后的中间状态及最终收敛结果。

重点关注：

- 短暂空白、闪烁或旧内容残留。
- Popup 的初始位置及后续 Layout 调整。
- Text、Icon、Background 与 Foreground 的同步。
- Focus、Theme、Visibility 及动态 Children 变化。
- 虚拟化 Row 创建、复用和内容更新。
- Asset 加载或 UI 内容延迟呈现。

必要时在输入后尽早采集 Screenshot 或 State，并观察关键中间阶段，再确认最终稳定状态。

不要求所有 GUI 测试机械增加多次截图，只验证当前行为实际存在的时序风险。

## 与自动化测试的关系

BRP 用于当前任务的真实 GUI 运行时验收，不属于长期 Unit Test / Integration Test 覆盖体系，也不替代 `rules/testing.md` 要求的回归保护。

如果 BRP 发现可稳定复现的问题，并且能够合理通过 Headless Test 表达，应提炼对应行为契约并增加确定性 Regression Test。

不要求将每个 BRP 操作场景转化为长期自动化测试。

## 工具能力边界

BRP 无法忠实模拟的 OS-level 行为，例如原生窗口移动、Resize、跨应用 Focus 或其他依赖真实操作系统输入的行为，不得通过不等价的 Custom Pointer 操作宣称验证通过。

BRP 不可用、必要工具失败或当前工具无法覆盖目标行为时，应明确记录：

- 已完成的验证及实际结果。
- 未能验证的行为和原因。
- 工具能力或运行环境的具体限制。

不得将未执行或证据不足的验证标记为通过，也不得静默切换到不等价的自动化方式并将结果视为标准 BRP 验证。
