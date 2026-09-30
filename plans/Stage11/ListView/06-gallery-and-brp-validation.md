# 完成 Gallery 与 BRP 验证闭环

## 目标

把 ListView 作为 facade 的真实消费者接入 Widget Gallery，用实际运行中的 UI 检验前五个方案的组合效果，尤其验证第一版“无 overscan”的实际体验、hidden scrollbar、virtualization entity 数量、disabled 输入拦截以及 Theme 切换。

这一阶段不再发明新的 ListView 核心语义；发现问题时回修对应 contract。

## Gallery 接入

Gallery 增加 ListView page、导航入口以及一个用于 demo 的业务 item type，并在 Gallery Plugin/app build 中调用：

```text
register_widgetry_list_view::<DemoItem>()
```

Gallery 继续只通过 `bevy_widgetry` facade 使用 ListView，不直接依赖 `bevy_widgetry_list_view`。

页面按四类组织。“Small List”并不是另一种 non-virtualized mode，只是数据量小到：

```text
visible range == 0..len
rendered range == 0..len
```

底层仍走完全相同的 virtualization。

## 1. Small List

使用少量 item，使其全部能够进入 viewport。

### UI 操作

实际展示并可操作：

- click row；
- click renderer descendant；
- hover / pressed；
- ArrowUp / ArrowDown；
- Home / End；
- Space / Enter；
- focus 与 active；
- selected 与 active 分离。

页面可显示当前：

```text
selected id / 当前 index
active id / 当前 index
```

用于肉眼确认“keyboard 只移动 active，Space/Enter 才提交 selected”。

### Programmatic 操作

提供 Gallery Button 驱动公开 API / model API，例如：

- `set_selected()`；
- update 当前某个 item；
- insert；
- remove；
- move；
- clear + repopulate。

这些操作用于观察 id/revision/selection/active 在正常小列表里的行为，不新增 Gallery 专用 ListView API。

### Style

能直接看到：

- root border/radius/focus；
- row normal；
- hovered；
- pressed；
- selected；
- active border。

Scrollbar 应始终不可见。

## 2. Virtualized Large List

使用约 10,000 items，viewport 只容纳少量 rows。

页面展示运行时可观察信息：

```text
Model items:    10000
Rendered rows:  N
Rendered range: a..b
```

Rendered rows/range 通过 public `WidgetryListViewItem` + hierarchy 查询得出，不暴露 private runtime。

### UI 操作

BRP/人工实际验证：

- 快速 mouse wheel / trackpad scrolling；
- PageUp / PageDown；
- Home / End；
- keyboard active 跨 viewport 自动滚动；
- window/list viewport resize。

重点观察第一版无 overscan 时是否出现明显边缘空白。

### Programmatic 操作

至少展示：

- 跳到远距离 index，例如 `set_selected(..., 7500)`；
- 修改 visible item，只应看到对应 row 内容变化；
- 修改 offscreen item，当前 UI 不生成该 row；滚入后显示最新值；
- insert/remove/move 后继续滚动；
- model shrink 时当前位置能 clamp 到合法 scroll range。

如需要展示 renderer call count，可以把计数作为 Gallery **自身 demo state**，但不为了 Gallery 给 library 暴露 renderer internal counter。

### Virtualization 可观察目标

10,000 model items 时 ECS 中 `WidgetryListViewItem` 数量应接近 viewport 实际可见行数，而不是 10,000。

Resize 后 rendered count/range 应随真实 viewport height 变化。

首次进入页面时同时观察 bootstrap 是否有明显一帧空白；若真实可见，再回到 virtualization 方案处理，而不是当前先加 workaround。

## 3. Disabled ListView

整个 root 挂 `InteractionDisabled`。

### UI

真实输入应被阻止：

- click 不改 active/selected；
- press 不出现 pressed behavior；
- keyboard navigation/selection 不工作；
- wheel/trackpad 不产生用户 scroll。

### Programmatic

仍然允许：

- `set_selected()`；
- programmatic ScrollPosition；
- model update/insert/remove/move/clear。

用这一场景明确展示 `InteractionDisabled` = 禁止用户输入，而不是冻结 component/model。

### Style

- root disabled border；
- rows foreground_disabled；
- hover/pressed/active visual 被 disabled priority 覆盖；
- 若逻辑上已有 selected/active，不要求清掉 state。

