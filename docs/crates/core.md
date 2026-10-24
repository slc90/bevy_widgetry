# Core

## 功能概述

提供 Widgetry 多个控件共同需要的 UI 基础设施，包括内容配色、默认字体、SVG Icon、有效 Disabled、Pointer State、Scene 错误处理、公共离屏渲染和 UI 准备阶段能力。

## 公共离屏渲染

`WidgetryOffscreenPlugin` 安装一个 App 共享的离屏层号池和 Image Asset 支持。`OffscreenTarget::allocate` 创建独立目标并租用唯一层号；消费者通过 BSN 创建相机与绘图 Scene，自行指定 Projection、ClearColor 和 MSAA，并将所有相机与几何绑定到该层号。

`OffscreenTarget::bind` 接收 UI Viewport、Camera 和专属 Scene，将目标交给 `OffscreenViewport`；消费者在 Owner 上挂载此 Component。绑定失败会释放目标、层号及传入的专属 Scene。绑定前的 Scene 构造失败由消费者调用 `OffscreenTarget::release` 回滚。消费者专属或共享 Mesh/Material 不由公共层删除。

消费者在 `OffscreenUpdate` Set 内调用 `OffscreenViewport::update`，该 Set 位于 UI Layout、Visibility 传播之后和原生 Viewport 纹理尺寸更新之前。返回当前物理尺寸，隐藏或零尺寸时返回 None 并停用相机；原生 Viewport 系统维护目标 Image 尺寸。移除 Owner 上的 `OffscreenViewport` 会回收专属 Image、层号和 Scene，且仅清除仍指向该相机的 Viewport 关联。删除一个目标不影响其他实例或宿主 Asset。

## 当前设计思路

Core 尽量把跨 Widget 的机制集中管理，同时让各 Widget 保有自己的业务 State。共享能力通过专门 Plugin、Component 或公开适配方法装配，不要求应用接受一套取代 Bevy 的 UI Runtime。

## 主要功能

- 默认字体设置：只对适用的新增 `TextFont` 使用缺省字体，保留调用方的显式字体。
- `WidgetryText` 和内容 Foreground：按最近颜色作用域、Theme 与覆盖解析 Text / TextSpan。
- `WidgetryIcon`：通过 SVG 生成图像，可设置、替换或清除显式颜色与 SVG 输入。
- 有效 Disabled：组合祖先继承、本地请求和控件原生限制，并投影到官方输入状态。
- Pointer：把有效 Mouse、Custom、Touch 的 Hover 命中投影到官方 Hovered / DirectlyHovered；管理所标记控件的 Pressed Ownership 清理。
- Scene：提供同步及 Deferred 构造的错误传播、失败预约 Entity 清理。
- 提供动态 UI 同帧准备与共享 Z-index 标识等能力。
- `exclusive_selection` 是 RadioGroup / Segmented 共用的内部选择能力，维护结构回退历史、首次选择策略及互斥 Checked 提交；不建立第二份 Selection，不通过 Facade 公开。
- `WidgetryInputTarget` 提供 Workspace 内组合控件根到实际输入实体的路由元数据；Window 的 Tab 与初始焦点解析使用同一目标，不承载控件的编辑或业务状态。

## 行为与状态契约

`WidgetryPointerPressQuery::started_this_frame(entity)` 只读查询共享 Pointer 层在本帧确立的新按压会话，供 `Update` 等后续阶段区分真实重按与 `Pressed` 恢复。结果每个 `PreUpdate` 重建；无关 Pointer 或鼠标键导致的 `Pressed` 恢复不计为新会话。

原生 Mouse 所属窗口失焦时终止受管按压，即使外部松手未产生 Pointer Cancel/Release，或失焦与重新聚焦合并在同一帧，也不会恢复旧会话。Custom/Touch Pointer 不以原生窗口焦点作为此取消条件。

存在 Bevy 默认 Mouse writer 时，Core 使用保持其移动/按键/滚轮语义的有序适配替换它，在 Winit 的失焦事件位置产生 `PointerAction::Cancel`。Cancel 后发送三键 Release 同步官方 `PointerPressState`，已清除的 click/drag 状态不会产生松手 Activate。同批次的后续新按下能建立独立会话，外部丢失 Release 也不会吞掉重新进入后的第一次按下。失焦帧使用本帧真实 Backend Hit 按相同 Layer/Depth/Pickable 规则保留 Mouse 命中，避免 Bevy 的整帧 Cancel 过滤丢弃前后按下。Custom/Touch 输入源不替换；宿主自行替换的 Mouse writer 不接管。原生 Button 接受的鼠标键均保留其原有 owner 语义。

内部限制可通过 `set_intrinsic_disabled_for<R>` 按理由类型独立设置和撤销；已有 `set_intrinsic_disabled` 使用独立默认理由。所有内部理由按 OR 组合，撤销其中一项不会解除其他理由。

UI Hierarchy 的 Disabled 有效值采用 OR 继承，本地请求即使在继承期间修改也必须保留。官方 Hovered 包含子代命中，DirectlyHovered 限于直接命中；共享 Pointer 投影拥有相应写入权。Widgetry 内容颜色只影响使用标记的 Text，原生 Text 仍保留自身定义。Scene Props 在构造后不充当运行时状态。

Disabled 不通过 Native Window Owner 关系跨窗口传播。单独禁用的内容使用对应 Theme Fallback。`WidgetryPointerPressed` 只清理标记控件所拥有的真实 Pointer Press 会话，不接管预先存在的程序化 Pressed 或官方正常 Activation。默认字体可通过 `WidgetryFontPlugin` 或 `WidgetryAppExt::set_default_font` 启用，诊断状态与 Z-index 标识供自定义控件复用。

## 边界情况与错误处理

- 无效 Scene 展开、必要 UI 结构缺失和非法 Icon 更新通过 `BevyError` / 宿主 Handler 传递。
- 构造失败仅清理本次新建的预约空 Entity，不回滚已经提交的业务副作用。
- SVG 加载失败、等待或过时更新需要按 Icon 的当前请求身份判断，不应破坏此前有效内容。
- 有效 Disabled 在 Deferred 操作实际 Apply / Flush 前不代表已生效。

## 已知限制

- Font、Icon 和 Scene 各能力有各自的装配前提；Core 不是全局业务 Model。
- 官方 Hover State 的独立写入者需要与共享 Pointer Plugin 协调。
- Headless UI 证明的是 ECS / Layout 阶段行为，不等于原生窗口的完整显示验证。


WidgetryPointerPlugin 安装本帧命中缓存。`pointer::refresh_hover` 在 Backend 完成本帧 Picking、事件批次开始之前，为指定 Pointer 恢复当前路由：有效的新捕获 owner 优先，否则按本帧 Backend Hit 的 Layer/Depth/Pickable 重建命中。事件批次已经开始时保留当前 HoverMap，由下一帧生成完整的 Out/Leave 与 Over/Enter 迁移，避免无声跳过事件。共享层只缓存本帧消息 ID，不重放旧帧命中；原生 Mouse 的有序失焦恢复与 Canvas 捕获结束共用该机制。此入口属于 Workspace 内输入组合能力，不通过 Facade 导出。
