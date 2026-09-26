# bevy_brp_extras 本地兼容说明

## 上游来源

- package：`bevy_brp_extras 0.22.7`
- 来源：crates.io 发布包
- repository：<https://github.com/natepiano/bevy_brp>
- release commit：`06f855e163e362875be4063c49a9515e2b2e86a4`
- 原始 `.crate` SHA-256：`45fe67740b61d7dac58c4d1506981595c4cce2922fd37cf6dbcc8582a9a034b1`
- license：MIT OR Apache-2.0；原始 license 文件保留在本目录

本目录保留 crates.io 发布包中规范化后的 `Cargo.toml`、运行所需源码、README、CHANGELOG
与 license；manifest 仅移除未随本兼容包保存的 example 和 integration test target 声明，
不包含上游 MCP server、monorepo 其他 package、构建产物和无关 example。

## 本地语义差异

### methods-only 装配

新增 `BrpExtrasPlugin::without_http_transport()` 与独立的 `ExternalTransport` 配置状态。
该入口安装或复用 `RemotePlugin`，注册上游完整 Extras methods，并保留 keyboard、mouse、
screenshot、diagnostics 与 deferred shutdown 的工作 system；它不安装 `RemoteHttpPlugin`、
不读取或声明 HTTP 端口，也不设置 Winit update mode。

上游默认入口、`with_port()` 和 `with_http_plugin()` 的行为保持不变。

### 跨帧 activity 状态

新增 `BrpExtrasActivity` Resource 与只读 `BrpExtrasActivityState` 快照，允许 external transport
安装线程安全的状态通知 callback。crate 内部使用可转移、可幂等取消的 RAII guard 跟踪 keyboard / mouse
release、逐字输入、drag、double-click、screenshot capture / worker / publication 与 deferred shutdown；
最后一项真实工作结束时恢复 idle。screenshot worker 完成 I/O 后会额外通知宿主消费结果，timeout 或
watcher cleanup 能取消同一 lease，避免 worker 后续 drop 重复释放。

未安装 callback 时 activity tracking 不改变上游默认 transport 的装配方式或公开 BRP method 行为。

## 移除条件

当采用的上游正式 release 同时提供等价的 external-transport / methods-only 装配入口与真实跨帧
activity lifecycle，并且 Gallery 外置 HTTP transport 能直接迁移到这些入口时，可以移除本地语义
补丁与 `[patch.crates-io]` 覆盖。