## 4. Disabled ListViewItem

Model 中固定若干 entry disabled，并提供 programmatic enable/disable Button。

### UI

- Arrow 可以让 active 落在 disabled item；
- click disabled item -> active 改变、selected 保持；
- Space/Enter 在 disabled active 上 no-op；
- enable 后恢复正常用户 selection。

### Programmatic

- `set_selected(disabled_index)` 仍允许；
- 切换 `set_disabled()` 只改变 interaction/style，不应导致 renderer content 重建；
- 滚出再滚回保持 disabled metadata。

### Style

Disabled row 使用 disabled foreground、transparent background，并 suppress active border/hover/pressed visual。

## Theme 验证

Gallery 已有全局 Dark/Light selector，不再给 ListView 页面复制 theme 控件。

四类场景都要观察 ThemeChanged，尤其 Large List 要验证两条路径：

1. row 已经 rendered -> 切 Theme -> 当前 row 立即换色；
2. 先切 Theme -> 再滚动让新 row 第一次生成 -> 新 row 直接使用当前 ThemeMode。

第二条用于防止“只有历史上收到 ThemeChanged 的 entity 才有新主题”这种虚拟化 bug。

## 自动化测试最终收口

Gallery 自身按现有 `rules/testing.md` 例外不写 unit/integration test，但在进入 BRP 前，要确认前五阶段的自动化测试矩阵已经完整覆盖 public behavior。

最终 integration matrix 至少能够证明：

- generic BSN + generic registration；
- ScrollArea viewport/content/hidden bar/keyboard ownership；
- Small List 全部显示；
- Large List 只生成 visible rows；
- overlap scroll 保留 overlap entity；
- visible revision 精确 rerender；
- offscreen update 延迟到滚入；
- insert/remove/move 与 stable id；
- click/keyboard/ValueChange id；
- programmatic silent selection + ensure visible；
- root disabled；
- item disabled；
- root/row style；
- ThemeChanged existing/new row；
- 至少一条真实 UI layout integration path。

这不是要求把每个 Gallery 按钮机械复制成测试，而是保证所有 library contract 都已经有自动化回归保护。

## BRP runtime 验证

严格按现有 `rules/gui-debugging.md`：

1. 发现并启动 `widget_gallery`；
2. 获取截图基线；
3. 打开 ListView page；
4. 根据当前场景发送真实 mouse/keyboard/scroll input；
5. 用截图检查可视结果；
6. 用 ECS query/component state 检查 selection/active/rendered rows/disabled 等可结构化确认的信息；
7. 验证 Theme；
8. 正常关闭 Gallery。

不要为了“流程完整”机械调用所有 BRP tool，只执行当前验证目标需要的输入和查询。

重点 runtime 判断：

- 无 overscan 是否实际出现空白；
- 首次 layout bootstrap 是否肉眼可见；
- resize 后 range 是否及时更新；
- 10k model 时 rendered entity 数量是否正确；
- hidden scrollbar 下 wheel/keyboard/programmatic scroll 是否符合 contract；
- root/item disabled 是否真实阻止用户输入；
- Theme 对 existing/new rows 是否一致。

如果无 overscan 实测良好，保持第一版设计；只有出现真实问题才回到第三方案调整。

## 工程完成条件

实现结束执行项目要求的必要验证，包括受影响 crate 与 workspace 的：

- fmt；
- check；
- clippy；
- build；
- test；
- Gallery compile/run；
- BRP GUI validation。

任何代码相关改动最后按 `AGENTS.md` 执行独立 code review：新的 reviewer subagent 调用 code-review skill，只返回 findings；原施工 agent 修复后再用全新 reviewer，直到无 findings。

`docs/architecture.md` 必须与最终 workspace member、facade 与 dependency graph 同步。

`rules/testing.md` 的进一步优化已明确留作独立任务，本次不顺手修改。

## 预期产出

得到一个在真实 Gallery 中可操作、可观察、可通过 BRP 验证，并且 library 自动化测试、workspace architecture 与 review 流程都闭合的 ListView。

## 与前后方案的关系

这是唯一执行链的最后一段，不新增核心 semantic。任何 runtime 发现都应回到其所属 contract 修正，例如 virtualization 问题回第三方案、input/disabled 问题回第四方案、theme/style 问题回第五方案，而不是在 Gallery 中打补丁。
