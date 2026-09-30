# 实现固定行高 virtualization 与 renderer lifecycle

## 目标

建立 ListView 最核心的固定行高 virtualization：只有 viewport 当前真正可见的 rows 存在于 ECS，但 ScrollArea 仍能通过真实 layout 感知完整列表总高度；滚动、resize 与 model 修改只做必要的 entity/content 更新。

第一版明确 **不做 overscan**。Gallery/BRP 会用真实快速滚动结果决定未来是否需要，而不是当前提前增加 public option 或复杂 buffering。

## Content hierarchy 与 runtime

`WidgetryScrollAreaContent` 下固定保持：

```text
WidgetryScrollAreaContent
├─ TopSpacer
├─ rendered WidgetryListViewItem rows...
└─ BottomSpacer
```

Top/Bottom spacer 是 crate-private implementation marker，只用于撑出未实例化区域高度。

内部 runtime 保存至少：

- viewport entity；
- content entity；
- top spacer entity；
- bottom spacer entity；
- 当前 rendered `Range<usize>`；
- 与 range 顺序严格一致的 `Vec<Entity>` rows。

必须持续满足：

```text
rows[i] <=> range.start + i
```

同时 content child 中的 row 顺序、`runtime.rows` 顺序、model index 顺序一致。Rows cache 是对同一批 UI entity 的引用，不是另一份 row tree。

## Visible range

第一版定义：

```text
rendered range == visible range
```

部分露出的边界 row 也算 visible。

使用：

```rust
start = floor(scroll_y / item_height)
end   = ceil((scroll_y + viewport_height) / item_height)
```

再 clamp 到 `0..model.len()`。

`viewport_height` 必须使用 `ComputedNode` 转成 logical px 后的真实尺寸；`ScrollPosition.y`、`item_height`、spacer height 都在同一 logical px 语义下计算。

需要正确处理：

- `scroll_y == 0`；
- viewport 高度不是 item_height 整数倍；
- 只露出一小部分 row；
- 空 model；
- model 长度小于 viewport 容量，此时自然得到 `0..len`，表现上“全部显示”，但底层仍是同一套 virtualization；
- 滚到末尾时 end clamp。

不增加 `virtualized: bool` 或 Small List 特殊模式。

## Spacer 数学 contract

```text
top    = range.start * item_height
rows   = range.len() * item_height
bottom = (len - range.end) * item_height
```

使得：

```text
top + rows + bottom == len * item_height
```

因此 ScrollArea 继续从真实 content layout 得到完整 content size 和合法 scroll range，无需 ListView 伪造 scrollbar geometry。

每个 row 的 **总纵向 layout extent** 必须严格等于 `item_height`。Style 中的 border/padding 要包含在高度内；禁止 vertical margin、row gap 等破坏 index ↔ y 数学映射的布局。

## Range reconciliation：方案 A

滚动/resize 改变 range 时按 index overlap 复用已有 row entity，不做 row pool/ring buffer。

例：

```text
old 100..120
new 101..121
```

行为固定为：

- despawn index 100 的 row；
- 保留 101..119 的 row entity 及其 renderer subtree；
- 为 index 120 创建一个新 row；
- 更新 rows Vec 和 content child order。

反方向滚动同理。

若新旧 range 无 overlap，例如：

```text
100..120 -> 5000..5020
```

则直接销毁当前 rendered rows，再创建新 range；第一版不为了大跳转维护 entity pool。

Range 扩张/缩小同样只在两端增删，不因 viewport resize 重建仍处于 overlap 的 rows。

## Row wrapper 与 renderer lifecycle

每个 rendered row wrapper 至少具有：

- `WidgetryListViewItem { id, index, ... }`；
- Bevy `ListItem`；
- Hovered/Pressed/Selected/InteractionDisabled/style 所需 component；
- renderer 生成的 direct children。

row wrapper identity 尽可能在 index overlap 内保持稳定；renderer 内容是否重建由 model entry identity/revision 决定。

### Model Changed 时的精确同步

Bevy 只告诉我们整个 `WidgetryListModel<T>` Changed，因此每次变化扫描 **当前 rendered rows**，不扫描整个大 model，也不全量 rerender visible rows。

对每个 rendered index：

