# File Dialog

## 功能概述

提供可独立使用的文件与目录选择组件，支持嵌入式内容以及自有 Native Window。它把目录浏览、选择、确认、后台文件系统工作和可视 UI 组合在同一套业务契约下。

## 当前设计思路

业务 Authority、文件系统请求和 UI 投影分离。每次构造产生独立 Session，后台 Worker 提交准备好的 Snapshot / Validation Reply；UI 根据当前业务状态展示可见行和编辑器，不把物理 Row 当作文件身份。可选 Storage Scope 在 Session 间共享偏好。

## 主要功能

- 模式：PickFile、PickFiles、PickDirectory、PickDirectories、SaveFile。
- 导航与目录历史、搜索、Filter、Sort、隐藏项控制、常用位置与 Pin、新建文件夹。
- 按可见范围生成文件列表，可显示名称、类型、修改时间和可读大小；Selection 与 Active 分离。
- 单选、多选、Ctrl / Shift Range、Keyboard Navigation、Filename / Path Editor 和 IME 守卫。
- 可作为 Embedded 内容或独立 Owned Window，并支持 Modal / NonModal。
- SaveFile 的 Overwrite 确认复用 MessageBox。
- 后台目录流式读取、路径验证、创建目录、位置查询，及可选的偏好持久化和手工 Backend Reply 接入。
- 提供主题、颜色覆盖和 Style 配置，包括行高、Overscan、Sidebar 宽度与字体间距。

## 行为与状态契约

每个 Session 最多产生一次 `WidgetryFileDialogResultEvent`，通知发生在对应结果 Authority 提交后。`World` API 即时提交；`Commands` 在实际执行时提交；后台回复只有被当前 Session / Token 接受后才生效。程序更新和交互对应的 ChangeEvent 报告真实已提交业务变化，同值不通知。Selection / Active 是业务状态而非复用 Row State；确认操作必须等待必要的 Pending Selection / Projection / Validation。取消不等待后台 I/O。SaveFile 仅返回目标 PathBuf，不替应用写文件。

## 边界情况与错误处理

- 过期 Session、EntryId、Generation、Validation Candidate 或失效 Root 不得修改当前 Session。
- 文件系统的 Partial / Failed 状态、超时及单项错误通过状态与诊断呈现；已到达条目可先显示，完成后统一排序。
- 扩展长度 Windows Path 在实际操作中保留原始 `PathBuf`；地址显示可以隐藏前缀，显示文本不能反向构造文件选择路径。
- Modal、IME、隐藏/禁用控件的 Tab 与 Escape 处理按当前输入归属执行；旧 Press 不能在滚动复用的新 Row 上提交选择。
- Owned Window 结束后清理其窗口和 Camera；外部销毁不冒充用户 Cancelled。Storage 写入失败不回滚已成立的结果。

## 已知限制

- 一个 Snapshot 最多 100,000 个 Entry；单次 Selection 最多排队 128 个 Intent；单个 Viewport 最多构造 256 行，Overscan 上限 8。
- 默认偏好只驻留 App 内存，除非宿主明确配置 Persistence Path。
- 后台任务取消不保证撤销已发生的 OS 文件系统副作用；退出 App 不无限等待阻塞 I/O。
- 同步读取依赖后台服务；Headless Contract 本身不直接创建 Native Window 或执行同步文件系统访问。

