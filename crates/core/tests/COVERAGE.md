# core Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- UI 构造期 / Materialize / 内容消费阶段。
- Font 默认 / 显式、Icon 未就绪 / 就绪与当前请求。
- Hierarchy 的本地 / 祖先 Disabled、Pointer 直接 / 后代 Hover、Scene 成功 / 失败。

## Stimuli

- BSN Scene Spawn / Apply、Theme、Foreground Reparent、Pointer Input / Hover Map。
- Insert / Remove / Flush / Commands、Asset 就绪与更新。

## Guards

- Icon Scene 必须提供非空初始路径；缺省和显式空路径均以 Error Severity 拒绝，并记录错误。
- 当前 Scene 操作只回收其新建预约空 Entity。
- Disabled OR 继承不能把祖先结果误记为本地请求。
- 以理由类型区分的内部禁用按 OR 组合，同值或撤销单一理由不会覆盖其他理由。
- Hover 只包括当前有效 Pointer / Window 命中。
- 原生 Mouse 失焦按有序输入取消，不制造普通松手 Button 的 Activate；重新聚焦后的真实点击仍可激活，三个鼠标键都遵循原生接受语义。

## Transitions

- Font Fallback 应用于适用的新 TextFont；显式配置保留。
- Scene 失败报告并清理本次预约，既有业务实体不回滚。
- Pointer 生命周期变化刷新 Hover；Disabled 变更在用户输入前生效。

## Invariants

- 原生 Text 不被 Widgetry 标记颜色修改。
- Foreground Source Identity / Theme / Disabled 与最终内容颜色一致。
- 图标异步请求只消费当前 SVG；新 Image 子树稳定、不拦截 Picking。
- 空闲 Frame 不无意义地重写已稳定的状态。
- UI 阶段顺序保证动态内容当帧完成必要准备。
- 字体 Fallback 仅处理新增且等于 Bevy 默认 Sentinel 的 TextFont；注册顺序保留显式配置，运行期改配置不重写既有文本。
- Icon 等待新 SVG 时保留旧图，只有当前请求就绪且尺寸非零才替换；共享像素不共享颜色与 Entity 生命周期。
- Nested / Forward Reference Scene 失败保留外层预约和业务副作用，连续失败不累计预约空实体，也不重复记录已记录错误。

## Couplings

- Disabled × 输入派发；Foreground × UI Hierarchy / Icon / Text。
- Pointer ID × Hover 命中归属；Scene 错误 × Host Handler / Entity Lifecycle。

## Coverage Map

- `color_schedule.rs`、`ui_schedule.rs`：System 顺序与动态内容准备。
- `default_font.rs`：默认字体策略。
- `content_colors.rs`：Foreground / Text / Theme / Disabled。
- `effective_disabled.rs`、`disabled_input.rs`：有效 Disabled 及官方输入阻止。
- `pointer.rs`：有效多 Pointer Hover 投影。
- `icon.rs`：初始路径校验、非空路径未加载时的构造、公开 Icon API、真实 Loader 和颜色 / 生命周期。
- `scene.rs`：成功 / 失败的同步与 Deferred Scene 组合及资源清理。

## 观察边界

`ui_schedule.rs` 在字体已就绪后验证 Build / Materialize 两个合法阶段的新建与替换内容；只推进一次 Update，检查 Fallback、Visibility、Stack 和真实 Measurement，不用额外等待掩盖当帧依赖。Disabled 投影在 Pointer / Keyboard Dispatch 前完成，Foreground 属于官方 Propagate，内容配色先于文字检测与 Content。

`pointer.rs` 的手工 HoverMap 只验证投影；真实 Backend 输入属于 test_utils Pipeline 与控件组合测试。Disabled 继承期间本地请求、Late Plugin、Detach / Reparent 和 Flush 都属于实际覆盖的生命周期输入。

