# Kamuri 80px 动画素材

45种现有语义使用独立的连续循环、统一80×80透明画布。功能性指尖、插入点、准星、方向轴或锁孔固定；运动发生在局部工具、角色前臂或辅助标记上。长时间使用的文字/精确定位光标保持较轻的主体占比。

`wait/` 中保留的 `body.png`、`arm.png` 和 `empty_spoon.png` 三张高分辨率透明图层是当前统一生成器 `build_kamuri_animations.py` 的输入素材。源角色复用这些图层，工具几何与运动由统一生成器创作，不引入新的第三方素材。

## 重建与预览

从仓库根目录执行，Python仅为制作工具，不进入Rust运行时依赖：

```pwsh
uv run --python 3.13 --with pillow --with numpy python gallery/tools/cursor/build_kamuri_animations.py --samples --output target/animation/kamuri80-samples
uv run --python 3.13 --with pillow --with numpy python gallery/tools/cursor/build_kamuri_animations.py
uv run --python 3.13 --with pillow --with numpy python gallery/tools/cursor/build_kamuri_animations.py --install
cargo fmt --all
```

默认完整输出在`target/animation/kamuri80`，包括45个RGBA atlas、APNG、动作条、静态首帧图集、`contact-sheet.png`、`manifest.json`和`index.html`。HTML提供持续循环及浅/深底切换；Manifest记录每种语义的帧数、时序、Hotspot、实际变化帧数量、固定热点区域变化数量和Atlas哈希。`--samples`只生成Default、Text、ZoomIn与Wait样本，不能与`--install`一起使用。

`--install`在确认固定热点区域不变后同步Gallery当前80px运行时PNG和应用语义目录，并从同一组创作规格生成Rust时序/热点数据。不安装时可用`--output`写入单独目录检查重建结果，不改变现有运行时资源。通用播放器只按实际语义加载图集；Gallery在Cursor页共享加载全部45种。动画美术品质仍需观察持续播放与原生效果。

## 原生 Windows 验证

仓库内的`crates/asset/tools/native_cursor_probe.py`独立使用`GetCursorInfo`、`GetIconInfo`和`GetDIBits`捕获已有应用的真实OS光标，无需安装外部Skill。它只用于开发验证，不参与最终程序运行，也不创建样本播放器窗口。Python依赖仍为Pillow和NumPy。

先启动Gallery，通过正常界面输入进入Cursor页并选择Kamuri主题。标准导航、滚动和截图使用BRP；完成后调用`brp_extras/pointer_control`的`release`并查询`status`，确认`phase`为`inactive`，再开始原生鼠标捕获。BRP Custom Pointer不会改变真实OS Cursor，不能替代以下验证。捕获期间保持目标区域可见，不再操作BRP指针或真实鼠标。

从生成器的`manifest.json`复制需要验证的动画条目，组成UTF-8 JSON数组，并为每项添加`position: [x, y]`，表示该目标中心在当前窗口中的**物理Client像素**坐标。`atlas`相对于该Target JSON所在目录解析，保留原有`frame_size`、`columns`、`frames`、`hotspot`和`frame_duration_ms`（或逐帧`durations_ms`）字段。例如，在`target/animation/kamuri80/targets.json`写入：

```json
[
  {
    "name": "Pointer",
    "atlas": "pointer_80.png",
    "frame_size": [80, 80],
    "columns": 8,
    "frames": 28,
    "frame_duration_ms": 70,
    "hotspot": [20, 34],
    "position": [600, 350]
  }
]
```

坐标仅为格式示例，必须根据当前窗口尺寸、DPI和实际滚动位置填写。只在同一批次中列出当前可见的目标；切换滚动位置后重新准备对应批次。窗口标题必须与实际原生标题完全一致，包括语言切换后的文案。

```pwsh
uv run --python 3.13 --with pillow --with numpy python crates/asset/tools/native_cursor_probe.py --title "Widget Gallery" --targets target/animation/kamuri80/targets.json --foreground --output target/animation/kamuri80/native
```

`--foreground`临时置前并聚焦目标窗口；成功、失败或中断退出时恢复原有置顶状态、先前前台窗口及鼠标位置。不加此参数时须自行保证窗口未被遮挡且能够播放动画。工具在目标位置小幅移动真实鼠标以恢复Mouse输入身份，然后按每项独立时序采样两个循环。

每项结果位于`<output>/<序号>-<语义>/`，包含`native.json`、`native.apng`和`native-contact.png`动作条。JSON保留真实`elapsed`采样时间、尺寸、Hotspot、屏幕位置、可见状态、位图SHA256、`semantic`和`matches_source_frame`；APNG按实际采样间隔播放。位图匹配沿用原算法，接受直接BGRA和两种常见预乘取整结果。

仅当全部采样可见、尺寸/Hotspot正确、至少出现两个不同位图且每张均匹配对应Atlas帧时退出0。窗口缺失、坐标越界/被遮挡、捕获期间鼠标移位、位图不匹配或动画不推进均返回明确错误和非零退出码。已经取得的捕获文件保留供排查；打开APNG和动作条观察实际运动。完整验收应覆盖45种语义，至少包括普通指针、文本和等待；输出证明原生位图身份与动画推进，不证明点击精度或逐帧显示延迟。

独立工具的输入、匹配、输出和异常清理回归可执行：

```pwsh
uv run --python 3.13 --with pillow --with numpy python -B -m unittest discover -s crates/asset/tools -p test_native_cursor_probe.py
```

Gallery可设置`GALLERY_CURSOR_BENCH_OUTPUT`为一个尚不存在的CSV路径，记录真实页状态、CPU更新区间、预览身份/帧、布局与实体数量，不主动驱动帧循环。配合现有`GALLERY_SPLASH_BENCH_OUTPUT`和`GALLERY_SPLASH_BENCH_IDLE=1`可记录呈现调度/GPU区间并观察离页后的实际睡眠。CSV输出目录须预先存在，启动时输出文件创建失败会交给Gallery错误处理。

可将Benchmark CSV与原生捕获分开核对，确认各语义预览的实际帧推进、页面与禁用对照状态。不将请求成功、静态图或仅Headless测试当作原生通过。

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

静态原生光标目标可显式设置 `animated: false`（JSON布尔值）。此模式仍校验可见性、尺寸、热点和源帧匹配，但要求全部采样位图保持同一SHA256；默认仍为动画验证。验证首帧时可使用原动画Atlas与 `frames: 1`，复用其首帧而不生成新素材。
