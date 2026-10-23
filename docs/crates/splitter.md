# Splitter

`bevy_widgetry_splitter` 提供固定 Pane A / Handle / Pane B 的双区域布局，通过 Facade 的默认 `splitter` Feature 与 `splitter` 模块导出。安装 `WidgetrySplitterPlugin` 后由原生 Bevy UI 执行最终布局。

用 BSN `@WidgetrySplitter` 构造，`pane_a` / `pane_b` 接收可选 Scene，直接 Patch 对应 Pane 布局实体，并可提供 Children 或嵌套 Splitter。省略 Scene 表示空 Pane。调用方管理内容、滚动、裁剪及交叉轴布局。

一次性 Props 的 `orientation` 默认为 `Horizontal`（A 左 B 右），`Vertical` 表示 A 上 B 下。`split_ratio` 默认 0.5，表示 A 在主轴扣除 `handle_thickness`（默认 8px）后的空间份额。比例必须有限且属于 `[0,1]`，Handle 厚度必须有限且为正。

Pane 原生 `Node` 的主轴 min/max 由调用方维护。Splitter 托管根的主轴有效 min/max，以及两个 Pane 与 Handle 的主轴尺寸、flex 分配字段。两侧最小外部尺寸之和加 Handle 为根最小尺寸；双侧有限最大外部尺寸之和加 Handle 为根最大尺寸。原生 `box_sizing` 保留，ContentBox 的 padding/border 计入外部尺寸，分配时再扣回，根自身的盒模型也遵循同一规则；BorderBox 不重复添加这些尺寸。调用方应保留内部三节点层级，不直接修改托管字段。交叉轴及 Pane 内容的外观不由 Splitter 管理。

`WidgetrySplitter::split_ratio(world, root)` 返回唯一持久比例状态。首次有效布局且根范围稳定后把初始比例修正到可达范围；之后被动 Resize 或约束改变仅影响实际几何，保留目标比例，空间恢复后再尝试该目标。主轴百分比 inset 与原生尺寸约束一起解算，不把它们固化为初帧的像素值。布局同步可跨原生 Layout pass 收敛，并请求后续重绘；初始化与被动变化不发送业务通知。

约束支持 Bevy 原生长度单位；百分比约束相对根内容主轴计算，根范围解算同一比例关系。Pane 最小约束合计百分比必须小于 100%，为 Handle 留出空间。矛盾、非有限或负约束以及内部节点丢失通过 ERROR 与宿主 error handler 报告，持续相同异常不重复记录日志。父级无法满足最小尺寸时保留溢出，由调用方组合 ScrollArea 或裁剪策略。

`set_split_ratio_in_world(world, root, ratio)` 同步校验、依据当前原生布局与两侧约束 Clamp 后提交；返回 `Ok(true)` 表示真实改变，合法同值为 `Ok(false)`。`set_split_ratio(commands, root, ratio)` 排队，在命令实际执行时完成同一校验与提交，失败交给宿主 error handler。请求必须有限且属于 `[0,1]`，当前必须已完成初始化并具有有效主轴布局；未布局、隐藏后无有效尺寸、损坏内部结构或不可满足的当前约束均返回错误，不延迟保存请求，不部分提交。

主动 Setter 保存最终可达比例，从 Splitter 根发出 `ValueChange<f32>`（`source` 为根、程序化操作 `is_final=true`），回调中可读到已经提交的比例；同值、初始化与被动布局变化保持静默。事件不承诺渲染布局同一时点已经完成。即使之前保存目标因被动约束暂时不可达，主动 Setter 仍按当下可达区间重新提交，不会继续保存一个不可达请求。

Handle 仅接受主指针按钮拖动，不加入Tab导航，也不响应Esc/方向键。按住后通过Pointer Capture持续接收区域外移动；每次Move按当前比例和当前可达区间处理本次主轴位移，边界外位移不积累，反向可立即恢复。真实变化从根发出 `ValueChange<f32>`，拖动中的 `is_final=false`；无变化或仅Release不额外通知。程序化更新与拖动读取同一状态，手势不保存第二份比例。

Release、Cancel、失焦、Capture/Pointer丢失、隐藏或禁用结束手势，清理Capture及Pressed，保留最后提交的比例。Despawn释放本控件拥有的Capture，不干扰其他Owner。

`resizable` Props默认true；只读 `resizable(world,root)` 与 `set_resizable_in_world` / `set_resizable(commands,root,bool)` 在运行时查询/更新此配置。false只禁止Handle手势，Pane内部Button/TextField等仍正常交互；关闭正在拖动的实例会立即结束手势。合法同值返回false、实际改变返回true，配置变化本身不发比例通知。原生根/祖先 `InteractionDisabled` 继续使整个子树继承禁用。两种禁用都不阻止合法程序比例Setter。

Handle 内居中绘制细线：一次性 `line_thickness` Props 默认 2px，必须有限、为正且不超过 `handle_thickness`。命中区域仍为整个 Handle；线条 `Pickable::IGNORE`，不会拦截输入。横向线条贯穿高度、纵向线条贯穿宽度，Hover/Dragging 不改变几何。

`WidgetryTheme::splitter` 提供 Normal、Hovered、Dragging、Disabled 线条颜色，优先级为 Disabled > Dragging > Hovered > Normal。`resizable=false` 与有效 Disabled 均弱化 Handle 并撤销其 Resize 光标；启用时使用原生 `EntityCursor` 的 EwResize/NsResize，交给既有 Widgetry Cursor 体系解析，支持 System/Kamuri。两侧 Pane 的外观保持调用方所有。

`colors` Props 初始化 `WidgetrySplitterColorOverrides`（四态 `Option<Color>`）；运行时由该类型的 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 查询、替换、清除。缺省字段随当前四套 Theme 解析，主题切换保留 Override 和比例。Setter 同值返回 false，真实变更返回 true；错误目标、非有限颜色或越界 alpha 返回 Error 且不修改已有配置。Commands 在执行时校验，不发送比例通知。
