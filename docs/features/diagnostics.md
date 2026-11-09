# 诊断与错误传播

Diagnostics 保存领域错误、来源身份、原始原因链和传播上下文。通过 Facade 使用 `bevy_widgetry::diagnostics`；直接依赖底层 crate 时，默认不包含 Bevy 适配，ECS 边界能力由 `bevy` Feature 开启。

## 领域失败

### 包装一次真实失败

下面的读取函数保留原始 `io::Error`，同时记录路径和操作。

```rust
use bevy_widgetry::diagnostics::{ErrorFrame, WidgetryFailure};
use std::{io, path::Path};

type ReadFailure = WidgetryFailure<io::Error>;

fn read_text(path: &Path) -> Result<String, ReadFailure> {
    std::fs::read_to_string(path).map_err(|source| {
        WidgetryFailure::new(
            source,
            ErrorFrame::new("read configuration").field("path", path.display()),
        )
    })
}
```

创建失败时会建立唯一身份并记录源头。不要把 `Ok(None)`、等待、取消等正常结果包装成错误。

### 增加调用上下文

```rust
let text = read_text(path).map_err(|failure| {
    failure.with_context(ErrorFrame::new("open project").field("project", project_name))
})?;
```

`with_context` 不创建新错误身份、不重新捕获栈。需要把下层错误转成自己的领域枚举时，使用 `map_kind`，并在枚举变体中保留原始错误类型。

```rust
#[derive(Debug)]
enum ProjectError {
    Configuration(std::io::Error),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(source) => write!(f, "读取项目配置失败：{source}"),
        }
    }
}
impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Self::Configuration(source) => Some(source) }
    }
}

let text = read_text(path)
    .map_err(|failure| failure.map_kind(ProjectError::Configuration))?;
```

### 查询来源

通过 `kind()` 匹配领域类别，通过标准 `Error::source()` 追溯原因。`origin()` 提供共享身份、首次纳入位置与原始线程回溯。普通 Display/Debug 不自动展开完整回溯；最终报告方按需要读取。

## Bevy 执行边界

### 将领域错误返回给 System

```rust
use bevy::prelude::*;

fn load_configuration() -> Result {
    let _text = read_text(std::path::Path::new("settings.ron"))
        .map_err(|failure| failure.into_bevy_error())?;
    Ok(())
}
```

转换使用 Error 严重级别，并保留原诊断。库不自动安装应用的 Error Handler。

### 一次返回多个失败

```rust
use bevy_widgetry::diagnostics::bevy::DiagnosticBatch;

fn load_all(paths: &[std::path::PathBuf]) -> bevy::prelude::Result {
    let mut failures = DiagnosticBatch::default();
    for path in paths {
        if let Err(failure) = read_text(path) {
            failures.push(failure);
        }
    }
    match failures.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
```

批处理会保留每个独立失败，而不是只留下第一项或把所有错误拼成字符串。每个循环成员的成功工作仍可完成。

## 已处理错误

### 记录实际采取的回退

```rust
let text = match read_text(path) {
    Ok(text) => text,
    Err(failure) => {
        let fallback = String::from("theme = \"dark\"");
        failure.record_handled("采用内置配置", "本次读取失败，使用已准备好的默认配置");
        fallback
    }
};
```

处理记录说明应用已经采取了措施，不会替应用执行恢复。仅查看、转换、打印或释放错误对象，不等于业务已处理。

## 持续故障与恢复

`bevy::FailureState<K>` 用于一个拥有者反复观察到的同一故障。K 是业务身份键，决定哪些观察属于同一次异常；不能只以错误文案相同判断。

同一键复用原诊断，实际重新验证成功后调用 `recover`。正常等待或对象删除不是恢复。独立操作请求仍各有自己的失败身份，不为了减少日志而混在一起。

## 宿主终结报告

`dispatch_error` 将受管诊断与原生 Bevy 错误分流。宿主可以逐项显示受管原因和回溯；同一 origin 的最终未处理报告只执行一次。原生错误保持自身严重级别和原始对象。

日志输出位置、过滤、错误提示及进程退出策略属于应用；Diagnostics 提供的是诊断数据和边界转换。
