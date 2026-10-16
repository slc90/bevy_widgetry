# Tooltip

## 功能概述

提供 Hover 后显示的辅助 Popup，可用于标签、操作提示和包含 Text / Icon 的富内容说明。

## 当前设计思路

Tooltip 锚点由当前有效 Pointer Hover 及 Hierarchy 解析；Content Factory 在实际显示时构造新的 BSN 内容。一个 App 同时只有一个显示中的 Tooltip，先后 Hover 的过渡使用 Cold / Warm 计时。

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

构造必须提供 Content Factory。不可用 Anchor、错误的 Window Target 或不再存在的 Owner 不能继续显示旧内容。Popup 内容应在首次创建帧具备正确 Picking IGNORE 与 UI 准备状态。

## 已知限制

- 当前不支持在 App 中同时显示多个 Tooltip。
- 延迟按真实时间推进；Headless Test 必须明确控制时间前进。
- Popup 显示内容是按次重建，而不是持久编辑的业务 State。

