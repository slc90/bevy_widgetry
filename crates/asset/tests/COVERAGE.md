# asset Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Embedded Source 未注册 / 已注册。
- Font、SVG、Cursor 静态和动画图集未加载 / 取得字节 / 已解码。

## Stimuli

- 注册 `WidgetryAssetPlugin`；通过 `BuiltinFont`、`BuiltinIcon`、`BuiltinCursorImage`、`BuiltinCursorAnimation`、`BuiltinShader` 加载与解码。

## Guards

- 素材必须经 Embedded Source 寻址，不从磁盘回退。

## Transitions

- Font 成功解析；SVG 使用各自语义标识。
- 45种静态备用与动画第一帧的图像对应。

## Invariants

- 45种Cursor素材与语义对应，均为80px RGBA，边界透明，完全透明像素无残余RGB。
- 45种Atlas与RGBA数据合法，各自动画非静态，首帧身份独立；3×3热点区域全帧固定，独立时序在帧边界和Duration::MAX下循环有效。
- 内建 Font / Icon / Cursor 路径确实指向可读取资源。
- DatePicker Today SVG 通过专用 Icon 语义标识读取真实 Embedded 字节。

## Couplings

- Path 标识、Embedded 注册与实际解码协作。

## Coverage Map

- `embedded.rs`：全部 Embedded Font、Icon、Cursor 与 HoldButton Shader 资源 Contract。


内建 HoldButton 与 Canvas Alpha 转换 Shader 都通过语义路径读取 Embedded Source，验证实际字节可加载。
