# Bevy Widgetry Architecture

本文件描述 `bevy_widgetry` 当前的整体架构、Workspace 组成以及内部依赖关系。

## Architecture Overview

`bevy_widgetry` 使用 Cargo Workspace 组织。

整体上由应用、顶层 facade、各功能 crate、共享基础设施和测试基础设施组成。

当前 Workspace 的主要结构为：

```text
gallery/
├── src/logging.rs
├── src/assets.rs
├── src/assets/
├── src/gallery.rs
├── src/renderer.rs
├── src/pages.rs
└── src/pages/

crates/
├── log/
├── asset/
├── bevy_widgetry/
├── core/
├── button/
├── combo_box/
├── text_field/
├── window/
└── test_utils/
```

## Workspace Roles

本节仅简要介绍各成员的职责和用途，不展开 API、文件分工或具体实现细节。

### `crates/bevy_widgetry`

顶层 facade crate。

负责聚合并统一暴露 Widgetry 各功能 crate，本身不承载具体控件实现。

### `crates/core`

共享基础设施 crate。

承载跨控件共享能力和整个 Widgetry 使用的基础设施，包括 App 级默认字体 fallback；通过 `asset` 加载内建得意黑。

### `crates/asset`

Widgetry 内建资源基础设施，集中存储静态文件、嵌入注册并提供语义资源标识。

依赖外部 `bevy` 和底层 `log`，不依赖 `core`；不解析 SVG，也不由顶层 facade 直接依赖或导出。

### `crates/log`

底层内部日志基础设施，提供固定 `bevy_widgetry` target 的 info/warn/error 宏。
只依赖外部 `bevy`，不配置 subscriber 或输出，不保存状态，不由 facade 导出。

### `crates/test_utils`

共享测试基础设施。

供各 crate 的测试复用，不属于正常生产依赖路径。

通过内部依赖 `core` 复用主题类型，提供统一的测试主题切换辅助函数，并提供线程局部日志捕获以复用诊断行为验证。

### `gallery`

Widgetry 的实际消费者和集成展示应用，用于人工体验、集成验证和展示当前控件能力。

应用日志由 `logging` module 配置 Bevy LogPlugin，使用固定启动本机时区，同时输出终端和 `gallery/logs/` 下每次启动新建的文件；WorkerGuard 由 main 持有到运行结束。

应用自有资源由内部 `assets` module 的 `GalleryAssetPlugin` 管理，与库内资源保持独立。

### `crates/window`

自定义窗口控件 crate，提供窗口界面、主题、原生窗口交互与 UI 生命周期管理。

## Dependency Graph

图中的箭头表示：

```text
A --> B
= A depends on B
```

实线表示正常生产依赖，虚线表示测试依赖。

```mermaid
flowchart TD
    subgraph Application
        gallery["gallery"]
    end

    subgraph Facade
        widgetry["crates/bevy_widgetry"]
    end

    subgraph Widgets
        button["crates/button"]
        combo_box["crates/combo_box"]
        text_field["crates/text_field"]
        window["crates/window"]
    end

    subgraph Infrastructure
        log["crates/log"]
        asset["crates/asset"]
        core["crates/core"]
        test_utils["crates/test_utils"]
    end

    gallery --> widgetry

    widgetry --> core
    widgetry --> button
    widgetry --> combo_box
    widgetry --> text_field
    widgetry --> window

    button --> core
    combo_box --> core
    text_field --> core
    window --> core
    window --> asset
    core --> asset
    asset --> log
    core --> log
    button --> log
    combo_box --> log
    text_field --> log
    window --> log

    test_utils --> core

    log -. dev .-> test_utils
    core -. dev .-> test_utils
    button -. dev .-> test_utils
    combo_box -. dev .-> test_utils
    text_field -. dev .-> test_utils
    window -. dev .-> test_utils
```
