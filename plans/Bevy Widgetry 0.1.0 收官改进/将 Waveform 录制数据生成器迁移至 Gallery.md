# 将 Waveform 录制数据生成器迁移至 Gallery

## 目标

`generate_recording.rs` 的实际用途是生成 Gallery Disk Replay 使用的模拟 `.widx` 波形录制数据。

磁盘文件读写、索引与后台读取属于 `waveform_file` 的职责；生成 Gallery 演示数据的可执行工具则应归属 Gallery。

本次只调整归属，不修改现有文件格式与运行行为。

## 文件迁移

原位置：

`crates/waveform_file/examples/generate_recording.rs`

新位置：

`gallery/examples/generate_recording.rs`

迁移完成后，移除原本已经空置的 `crates/waveform_file/examples/` 目录。

Cargo 会自动识别 `gallery/examples/` 下的 Example，无需额外声明 `[[example]]`。

## 依赖调整

将原来的 `bevy_widgetry_waveform` 类型导入改为使用 Gallery 已有的 `bevy_widgetry::waveform` 导出，包括：

- `WaveformMetadata`
- `WaveformSessionKind`
- `WaveformUnit`

`build_recording()` 继续由已有的 `bevy_widgetry_waveform_file` 依赖提供。

不增加不必要的 Cargo 依赖。

## 保持生成行为不变

保留当前默认参数：

- 录制长度：60 秒
- 采样率：8000Hz
- 通道数：8
- 随机种子：42
- 数据单位：Volts
- 输出路径：`gallery/data/waveform/replay.widx`

保留 `--output`、`--seconds`、`--rate`、`--channels`、`--seed` 参数。

保留正弦信号、采样缺口、正负尖峰及可复现的数据生成逻辑。

保留 MinMax、LTTB 两种索引的构建流程、参数校验、溢出检查、目录创建，以及目标文件存在时拒绝覆盖的行为。

不将生成的 `.widx` 测试数据作为新的内嵌资源提交。

## 新运行方式

从仓库根目录执行：

```powershell
cargo run -p widget_gallery --example generate_recording --release
```

同步更新文档中使用旧包名运行 Example 的命令。

Gallery 通过 `GALLERY_WAVEFORM_REPLAY` 指定录制文件的能力保持不变。

## 验收

确认新的 Gallery Example 能够构建和执行，迁移后的单元测试通过。

生成的文件能够通过 `Recording::open()` 打开，并在 Gallery Disk Replay 中正确显示和查询。

确认 `waveform_file` 原有单元测试通过，`benches/random_read.rs` 仍能正常构建。

检查旧 Example 路径和运行命令的引用均已同步。

## 边界

不修改 `.widx` 文件格式，不改动 `Recording`、`DiskWaveformSource`、LTTB 索引实现及公开 API。

不修改 Gallery Disk Replay 的读取行为，不扩展 Waveform 功能。
