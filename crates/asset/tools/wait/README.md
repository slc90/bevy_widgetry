# Wait 动画制作素材

Wait 使用固定 320×320 RGBA 母版，导出 64×64、24 帧、8 FPS、3 秒循环。
hotspot 为加载环中心 [25,47]，所有帧保持同一坐标。
人物主体固定，持勺前臂使用连续曲线运动，送到嘴边后显示真实空勺内壁，再放回布丁处。
加载环独立旋转，不使用全身 Crossfade 或逐帧重新对齐。

body.png、arm.png 和 empty_spoon.png 是同一 1254×1254 canvas 的真实透明图层。
body.png 包含移除持勺前臂后由模型补画的头发和衣物，arm.png 包含独立前臂、手和勺子。
empty_spoon.png 仅包含模型绘制的空勺内壁，切换它不改变手和勺子的轮廓。

从仓库根目录检查关键姿势：

```pwsh
python crates/asset/tools/build_wait_animation.py --output-dir target/animation/kamuri-wait --keyframes-only
```

检查关键姿势后导出完整帧、atlas、frame0、APNG、浅/深底 GIF、hotspot 检查图、metadata 和格式报告：

```pwsh
python crates/asset/tools/build_wait_animation.py --output-dir target/animation/kamuri-wait
```

生成使用 Python 3 与 Pillow，source path 默认取本目录。
确认素材通过后可增加 --runtime-dir crates/asset/src/assets/cursors/kamuri 安装 runtime atlas，并仅替换静态 atlas 的 Wait 格。
生成脚本验证 24 帧唯一性、透明边缘及 atlas crop 与源帧像素一致。
生成报告不自动宣称美术、动画或 Windows Mouse 验收通过。

## 图像生成

使用 built-in imagegen，依次完成以下制作要求：

1. 以已制作的 Wait 透明高分辨率图为参考，只移除 loading ring 与零散装饰，保留淡紫长卷发、半垂蓝眼、恰好两条深蓝丝带、黑色校服、布丁与勺子，补齐遮挡，导出透明人物。
2. 移除 screen-left 的持勺手、前臂与勺子，真实补画被遮住的脸、头发与衣物，保留肩袖和 screen-right 持布丁手，为全部边缘留下透明 padding。
3. 以独立持勺前臂为参考，只移除勺内布丁并补画空勺内壁，保留手、袖口、勺柄和轮廓。最终只取内壁纹理，手和轮廓始终使用同一图层。

模型负责角色像素与遮挡修补，代码负责局部图层 mask、固定对齐、运动曲线、环形几何、合成、一次降采样和格式 QA。
