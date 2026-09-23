# Bevy Widgetry：BRP GUI 调试接入与 Codex CLI 工作流改造方案

## 目标

把 `bevy_brp_mcp + bevy_brp_extras` 正式接入当前 `bevy_widgetry` 项目，使 Codex CLI 在开发 Widget / Gallery 时能够直接通过 Bevy Remote Protocol（BRP）完成运行时 GUI 调试与验证，包括：

- 启动 / 发现 `widget_gallery`；
- 截取 Gallery 全窗口、Camera viewport 或指定 Entity；
- 模拟鼠标移动、点击、双击、拖拽、滚轮；
- 模拟键盘按键和文本输入；
- 查询 Entity / Component / Resource 与运行时状态；
- 获取日志、诊断信息并正常关闭 Gallery；
- 将 BRP 固化为 Codex CLI 的默认 GUI 调试工作流，避免默认退回 PowerShell / Win32 截图和鼠标脚本。

当前项目使用 Bevy `0.19.1`。本方案固定使用与 Bevy 0.19 对应的稳定版：

```text
bevy_brp_mcp    = 0.22.6
bevy_brp_extras = 0.22.6
```

上游项目：

- https://github.com/natepiano/bevy_brp
- https://crates.io/crates/bevy_brp_mcp
- https://crates.io/crates/bevy_brp_extras

Codex MCP 配置参考：

- https://developers.openai.com/docs/config-file/config-basic
- https://developers.openai.com/learn/docs-mcp

---

## 实施原则

Codex 执行本方案时仍必须遵守项目现有 `AGENTS.md` 和 `rules/`。

特别注意：

1. 修改代码前先读取项目要求的规则文件。
2. Windows 命令统一使用 PowerShell 7（`pwsh`）。
3. 不主动读取 `requirements/`。
4. 外部依赖必须由 root `Cargo.toml` 的 `[workspace.dependencies]` 统一管理。
5. 本次 BRP 能力只接入 `gallery`，不要污染 `bevy_widgetry` 库本体和其他生产 crate。
6. 不为未来假设添加额外抽象；先完成当前 Gallery GUI 调试闭环。
7. 不要求本次顺手给所有 UI Entity 添加 `Name`。Entity `Name` 可作为以后提高定位稳定性的独立优化。
8. 任何代码 / Cargo / 工程配置修改完成后，继续执行项目现有的自动 Code Review 流程。

---

# 一、安装 Codex 使用的 BRP MCP Server

## 1. 检查 / 安装 `bevy_brp_mcp`

先检查：

```powershell
bevy_brp_mcp --version
```

如果命令不存在，或版本不是 `0.22.6`，安装固定版本：

```powershell
cargo install bevy_brp_mcp --version 0.22.6
```

安装后再次确认：

```powershell
bevy_brp_mcp --version
```

期望版本为 `0.22.6`。

## 2. 添加项目级 Codex MCP 配置

在仓库根目录创建：

```text
.codex/config.toml
```

内容：

```toml
[mcp_servers.brp]
command = "bevy_brp_mcp"
args = []
```

如果 `.codex/config.toml` 已存在，则合并该 section，不覆盖其他项目配置。

优先使用项目级配置，不主动修改用户全局 `~/.codex/config.toml`。

## 3. MCP 配置生效边界

执行：

```powershell
codex mcp list
```

期望看到 `brp`。

如果当前 Codex 会话在创建 `.codex/config.toml` 后没有动态加载新 MCP，不要使用 PowerShell / Win32 GUI 自动化作为替代方案。

应完成本阶段其余文件修改并明确告诉用户：

```text
BRP MCP 已配置；请重启一次 Codex CLI，使项目级 MCP 配置生效，然后继续 BRP smoke test。
```

这是本方案允许的唯一必要人工切换点。

如果项目级配置因为项目未被 Codex 信任而未加载，应报告该条件，不要擅自改写用户全局 Codex 配置。

---

# 二、给 Widget Gallery 接入 `bevy_brp_extras`

## 1. root `Cargo.toml`

当前项目规则要求 Workspace 外部依赖统一管理，因此在 `[workspace.dependencies]` 中新增：

```toml
bevy_brp_extras = "=0.22.6"
```

不要直接在 `gallery/Cargo.toml` 中维护版本号。

## 2. `gallery/Cargo.toml`

在 `[dependencies]` 中新增：

```toml
bevy_brp_extras.workspace = true
```

最终仍保持 Gallery 作为 BRP 依赖的唯一直接消费者。

## 3. `gallery/src/main.rs`

新增 import：

