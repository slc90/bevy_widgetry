# Message Box

## 功能概述

提供非阻塞的 Modal MessageBox，可在独立对话框中承载自定义正文，并通过结果事件向应用报告用户选择。

## 当前设计思路

MessageBox 复用 Widgetry Window、Button 和 ScrollArea：对话框的结果与关闭生命周期由 MessageBox 负责，正文由调用方的 `SceneList` 组合并置于可滚动区域，结果按钮位于滚动区域外，窄空间下可换行。应用创建对话框后继续运行，不等待同步返回值。

## 主要功能

- `widgetry_message_box` 接收 Native Parent、标题、按钮组合与正文 SceneList。
- 按钮组合支持 Ok、YesNo、YesNoCancel。
- 用户选择通过 `WidgetryMessageBoxResultEvent` 报告 Ok、Yes、No 或 Cancel。
- 以 Modal Child 方式阻挡 Parent 的 Pointer 操作；窗口、正文与按钮按 Theme 显示。

## 行为与状态契约

Parent 必须是有 Widgetry Window 绑定的有效 Native Window。一个 MessageBox 最多决议一次，重复请求保留首先接受的结果。结果 Observer 在清理前能读取对话框 State，并可排队业务操作；这些 Consumer 完成后再回收 Owned Window / Camera。多个 Dialog 的资源和 Parent Blocker 归属隔离。

默认标题使用 Window 的 `WidgetryWindowTitle` 绑定。可使用该类型的 `set_in_world` / `set` 对 Dialog Root 更新原生标题和显示文字，不影响正文、结果或窗口身份。

Window 和结果按钮的样式归属在构造及真实关系变化时维护，稳定对话框不再为建立已有关系逐帧递归正文。延后挂接或容器重挂会补偿绑定，Owner 移除后恢复、部件或 Root 标记移除后释放原归属；调用方正文中的普通 Button 不被接管。真正缺失受管 WindowRoot / 结果 Button 时传播 Error 并保留重试，同类持续故障的诊断去重，恢复后记录一次信息。

组合配色在首次构造、Theme 或 Override 变化时解析并缓存；Body 和结果按钮按有效 Disabled、Hover / Pressed 的添加与移除以及动态部件变化局部更新。稳定帧不重复解析配色或递归业务正文。Window 使用同一组合配色并独立处理其内部输入。由 FileDialog 接管的覆盖确认继续使用 FileDialog 提供的配色处理内部变化，不以 MessageBox 自身 Theme 或 Override 覆盖最终 Owner。

## 边界情况与错误处理

结果按钮或明确的 Cancel 输入属于业务决议路径；外部销毁与 Parent 生命周期结束是资源清理路径，不能无条件伪造 Cancel。无效 Parent、构造失败或失效资源需要按 Error 契约报告。Disabled Action 不应提交结果。

Cancel 按钮、Escape 和 Native Close Request 都提交 Cancel 结果。结果 Event 的 Entity 指向 Dialog Root，Observer 可在关闭前读取 State 并排队操作；重复决议不覆盖首次结果。

`WidgetryMessageBox::resolve` 返回 `MessageBoxFailure`。`MessageBoxError::Resolve` 保存 Root、请求结果和缺失的 MessageBox / 决议状态；已确认归属的结果输入缺少状态时同样传给宿主。正常 Cancel、未属于 MessageBox 的输入过滤和活跃 Root 上的重复决议不创建错误。结果先提交，再通知并 flush Observer 命令，之后才标记 Closing 和回收资源。

颜色查询 / 设置 / 清空也返回领域失败，受管配色拒绝与缺失部件可匹配；非法配色保留 Core 的 cause、origin / ID 和嵌套部件 / 状态 / 字段，失败不提交。Scene 与 Window / Button 的实际失败保留下层诊断，仅补充 MessageBox、部件和操作现场；ECS 边界显式转换，传播不重复记录源头或声称已处理。

归属、派生状态、调色板和部件结构的持续故障按 MessageBox / 部件 / 阶段保存诊断。多个实例和多个部件的实际失败全部聚合传播，健康部件继续更新；重新验证成功才记录恢复，Root / 部件结束归属时直接释放诊断。派生样式补建会重新索引既有受管部件，不依赖它们再次改变 Owner；静止帧继续复用配色与部件索引。

## 已知限制

- 只提供当前三种结果按钮组合。
- 不阻塞调用线程等待用户决议。
- 依赖桌面 Window / Modal 组合能力，Headless 测试不能证明所有 OS Window 行为。


结果按钮继承基础 Button 的 TabIndex(-1) Pointer 焦点能力；点击或获得焦点后的 Enter/Space 激活仍产生原有结果，不额外声明 Tab 顺序。
