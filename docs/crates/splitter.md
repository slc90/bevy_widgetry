# Splitter

`bevy_widgetry_splitter` 提供固定 Pane A / Handle / Pane B 的双区域布局，通过 Facade 的默认 `splitter` Feature 与 `splitter` 模块导出。安装 `WidgetrySplitterPlugin` 后由原生 Bevy UI 执行最终布局。

用 BSN `@WidgetrySplitter` 构造，`pane_a` / `pane_b` 接收可选 Scene，直接 Patch 对应 Pane 布局实体，并可提供 Children 或嵌套 Splitter。省略 Scene 表示空 Pane。调用方管理内容、滚动、裁剪及交叉轴布局。

一次性 Props 的 `orientation` 默认为 `Horizontal`（A 左 B 右），`Vertical` 表示 A 上 B 下。`split_ratio` 默认 0.5，表示 A 在主轴扣除 `handle_thickness`（默认 8px）后的空间份额。比例必须有限且属于 `[0,1]`，Handle 厚度必须有限且为正。

Pane 原生 `Node` 的主轴 min/max 由调用方维护。Splitter 托管根的主轴有效 min/max，以及两个 Pane 与 Handle 的主轴尺寸、flex 分配字段。两侧最小外部尺寸之和加 Handle 为根最小尺寸；双侧有限最大外部尺寸之和加 Handle 为根最大尺寸。原生 `box_sizing` 保留，ContentBox 的 padding/border 计入外部尺寸，分配时再扣回，根自身的盒模型也遵循同一规则；BorderBox 不重复添加这些尺寸。调用方应保留内部三节点层级，不直接修改托管字段。交叉轴及 Pane 内容的外观不由 Splitter 管理。

`WidgetrySplitter::split_ratio(world, root)` 返回唯一持久比例状态。首次有效布局且根范围稳定后把初始比例修正到可达范围；之后被动 Resize 或约束改变仅影响实际几何，保留目标比例，空间恢复后再尝试该目标。主轴百分比 inset 与原生尺寸约束一起解算，不把它们固化为初帧的像素值。布局同步可跨原生 Layout pass 收敛，并请求后续重绘；初始化与被动变化不发送业务通知。

约束支持 Bevy 原生长度单位；百分比约束相对根内容主轴计算，根范围解算同一比例关系。Pane 最小约束合计百分比必须小于 100%，为 Handle 留出空间。矛盾、非有限或负约束以及内部节点丢失通过 ERROR 与宿主 error handler 报告，持续相同异常不重复记录日志。父级无法满足最小尺寸时保留溢出，由调用方组合 ScrollArea 或裁剪策略。

当前提供基础布局与比例查询，尚无运行时 Setter、拖动及 Handle 视觉皮肤。
