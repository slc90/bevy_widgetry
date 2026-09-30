# 扩展 ScrollArea contract 并建立 BSN ListView 外壳

## 目标

让 `WidgetryListView<T>` 保持 Widgetry 统一的 BSN 使用方式，同时真正复用现有 ScrollArea 的 viewport、`ScrollPosition`、wheel/trackpad、content geometry 与滚动范围，不重新实现一套 scroll container。

这一阶段还要解决两个会直接影响 correctness 的组合问题：ListView 需要稳定访问 ScrollArea content entity，以及 ListView 与 ScrollArea 在同一个 focus root 上时不能同时消费同一组 keyboard 输入。

## ScrollArea 必要 contract 扩展

只修改 ListView 实际需要的部分，不顺手重构 ScrollArea。

### Public content marker

现有 ScrollArea 已公开 `WidgetryScrollAreaViewport`，但 content marker 仍是 crate-private。ListView 是独立 sibling crate，必须能稳定定位 spacer/row 应挂载的 content entity，因此把私有 marker 提升并命名为：

```text
WidgetryScrollAreaContent
```

这是 runtime integration API，不是为了测试扩大 visibility。

### Keyboard-scroll 开关

ScrollArea 当前自己支持 keyboard scrolling；ListView 又需要接管 Arrow/Home/End/Space/Enter，并自己定义 PageUp/PageDown。若两者共用 root 且都响应，会产生双重输入语义和事件传播冲突。

因此 ScrollArea 增加一个默认保持现状的 keyboard-scroll 开关：

- 普通 ScrollArea 默认继续启用当前 keyboard 行为；
- ListView 构造内部 ScrollArea 时关闭 keyboard scroll；
- wheel/trackpad、`ScrollPosition`、程序化滚动完全不受影响。

这样修改不会改变现有 ScrollArea consumer 的默认行为。

## BSN hierarchy 与 root ownership

`WidgetryListView<T>` 不在外面再包一层独立 root，而是把 `@WidgetryScrollArea` 组合到 **同一个 root entity** 上。

目标结构概念上为：

```text
WidgetryListView root
= WidgetryListView<T>
+ ScrollArea root identity
+ focus/tab identity
+ ListView chrome

└─ WidgetryScrollAreaViewport
   └─ WidgetryScrollAreaContent
      ├─ TopSpacer
      └─ BottomSpacer
```

动态 rows 由下一方案加入。

同 root 设计的好处是：

- ListView 只有一个 focus root；
- root `TabIndex`、border、disabled 都没有两层同步问题；
- wheel/ScrollPosition 仍直接落在 ScrollArea runtime；
- 后续 accessibility root 与 logical active projection 都集中在一个 entity。

## SceneComponent public usage

调用方正常写法保持 BSN：

```rust
bsn! {
    @WidgetryListView::<FileEntry> {
        @source: model_entity,
        @item_height: 32.0,
        @renderer: WidgetryListViewRenderer::new(
            |index, file: &FileEntry| {
                bsn_list![(Text(file.name.clone()))]
            }
        ),
    }
}
```

Bevy 0.19.1 的 SceneComponent derive 会带 generic 参数，BSN parser 也按 Rust type path 解析 generic type，因此“显式 `T` 的 generic SceneComponent”是设计方向。最终 turbofish 的精确宏拼写由 compile test 验证；如果宏对某个具体 token 形式有限制，只调整书写形式，不改变 public design 中“调用方明确 item type `T`”这一点。

## ListView 创建期配置

Scene props：

- `source`：required；
- `renderer`：required；
- `item_height`：默认 32 logical px，必须 finite 且 `> 0`。

创建完成后，source/item_height/renderer 第一版都视为 immutable configuration，不提供 runtime replacement API。

ListView 必须处在 **有界纵向 layout** 中，因为 virtualization 需要真实 viewport height。高度来源是实际 UI layout 的 `ComputedNode`，不是额外 public prop。调用方可以通过 root Node patch、父 flex/grid、percent height 等正常 Bevy layout 方式给 ListView 有限高度。

## 固定 ScrollArea 配置

ListView 内部固定：

- `ScrollAxis::Vertical`；
- vertical scrollbar `Hidden`；
- horizontal 不作为 ListView 功能暴露；
- scrollbar 不可见，但 wheel/trackpad 和 `ScrollPosition.y` 照常工作；
- ScrollArea keyboard scrolling 关闭，由 ListView 后续接管 keyboard semantics。

因此第一版 ListView public API 不提供 scrollbar policy 选择，也不提供 horizontal mode。

## 官方 ListBox 的边界

ListView root **不能**挂 `bevy_ui_widgets::ListBox` marker。

原因不是视觉，而是 runtime observer：当前应用里 ComboBox 等可能已经全局安装 `ListBoxPlugin`。官方 ListBox observer 会扫描当前真实 `ListItem` descendants 来处理 click、keyboard、focus 和 `ActiveDescendant`；虚拟化只保留 visible rows，官方 observer 看到的是不完整数据集合，会与我们自己的 logical state 冲突。

因此 hierarchy contract 提前固定：

- root 后续自己挂 `AccessibilityNode(Role::ListBox)` 与 `ActiveDescendant`；
- 不挂官方 `ListBox` marker；
- rendered row 可以继续使用官方 `ListItem`，复用它的 `Role::ListItem` 与 `Selectable` required component。

这不是重新实现 ListItem，而是只绕开不适合虚拟化 source-of-truth 的官方 ListBox group behavior。

## Plugin dependency

`WidgetryListViewPlugin` 应自动补齐 ListView 自身明确依赖的 Widgetry infrastructure，例如：

- `WidgetryScrollAreaPlugin`；
- Theme；
- ForegroundColor propagation。

与现有 Widgetry plugin 风格一致，避免要求调用方额外知道内部 sibling dependency。

但应用级真实 input/picking/InputFocus 派发仍沿用现有项目 contract，不因为 ListView 再造一套 app bootstrap。

## 本阶段测试

Integration test 从 public API 验证：

- `@WidgetryListView::<T>` generic BSN 能编译、spawn；
- root 上存在 ListView 与 ScrollArea 所需 identity，而不是额外 nested focus root；
- 能通过 public marker 找到唯一 viewport/content；
- viewport 有原生 `ScrollPosition`；
- vertical scrollbar 是 Hidden；
- root Node patch 不被 Scene 默认覆盖；
- root 使用 ListView 所需 TabIndex；
- ScrollArea keyboard-scroll 对普通 ScrollArea 默认不变，而 ListView 内关闭；
- source/renderer 缺失或 item_height 非法时按项目错误 contract 失败。

这些 integration test 验证的是 Widgetry 与 Bevy/ScrollArea 的真实组合边界，不重复测试 Bevy 自己的内部 scroll 算法。

## 预期产出

得到一个可以从 facade 通过 BSN 构造的 ListView shell：有真实 ScrollArea viewport/content/ScrollPosition、有唯一 focus root、有稳定 runtime marker、bar 隐藏且 keyboard ownership 已明确，但尚不要求生成动态 rows。

## 与前后方案的关系

前一方案提供 generic type/config；本方案把它变成真实 UI shell。下一方案只需要依赖 `WidgetryScrollAreaViewport`、`WidgetryScrollAreaContent`、`ScrollPosition` 和固定 item_height，即可实现 virtualization，不需要知道 ScrollArea 内部 grid/gutter/scrollbar 细节。
