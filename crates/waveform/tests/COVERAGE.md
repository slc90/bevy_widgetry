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
- `session.rs`：同根实体 Live 32 → Replay 64 → Replay 8 → Live 切换、元信息验证、显示配置保留、V 单位换算、归零与暂停、停止保留画面、清空会话、排队更新及提交后通知、实例错误查询。
- 会话替换使所有旧 Raw / Reduced 表示失效；Renderer 的 Camera、Mesh、Image 和 Entity 身份保持稳定，下一帧清除旧几何（`renderer.rs`）。


- `gaps.rs`：逐通道显式缺口、真实零值、MinMax 有效段分离、跨桶边界/环回/缺口淘汰的增量与完整重建一致性、非法缺失区间拒绝。
- `requests.rs`：Pending 与过期 Ready 保留一致快照，Live 追到最新窗口；同目标 request 身份稳定，密度/会话变更使旧身份失效，共享 Source 的读取者互相隔离；Replay Pending 继续、真错误停止且不自动重试。
- 几何 Unit Test 覆盖 Polyline 显式断点、自定义 Reducer 稀疏代表点仍连线、独立有效样本可见及全缺失时透明空 Mesh。
- `duration.rs`：8s/30s 左起填充、35s/10s 累计时间滚动、同一时刻扩大/缩短窗口、缺口保留、候选/无效实体校验、提交后通知、Pending / 历史不足保留快照、暂停重算与新会话归零、空暂停 Replay 不读取；缓存随当前窗口缩小。
- 控制栏 Unit Test 使用真实 ComboBox / Layout，覆盖初始化选中、API 投影、选项更新同一状态、稳定 Model / Combo 实体及随根清理。
- 取消大窗口 Pending 后缩到小窗口，不将旧暂存容量转为小窗口的长期缓存（`duration.rs`）。
