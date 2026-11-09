# file_dialog 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Session／Generation、目录历史、Selection／Active、Search／Filter／Sort 和文件确认／覆盖状态。
- 异步目录 Snapshot 的 Loading／Partial／Ready／Failed，Storage Scope／Prefs，Embedded／Owned Window 和 IME／Focus。

## Stimuli
- BSN 构造、公开 World／Commands API、目录导航／批量选择、Keyboard／Pointer／IME、后台读取与校验回复。
- Streaming I/O、偏好读写、窗口关闭、Theme／Override 与 Session 重开。

## Guards
- Root、Session、Token、Revision、Mode 和 Entry Kind 必须匹配当前请求；旧回复及旧 Row 输入不得提交。
- Pending Selection／Projection／Validation 时不能越过确认守卫；Disabled、Focus 和活动 IME 组合态不得产生错误提交。
- 异步队列、Job 和 Snapshot 维持容量及取消边界；Owned Window 保持禁止用户 Resize 的策略。

## Transitions
- 启动 → 目录 Partial／Ready／Failed，导航／搜索／选择 → 保存／覆盖确认／取消／重开。
- Storage Scope 偏好合并与迟到回复丢弃；Owned Window 在结果消费后完成自有资源清理。

## Invariants
- 每 Session 最多提交一次 Result，通知时业务 Authority 已提交；新 Session 不接受旧 Generation 的结果。
- `EntryId` 与原始 `PathBuf` 是业务身份，不从 Lossy UI 文本重建；PickFile 草稿与 Selection 分离。
- 后台 I/O 不占用 App Update 或耗尽共享 I/O 池；结果不因偏好存储失败而回滚，取消／关闭安全回收 Job 与缓冲。
- 动态 Row、Editor、筛选／搜索和确认面板的显示状态与当前 Session、Disabled、Theme／Override 一致；颜色归属由 FileDialog Owner 管理。
- 非法请求及底层存储／布局／颜色错误保持原始原因链、请求现场和宿主传播语义；失败不破坏其他 Session。

## Couplings
- Session／Generation × 异步回复；Selection／Search × Validation／Confirm；Owned Window × IME／Modal；Theme × 动态 Row／MessageBox。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `contract.rs` | 公开 Session、目录、选择、确认／取消及错误契约。 |
| `diagnostics.rs` | 请求／后台／颜色／存储失败的领域诊断、归属与宿主传播。 |
| `folder_shutdown.rs` | 目录读取与关闭、取消、后台资源回收协作。 |
| `runtime.rs` | 受管异步快照、Job、存储、容量、Session 隔离与迟到回复。 |
| `style.rs` | 真实 UI／IME／Layout、动态内容和 Theme／Owner 投影。 |
| `window.rs` | Owned Window、Modal、Focus、Overwrite、标题与生命周期。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/style.rs` | 样式刷新、字体与 Shell 的局部更新及结构恢复。 |
| `src/runtime.rs` | 目录流容量、CPU 消费预算和异步退出的局部时序。 |
