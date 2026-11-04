# Gallery integration coverage

`build_icon.rs` 通过临时 Cargo 应用直接复用正式 Gallery build.rs，覆盖缺失/无效 ICO、资源编译器不可用、有效图标构建。失败在无 Backtrace 环境变量子进程中产生一次源头事件及一次含原始栈的终结报告，正常构建通过 Win32 FindResource/SizeofResource 确认 EXE 中存在真实 GROUP_ICON 资源；运行期 UI 行为另按 BRP 验收。
