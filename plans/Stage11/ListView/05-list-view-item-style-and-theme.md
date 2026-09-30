# 完成 ListView 与 ListItem style/theme

## 目标

在 headless behavior 已稳定后，为 ListView 提供克制、与现有 Widgetry theme 一致的默认视觉。Style 不新增业务 state、不新增 ColorTheme token，也不把 renderer 变成 row chrome 的 owner。

用户对 ListView style 的硬要求只有圆角和 border，其余由本方案固定，减少 public style surface。

## ListView root style

Root 视觉只强调外壳：

- border: 1px；
- border radius: 4px；
- rounded clipping，保证内部 rows 不画出圆角边界；
- 不额外设置 ListView background；
- 不做 root hover / pressed 配色；
- vertical scrollbar 始终 Hidden。

Border state：

```text
normal   -> control_border
focused  -> control_border_active
disabled -> control_border_disabled
```

Disabled 优先于 focus。

Root style 只反映整个控件的 focus/disabled，不把 pointer hover 扩散成整块 ListView 外框 hover，避免鼠标进入列表时 chrome 过度变化。

## Row layout

每个 `WidgetryListViewItem` wrapper：

```text
width: 100%
height: item_height
flex_shrink: 0
horizontal padding: 8px
border: 1px
border radius: 3px
vertical margin/gap: 0
```

这里的 `height=item_height` 指总 layout extent；border/padding 必须包含在固定高度里，不能改变第三方案的 virtualization 数学。

Renderer 只填充 row direct children，不控制 row padding/background/border/selection chrome。

## Row state priority

背景优先级固定为：

```text
effective disabled > pressed > hovered > selected > normal
```

其中 effective disabled = root disabled 或 item disabled。

背景映射：

```text
normal             -> transparent
selected           -> item_background_selected
hovered            -> item_background_hovered
pressed            -> control_background_pressed
effective disabled -> transparent
```

这保持现有 ComboBox item 的“hover 覆盖 selected”习惯，同时给 ListView row 增加 Pressed visual。

Foreground：

```text
normal / selected / hovered / pressed -> foreground
effective disabled                     -> foreground_disabled
```

Row 挂 `Propagate<ForegroundColor>`，renderer 内普通 Text/Icon 默认继承当前 foreground；renderer 如显式覆盖自己的颜色，则属于业务内容自己的决定。

## Active visual

Active 与 selected 是正交 state，所以不通过背景抢优先级，而通过 row border 表达：

```text
root focused
+ row.id == logical active
+ row 非 effective disabled
    -> control_border_active

其他情况
    -> transparent border
```

因此可以同时出现：

- Item A selected：selected background；
- Item B active：active border；

或者同一个 item 同时 selected + active：背景和 border 同时存在。

Root/item disabled 时 suppress active border，disabled visual 优先；logical active 本身不因此清除。

## Hover / Pressed source

Style 不维护自己的 hover/pressed state：

- hover 读取 Bevy `Hovered`；
- pressed 读取第四方案维护的 Bevy `Pressed`；
- selected 读取当前 row `Selected` projection；
- disabled 读取 effective disabled；
- active 由 root logical state + focus + row id 推导。

这样视觉 resolver 只是纯 projection，不产生第二套 interaction state machine。

## Theme

不为 ListView 增加新的 `ColorTheme` token，完全复用现有：

- `control_border`；
- `control_border_active`；
- `control_border_disabled`；
- `control_background_pressed`；
- `item_background_hovered`；
- `item_background_selected`；
- `foreground`；
- `foreground_disabled`。

`ThemeChanged` 时立即刷新：

- 当前 ListView root；
- 当前 rendered rows；
- 当前 selected/hovered/pressed/active/disabled 所对应的正确新 theme color。

Virtualization 特别要求：**新生成 row 不能依赖过去是否收到 ThemeChanged event**。创建 row 时直接读取当前 `ThemeMode`，所以“先切到 Light，再滚入一个之前未实例化的 row”必须直接得到 Light style。

## Style resolver 组织

建议像现有 Button/RadioGroup 一样，把 state -> 完整 style 输出收敛成 resolver/apply 函数，避免 background、border、foreground 分别在不同 system 中得出不一致结果。

Root 和 row 可以分别有自己的 style data/resolver；不要为了复用强行抽到 core。

Interaction state 添加/删除都要覆盖：例如 `Pressed`/`InteractionDisabled` 的 RemovedComponents 需要触发重新解析，不能只监听 Added/Changed。

## 本阶段测试

### Unit test

使用每个 token 都不同的测试 `ColorTheme`，验证：

- `disabled > pressed > hovered > selected > normal`；
- foreground disabled 优先；
- active border 与背景 priority 正交；
- active + selected 可同时表达；
- disabled suppress active border；
- root disabled > focus。

### Integration test

验证真实 Component projection：

- root normal/focus/disabled border；
- row normal/selected/hovered/pressed/disabled；
- item disabled 与 root disabled effective style；
- active row border；
- renderer descendant 的 `ForegroundColor` propagation；
- `ThemeChanged` 立即刷新已有 root/rows；
- 先切 Theme、再滚动创建新 row，直接使用当前 ThemeMode；
- style update 不破坏 row 固定 item_height、padding/border geometry。

## 预期产出

完成 ListView 与 ListItem 默认视觉，所有颜色都由当前 theme + headless state 推导，没有额外 style state 副本，也没有把 renderer 与 virtualization contract 搅在一起。

## 与前后方案的关系

前一方案提供所有 style input，本方案只做 visual projection。下一方案用 Gallery/BRP 从真实用户视角同时验证 interaction、virtualization、disabled 和 Theme，不再新增核心 style semantics。
