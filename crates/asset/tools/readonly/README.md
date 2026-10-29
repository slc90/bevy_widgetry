# ReadOnly 光标制作源

> 以下为旧版制作记录。当前全套80px动画由[统一生成器](../README.md)制作；保留本目录高分辨率源供复现和创作参考，旧版安装参数不适用于当前资源目录。

`source.png` 是 built-in imagegen 生成的真实 RGBA 母版；角色与蓝白挂锁一并生成，参考现有 Kamuri static atlas 校正角色，方案中的概念图只提供挂锁语义与姿势参考。提示词见 `prompt.txt`。

运行时只嵌入 static atlas；本目录母版及脚本不作为运行时资源加载。构建使用 Python 3 和 Pillow，从母版透明边界裁切、保持比例一次降采样至 46px 以内，并在 48px 切片留透明边缘。代码不绘制或修补角色。

从仓库根目录重新生成并安装：

```pwsh
uv run --with pillow python crates/asset/tools/build_readonly_cursor.py --runtime-atlas crates/asset/src/assets/cursors/kamuri/static_atlas_96.png
```

只写入 index 44（第六行第五列）的 96px Slot，验证其它像素完全不变；输出 `target/cursor/readonly/readonly_48.png` 和 `report.json`。不提供 `--runtime-atlas` 时只输出预览与报告。

ReadOnly 使用 48px 切片，热点 `(14,37)` 位于挂锁钥匙孔。原有 0..43 索引和热点、Pointer/Wait 动画保持原值。生成报告只证明像素打包约束；真实 Windows Mouse 视觉验收另行执行。
