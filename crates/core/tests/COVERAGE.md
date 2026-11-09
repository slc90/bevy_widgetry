# core 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- UI Scene 构造、Materialize 与内容消费阶段；默认／自定义字体、Icon 资源请求和当前 Theme。
- 本地／祖先／内部 Disabled、Pointer Hover、嵌套 Foreground 和 Scene 成败状态。

## Stimuli
- BSN Spawn／Apply、Hierarchy Reparent、Theme／Foreground 变化、Pointer／Keyboard、Asset Ready 与同步／排队命令。

## Guards
- Icon 初始路径必须有效且非空；Scene 失败只回收本次新建的预约实体，不破坏既有业务实体。
- Disabled 不丢失独立的本地／内部原因；Hover 只反映有效 Pointer／Window 命中。
- 鼠标失焦／取消遵循原生输入顺序，不伪造 Activate。

## Transitions
- 默认字体准备与动态 Text 投影；Icon 请求、等待、成功或失效；Disabled／Hover 更新与 Scene 失败清理。

## Invariants
- Widgetry 前景投影不擅自修改普通 Bevy Text；Foreground、Theme、Disabled 与最终内容颜色一致。
- 字体 fallback 只接管符合默认条件的新 TextFont，保留显式字体和宿主初始化顺序。
- 异步 Icon 只提交当前请求的有效 SVG；未就绪时保留旧内容，图像子树不拦截 Picking。
- 同帧调度完成必要的 Font、Visibility、Layout 和内容消费；稳定帧不持续改写状态。
- Scene／State／Offscreen 错误保留原因、origin 和当前操作上下文；持续故障去重，实际修复后恢复，互不相关的业务实体保持完整。

## Couplings
- Disabled × 输入分发；Foreground × 文字／Icon／Hierarchy；Pointer Hover × 多窗口；Scene 失败 × Handler／实体生命周期。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `color_schedule.rs` | 前景配色系统次序与动态内容更新。 |
| `content_colors.rs` | Text／Icon 的 Foreground、Theme 与 Disabled 投影。 |
| `default_font.rs` | 内建字体、fallback 语义与宿主显式字体保留。 |
| `disabled_input.rs` | 有效 Disabled 在真实 Pointer／Keyboard 中的守卫。 |
| `effective_disabled.rs` | 本地、祖先与内部 Disabled 理由组合和恢复。 |
| `icon.rs` | Icon 公开 API、异步资源与布局／生命周期。 |
| `icon_diagnostics.rs` | Icon 输入校验、请求身份及宿主错误传播。 |
| `offscreen_diagnostics.rs` | Offscreen 资源分配／回收故障、诊断和隔离。 |
| `persistent_failures.rs` | 持续错误身份、恢复语义及多 Owner 并存。 |
| `pointer.rs` | 多 Pointer 的 Hover 投影与失效清理。 |
| `scene.rs` | 同步／排队 Scene 构造失败和预约实体回收。 |
| `scene_diagnostics.rs` | 嵌套 Scene 错误包装、源头身份和传播。 |
| `state_diagnostics.rs` | 颜色、Disabled 和状态 API 诊断。 |
| `ui_schedule.rs` | 真实 UI 调度先后与同帧准备。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/font.rs` | 默认字体注入与内容消费时序。 |
| `src/icon.rs` | 图像尺寸、资源更新与局部边界。 |
| `src/offscreen.rs` | Offscreen 层号和资源回收的局部错误处理。 |
