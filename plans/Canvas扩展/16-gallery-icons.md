# Gallery：用 Canvas 绘制图标与几何图形

## 目标

扩展现有 Canvas Gallery 页面，增加 Icons & Shapes
演示，展示基础几何、Path、Layer、Group、渐变、复制和对齐如何共同构成一个可编辑图标作品。

## 演示内容

准备一组由 Canvas Document API 创建的矢量图标： - 几何组合图标。 - 简单
Logo/徽章。 - 星形、多边形和圆形装饰。 -
带线性或径向渐变的按钮/装饰图标。

作品由真实 Shape、Path、Group、Layer 和 Paint
数据组成。用户可以选中局部图形、调整颜色与渐变、移动/缩放、分组、切换
Layer、复制和对齐。

## 页面组织

继续使用现有 `GalleryPage::Canvas` 和 `CanvasDemoPlugin`，在 Canvas
页面内增加演示选择器，不新增独立顶层 GalleryPage。每个演示拥有独立
Document；切换演示时释放旧 View/Renderer 资源并创建新场景。

工具栏和属性面板通过 Canvas 公开 API 操作 Document，不直接改写 Canvas
内部组件。Gallery 专属演示数据与页面状态留在 Gallery crate。

## 完成条件与验证

-   保留现有 Basics 演示，包括双 View 共享 Document、Path 编辑和导出。
-   Icons & Shapes 初始作品由可编辑矢量项构成，不是 PNG 背景。
-   能实际选择、移动、分组、切换 Layer、编辑渐变、复制和对齐。
-   SVG/PNG 导出包含当前 Document 的正式图形。
-   页面切换时旧 Document/View/Renderer 资源正确释放。
-   Gallery 能正常编译。
-   BRP 验证选择、拖动、渐变修改、Undo/Redo，并用 Screenshot 检查结果。

## 设计边界

本演示聚焦通用图标与几何组合，不承担流程图 Connector 或龙娘作品展示。