1. entry id 与 row 上次 id 不同  
   说明结构变化导致当前 index 换成另一 entry；更新 row identity/index，递归销毁旧 renderer direct children，再执行 renderer。
2. id 相同、revision 不同  
   保留 row wrapper entity，只替换 renderer children。
3. id/revision 相同、disabled 不同  
   只同步 `InteractionDisabled` 与 style projection，不调用 renderer。
4. 都相同  
   row 不做任何结构更新。

因此修改单个 visible item 的典型成本是 O(rendered_rows) 的 id/revision/disabled 比较 + 1 次 renderer，而不是 O(model.len) 或 rerender 全部 visible rows。

修改 offscreen item 不调用 renderer；以后滚入时直接基于最新 value 构造。

Renderer direct children 视为 ListView 管理的完整内容区域，revision rerender 时可以 wholesale replace；第一版不额外增加 ItemContent wrapper entity。

## Structural model changes

Insert/remove/move 会改变 index → entry 映射，row 同步必须同时比较 stable id，而不是只看 revision。

`move_item()` 保持 id/revision，因此选中/active 的逻辑对象后续可以按 id 跟随；当前 visible index 上若变成别的 id，则对应 row rerender。

当 model 变短时，需要先把 `ScrollPosition.y` clamp 到新合法范围：

```text
max_scroll = max(model.len * item_height - viewport_height, 0)
```

再计算 visible range，避免原先很高的 scroll offset 在 shrink 后短暂落进空 range。

## Bootstrap 与 scheduling

初始 BSN 展开时 viewport 的真实 layout height 还不可用，因此 Scene 只创建 ScrollArea shell 和 spacers，不猜 row count。

首次拿到有效 `ComputedNode` viewport size 后再建立第一批 rows。

运行期 range reconciliation 的触发来源包括：

- `ScrollPosition` 变化；
- viewport `ComputedNode` size 变化；
- `WidgetryListModel<T>` Changed；
- 首次获得有效 viewport size。

这些变化应汇入同一 reconciliation 路径，而不是各自维护一套 row rebuild 逻辑。

目标 scheduling 是尽量在本帧 UI layout 前把 rows/spacers 准备好，让新 rows 当帧参与 layout。Bevy 0.19.1 的精确 system set/order 在实现时依据真实 `UiSystems` 确认；这是 implementation ordering 细节，不改变本方案数据结构和行为 contract。

允许通过 Gallery/BRP 实测两个未知表现：

- bootstrap 是否出现明显一帧空白；
- 无 overscan 快速 wheel 时是否出现边缘空白。

在没有真实问题前，不加入 overscan 或复杂双 layout bootstrap。

## 本阶段测试

### Unit test

优先把数学与 diff 抽成可单测的纯逻辑：

- visible range：顶部、部分可见、非整数 viewport、末尾 clamp、empty model、zero/无效 viewport；
- scroll max/clamp；
- range overlap：向下/向上 1 行、扩张、缩小、无 overlap；
- `rows[i] <=> range.start + i` invariant。

### Integration test

从 public/真实 ECS 组合验证：

- Small List 在 viewport 足够大时所有 model item 都成为 rendered rows；
- Large List 例如 10,000 项时仅有 visible row 数量；
- `100..120 -> 101..121` 时 overlap row entity id 保持，只删/建两端；
- renderer 用 `Arc<AtomicUsize>` 或调用历史记录验证单个 visible revision 只重建目标 row；
- offscreen update 不调用 renderer，滚入后显示最新 value；
- disabled metadata change 不调用 renderer；
- model shrink 后 ScrollPosition 和 range 正确；
- 至少一组真实 `UiPlugin` layout test，把 viewport size -> visible range -> spacer -> ScrollArea content height 串起来，不全部通过手工伪造 ComputedNode。

## 预期产出

得到一个小数据自然全部显示、大数据只实例化可见 rows 的稳定 virtualization core。滚动、resize、model update、structural change 和 renderer lifecycle 都成立，但 logical selection/active/input semantics 仍由下一方案补齐。

## 与前后方案的关系

本方案依赖第一方案的 stable id/revision 和第二方案的 ScrollArea runtime marker。下一方案必须把 logical selection/active 建立在 stable id 上，只把当前 row entity 作为 projection；不能反过来让 virtualization entity 成为业务 state authority。
