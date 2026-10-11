# 千石冠 Splash 素材契约

运行时只嵌入 body_open、body_closed、arm、ribbon_upper、ribbon_lower 五张原始 PNG。
body_closed.png 对应对齐后的闭眼图，睁闭眼仅在 [662,378,1022,616] 内有 55,209 个像素不同。
原始主体、脸型、发丝、制服与眨眼像素保持不变。

源图保留 1672×941 画布，crop_offset 为零，alpha_bbox 为 alpha≥16 的完整非透明范围。
可见 artboard 为 1672×918，从原始源图顶端裁到主体仍有完整底边的最后一行，去除底部透明留白。
运行时与浏览器预览使用同一 artboard，不分别拉伸 X/Y。
cover 与手心 reveal 锚点均从可见 artboard 计算。
draw_origin 包含默认 rotation，围绕 pivot 放置 source_pivot。
200ms 入场采用单调 cubic easing，从底边下方进入，避免 overshoot 再次抬起主体底边。

双丝带通过各自 source_pivot、scale 汇合在 [795,132]。
arm 是前臂与手掌，不是带肩部的完整手臂，source_pivot [1370,880] 对应肘部连接端。
前臂统一 scale 0.85，默认 rotation 20°，活动范围 14°–26°，肘部 pivot [1250,1015] 位于画面下方。
上臂由原始主体的右侧袖子承托，前臂延伸到画面底边，PNG 的平直截断端在整个活动范围都不进入可见 artboard。
前臂在整个活动范围覆盖原始主体右手的 1,938 个 skin 像素，保留主体左手，避免出现第三只手。
不再使用另行生成的 sleeve_patch 矩形补画，右下发丝与衣料完整来自同一主体。

底画采用全窗 pastel gradient：#f8eef7 → #e4e1f6 → #f4e3ef，alpha 恒为 1。
由 Scene / shader 构造，不包含静态角色副本。

执行 python gallery/tools/kamuri_assets.py --output <仓库外目录>。
该工具需要 Pillow，只读取 PNG，核对 checksum、bbox、眨眼范围、底边以及 14°–26° 前臂对右手的覆盖和截断端位置，并输出浏览器 canvas 预览。
预览包含默认、闭眼、转动两端、双丝带独立转动、深浅底画和空底画。
机器校验不代替 Windows DX12 Gallery 的构图、采样与动画过程验收。

原始素材发丝边缘有少量浅色 matte，在深色背景下更明显。
