# Kamuri 80px 动画素材

45种现有语义使用独立的连续循环、统一80×80透明画布。功能性指尖、插入点、准星、方向轴或锁孔固定；运动发生在局部工具、角色前臂或辅助标记上。长时间使用的文字/精确定位光标保持较轻的主体占比。

源角色复用本仓库已有的`wait/body.png`、`arm.png`、`empty_spoon.png`高分辨率透明图层；工具几何与运动由本次生成器创作，不引入新的第三方素材。原图层的生成记录保留在[Wait制作源](wait/README.md)，旧版48/64px脚本保留供历史复现，不再安装到当前资源目录。

## 重建与预览

从仓库根目录执行，Python仅为制作工具，不进入Rust运行时依赖：

```pwsh
uv run --python 3.13 --with pillow --with numpy python crates/asset/tools/build_kamuri_animations.py --samples --output target/animation/kamuri80-samples
uv run --python 3.13 --with pillow --with numpy python crates/asset/tools/build_kamuri_animations.py --install
cargo fmt --all
uv run --python 3.13 --with pillow --with numpy python skills/animated-cursor-authoring/scripts/cursor_tool.py target/animation/kamuri80/manifest.json --output target/animation/kamuri80/checked
```

默认完整输出在`target/animation/kamuri80`，包括45个RGBA atlas、APNG、动作条、静态首帧图集、manifest和HTML。`checked/index.html`提供完整自动循环与浅/深底切换，`checked/qc.json`记录技术检查及需要人工观察的接缝候选。`--install`同步当前80px运行时PNG和Asset语义目录；Rust的时序/热点数据由同一组创作规格生成。普通应用只按实际语义加载图集；Gallery在Cursor页共享加载全部45种。

技术检查包含尺寸、RGBA、透明边缘/隐藏RGB、实际变化帧、固定热点像素、首帧回退一致性与独立时序。接缝指标只辅助检查，美术品质仍需看持续播放与原生效果。

## 原生 Windows 验证

[通用Skill](../../../skills/animated-cursor-authoring/SKILL.md)包含实际用过的检查与Windows观察脚本。`native_cursor_probe.py`是仓库入口，转发到Skill内的通用工具。样本模式打开临时原生窗口；现有应用模式需要真实可见窗口标题和物理client坐标，可比较OS位图与源图集。其结果与BRP Custom Pointer分开记录；标准GUI导航/截图仍通过BRP完成。

Gallery可设置`GALLERY_CURSOR_BENCH_OUTPUT`为一个尚不存在的CSV路径，记录真实页状态、CPU更新区间、预览身份/帧、布局与实体数量，不主动驱动帧循环。配合现有`GALLERY_SPLASH_BENCH_OUTPUT`和`GALLERY_SPLASH_BENCH_IDLE=1`可记录呈现调度/GPU区间并观察离页后的实际睡眠。CSV输出目录须预先存在，启动时输出文件创建失败会交给Gallery错误处理。

本次验证覆盖全部45种真实OS动画，每种两个循环，核对80px、固定热点和源帧身份；Gallery记录确认所有45种各自的全部帧都在推进，另含两个禁用对照。原生鼠标和BRP指针应分段操作，完成BRP后先release并确认inactive。运行时证据输出在`target/animation/kamuri80`，不将请求成功、静态图或仅Headless测试当作原生通过。

## 独立动作与时序

方向调整属于同一语义家族，共享沿声明方向的shuttle运动；其他工具分别设计。毫秒列是每帧时长，循环时长为帧数与该值之积。

