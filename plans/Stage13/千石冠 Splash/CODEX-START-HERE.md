# Codex：从这里开始

这是 **Bevy 0.20 千石冠 Splash 完整实施包**，包含技术总方案、Plan Decomposer 拆分出的唯一顺序小方案、先前生成的图片/GIF，以及用于复核实际素材的机器元信息。

**目标仓库：** https://github.com/slc90/bevy_widgetry 。本包没有修改此仓库；接下来的代码实现、Windows 编译和真实截图由 Codex 在用户环境中执行。

## 请依顺序阅读

1. `master-plan.md`：完整目标、背景、Bevy 0.20 API、关键决策、跨阶段约束、异常/回归。
2. `decomposed-plan/00-overview.md`：唯一执行顺序，按 01→07 的顺序逐份完成。
3. `metadata/asset-quality.md` 与 `metadata/asset-manifest.json`：真正可用的文件、已核对的 blink 像素、**尚未修好的手臂/丝带锚点与缺失的干净场景背景**。
4. `metadata/sources.md`：仅与 Bevy v0.20.0 正式 tag 对照，避免误用其他版本 API。

## 重要提醒

- 正式运行时从 `assets/selected/` **挑选验收通过的贴图**，其余 `reference` / `experiments` / `qa` 不能一股脑嵌入 Gallery EXE。`approved_key_art.png` 里有角色，不是纯背景贴图。
- 当前 `body_open.png` 和 `body_closed_aligned.png` 的切换经过 Python 像素对照（变化约 3.509%，仅眼睛附近），用户已认可视觉效果。
- 当前 `arm_candidate.png`、`ribbon_upper_candidate.png`、`ribbon_lower_candidate.png` 仅有视觉确认，尚无严谨的枢轴对位/转动遮挡验证。**务必先完成素材标准化与背景补全，不要跳过阶段 01。**
- 使用现有 Bevy UI (`UiTransform`, `ImageNode`, `UiTargetCamera`, `UiMaterialPlugin`)，避免无故新增视频依赖与 sprite render feature。
- 保留 AppLogging + `block_on(transparent_render_creation())` + `prepare_native_window` 等启动流程；只向 Gallery 增加必要的模块与就绪信号，不修改远程 GitHub 仓库直到用户在本地授权实施。
- Splash 演出 nominal 1s；GPU 展示 ready 之前不开始时钟，Gallery 不 ready 则在 800ms 稳定姿势等待，不可以把 timeout 当作成功完成。
- 因窗口为 `CompositeAlphaMode::PreMultiplied` 的透明 DX12 Window，shader 的 alpha 混合和 offscreen Image 格式须在 Windows DX12 验证。
- 不应在完成最后波纹帧前移除 Splash；也不应让不透明 fallback 节点残留在波纹透明区域下面。

## 本包结构

```text
kamuri_splash_codex_package/
  CODEX-START-HERE.md
  master-plan.md
  decomposed-plan/
    00-overview.md
    01-...md  .. 07-...md
  assets/
    selected/       # 用户确认的运行时候选图（非全数验收完成）
    reference/      # 官方来源截图、合并丝带、完整挥手/标注图
    experiments/    # AI 分层 / 表情实验历史
    qa/             # 两种眨眼 GIF 与像素差异可视化等
  metadata/
    asset-manifest.json  # SHA256/尺寸/alpha bbox
    asset-quality.md     # 真实问题和补画约束
    sources.md           # 官方 v0.20.0 与项目代码依据
```

## 完成报告的最低内容

逐阶段写明改动文件、偏离或采纳的技术选择、编译/测试命令与结果、Windows DX12 的真实截图/时序/性能数据，以及未解决的缺陷；不把生成图片当作已对齐素材，不把计划中的 60 FPS 写成已经测得。
