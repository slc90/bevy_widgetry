# 原生 Cursor 验证工具

`native_cursor_probe.py` 用 GetCursorInfo / GetIconInfo / GetDIBits 采样真实 Windows 光标，并通过调用方提供的目标 JSON 比较尺寸、热点及源图集帧。支持静态目标的 `animated: false`；默认要求动画产生不同位图。工具不参与应用运行，不包含角色素材。

完整调用方式、目标坐标、BRP Pointer 清理、采样边界和 Kamuri 示例见 [Gallery 素材工具说明](../../../gallery/tools/cursor/README.md)。角色生成器、源图层和动作说明均已移至该目录，生成器只安装 Gallery 资源。

```pwsh
uv run --python 3.13 --with pillow --with numpy python -B -m unittest discover -s crates/asset/tools -p test_native_cursor_probe.py
```
