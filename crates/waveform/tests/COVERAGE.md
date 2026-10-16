# waveform Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Source 未填满 / 稳定 / 滚动、Cursor Boundary、Reduction Density。
- Source 成功 / 失败、渲染可见 / 隐藏 / 销毁、Polyline / Envelope。

## Stimuli

- Cursor 连续前进、跳转、倒退；Source Partial / Failed Read。
- BSN Scene、实际 Layout Width / Resize、Style / Theme、Despawn。

## Guards

- Config 的 Sample Rate、Duration、Channel Range 有效。
- Source 返回请求范围的完整有限数据；零尺寸不创建有效可见几何。

## Transitions

- 初始填充 → 增量滚动；Density 改变重建 Reduced Output。
- 失败保留先前已提交 Display；相同 Boundary / Density 为 NoOp。
- 相同 Cursor / Density 在读取失败后重试并恢复，成功后的稳定帧清空读取和 Reduction Stats；密度改变经过零宽再恢复时重建输出。
- Render Scene 从准备转为可见 / 隐藏并最终释放资源。

## Invariants

- Channel Data、Viewport 与 Reduced Output 同步提交；失败不留下半完成画面。
- Ring / Reduction 工作量与容量有界。
- Mesh、Camera 和 Renderer Layer 的资源归属隔离。
- Theme / Palette Override 不破坏已有几何。
- NoOp 不读取 Source、不执行 Reduction；增量 Reduction 工作量只随新增 Sample 与边缘 Bucket 增长。
- Renderer 保持单 Viewport / Camera / Mesh，失败保留已提交 Mesh，Root 同时管理 Entity 和 Asset；Disabled / Override / Theme 的 Palette 与 Background 解析后，Clear 恢复当前状态 Theme。

## Couplings

- Source Read × Cursor / Transaction。
- Output Density × Physical Layout Width。
- Renderer Lifecycle × Asset 释放和 View Isolations。

## Coverage Map

- `headless.rs`：Source / Cursor / Ring / Reducer 的更新、失败与增量数据 Contract。
- `renderer.rs`：BSN、Layout、Mesh、资源生命周期、Style / Theme 和多 View 隔离。

