//! 为桌面 UI 提供按原生窗口独立协调的系统鼠标光标。
//! 控件使用 Bevy EntityCursor 声明 hover 光标，调用方可为持续 drag 或 native 窗口操作持有 CursorLease。
//!
//! 支持从命中实体继承最近祖先的光标声明、实际 Disabled 的 NotAllowed 提示与 drag 期间的固定光标。
//! native Move/Resize 期间让出光标控制权，结束或 owner 失效后恢复当前 hover 光标。
//!
//! 仅 PointerId::Mouse 影响 OS 光标，Touch 与 Custom 输入不会取得 lease。
//! 将 lease 插入交互 owner，正常结束时移除，释放、取消、失焦、禁用或销毁会兜底清理。
//! 本 Plugin 不捕获 Pointer，也不改变 Pointer event 派发。
//! 宿主不得同时安装 Bevy CursorIconPlugin。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod coordination;
mod lease;

pub use coordination::WidgetryCursorPlugin;
pub use lease::CursorLease;
