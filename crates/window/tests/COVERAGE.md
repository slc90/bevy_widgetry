# window Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Borrowed / Owned Root、Native Window / Camera 绑定、Modal Child Count。
- Theme / Image Background、Image Ready / Pending、Layout / Opacity。
- Focus / IME Qualification 与 Window Lifecycle。

## Stimuli

- Native Window Preparation、Scene Spawn / Despawn、WindowClosed。
- Modal Child 创建 / 结束、Theme 切换、Asset 加载、真实 Layout。
- Focus / IME Mode、Window 属性与错误配置。

## Guards

- Native Window 与 Camera 必须有效且唯一绑定。
- 借用资源不得被 Root Cleanup 删除。
- Cover 仅在图片和 Layout 尺寸有效时更新。

## Transitions

- Native / Camera 初次绑定；重复或非法绑定拒绝。
- Owned Root 结束清理自有资源；Modal 最后一个 Child 结束释放 Blocker。
- Theme / Image 背景投影、Cover 裁剪、IME Mode 改变时取消旧 Composition。

## Invariants

- 不同 Window 的 Owner / Camera / Entity 集合隔离。
- Theme 与 Image 背景互斥；Image Opacity 不复合积累。
- Image 未就绪不强行渲染 Theme 背景。
- 失效 Native / Duplicate Camera 不破坏既有有效绑定。
- Cover 在当前帧 Layout 后居中裁剪，Asset 首次就绪即使没有新的 Layout 变化也会同步；无效尺寸恢复后重新裁剪，空闲 Frame 不重写 ImageNode。
- 内容 Host 的局部 Disabled 独立决定 Frame 前景，Color Override / Theme / 恢复遵守该状态。
- Embedded / Managed Window 禁用时保留 Focus 和路由但不 Activate，隐藏时迁移 Focus；TextInput / 普通 EditableText、Editable / ReadOnly / Static 的 IME Qualification 分别覆盖，Mode 改变取消旧 Composition。

## Couplings

- Window Lifecycle × Camera Ownership / Modal Blocker。
- Image Readiness × UI Layout / Cover。
- Focus × Modal / Disabled / IME。

## Coverage Map

- `window.rs`：Native 属性、绑定、Camera / Scene Slot、Controls 与 Theme。
- `owned_window.rs`：Owned Lifecycle、Modal 资源隔离、Focus / IME。
- `background.rs`：Theme / Image、Opacity、Cover 与实际 Layout 时序。
- 源码 Module Unit Test 负责更细的 Resize Mapping、Modal 关系和 Native Request。
- 样式源码 Unit Test 保护稳定帧零重复 Palette / 部件应用、单按钮 Hover / Pressed / Disabled 添加与移除的局部工作量、托管最终 Palette 与 Owner 移除、Theme / Override、内容身份、部件重挂与销毁、业务子树 / 嵌套 Window 边界及失败重试 / 恢复；缺失输出不阻塞其他窗口，错误保持宿主 Error 级别。

## 覆盖边界

Headless 测试不能证明原生窗口移动、Resize 或最大化是否在 OS 侧成功；依赖 `WINIT_WINDOWS` 的 Native Maximize Guard 未由现有 Headless 测试完整验证。