```rust
use bevy_brp_extras::BrpExtrasPlugin;
```

在 `DefaultPlugins` 已加入 `App` 后，将 `BrpExtrasPlugin` 加入 Gallery 的 plugin 列表。例如：

```rust
.add_plugins((
    BrpExtrasPlugin,
    GalleryAssetPlugin,
    WidgetryWindowPlugin,
    WidgetryButtonPlugin,
    WidgetryCheckBoxPlugin,
    WidgetryComboBoxPlugin,
    WidgetryRadioGroupPlugin,
    WidgetryTextFieldPlugin,
    GalleryPlugin,
))
```

`BrpExtrasPlugin` 在 native 平台会配置 BRP 所需的 Remote / HTTP transport，并默认监听 BRP 端口 `15702`。

本次不要另行手工添加一套重复的 `RemotePlugin` / `RemoteHttpPlugin`，除非实际编译或运行证明现有组合存在必要冲突。

---

# 三、同步 `docs/architecture.md`

当前 `docs/architecture.md` 对 Gallery 的角色描述为人工体验、集成验证和展示应用。

接入 BRP 后，这一 architecture 事实发生了扩展，应同步 Gallery 小节，使其明确包含 BRP 辅助的运行时 GUI 验证能力。

建议将 Gallery 的角色描述调整为类似：

```markdown
### `gallery`

Widgetry 的实际消费者和集成展示应用，用于人工体验、BRP 辅助的运行时集成验证，以及展示当前 Widget 能力。

Gallery 接入 `bevy_brp_extras::BrpExtrasPlugin`，为 Codex CLI 提供基于 BRP 的截图、输入模拟、运行时状态检查和应用生命周期控制；该调试能力只属于 Gallery，不进入 Widgetry 库的生产依赖路径。
```

保留原有 logging、asset 等事实描述，不因本任务重写无关内容。

---

# 四、把 BRP 固化到 Codex 工作流

## 1. 修改 `AGENTS.md`

在“根据任务内容继续读取”一节增加一条路由：

```markdown
* 涉及 Widget / Gallery 的可视表现、GUI 交互、focus、picking、layout、theme 或运行时界面验证：`rules/gui-debugging.md`
```

`AGENTS.md` 只负责路由，不在这里复制具体 BRP 操作细节。

---

## 2. 新增 `rules/gui-debugging.md`

创建下面这份规则文件：

```markdown
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
2. 使用 `brp_launch` 启动 `widget_gallery`。
3. 使用 `brp_extras_screenshot` 获取当前 Gallery 界面作为基线。
4. 根据当前验证目标使用必要的 BRP 输入工具。
5. 使用截图、ECS 查询、Component / Resource 状态、日志或诊断信息检查结果。
6. 验证结束后通过 BRP 正常关闭 Gallery。

不要为了“完整”机械调用全部工具，只执行当前行为所需要的步骤。

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
```

---

## 3. 修改 `rules/development.md`

不要改变现有 Type-Driven Development / Test-Driven Development 主流程。

在开发流程说明中加入一个“按需 GUI 验证”阶段，语义应明确为条件执行，而不是所有 Rust 修改都启动 Gallery。

建议加入：

```markdown
## GUI 行为的运行时验证

当当前行为具有可观察 GUI 表现，或必须通过实际 pointer / keyboard / focus / picking / layout 等交互确认时，在相关自动化测试通过并完成必要重构后，按照 `rules/gui-debugging.md` 使用 Widget Gallery 进行运行时验证。

因此涉及 GUI 行为的开发循环可以表现为：

```text
Type
→ Red
→ Green
→ Refactor
→ BRP GUI 验证
```

BRP GUI 验证是条件阶段。纯逻辑修改或能够完全由自动化测试覆盖、且不改变 GUI 可观察行为的修改，不需要为了流程形式启动 Gallery。
```

保留 Gallery 不要求 TDD 的现有例外。

---

## 4. 修改 `rules/testing.md`

替换现有 `### Gallery 例外` 内容，使其从“人工运行确认”升级为“运行时验证，Codex 默认使用 BRP”。

建议改为：

```markdown
### Gallery 例外

`gallery/` 仅用于 Widget 展示，不要求编写 unit test、integration test 或其他自动化测试。

对 `gallery/` 的修改应保证能够正常编译。

涉及展示效果或交互行为时，应按照 `rules/gui-debugging.md` 通过 Widget Gallery 进行运行时验证；BRP 是 Codex 执行此类验证的默认方式。

本例外优先于本文件中的一般测试义务。
```

不要修改其他与本任务无关的测试规则。

---

# 五、首次 BRP Smoke Test

