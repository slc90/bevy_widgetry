# Widgetry 公开控件命名统一重构方案

## 目标

统一 Widgetry 对外公开的控件及其配套 API 命名，使 Widgetry 自有类型明确使用 `WidgetryXXX` 前缀。

本次只做命名重构，不改变控件行为、场景结构、生命周期、所有权模型或公开能力。

## Window

Window 不改造成普通 BSN `SceneComponent`。继续保留当前“原生 Window + Widgetry UI root + Camera”的模型。

公开 API 调整如下：

| 当前名称                                 | 新名称                         |
| ---------------------------------------- | ------------------------------ |
| `WindowPlugin`                           | `WidgetryWindowPlugin`         |
| `WindowControlsConfig`                   | `WidgetryWindowControlsConfig` |
| `ModalWindow`                            | `WidgetryModalWindow`          |
| `window(...)`                            | `widgetry_window(...)`         |
| `owned_window(...)`                      | `owned_widgetry_window(...)`   |
| 当前 `widgetry_window(Window) -> Window` | `prepare_native_window(...)`   |

其中：

- `widgetry_window(...)`：绑定调用方已有的原生 Window 和 Camera，构造 Widgetry 窗口 UI。
- `owned_widgetry_window(...)`：由 Widgetry 创建并管理原生 Window 和 Camera。
- `prepare_native_window(...)`：继续保持 `pub`，负责在原生 Window 创建前设置透明、无系统装饰和 alpha 合成等 Widgetry 所需属性。

当前 Window 的两种构造模式和原生资源所有权语义全部保持不变。现有实现确实分别承担外部绑定与 owned 两条路径。

## MessageBox

整套公开 MessageBox API 统一增加 `Widgetry` 前缀：

| 当前名称                | 新名称                          |
| ----------------------- | ------------------------------- |
| `MessageBox`            | `WidgetryMessageBox`            |
| `MessageBoxPlugin`      | `WidgetryMessageBoxPlugin`      |
| `MessageBoxButtons`     | `WidgetryMessageBoxButtons`     |
| `MessageBoxResult`      | `WidgetryMessageBoxResult`      |
| `MessageBoxResultEvent` | `WidgetryMessageBoxResultEvent` |
| `message_box(...)`      | `widgetry_message_box(...)`     |

MessageBox 的构造方式、结果事件和按钮组合语义不变。当前公开 API 正是由这些名称组成。

## Icon

整套公开 Icon API 统一增加 `Widgetry` 前缀：

| 当前名称     | 新名称               |
| ------------ | -------------------- |
| `Icon`       | `WidgetryIcon`       |
| `IconProps`  | `WidgetryIconProps`  |
| `IconPlugin` | `WidgetryIconPlugin` |

BSN 使用同步变为：

```rust
@WidgetryIcon {
    @path: ...,
    @max_size: ...,
    @color: ...,
}
```

`WidgetryIcon` 的 SceneComponent、运行期状态和现有方法行为保持不变。当前 `Icon` 同时作为 BSN 入口和运行期组件使用。

## 同步修改范围

重命名必须贯穿所有直接消费者，而不是只修改定义处。

重点包括：

- `crates/core`

  - Icon 的公开类型、插件、Props 和内部引用。

- `crates/combo_box`

  - 当前 Field 直接导入 `Icon`、使用 `@Icon` 并查询 `Icon`，全部同步为 `WidgetryIcon`。
  - `WidgetryComboBoxPlugin` 对 Icon 插件的依赖同步改名。

- `crates/window`

  - Window 自身公开 API。
  - 标题栏等内部对 Icon API 的引用。
  - Window 插件内部安装 Icon 插件的引用。
  - Window crate 现有单元测试和集成测试中的旧名称。

- `crates/message_box`

  - 公开 MessageBox API。
  - 对 Window API 的依赖同步使用 `WidgetryWindowPlugin`、`WidgetryWindowControlsConfig`、`WidgetryModalWindow` 和 `owned_widgetry_window(...)`。
  - MessageBox 自身测试同步使用新名称。

- `crates/bevy_widgetry`

  - facade 的 Icon 显式 re-export 改成新名称。当前 facade 显式导出了 `Icon / IconPlugin / IconProps`。
  - `window`、`message_box` 模块继续保持现有模块结构。
  - `tests/public_api.rs` 全部改为新的公开 API 名称；该测试目前直接覆盖 Icon、Window、MessageBox 三组旧 API。

- `gallery`

  - 主窗口初始化：

    - `WindowPlugin as WidgetryWindowPlugin` 的别名消失，直接使用 `WidgetryWindowPlugin`
    - 旧的 Window preparation 调用改为 `prepare_native_window(...)`
    - `window(...)` 改为 `widgetry_window(...)`
    - `WindowControlsConfig` 改为 `WidgetryWindowControlsConfig`
    - `@Icon` 改为 `@WidgetryIcon`

    当前 Gallery 主入口同时使用了上述旧 API。

  - 独立窗口示例：

    - `owned_window(...)` → `owned_widgetry_window(...)`
    - `WindowControlsConfig` → `WidgetryWindowControlsConfig`。

  - MessageBox 示例：

    - MessageBox 类型、结果事件、按钮枚举和构造函数全部切到新名称。

## 私有实现名称

本次规则针对 Widgetry **公开控件/API**。

不因为此次重构机械地给所有私有实现类型增加 `Widgetry` 前缀。例如：

- `WindowRoot`
- `WindowContent`
- `OwnedWindow`
- `MessageBoxScene`
- `MessageBoxState`
- `MessageBoxAction`
- `IconMaterialized`
- `IconPendingUpdate`

这些属于 crate 内部实现概念，不是此次公开命名统一的目标。

若某个私有标识只是引用了被重命名的公开类型，则只同步对应引用，不额外扩大命名重构范围。

## 文档

更新描述**当前代码状态**的维护中文档和源码注释，使其中公开 API 名称与代码一致。

`docs/architecture.md` 当前仍直接描述 `core` 的 `Icon`，应更新为 `WidgetryIcon`。

`plans/Stage*` 属于历史设计与实施记录，本次不追溯修改，保留其当时使用的名称。

## 验证目标

本次不新增行为或改变测试语义。

现有测试只需同步 API 名称，并继续验证原有行为。尤其应确保：

- facade 的 public API 测试只能使用新的公开名称；
- Window 外部绑定与 owned 两种路径仍正常；
- `prepare_native_window(...)` 仍可供外部调用方在原生窗口创建前使用；
- ComboBox 和 Window 内建图标继续使用 `WidgetryIcon`；
- MessageBox 的结果事件、按钮组合及窗口所有权行为保持原样；
- Gallery 全部使用新的公开命名，不再依赖旧名称。

重构完成后，源码中的旧公开名称不保留兼容 alias；这是一次直接的 API rename。
