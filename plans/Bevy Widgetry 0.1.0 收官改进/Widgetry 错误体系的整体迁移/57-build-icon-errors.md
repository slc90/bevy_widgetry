# 明确构建期图标失败的诊断与终结边界

构建脚本不在ECS中运行，因此需要保留构建期领域错误，并由构建入口显式终结报告，不能等待Bevy全局Handler。

## 修改落点与前置能力

涉及：`crates/app_icon_build/src/lib.rs`、`crates/app_icon_build/Cargo.toml`、`gallery/build.rs`、`相关构建期诊断适配`。

实际前置为[01 建立领域错误的统一诊断载体](01-diagnostic-payload.md)、[03 让测试能观察完整错误生命周期](03-diagnostic-test-support.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 构建期错误的内容

set_exe_icon当前返回io::Result，缺CARGO_MANIFEST_DIR、路径非UTF8、读取文件及winresource编译错误会丢少量操作语境。建立构建领域分类，保留原io/编译source、解析后path和步骤，使用共享轻量诊断机制捕获源头现场。

共享载体的依赖落点按公共诊断方案确定；不得仅为构建日志拉入完整Bevy renderer、UI或宿主App。这是依赖边界，不是为了统一强造一个ECS。

## 构建入口负责报告

gallery/build.rs现在直接返回set_exe_icon结果。改为由构建入口配置适用于Cargo输出的诊断接收，并显式处理失败：源头一次ERROR、最终未处理才输出原栈与ID、返回构建失败状态。库不安装全局Subscriber，不用panic/unwrap作为正常失败路径。

保留cargo:rerun-if-changed、路径相对调用包解析和winresource编译行为。运行时Gallery的日志文件/Handler不参与构建过程。

## 完成条件

- 缺环境、非UTF8路径、丢失/无效ICO和资源编译失败保存原source/path/阶段。
- 以子进程触发build失败，在Cargo可见输出中观察源头事件和最终报告，不依赖RUST_BACKTRACE。
- 有效资源仍嵌入EXE，rerun-if-changed行为与正常build保持。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_app_icon_build
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步app_icon_build说明；若增加集成测试则建立相应覆盖文档。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

不改变现有图标素材和生成工具，不引入运行期窗口或渲染依赖。

## 顺序衔接

前一项：[56 让 Canvas PNG 导出在 Render 与 Job 间保留诊断](56-canvas-png-errors.md)；后一项：[58 让宿主日志保存必要诊断而不默认丢弃](58-app-logging-preservation.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
