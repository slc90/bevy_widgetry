# 增加可编辑 Canvas Text 项

## 目标

将文字作为 Document
中的正式绘图项，用于图标标题、流程图节点说明、插画标注和表情包文字。Text
能参与选择、变换、分组、Layer、复制和导出。

## Text 数据模型

新增 `CanvasText` Item，保存： - 文本内容，允许显式换行。 - 字号，使用
Document 单位。 - 文字颜色。 - 水平对齐方式（Start / Center / End）。 -
局部 Transform。

首版一个 Text Item 使用单一字号、颜色和对齐方式。字体使用 Canvas
的默认字体资源；不提供富文本、混合字号、字重系统或沿路径文字。

## 创建与编辑

CanvasView 提供 Text 放置工具。用户点击画布确定位置后创建 Text
Item。Text 内容和属性通过 Document API 修改；Canvas
本身提供读取/更新接口，不在 Canvas crate 内实现完整文本编辑器或 IME。

Gallery 的属性面板使用现有 `WidgetryTextField`
编辑选中文本的内容，并使用数值/颜色控件调整字号、颜色和对齐。这样 Canvas
crate 不需要为了内嵌编辑框而依赖兄弟 TextField
crate；其他宿主应用也可以使用自己的属性面板。

Text 是普通 CanvasItem，可以放入 Layer 或 Group。Text 的 Transform 与
Group 变换按统一层级规则组合。

## Renderer、命中与导出

每个 View 的 Renderer 为 Text 创建独立文本代理，使用 Bevy 2D Text
渲染能力。字体布局结果同时用于可见渲染、文本边界、Picking、选择框和对齐计算，不能用估算字符数替代实际布局尺寸。

SVG 导出写入 SVG Text 内容和基本字号/颜色/对齐属性；PNG 通过 Canvas
Renderer 输出。Text 不作为普通 Shape Mesh 缓存，也不把字形轮廓永久转换为
Path。

## 完成条件与验证

-   Text 创建、内容更新、字号/颜色/对齐更新、移动、选择、删除均有测试。
-   Text 可进入 Layer 和 Group，并参与顺序、Undo/Redo 和复制。
-   多 View 共享已提交 Text，View 本地属性编辑草稿不影响其他
    View，提交后同步。
-   文本布局边界与实际渲染一致，并用于命中、选择框和对齐。
-   SVG/PNG 导出包含 Text，位置、字号、颜色和对齐与 Canvas 一致。
-   TextField 获得焦点时，Canvas Delete/Undo 快捷键不抢占文本编辑。
-   更新 Coverage Model、Canvas 功能文档和设计文档。
-   Gallery 通过 BRP 验证文字创建、属性编辑、移动和导出。

## 设计边界

不实现完整字体排版软件、自动换行布局、富文本、TextPath、字体编辑器或 IME
内嵌编辑层。文字内容由宿主属性面板编辑。
