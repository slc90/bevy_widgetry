# window 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Borrowed／Owned Window Root、Native Window／Camera 绑定、Modal Child Count、Focus／IME 和关闭生命周期。
- Theme／Image 背景、Asset Ready、Opacity、Cover 与实际 Layout。

## Stimuli
- Native 准备、BSN Spawn／Despawn、WindowClosed、Modal Child、Theme／Override、图片加载、Focus／IME 和标题修改。

## Guards
- Native Window／Camera 绑定必须有效且唯一；Borrowed 资源不可被 Root Cleanup 误删。
- Cover 仅在图片资源和布局尺寸有效时更新；失效／重复绑定不得破坏既有健康实例。

## Transitions
- Window 与 Camera 绑定／取消、Owned Root 生命周期、最后 Modal Child 释放 Blocker。
- Theme／Image 切换、资产加载后 Cover 裁剪、Focus／IME 模式更新和旧组合态取消。

## Invariants
- 不同 Window 的 Owner、Camera、Focus 和资源独立；关闭 Owned Root 只清理自有资源。
- Theme 与 Image 背景投影不冲突，Opacity 不重复累乘；图片未就绪和无效布局时不强行投影错误背景。
- Cover 根据实际 Layout 居中裁剪，在资产就绪／布局恢复时更新；稳定帧不反复改写 ImageNode。
- Disabled／Hidden／Modal 影响真实输入和 Focus，但不改变合法业务状态；IME 对可编辑、只读与静态目标的资格正确。
- 标题绑定、颜色 Owner／Override、动态托管部件的恢复不修改正文或其他窗口；错误保留原始诊断及请求身份。

## Couplings
- Window Lifecycle × Camera／Modal Ownership；Image × Layout／Cover；Focus × IME／Disabled；Theme × 动态 Title／Body。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `background.rs` | Theme／Image 背景、Opacity、Cover 与布局时序。 |
| `diagnostics.rs` | Native／Camera／标题／颜色结构故障和宿主传播。 |
| `owned_window.rs` | Owned／Modal 生命周期、Focus／IME 与资源归属。 |
| `title.rs` | 标题绑定、截断、动态更新与窄窗口布局。 |
| `window.rs` | Native 属性、Scene／Camera 绑定、控件及 Theme。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/style.rs` | 托管色板、部件 Owner、Theme／Override 和故障恢复。 |
| `src/title.rs` | 标题投影、绑定身份与动态变化。 |
| `src/input.rs` | Focus／IME 资格及输入清理的内部规则。 |
