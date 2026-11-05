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
- `title.rs`：长标题单行裁剪、窄窗口下关闭按钮保留尺寸并位于窗口范围内，标题栏保持 36 逻辑像素高；标题栏 Popout 向下超出标题栏仍不被裁剪。
- `owned_window.rs`：Owned Lifecycle、Modal 资源隔离、Focus / IME。
- `owned_window.rs` 同时保护复合 Root 的 InitialFocus Priority 与实际输入目标之间的路由，以及映射后的 Tab 来回导航。
- 显式清空 Focus（含隐藏旧页面）后，普通键与快捷键 Release 不恢复旧 Editor，也不选择新 Editor；Tab 仍显式进入可见 Tab Stop。
- `background.rs`：Theme / Image、Opacity、Cover 与实际 Layout 时序。
- `title.rs`：显式标题标记、额外文本 / 正文隔离、多窗口和嵌套窗口、无绑定、延后构造与 Native 更新、重复同值、无效 / 歧义绑定的原子拒绝、排队执行期验证及持续错误 / 恢复；正常 WindowClosing→延迟 WindowClosed 不报故障，不带关闭信号的 Native 丢失仍传播 Error，其他窗口继续同步。
- 源码 Module Unit Test 负责更细的 Resize Mapping、Modal 关系和 Native Request。
- `runtime_error` 源码 Unit Test 保护 TitleState / WindowStyle 丢失后的外置诊断身份、补建后的真实恢复与 Root 销毁释放；状态被移除或补建会使标题重新校验。
- `native_icon` 源码 Unit Test 保护 PNG / RGBA 的具体 source 与路径 / 格式 / 尺寸、真实 System 的主窗口缺失 / 歧义传播、相同集合的共享 origin 和不同集合的独立 origin。等待 native window 不产生错误或恢复；隐藏的真实 winit 窗口在所属线程应用 Window / Taskbar Icon 后仅记录一次关联恢复，已应用后不重复查询。此用例只验证设置调用及生命周期，不证明任务栏实际显示的像素。
- `render` 源码 Unit Test 保护 adapter 错误适配与真实 DX12 device 请求失败的原生 source、backend / adapter 现场及 ECS 转换后的原始诊断，不代表透明窗口完整渲染或 Present 验收。
- 样式源码 Unit Test 保护稳定帧零重复 Palette / 部件应用、单按钮 Hover / Pressed / Disabled 添加与移除的局部工作量、托管最终 Palette 与 Owner 移除、Theme / Override、内容身份、部件重挂与销毁、业务子树 / 嵌套 Window 边界及失败重试 / 恢复；缺失输出不阻塞其他窗口，错误保持宿主 Error 级别。

## 覆盖边界

`diagnostics.rs` 验证非法绑定的全部领域失败抵达真实 Handler、Borrowed 资源保留；Owned 相机绑定被移除后仍回收实际分配。多个标题 / 样式故障各自复用原始 origin，真实修复记录对应恢复、销毁不伪恢复、再次故障产生新 ID；三个歧义标题的 archetype 次序变化不替换诊断。非法嵌套配色保留 Core cause / 原始 origin 位置与部件 / 状态现场，失败不提交。部分 Owned / Borrowed Scene 的下层诊断、普通 / Panic Severity 和 payload 保留，失败不删除外部 Native / Camera，也不遗留 Owned 分配。

Headless 测试不能证明原生窗口移动、Resize 或最大化是否在 OS 侧成功；依赖 `WINIT_WINDOWS` 的 Native Maximize Guard 未由现有 Headless 测试完整验证。

