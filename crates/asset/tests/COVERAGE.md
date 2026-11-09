# asset 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Embedded Source 未注册／已注册；内建 Font、Icon、Shader 和 Cursor 等资源的寻址与解码状态。

## Stimuli
- 注册 `WidgetryAssetPlugin`，通过 `BuiltinFont`、`BuiltinIcon`、`BuiltinShader` 等语义入口读取内嵌资源。

## Guards
- 内建素材必须经 Embedded Source 读取，不依赖磁盘回退；路径应指向有效、可解码的数据。

## Transitions
- 注册 Source → 根据语义标识获得字节 → 解析字体、SVG、Shader 或光标资源。

## Invariants
- 内建字体、导航／搜索／日历 Icon、HoldButton 与 Canvas Shader 的语义路径对应实际资源。
- Cursor 静态后备与图集素材的身份、像素、透明边缘、热点和帧时序满足既有素材契约。
- 字体 fallback 所需资源保持可获取，素材身份不因资源读取方式而改变。

## Couplings
- 语义路径 × Embedded Source 注册 × 解码结果；Cursor 静态后备 × 动画图集。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `embedded.rs` | 内建 Font、Icon、Cursor、Shader 的路径、字节、格式及素材契约。 |
