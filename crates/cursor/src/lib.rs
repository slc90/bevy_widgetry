//! 为桌面 UI 提供按原生窗口独立协调的鼠标光标与可选 Kamuri 主题。
//! 控件使用 Bevy EntityCursor 声明 hover 光标，调用方可为持续 drag 或 native 窗口操作持有 CursorLease。
//!
//! 支持从命中实体继承最近祖先的光标声明、实际 Disabled 的 NotAllowed 提示与 drag 期间的固定光标。
//! native Move/Resize 期间让出光标控制权，结束或 owner 失效后恢复当前 hover 光标。
//! WidgetryCursorHint 声明十种业务语义，Kamuri 下显示独立图像，System 下使用各自的 fallback，最近的声明优先，同一 entity 的 EntityCursor 优先。
//!
//! 仅 PointerId::Mouse 影响 OS 光标，Touch 与 Custom 输入不会取得 lease。
//! 将 lease 插入交互 owner，正常结束时移除，释放、取消、失焦、禁用或销毁会兜底清理。
//! 本 Plugin 不捕获 Pointer，也不改变 Pointer event 派发。
//! WidgetryCursorTheme 是展示配置，默认 System，宿主可插入或替换为 Kamuri，与颜色主题独立。
//! Kamuri 提供34种系统语义与10种扩展语义；43款静态图为48px，hotspot对应各自的功能锚点。
//! Pointer 使用64px、12帧、8 FPS动画，hotspot [22,54] 对应按钮中心。
//! 进入 Pointer 从 frame0 开始，同一语义不重启，Native、失焦、退出语义或窗口关闭时停播。
//! 未就绪的 atlas 使用静态 Pointer，静态图也不可用时保留对应的 System 图标。
//! 调用方的 EntityCursor::Custom 保持原图像。
//! 有焦点的 Pointer 动画按需请求 redraw，其余情况不主动唤醒，宿主可保持 desktop_app。
//! GUI 宿主须先装配 Bevy asset/image 能力，Plugin 自动注册内建 Cursor 素材，纯 headless 环境维持 System fallback。
//! 宿主不得同时安装 Bevy CursorIconPlugin。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod coordination;
mod lease;
mod presentation;
mod theme;

pub use coordination::WidgetryCursorPlugin;
pub use lease::CursorLease;
pub use theme::{WidgetryCursorHint, WidgetryCursorTheme};
