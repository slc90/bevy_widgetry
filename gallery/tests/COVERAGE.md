# gallery 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 构建时 Icon 资源有效／缺失／无效；Gallery 内建 Cursor 素材和完整主题映射。

## Stimuli
- 复用正式 `build.rs` 的临时应用构建，资源编译器失败／恢复、角色图集与 Cursor Theme 变化。

## Guards
- 缺失／非法 ICO 和不可用资源编译器必须报告构建失败；失败不伪造可用 EXE 图标。

## Transitions
- 资源校验 → Windows 图标编译 → EXE 资源读取；Cursor Theme 从语义声明映射到静态／动画素材。

## Invariants
- 正常构建生成真实 Win32 `GROUP_ICON` 资源；失败保留源头诊断与终结报告。
- Gallery 的 Cursor 素材、固定热点、帧身份、系统／Hint 映射及时间相位符合内建主题契约。
- Gallery 专有素材由 Gallery 自身保护，通用 Cursor 库不依赖这些具体角色数据。

## Couplings
- 构建资源 × Win32 EXE 图标；Cursor 素材 × Theme 语义／动态生命周期。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `build_icon.rs` | 正式 Gallery 图标构建的成功与失败、EXE Win32 资源契约。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/assets/cursors/tests.rs` | 完整角色图集的像素、热点、备用帧和资源身份。 |
| `src/cursor_theme/tests.rs` | 系统／Hint 映射、Lease／动画时序与资产故障恢复。 |
