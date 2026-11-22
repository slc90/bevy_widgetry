# 建立 Document 级 Undo/Redo 与编辑事务

## 目标

让现有 Shape 创建、删除、变换、Paint 修改、Path 拓扑编辑、顺序调整，以及
Layer/Group
操作都能撤销和重做，并建立后续所有新编辑功能必须接入的统一事务边界。

## 历史属于 Document

在 `CanvasDocument` 上增加 `CanvasHistory` Required Component。历史与
Document 同生命周期，不依赖 CanvasPlugin，因此在没有 View 或 Renderer 的
Headless World 中也可以执行 Undo/Redo。

多个 View 共享同一份历史。任意 View 提交的 Document
修改都进入同一历史栈；从另一个 View 执行 Undo 后，所有 View
都观察到同一份 Document 回退。View 的 center、zoom、selection、活动
Layer、工具模式和未提交草稿不进入 Document 历史。

## 一次用户意图对应一个历史条目

-   一次成功的程序化 Document API 修改形成一个历史事务。
-   批量移动、批量删除、分组、取消分组、粘贴等复合操作整体提交，不能拆成若干可部分撤销的记录。
-   拖动类手势在 Pointer Down 时建立编辑事务；中间 Pointer Move
    更新同一事务；Pointer Release 或现有语义中的普通 Cancel
    将最后已提交状态作为一个历史条目提交。
-   矩形、椭圆、Pen 和基础绘图工具的预览只存在于 View；最终创建 Item
    时才提交一条历史记录，取消草稿不产生记录。
-   Gallery
    属性面板的一次确认/提交形成一个事务；不要将同一属性输入框的每个字符都变成独立历史项。

Document 同一时间只允许一个写入型交互事务。第二个 View
可以继续平移、缩放和观察，但不能启动与当前事务交错的写入手势；API
返回明确的 Busy 结果。事务通过 Document 生成的 edit token
标识；手势内部的更新 API 必须携带该 token。事务打开期间，不带 token
的普通写入 API 返回 Busy，Undo/Redo 也返回 Busy。这样 before/after
状态与历史顺序有唯一解释。

## 记录内容与 Entity 身份

历史记录保存可逆的 Document 操作差异，包括受影响 Item 的组件数据、所属
Layer/Group、直属顺序位置，以及必要的关联引用。不要保存 Renderer
Proxy、Mesh、材质或 View 临时手柄。

删除 Item 时不能立即 despawn 后再依靠旧 Entity ID
恢复。删除对象从可见层级中分离，保留为带 `CanvasTombstone`
标记的实体；Undo 将同一 Entity 重新挂回原容器，Redo 再次分离。创建操作的
Undo 也通过分离创建的实体实现，Redo
重新挂回。历史记录被清理时，只有不再被任何 Undo/Redo 记录引用的
tombstone 才能释放。

历史栈采用有上限的记录数量，默认保留最近 100 个已提交事务；允许 Document
调整容量。新修改发生在 Undo 之后时，清空 Redo
分支，并释放仅被该分支引用的 tombstone。Document
销毁时释放其历史与所有保留实体。

## API 与输入

提供 Document 级 `undo`、`redo`、`can_undo`、`can_redo`
查询/操作，并提供显式复合编辑事务入口，使调用方可以将多个 Document API
操作合并成一个历史条目。Gallery 初始化一份演示 Document
时可以在单个事务中创建整组初始内容，再开始正常用户历史。没有可撤销或可重做记录时返回明确的未改变结果，不产生错误。

CanvasView 在自身拥有键盘焦点且没有把输入交给文字属性编辑器时处理
Ctrl+Z、Ctrl+Shift+Z（以及项目采用的 Redo 快捷键）。Undo/Redo 不回退
View 的选择和视角；若选择中的 Item 因 Undo/Redo
不再属于当前可见文档层级，按现有 Selection Repair 规则清理。

所有后续 Canvas 修改 API
必须通过同一历史提交路径。不得在后续功能中直接写入组件而绕过历史。

## 完成条件与验证

-   现有 Shape 创建、删除、Transform、Geometry、Paint、Path
    拓扑和顺序操作均可 Undo/Redo。
-   Layer 创建/删除/排序/显隐/锁定、Group 分组/取消分组/变换均可
    Undo/Redo。
-   拖动手势只形成一个历史条目；取消保留已提交的最终状态并形成一个条目。
-   Undo 删除后恢复相同 Entity；Redo 后再次分离；历史清理后 tombstone
    正确释放。
-   新编辑清空 Redo 分支；历史容量限制正确。
-   多 View 共享历史，但 View 本地状态不被回退。
-   Document 销毁后历史记录与保留实体释放。
-   更新 Integration Test Coverage Model 和 Canvas 功能/设计文档。
-   Gallery 通过 BRP 验证拖动、Undo、Redo 和多 View 同步。

## 设计边界

不实现分支历史、时间轴历史浏览、协作编辑或跨应用版本控制。历史只覆盖
Document 中已提交的绘图修改。
