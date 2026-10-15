# Table

## 功能概述

提供具有异构 Cell 值、二维虚拟化和 Row / Column / Cell Selection 的 Table 组件。适用于可滚动、可调整列宽的数据表格。

## 当前设计思路

Model 保存 Generic Row 与 Column 定义，按 Column Projection 提供 Cell Value。按值类型注册的 Cell / Header Renderer 负责内容；Table View 分别管理 Layout、Selection、Focused Cell、Scroll 和交互状态。可见 Cell 只是 Row / Column ID 组合的投影。

## 主要功能

- Model 支持 Row / Column Insert、Remove、Move、Edit，保持 Stable ID 和 Revision。
- 分别显示 Body、Column Header、Row Header 与 Corner，Headers 按各自 Scroll Axis 对齐。
- Cell / Header 可拥有不同值类型和 BSN Renderer。
- Layout 支持 Row Height、Header 尺寸、Fixed / Flexible Column Width 与最小列宽。
- Pointer 选择 Row、Column 或 Cell，Keyboard 在 Cell 间移动 Focus 并 Reveal。
- 公开 API 可调整 Selection、Focused Cell 与列宽。
- Column Header Resize Handle 提供 Start、Resized、End 或 Cancel 通知。
- 按区域配置 Theme 与颜色覆盖，支持 Disabled 状态。

## 行为与状态契约

Row / Column Stable ID 仅在所属 Model 内有效；移动保持身份，删除后不复用。Selection 与 Focused Cell 分别代表已选目标和导航位置，程序设置先提交再通知，同值不重复通知；Model Repair 默认静默。两个 View 可以共享一个 Model，但拥有各自 Layout、列宽、Selection、Focus、Scroll 和 Disabled；Renderer Registry 按 Cell / Header 值类型在 App 级注册。

## 边界情况与错误处理

缺失 Renderer、无效 Model / Source、非法 Scene 配置或失效 ID 必须明确反馈。Resize 遇到 Column / Header 失效、Cancel 或 Disabled 时结束当前 Gesture，不能对另一个身份继续写宽度。滚动中的可见 Cell 复用不改变业务 ID。

## 已知限制

- 目前同一个 Model 的多个 View 共享其 Column / Projection 定义；不支持为同一 Model 的不同 View 分别定义 Column Schema。
- 相同 Cell 值类型的 Renderer 当前在 App 级注册，不提供任意逐 View Renderer Registry。
- 二维虚拟化仅构造可见范围，不表示任意规模的 Model 操作都具有常数成本。

