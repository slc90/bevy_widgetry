# tabs 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 唯一 Selection、Header Checked、对应 Page Display、Focus、Disabled、Theme 和 Header 滚动位置。

## Stimuli
- 固定／动态 BSN 配对、公开选择 API、真实 Pointer／Keyboard、Page／Header 结构变化、Theme 和布局 Resize。

## Guards
- Root、配对集合和 Index 合法；已初始化配对关系损坏与初始空集合应按不同错误处理。
- Disabled Header／Root 阻止用户切换，但程序化选择仍可用；非对应方向键不修改状态。

## Transitions
- 初始化 → 同步／排队改选 → Header／Page 投影；切换时隐藏 Page 焦点回收、滚动定位与配对错误修复。

## Invariants
- 同一 Tabs 只有一个 Header 选中、一个 Page 显示；切换不销毁页面实体、正文或应用滚动状态。
- Selection、Checked 和 Display 同帧完成，通知可读已提交状态；合法同值静默，失效请求不部分提交。
- 隐藏页面的旧输入焦点不能继续消费键盘；切回页面不自动抢回此前的编辑焦点。
- Theme／Override、Header 自然／统一尺寸、溢出滚动与当前选择相互独立；当前 Header 可见定位不应强行覆盖用户手动滚动。
- Scene／结构／配色错误保留原始 Root、配对关系及诊断身份，多实例故障隔离。

## Couplings
- Selection × Header／Page Visibility；Focus × Keyboard／隐藏页面；Theme × Header Layout；滚动 × 选中可见定位。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 无效配对、Source／样式结构和 Command 错误传播。 |
| `keyboard.rs` | 两轴方向键、Focus、Disabled 与通知时序。 |
| `scrolling.rs` | Header 溢出滚动、选中可见定位和缩放／Resize。 |
| `selection.rs` | 场景配对、公开选择、同步／队列操作及实例隔离。 |
| `style.rs` | Header／Page 样式、布局、文本裁剪与 Theme／Override。 |
| `text_input.rs` | 页面切换对真实 TextField／Keyboard Focus 的影响。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/selection.rs` | 缺失状态或配对部件时的结构错误分类、Root／Header／Part 身份。 |
| `src/scrolling.rs` | 多个 Header 布局部件失效时的诊断归属与真实 System 错误聚合。 |
