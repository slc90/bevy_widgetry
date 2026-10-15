# 通过可选 app_logging Feature 公开宿主日志能力

## 目标与范围

在 Facade 中增加独立 `app_logging` Feature 和对应模块，导出已有 `AppLogging`、`file_layer`、`terminal_layer`、`WorkerGuard`。迁移 Gallery 的应用级日志导入，保留日志实现仍在 `crates/app_logging/`。

## 修改设计

该 Feature 显式按需启用，不捆绑到所有控件，也不因为启用某个控件自动安装 Subscriber。Gallery 启用此 Feature 后通过 Facade 使用日志能力，移除不再需要的直接 `bevy_widgetry_app_logging` 依赖；内建 Asset 的直接依赖和独立 `app_icon_build` 的 Build Dependency 不因此删除。

保留现有使用次序和所有权：`AppLogging::new` 准备目录、Writer 和本地时间；`install` 返回必须持有到退出的 Guard；宿主通过官方 LogPlugin 装配输出层。未 install 时 Layer Factory 仍返回 None，准备失败仍以 Error 严重级别报告。Facade 只重导出，不保存 Guard、不替宿主挑日志目录或过滤策略。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

单独启用 `app_logging` 可使用公开入口；未启用时不引入它的可选依赖。Gallery 迁移后仍只有原有日志装配路径，退出前保持 Guard 有效，文件和 Terminal 输出格式不变。覆盖未安装、准备失败和正常退出等现有契约，不把这一宿主能力误当作必须发注册完成日志的 Widget Plugin。

同步 Facade、app_logging 和 Gallery 使用文档，不增加 EXE ICO 的运行时公开入口。

## 预期产出

可通过统一入口选择的宿主日志能力，以及不再绕过该入口的 Gallery 日志使用方式。

## 与前后方案的关系

顺序上接 [方案 04：为 Facade 控件建立真正的可选编译边界](04-为-Facade-控件建立真正的可选编译边界.md)。

完成后进入 [方案 06：移除三个 Plugin 内重复的 UI 基础注册](06-移除三个-Plugin-内重复的-UI-基础注册.md)。

依赖已经完成的 Facade Feature 结构；宿主日志能力作为独立 Feature，不混入内建 Asset 或构建期 ICO 能力。
