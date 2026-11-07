# Tooltip

## 功能概述

提供 Hover 后显示的辅助 Popup，可用于标签、操作提示和包含 Text / Icon 的富内容说明。

## 当前设计思路

Tooltip 锚点由当前有效 Pointer Hover 及 Hierarchy 解析；Content Factory 在实际显示时构造新的 BSN 内容。一个 App 同时只有一个显示中的 Tooltip，先后 Hover 的过渡使用 Cold / Warm 计时。

`WidgetryTooltipHeadless` / `WidgetryTooltipHeadlessPlugin` 独立提供同一 Hover、延迟与全局单实例协调，不要求默认 Content Factory、Popover 或配色。默认 `WidgetryTooltip` 使用同一行为；默认显示 Observer 只处理默认实例。

独立 Plugin 确保安装 Core Pointer 能力，保留原生 WindowFocused 失焦到 Cancel 的转换；无需依赖其他 Widgetry 控件间接安装。

`ShowTooltip` 是本轮展示请求，包含 Anchor `source` 与不可自行构造的展示身份，不表示 UI 已创建。外部 Style 在排队操作实际执行时先检查 `is_current(world)` 和 `popup(world)`，再以 BSN 构造自有内容；成功后调用 `register_popup(world, popup)` 登记根。登记不构造 UI，不限制根的 Parent 或定位方式。根必须是本次新建、只拥有该提示内容的 Entity，不得登记 Anchor 或承载其他界面的容器。构造失败由 Scene 调用方清理根并传播错误。

登记成功返回 `true`。重复登记已拥有展示身份的根返回 `false`，保留该根；重复或过期请求登记另一份未拥有身份的新根时返回 `false` 并清理新根。不存在的根及 Anchor 自身返回 `TooltipError::InvalidPopup`。`TooltipPopupOwner` 提供只读 `source()` / `presentation()`，由登记 API 管理，不直接移除或替换该 Component。根及后代在首次实际 Picking 前设置 IGNORE。

`HideTooltip::presentation()` 与对应 Show 身份相同。库使用全局归属清理登记的根和 Children，Anchor Despawn、移除 Headless、离开或输入源失效均适用，不依赖 Anchor 的直接 Children 或局部 Observer。重放过期 Hide 不影响较新的展示。外部 Style 无须复制 Hover 状态机或自行清理已登记的根；其他非 Entity 资源可以通过公开 Hide 事件及其展示身份释放。

Popup 构造时直接采用 Anchor 当前有效的 Theme / 覆盖配色；后续仅在新建、Anchor 绑定或覆盖色变化、主题更新及必要颜色输出重新加入时同步 Background / Border / Foreground。稳定显示不重复解析相同配色，新增富内容仍通过既有前景传播获得颜色。配色更新不重建内容，也不跳过 Hover 检测或真实时间推进。

## 主要功能

- Hover Anchor 自动定位以及祖先绑定查找。
- 初次 Hover 延时 200ms；短时间连续切换时 Warm 延时 50ms，离开后保留 300ms 的 Warm 窗口。
- Popup 根据目标和 Window 空间在下、上、右、左候选位置中选择。
- 每次展示按 Factory 构造内容；Theme 更新 Popup 的 Background、Border 与 Foreground。
- Disabled 目标仍可展示说明。

## 行为与状态契约

计时使用真实经过时间。相同 Anchor 持续 Hover 不重复构造 Content；Owner / Window 目标改变时重新进行 Cold 等待。离开、Cancel、目标或 Pointer 失效时清理 Popup，不保留孤儿 Entity。只有有效输入源可决定 Owner，无输入时要求唯一有效 Hover Source。

## 边界情况与错误处理

默认 Style 构造必须提供 Content Factory；纯 Headless 不需要。默认 Scene 初始化和运行期 Factory 构造缺失内容均返回 `TooltipError::MissingContent`，保留 Anchor；不同无效请求各自创建诊断。颜色 API 返回 `TooltipFailure`，区分失效目标、缺失配置、托管拒绝，并在 Core 颜色错误上保留原 source / origin 及字段上下文。

显示 Popup 时追加 Anchor / Popup 身份；嵌套内容 Scene 失败保留原诊断、ID、source 和 Backtrace，清理失败 Popup 后通过 `Severity::Error` 交给宿主。既有 Popup 缺失 Anchor 颜色时保留原 Query 错误，每个 Popup 独立保存持续异常身份；多 Popup 失败批量传播到宿主，持续失败不重新捕栈或重复源头 ERROR，恢复时记录一次 INFO。纯传播不产生 handled DEBUG。

正常 Hover 候选切换、无目标查询、重复显示、隐藏、Owner / Pointer 失效和构造执行前的源实体销毁属于合法状态路径，不创建诊断。不可用 Anchor、错误的 Window Target 或不再存在的 Owner 不能继续显示旧内容。Popup 内容应在首次创建帧具备正确 Picking IGNORE 与 UI 准备状态。

## 已知限制

- 当前不支持在 App 中同时显示多个 Tooltip。
- 延迟按真实时间推进；Headless Test 必须明确控制时间前进。
- Popup 显示内容是按次重建，而不是持久编辑的业务 State。

