# Gallery integration coverage

`build_icon.rs` 通过临时 Cargo 应用直接复用正式 Gallery build.rs，覆盖缺失/无效 ICO、资源编译器不可用、有效图标构建。失败在无 Backtrace 环境变量子进程中产生一次源头事件及一次含原始栈的终结报告，正常构建通过 Win32 FindResource/SizeofResource 确认 EXE 中存在真实 GROUP_ICON 资源；运行期 UI 行为另按 BRP 验收。


`src/assets/cursors/tests.rs` 保护全部45份角色动画的像素、布局、固定热点、独立身份、静态首帧后备和公开帧选择。`src/cursor_theme/tests.rs` 保护完整34系统/11Hint映射、真实Time相位、Native/Lease/Capture/Blur/Disabled交错、默认System、惰性冷加载、共享失败诊断及恢复。库的通用测试使用合成RGBA图集，不反向依赖Gallery。
