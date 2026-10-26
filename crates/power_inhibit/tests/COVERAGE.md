# PowerInhibit 覆盖模型

- `resource.rs` 从外部通过真实 Bevy `ResMut` 调用公开 API：无窗口插件、无更新时钟，重复开关、重新启用、移除后重建 Resource、App 释放。
- 私有调用边界的 Unit Test 覆盖创建/两类设置失败、回滚失败、单类/双类清除失败、重试、部分清除后的开启、销毁失败日志，以及每类型计数与单次句柄释放。
- 私有 FFI Unit Test 使用真实 Win32 API，检查关闭、移除 Resource、释放 App 后句柄无效，并覆盖跨线程开关和原始 Win32 错误传播。
- API 成功与句柄释放不代表已观测 DISPLAY/SYSTEM 系统列表、实际空闲超时策略或窗口最小化/失焦/托盘隐藏期间的系统请求。该边界需独立 Windows 实机验收。
