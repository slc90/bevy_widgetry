# asset Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Embedded Source 未注册 / 已注册。
- Font、SVG、Cursor 静态和动画图集未加载 / 取得字节 / 已解码。

## Stimuli

- 注册 `WidgetryAssetPlugin`；通过 `BuiltinFont`、`BuiltinIcon`、`BuiltinCursorImage`、`BuiltinShader` 加载与解码。

## Guards

- 素材必须经 Embedded Source 寻址，不从磁盘回退。

## Transitions

- Font 成功解析；SVG 使用各自语义标识。
- Pointer / Wait 静态备用与动画第一帧的图像对应。

## Invariants

- 45 款静态 Cursor 的素材与语义项对应，ReadOnly 使用独立透明切片；普通切片为 48px，Pointer/Wait 为 64px，Slot 剩余区域透明。
- Pointer、Wait Atlas 与 RGBA 数据合法；Wait 使用对应透明帧。
- 内建 Font / Icon / Cursor 路径确实指向可读取资源。

## Couplings

- Path 标识、Embedded 注册与实际解码协作。

## Coverage Map

- `embedded.rs`：全部 Embedded Font、Icon、Cursor 与 HoldButton Shader 资源 Contract。

