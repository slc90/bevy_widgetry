# Waveform

## 功能概述

显示多 Channel 时间序列 Sample 的 Waveform，同时提供可单独复用的数据读取、Ring Buffer、Reduction 与 Runtime 查询能力。

## 当前设计思路

Source 按 Frame Range 提供 Planar Data；Runtime 使用 Cursor、Sample Rate 与可见时间范围维护 Raw / Reduced Output。Renderer 根据 UI 实际宽度确定输出密度，绘制对应 Channel，而不是按输入 Sample 数盲目创建可见节点。

## 主要功能

- `WaveformSource` 抽象读取指定半开 Frame Range 的各 Channel 数据。
- `WaveformCursor` 控制时间位置，支持连续前进、回退与跳转。
- `MinMaxReducer` 及可替换的 `WaveformReducer` 根据输出宽度汇总波形。
- `WaveformRuntime` 提供范围、Channel、Revision、Reduced Output 与 Update Stats 查询。
- `WaveformStyle` 支持 Palette、Line Width 与 Background 更新。
- 数据处理与 UI 渲染可分别装配 `WaveformPlugin` / `WaveformRenderPlugin`。

## 行为与状态契约

Config 在 Runtime 构造时确定 Sample Rate、可见 Duration 和各 Channel Range；容量必须对应整数 Frame 数。相同 Sample Boundary 且输出密度未变时可 NoOp，宽度改变会重新生成表示。Source 返回的数据必须覆盖请求范围，且所有 Channel 为完整的有限 Sample；失败时不能用部分数据覆盖先前已提交的有效输出。

读取失败后，相同 Cursor 和输出密度仍可重试。成功更新后的稳定帧报告 NoOp、零读取和清空的 Reduction Stats；同一 Sample Boundary 上改变输出密度会重新生成表示，经过零宽后恢复密度也遵循该规则。

每个 Channel 的幅值 Range 必须有限且递增。

## 边界情况与错误处理

非法 Config、无效 Channel Data、Source 读取失败按对应 Result / Runtime 状态处理。Layout 为零或隐藏时不能生成无意义绘制；重新可见后更新当前表示。Root 结束时要清理属于自身的 Renderer Resource。

## 已知限制

- Source 读取当前是同步调用；调用方需要保证实际数据源满足自己的实时预算。
- 某些无变化帧仍会经过少量 Runtime 检查，是否优化需要独立 Benchmark 证据。
- Headless 数据正确性测试不代表完整 GPU 绘制性能。