MCP 在新的 Codex CLI 会话中成功加载后，执行一次最小端到端验证。

## 1. MCP 可见性

确认：

```powershell
codex mcp list
```

存在 `brp` server。

## 2. 发现 Gallery

调用：

```text
brp_list_bevy
```

应找到：

```text
widget_gallery
```

## 3. 启动 Gallery

使用：

```text
brp_launch
```

启动 `widget_gallery`。

默认 BRP port 为：

```text
15702
```

## 4. 截图验证

调用：

```text
brp_extras_screenshot
```

确认能够获得 Gallery 的有效 PNG，而不是依赖 Windows 桌面截图。

## 5. 输入验证

至少完成一次简单、无破坏性的鼠标输入链：

```text
brp_extras_move_mouse
→ brp_extras_click_mouse
→ brp_extras_screenshot
```

根据第一次 screenshot 选择一个现有、可安全交互的 Gallery 控件；不要在方案中硬编码屏幕坐标。

如果当前控件的交互结果可以通过 Component / Resource / 日志精确确认，则同时使用 BRP 结构化查询确认。

## 6. 关闭 Gallery

使用 BRP shutdown 能力正常结束应用。

Smoke test 成功意味着以下链路成立：

```text
Codex CLI
    ↓ MCP stdio
bevy_brp_mcp
    ↓ BRP / localhost
Widget Gallery + BrpExtrasPlugin
    ↓
Bevy screenshot / input / ECS state
```

---

# 六、工程验证

完成文件修改后执行项目必要验证：

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
```

随后执行上面的 BRP smoke test。

如果因首次新增 `.codex/config.toml` 导致 MCP 必须重启 Codex CLI 后才能出现，则：

1. 先完成编译 / 测试与所有静态文件修改；
2. 明确报告需要重启一次 Codex CLI；
3. 用户重启后继续 smoke test；
4. 不把“未执行 GUI 验证”伪装成已经验证完成。

---

# 七、Code Review

本任务会修改 Rust、Cargo 和工程配置，因此必须遵守当前 `AGENTS.md` 的自动 Code Review 流程：

1. 实施完成并执行必要验证；
2. 启动新的 reviewer subagent；
3. reviewer 调用 `$code-review` Skill 审查完整 working-tree change；
4. 如果有 findings，由原施工 agent 修改；
5. 必要时启动全新的 reviewer 进行第二轮；
6. 最多两轮，按现有项目规则处理剩余 findings。

BRP smoke test 应在最终报告中单独说明是否成功，不应被 Code Review 结果替代。

---

# 八、完成标准

只有同时满足以下条件，才视为本方案实施完成：

- `bevy_brp_mcp 0.22.6` 已安装并可执行；
- 项目级 `.codex/config.toml` 已配置 `brp` MCP；
- root Workspace 统一声明 `bevy_brp_extras = "=0.22.6"`；
- `gallery` 使用 workspace dependency；
- `Widget Gallery` 注册 `BrpExtrasPlugin`；
- `docs/architecture.md` 同步记录 Gallery 的 BRP 运行时验证角色；
- `AGENTS.md` 能把 GUI 任务路由到 `rules/gui-debugging.md`；
- `rules/gui-debugging.md` 已建立；
- `rules/development.md` 包含条件性的 BRP GUI 验证阶段；
- `rules/testing.md` 的 Gallery 例外已改为 BRP 默认运行时验证；
- `cargo fmt / check / clippy / test` 通过，或失败原因被明确报告；
- 在加载 MCP 的 Codex 新会话中完成 `发现 → 启动 → 截图 → 输入 → 再截图 / 状态确认 → shutdown` 的 BRP smoke test；
- 按项目规则完成自动 Code Review；
- 没有把 BRP 依赖引入 Widgetry 库的生产 crate；
- 没有把 PowerShell / Win32 GUI 自动化作为 BRP 失败后的静默 fallback。

---

# 九、明确不在本次范围内

本次不要顺手做以下工作：

- 不给全部 Gallery / Widget Entity 批量添加 `Name`；
- 不设计新的 Widget 测试框架；
- 不把 BRP 能力抽成新的 Workspace crate；
- 不引入通用 Windows Computer Use / Win32 automation；
- 不修改 Widgetry 公共 API 仅为了方便 MCP；
- 不重新设计现有 TDD / Code Review 规则；
- 不把 BRP GUI smoke test 包装成 CI，除非另开任务专门设计。

本方案只负责建立：

```text
Codex CLI → BRP MCP → Widget Gallery
```

这一条稳定、可重复的本地 GUI 调试与验证路径。
