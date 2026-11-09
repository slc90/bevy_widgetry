# button 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Normal、Hovered、Pressed、Disabled、Loading；本地／祖先／内部 Disabled 原因、Theme 与局部颜色覆盖。
- Headless／默认内容结构、LinkButton URL 原文和受管理文字／Icon。

## Stimuli
- BSN 构造、真实 Pointer／Keyboard 输入、Loading／Disabled 改变、Theme／Override 更新及公开 URL API。

## Guards
- 有效 Disabled 阻断用户 Activate，合法程序化更新仍允许；无默认 Style 不强制创建视觉子树。
- 非法 URL、缺失运行时能力和错误目标必须拒绝并保持原业务状态。

## Transitions
- Normal ↔ Hover ↔ Pressed ↔ Disabled；Loading 增减与其他禁用理由独立。
- Link URL／文字同步与排队修改，真实输入触发一次链接打开或普通 Activate。

## Invariants
- 配色优先级为 Loading > Disabled > Pressed > Hover > Normal；Loading 使用 Normal 配色，恢复时按剩余禁用理由和交互状态投影。
- 官方 Activate 的 Mouse／Keyboard 语义、取消／拖出边界与焦点归属不重复；外部原生 ButtonPlugin 不造成双重激活。
- 颜色更新同步投影到受管理内容，保持调用方布局与 Children；Link URL 保留输入原文，非法请求不部分提交。
- 链接打开失败保留原始错误来源及诊断，不访问真实网页。

## Couplings
- Loading × Disabled／Pointer；Theme／Override × 内容前景；URL／Text × 真实链接输入与队列时机。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 状态、颜色和托管部件的领域诊断与 Scene／Command 传播。 |
| `headless.rs` | 无默认视觉的 Button、真实 Picking／Keyboard、Disabled 和激活契约。 |
| `link_button.rs` | 链接文字、URL 提交、样式和真实布局。 |
| `link_diagnostics.rs` | 非法链接及颜色请求的原因链与诊断身份。 |
| `loading.rs` | Loading 生命周期、禁用理由、动画／内容与输入守卫。 |
| `open_url.rs` | Headless 链接组合、公开 URL 更新和非法请求拒绝。 |
| `optional_loading.rs` | 独立 Headless + Loading API、通知、输入和错误处理。 |
| `widgetry_button.rs` | 默认 Button 的交互状态、Theme／颜色覆盖和内容传播。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/open_url.rs` | 私有链接打开边界、输入去重、执行时 URL 与原生失败。 |
| `src/link.rs` | 链接托管内容失效时的样式失败聚合与提交原子性。 |
| `src/style.rs` | 局部配色优先级及样式更新契约。 |
