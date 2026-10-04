//! 提供可独立使用的 FileDialog 业务 state，适用于文件、目录和保存目标选择。
//! 调用方通过 Widget API 提交操作，通过只读 state 和 result event 获取结果。
//!
//! 支持单文件、多文件、单目录、多目录和 SaveFile mode。
//! 导航、过滤、selection 与保存候选分别维护独立 state。
//! storage 默认保留在 App 内存，通过显式 scope 共享偏好。
//!
//! 每次构造生成独立 session，一个 session 最多产生一次 result。
//! resolved root 可通过 Reopen 创建新 session，使用 scope 或实例的最近访问目录，旧 session reply 不再适用。
//! Props 只初始化一次，运行时查询以 State 为准。
//! headless contract 不创建 native Window，不同步访问 filesystem。
//! SaveFile 只返回目标路径，实际文件写入由调用方负责。
//! HeadlessPlugin 自动执行后台目录读取、路径校验、目录创建和常用位置查询。
//! 自动服务可通过 RuntimeOptions 关闭，供自定义 reply adapter 直接提交已准备的数据。
//! 目录读取期间先显示已到达条目，完成后应用全局排序，单项失败以 Partial 和有界摘要提供。
//! 默认不写盘，只有宿主显式提供 Persistence.path 才读取和保存偏好。
//! RuntimeStatus 区分后台错误、memory committed 与 disk committed，flush_preferences 请求异步保存。
//! 非本平台的持久化路径标记为 unavailable_paths，普通显示偏好仍可恢复。
//! 晚到的偏好只影响后续 session，不改变当前目录或正在编辑的内容。
//! Cancel 不等待 OS I/O，服务保持固定容量，App 退出不等待尚未返回的阻塞调用。
//! backend 通过 Started 与准备好的 snapshot 提交目录事实，通过 validation candidate 提交确认结果。
//! Validated 表示 backend 已校验路径、parent 与 kind，exists 只决定 SaveFile 是否等待 overwrite。
//! filter/search/sort 可用缓存 entries 重新计算 projection，原始路径和 stable EntryId 保持不变。
//! selection job 保留 Ctrl+A、Ctrl 点击与 Shift 区间的输入顺序，Ctrl+A 只包含当时已经到达的条目。
//! direction、Home/End 与 Page navigation 更新 active，selection 与 active 可以分别存在。
//! pending selection/projection 时不能 Confirm，Cancel 不等待 backend。
//! overwrite decision 必须携带固定候选 token，No 返回当前 session，修改文件名使旧候选失效。
//! New Folder 只提交明确请求，关闭 session 只拒绝应用旧 reply，不保证撤销已经执行的 OS 副作用。
//! World apply 立即提交操作或 pending state，Commands queue 在实际执行时提交，后台 state 在 deliver 接受 reply 时生效。
//! ChangeEvent 通知已提交的业务 state，包括 pending transition，同值 setter 不通知。
//! result event 的 observer 可读取已提交结果，不承诺多个 observer 的固定顺序。
//! 同 scope 的 preferences 按 field delta 合并，Cancel 不修改 last_picked_dir，storage 失败不回滚 result。
//! memory snapshot 可以 import/export，saved revision acknowledgement 不把新 revision 误标为已保存。
//! 路径使用 PathBuf/OsString，lossy name 只用于搜索和显示，不能反向生成选择路径。
//! snapshot 最多 100k entries，每次 selection 最多排队 128 intents，超限产生明确错误。

mod api;
mod behavior;
mod confirmation;
mod filesystem;
mod filter;
mod model;
mod persistence;
mod runtime;
mod selection;
mod snapshot;
mod storage;
mod worker;

pub use api::{
    WidgetryFileDialog, WidgetryFileDialogChangeEvent, WidgetryFileDialogHeadlessPlugin,
    WidgetryFileDialogPlugin, WidgetryFileDialogResultEvent,
};
pub use confirmation::{WidgetryFileDialogCandidate, WidgetryFileDialogValidationJob};
pub use filesystem::{
    WidgetryFileDialogBackend, WidgetryFileDialogFileSystem, WidgetryFileDialogLocation,
    WidgetryFileDialogNativeFileSystem,
};
pub use filter::{WidgetryFileDialogFilter, WidgetryFileDialogFilterId};
pub use model::{
    WidgetryFileDialogAction, WidgetryFileDialogConfirmation, WidgetryFileDialogDirectoryState,
    WidgetryFileDialogEntry, WidgetryFileDialogEntryId, WidgetryFileDialogEntryKind,
    WidgetryFileDialogFolderRequest, WidgetryFileDialogMode, WidgetryFileDialogMove,
    WidgetryFileDialogNavigation, WidgetryFileDialogProps, WidgetryFileDialogReply,
    WidgetryFileDialogResult, WidgetryFileDialogSelection, WidgetryFileDialogSessionId,
    WidgetryFileDialogSessionState, WidgetryFileDialogSort, WidgetryFileDialogState,
    WidgetryFileDialogToken,
};
pub use runtime::{
    WidgetryFileDialogPersistence, WidgetryFileDialogRuntimeOptions,
    WidgetryFileDialogRuntimeStatus,
};
pub use selection::{WidgetryFileDialogPreparedSelection, WidgetryFileDialogSelectionJob};
pub use snapshot::{
    WidgetryFileDialogEntryData, WidgetryFileDialogQuery, WidgetryFileDialogSnapshot,
};
pub use storage::{
    WidgetryFileDialogStorage, WidgetryFileDialogStorageSnapshot, WidgetryFileDialogStorageState,
};