| 语义 | 帧数 | 毫秒/帧 | 固定热点 | 动作 |
| --- | ---: | ---: | --- | --- |
| Default | 32 | 80 | (10, 12) | A ribbon flows down a stationary arrow; the point stays still. |
| Pointer | 28 | 70 | (20, 34) | A fingertip rests on a button while a soft press wave expands. |
| ContextMenu | 36 | 80 | (10, 12) | Menu rows are explored in order beside a fixed arrow. |
| Help | 40 | 80 | (10, 12) | A question mark traces a small inquisitive nod; its dot stays put. |
| Text | 32 | 90 | (17, 47) | An ink bead travels vertically along a steady insertion beam. |
| VerticalText | 30 | 90 | (28, 54) | An ink bead writes left to right along a horizontal beam. |
| Cell | 32 | 80 | (26, 54) | Four neighbouring grid cells are visited without moving the active cell. |
| Crosshair | 40 | 60 | (27, 53) | Separated calibration arcs rotate around a fixed fine cross. |
| Grab | 32 | 80 | (26, 51) | Open fingers invite a grasp, with a relaxed wrist sway. |
| Grabbing | 28 | 60 | (26, 53) | A closed grip compresses a held capsule; the grip centre is fixed. |
| Move | 32 | 60 | (29, 53) | Four cargo beads depart along the four move axes. |
| Copy | 36 | 70 | (24, 54) | A second document peels away and settles behind the source. |
| Alias | 40 | 65 | (24, 55) | A route bead follows a bent shortcut arrow. |
| NoDrop | 32 | 75 | (25, 54) | A parcel approaches a barrier and gently recoils. |
| EResize | 28 | 65 | (28, 54) | An eastbound shuttle travels inside fixed resize endpoints. |
| NResize | 30 | 60 | (28, 54) | A northbound shuttle travels inside fixed resize endpoints. |
| NeResize | 32 | 60 | (28, 54) | A northeast shuttle follows the diagonal resize axis. |
| NwResize | 34 | 60 | (28, 54) | A northwest shuttle follows the diagonal resize axis. |
| SResize | 28 | 70 | (28, 54) | A southbound shuttle travels inside fixed resize endpoints. |
| SeResize | 30 | 70 | (28, 54) | A southeast shuttle follows the diagonal resize axis. |
| SwResize | 32 | 70 | (28, 54) | A southwest shuttle follows the diagonal resize axis. |
| WResize | 34 | 65 | (28, 54) | A westbound shuttle travels inside fixed resize endpoints. |
| EwResize | 36 | 60 | (28, 54) | Two shuttles breathe outward along the horizontal axis. |
| NsResize | 32 | 65 | (28, 54) | Two shuttles breathe outward along the vertical axis. |
| NeswResize | 36 | 65 | (28, 54) | Opposed shuttles separate along the northeast diagonal. |
| NwseResize | 40 | 60 | (28, 54) | Opposed shuttles separate along the southeast diagonal. |
| ColResize | 32 | 75 | (28, 54) | Column rails remain anchored while side handles slide outward. |
| RowResize | 36 | 70 | (28, 54) | Row rails remain anchored while upper and lower handles slide outward. |
| AllScroll | 48 | 50 | (29, 53) | A compass bead visits all four scroll directions clockwise. |
| ZoomIn | 32 | 75 | (25, 52) | An inner lens ring grows beneath a fixed plus and lens centre. |
| ZoomOut | 36 | 75 | (25, 52) | An inner lens ring contracts beneath a fixed minus and lens centre. |
| Progress | 40 | 55 | (10, 12) | An arrow remains usable while three task beads chase around a dial. |
| Wait | 48 | 65 | (25, 56) | Kamuri scoops pudding while the fixed-centre waiting ring turns. |
| NotAllowed | 40 | 85 | (25, 54) | A perimeter sentinel guards a stationary prohibition slash. |
| AddSelection | 32 | 70 | (26, 54) | An extra tile joins a selection beside a fixed plus badge. |
| RemoveSelection | 36 | 70 | (26, 54) | A tile withdraws from a selection beside a fixed minus badge. |
| RangeSelection | 40 | 65 | (16, 56) | A span extends from a fixed origin and returns along its ruler. |
| DragSort | 36 | 60 | (26, 54) | Two list rows exchange places around a fixed grip. |
| InsertDrop | 32 | 80 | (25, 56) | A card descends into a gap above a stationary insertion bar. |
| ReplaceDrop | 40 | 70 | (25, 54) | An incoming card cross-slides over an existing card. |
| HorizontalValueAdjust | 36 | 60 | (28, 54) | A tiny scale sweeps under a stationary horizontal adjustment handle. |
| VerticalValueAdjust | 40 | 60 | (28, 54) | A tiny scale sweeps under a stationary vertical adjustment handle. |
| PanReady | 40 | 70 | (25, 54) | A relaxed palm awaits a pan; a map contour circles behind it. |
| Panning | 32 | 55 | (25, 54) | A gripped map flows beneath the fixed panning anchor. |
| ReadOnly | 48 | 80 | (25, 56) | A light scans a locked page; the closed shackle and keyhole never move. |
