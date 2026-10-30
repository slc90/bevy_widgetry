# 清理被统一生成器取代的 Cursor 素材工具

## 目标

当前 45 种 Kamuri Cursor 动画已经统一使用 80px 规格，由 `build_kamuri_animations.py` 生成。

旧版 ReadOnly 48px、Wait 64px 生成工具不再适配当前运行时资源目录，应当删除，不再为了历史复现保留。

## 删除范围

从 `crates/asset/tools/` 删除：

- `build_readonly_cursor.py`
- `build_wait_animation.py`
- `readonly/` 整个目录，包括旧版 `README.md`、`prompt.txt`、`source.png`
- `wait/README.md`

旧版制作母版、规格说明和安装脚本一起清理，不保留不再使用的历史版本。

## 保留当前素材生成链

当前 `build_kamuri_animations.py` 仍然直接读取以下三个文件：

- `wait/body.png`
- `wait/arm.png`
- `wait/empty_spoon.png`

这三张图片必须保留。

同时保留统一生成器 `build_kamuri_animations.py`、Windows 原生验证入口 `native_cursor_probe.py`、现有 80px PNG、`cursor_animation.rs` 以及运行时使用的全部资源路径和语义定义。

Asset 中的字体、SVG 图标、许可证和 Shader 不受影响，尤其保留 `canvas_straight_alpha.wesl` 与 `hold_button.wesl`。

Gallery Cursor Benchmark 也不属于本次删除范围。

## 文档同步

更新 `crates/asset/tools/README.md`：

- 删除旧版 48px/64px 工具仍供历史复现的描述。
- 删除指向 `wait/README.md` 等已删除文件的链接。
- 保留当前 80px 统一生成器的使用说明。
- 说明 `wait/` 中保留的三张高分辨率图片是当前生成器的输入素材。

检查其他直接引用旧工具的说明，仅移除失效引用。

## 验收

使用现有 Python/Pillow/Numpy 环境运行当前统一生成器及其检查流程。

确认 45 种 80px 动画均可生成，静态回退、热点坐标、时序及嵌入资源路径符合现有契约。

确认 Asset 集成测试通过，Gallery Cursor 页面仍可正常加载并播放动画。

不得残留指向已删除文件的失效引用。

## 边界

不重新设计 Cursor 动画，不修改美术资源，不改变运行时 API 或其他控件功能。
