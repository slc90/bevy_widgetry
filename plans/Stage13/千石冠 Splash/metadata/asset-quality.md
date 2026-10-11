# 千石冠 Splash：素材实测记录与缺口

> 自动检查日期：2026-10-11。对文件和像素做了校验，但**没有在 Windows 上运行 Bevy**。具体可从 `asset-manifest.json` 用 SHA256 对照文件。

## 已打包内容

- `assets/selected/`：6 张与用户最后确认方向对应的主要候选。已复制 PNG 原始像素，未冒充为裁剪后、锚点标定后的可直接运行资源。
- `assets/reference/`：用户提供的官方角色截图、两根丝带合并素材、完整挥手立绘和用户标注图。官方截图是**参考素材**，不要把网页的角色图当作可自由分发的动画资源。
- `assets/experiments/`：以前生成的多版本插画、主体和失败的眨眼素材；只用于比对和修补，不属于最终运行时资产。
- `assets/qa/`：眨眼对比动图、像素差异图、局部修补预览及后续生成的总览缩略图。

## 文件事实（可测）

| 文件 | 尺寸 | alpha≥16 的非透明 bbox，XYXY | 状态 |
|---|---|---|---|
| `approved_key_art.png` | 1672×941 | `无透明通道，完整不透明` | approved_visual_reference |
| `body_open.png` | 1672×941 | `[157, 112, 1655, 924]` | candidate_runtime |
| `body_closed_aligned.png` | 1672×941 | `[157, 112, 1655, 924]` | candidate_runtime_verified_pixels |
| `arm_candidate.png` | 1672×941 | `[952, 226, 1496, 908]` | candidate_unaligned |
| `ribbon_upper_candidate.png` | 1672×941 | `[193, 80, 990, 247]` | candidate_unaligned |
| `ribbon_lower_candidate.png` | 1672×941 | `[209, 256, 808, 480]` | candidate_unaligned |


`body_open.png` 和 `body_closed_aligned.png`：尺寸均为 **1672×941**；整幅改变的像素 **55,209 / 1,573,352 = 3.509%**，变化边界 `(662,378)–(1022,616)`。该 bbox 以外 RGBA 四通道一致；bbox 内会有预期眼部像素/Alpha 差异，不应误读为两图逐像素完全一致。

## 已知必须解决的缺口（不允许跳过第一阶段）

1. **干净的场景底画缺失。** `approved_key_art.png` 是人物与樱花背景合成，不是仅背景。若直接当作 background，移动主体将出现静止副本。要在 stage 01 生成不含角色的同风格底画，或者采用纯不透明、视觉匹配的程序化画布。不要声称已从合成图中无损恢复完整背景。
2. **`arm_candidate.png` 并非严格从同一源图逐像素切开**，转动后肩部可能露出断裂或主体重影。需要锚点对齐、补画被遮住的肩部，并做旋转后的预览和实机检查。
3. **`ribbon_upper_candidate.png` 与 `ribbon_lower_candidate.png` 的不透明 bbox 及根部位置不同**。它们是两张分开生成的候选，不是天然在同一个坐标系里精确重合的拆分结果；需要调整共同根点与裁剪偏移，必要时参考 `ribbon_pair_reference.png` 做重新裁切或编辑。
4. **透明像素边缘存在图像生成的彩色毛刺/黑边候选**，尤其头发和卷曲末端；在深/浅背景上均检查 alpha 边缘、premultiplied/straight alpha 采样行为。
5. **原始资源尺寸不是 1920×1080**，不可用独立 X/Y 缩放拉伸人物。为每个候选保留 1672×941 图层空间与设计画布的映射，最终裁剪时记录偏移。
6. **只有用户视觉确认，不等于已做过 Windows/DX12 Bevy 实际渲染验证。** 不能在报告中将 60 FPS、Shader、输入/资源状态写成已通过。

## 眨眼对照

`assets/qa/blink_before.gif` 是原始睁/闭眼分别生成素材切换，头发和衣服会变；`assets/qa/blink_after.gif` 用的是局部眼睛修正后切换，用户在对话中已经认可效果。Codex 必须使用 `assets/selected/body_closed_aligned.png`，而不是 `assets/experiments/body_closed_original.png`。

## 资源来源

动画人物的画面参考来自用户给出的官方角色页：<https://slow-start.com/character/?chara=kamuri>。本包艺术素材主要为本对话生成并针对个人工程测试整理，**不是官方授权美术素材包**。保留参考出处，勿将官方截图伪装为自主原创或附带商用许可。
