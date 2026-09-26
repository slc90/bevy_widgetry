# 实现 reserved gutter 与 Auto scrollbar 收敛

## 目标

完成 ScrollArea 最关键的 functional geometry：per-axis policy、12px reserved gutter、无 Corner 的 H/V Grid 关系，以及 Both + Auto 时不会卡住或循环的双轴 fixed-point 求解。

这一阶段仍属于 headless 行为，不处理颜色、圆角等 visual style。

## scrollbar policy

每个 axis 独立应用：

```text
Auto   → 只有实际 overflow 时 scrollbar + gutter 参与 layout
Always → scrollbar + gutter 始终参与 layout
Hidden → scrollbar + gutter 不参与 layout，但该轴滚动能力仍存在
```

axis 不存在时对应 policy 忽略。

Auto 的 overflow 判定使用严格大于：content_size > viewport_size；相等视为不需要 scrollbar。

Always 且无 overflow 时保留 gutter；官方 thumb 填满 track，滚动范围为 0。

## reserved gutter 几何

root 使用 2 列 × 2 行的默认 Grid：

```text
┌──────────────────────┬──┐
│      Viewport        │ V│
├──────────────────────┤ V│
│         H            │ V│
└──────────────────────┴──┘
```

准确 placement：

```text
Viewport → column 1, row 1
H        → column 1, row 2
V        → column 2, row 1..=2
```

Grid track：

```text
columns = [1fr, auto]
rows    = [1fr, auto]
```

H 的 cross-axis thickness 是 height，V 的 cross-axis thickness 是 width。默认 thickness = 12 logical px，由 construction config 提供，同一实例的 H/V 共用一个值。

没有 Corner entity；右下角属于全高 V scrollbar。只有 H 时 H 占满宽度；只有 V 时 V 占满高度。

隐藏 scrollbar 使用 Node.display = None，使对应 auto row / column 自然收缩到 0。

## Auto fixed-point

每次新的求解从最小候选集合开始：

```text
Always → visible
Hidden → hidden
Auto   → hidden
```

在一次求解中 scrollbar 只允许 hidden → visible，不允许 visible → hidden。检查顺序固定为：

```text
Vertical → Horizontal
```

每次真实 layout 后：

1. 按当前候选状态读取 Viewport / Content 的实际 ComputedNode 几何。
2. 先判断 Vertical Auto 是否需要加入。
3. 再判断 Horizontal Auto 是否需要加入。
4. 若加入了任一 scrollbar，等待下一次 ui_layout_system 使用新 Grid 几何重新 layout。
5. 某一 pass 没有新增 scrollbar 时，本轮求解稳定。

只有 H/V 两个 Auto axis，因此一次求解最多发生两次“新增”，不会形成 visible ↔ hidden 循环。

## 为什么必须跨 layout pass

Scrollbar gutter 会改变 Viewport 尺寸，而 arbitrary user content 可能因为宽度变化发生 wrap / reflow，再反过来改变 content_size。因此不能仅用当前一帧的数值数学推算另一个 axis，也不能假设同一个 ui_layout_system 内完成收敛。

实现使用每个 ScrollArea root 自己的 private convergence Component，区分：

- 正在从最小集合求解。
- 已经稳定。

该 state 只记录求解过程与必要的稳定测量，不是公开 API，也不是 authoritative resolved visibility state。最终 scrollbar 的 Node.display 与 layout geometry 才是派生输出。

稳定后如果 root / viewport / content 的相关实际几何发生变化，则重新从最小集合开始一次新的求解，从而允许以前显示的 Auto scrollbar 被撤掉后重新判断。

## 事件驱动运行要求

Gallery 固定使用 WinitSettings::desktop_app()。Auto convergence 可能需要连续两个以上 UI layout pass，因此 private solver 在“尚未稳定”时必须请求 RequestRedraw，确保事件驱动应用继续产生后续 frame，直到进入 stable。

不要通过把整个 Gallery 改成 Continuous / 高频 Reactive update 来解决收敛问题。

## runtime config

construction props 中的 axis / scrollbar_visibility / scrollbar_thickness 在 Scene 展开时复制到 private ScrollAreaConfig Component，后续 headless system 只读取这个 Component。

```text
public construction input
→ WidgetryScrollAreaProps
→ scene() 初始化
→ private ScrollAreaConfig
→ runtime systems 读取
```

不支持外部运行时修改这些配置，不提供 Changed<ScrollAreaConfig> contract。

## 自动化测试

fixed-point 逻辑优先做 module unit test，至少覆盖语义组合：

- no overflow。
- X-only。
- Y-only。
- both。
- V 出现导致 viewport 变窄，继而诱发 H。
- H 出现导致 viewport 变矮，继而诱发 V。
- content == viewport 边界。
- Auto / Always / Hidden 混合。
- 从已有 H+V 的稳定状态发生几何变化后，新求解可以回到 none / single axis，而不是永久粘住旧 scrollbar。
- 单次求解只单调增加且在有限步骤内稳定。

跨真实 Bevy layout 的视觉稳定性留给 Gallery + BRP；Rust test 只在能够稳定构造真实 UI layout 环境时增加必要 integration coverage，不为了测试自建另一套 layout engine。

## 预期产出

Scrollbar policy 与 gutter 成为确定的 functional geometry；Both + Auto 对 cross-axis feedback 有确定、无循环、可在 event-driven app 中完成的收敛模型。

## 与前后方案的关系

依赖前一个 headless 方案提供的 axis、Viewport、Content、ScrollPosition 与 internal hierarchy。下一方案负责用 BSN Scene 真正创建 Grid、Content、Scrollbar / Thumb，并把这里的 geometry contract 映射到 visual style。
