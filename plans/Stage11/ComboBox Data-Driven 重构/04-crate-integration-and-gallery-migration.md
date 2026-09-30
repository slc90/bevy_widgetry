# 完成 crate 集成与 Gallery 迁移

## 目标

在新的泛型 ComboBox、selection authority 和 ListView Popup 已经成立后，清理旧源码结构与 workspace 依赖，并把 facade 和 Gallery 全部迁移到新的 model-driven API，确保仓库中的真实消费者不再依赖旧固定 option factories。

## 范围

负责：

- ComboBox crate 源码模块收敛；
- `Cargo.toml` 的 `combo_box -> list_view` 生产依赖；
- plugin imports/re-exports 的整理；
- facade 保持新的 ComboBox API 可见；
- Gallery ComboBox 页面迁移；
- TitleBar Theme ComboBox 迁移；
- Gallery 中展示动态 CRUD 与 per-item disabled 能力。

不负责：

- 重新设计 selection 或 Popup 行为；
- 为旧 API 保留 compatibility layer；
- 最终 integration test 覆盖与 architecture 文档收口。

## 预期产出

完成后 workspace 中正常构造 ComboBox 的代码都使用 `WidgetryListModel<T> + WidgetryComboBox<T>`；旧 `WidgetryComboBoxOptionFactory`、`@options` 与旧 option module 不再被 Gallery 或 facade 消费。Gallery 能人工展示新的 data-driven 能力，而不只是把旧静态例子机械翻译成新 API。

## 与前后方案的关系

本方案只在前三阶段的内部 contract 已经稳定后执行，避免消费者在中间态上反复迁移。下一阶段将针对这里形成的最终 repository shape 重写 integration tests，并同步 architecture 文档。

---

## 一、源码组织收敛

预计 ComboBox crate 收敛为：

```text
crates/combo_box/src/
├─ lib.rs
├─ combo_box.rs
├─ field.rs
├─ popup.rs
└─ registration.rs
```

删除：

```text
option.rs
```

职责：

```text
combo_box.rs
    generic SceneComponent / props / public API

field.rs
    Button shell
    selected-value projection
    disabled mirror
    dropdown icon

popup.rs
    Popover shell
    visibility
    outside click
    reselect close
    geometry

registration.rs
    WidgetryComboBoxPlugin typed runtime registration
    initialization / typed systems
```

这只是按新的真实职责收敛现有文件，不为了和 ListView 文件结构完全一致而继续机械拆分。若实现过程中某些 typed behavior 留在 `combo_box.rs` 或 `popup.rs` 更自然，只要职责边界清晰，不需要为了目录美观强行“一种 type 一个文件”。

## 二、删除旧 ComboBox 专属列表实现

随着 Popup 已经由 ListView 接管，应彻底清理旧实现残留：

```text
ComboBoxOption
ComboBoxOptions
WidgetryComboBoxOptionFactory
旧 option row scene
旧 Selected component 管理
旧 option style systems
旧 ListBox/ListItem hierarchy assumptions
旧 ValueChange<usize>
```

不保留旧 `options` API 作为兼容层，也不建立“把 factories 临时灌进 WidgetryListModel”的过渡实现。

这是一次直接 architecture replacement，避免 crate 内同时存在两条 option source path。

## 三、Cargo 依赖

`crates/combo_box/Cargo.toml` 新增：

```text
bevy_widgetry_list_view = { path = "../list_view" }
```

形成：

```text
combo_box --> list_view
```

同时根据最终实现删除 ComboBox 已不再直接需要的列表基础设施依赖或 imports。`WidgetryButton`、asset、icon、Popover 等 ComboBox 仍然直接组合的能力继续由 ComboBox plugin 管理。

兄弟 crate 依赖的理由必须保持明确：ComboBox architecture 现在正式建立在 ListView 之上，而不是单纯为了复用一个 helper。

不得引入任何反向 `list_view -> combo_box` 依赖，也不得把 ComboBox-specific glue 搬入 `core`。

## 四、facade 公共入口

顶层 facade 继续通过：

```rust
pub mod combo_box {
    pub use bevy_widgetry_combo_box::*;
}
```

暴露新的泛型 ComboBox API。

对应可见内容包括：

```text
WidgetryComboBox<T>
WidgetryComboBoxProps<T>
WidgetryComboBoxPlugin
WidgetryComboBoxAppExt
```

ComboBox 使用的 `WidgetryListModel<T>`、`WidgetryListItemId`、`WidgetryListViewRenderer<T>` 本身继续由 facade 的 `list_view` module 暴露，不需要在 ComboBox module 再复制 re-export，除非现有公共 API 风格明确要求这么做。

facade 不承载具体 ComboBox 行为实现。

## 五、Gallery ComboBox 页面迁移

Gallery 不再创建：

```rust
Vec<WidgetryComboBoxOptionFactory>
```

改为创建独立：

```rust
WidgetryListModel<T>
```

source entity。

现有 Text / Icon + Text / Icon / Disabled 示例继续保留，用来证明 renderer 仍然支持 arbitrary SceneList 内容。

新的 Gallery 示例不应只是把三个固定选项搬到 model 里就结束，而要至少能观察到此次改造真正新增的能力：

```text
动态 insert
动态 remove
动态 move
修改 selected value
per-item disabled
```

不需要复制 ListView Gallery 的所有操作按钮；只覆盖 ComboBox 自己需要验证的组合行为即可。

例如可以保留几种不同 renderer 内容的 ComboBox，再为其中一个 data-driven 示例提供少量操作，让人工验证者能够看到：

- model 插入/删除后 Popup rows 跟随变化；
- selected id 在 move 后仍指向同一业务 item；
- selected item 内容修改后 Field 同步；
- selected item 删除后 FieldContent 为空但 Button 仍可打开剩余选项；
- disabled item 出现在 Popup 中但不能通过用户输入选择。

## 六、Gallery source 生命周期

与 ListView Gallery 一样，ComboBox 的 model 应作为独立 source entity 存在，而不是把数据藏在页面 row hierarchy 内。

如果页面会因为导航显隐而重新创建 UI，则 source 生命周期应根据 Gallery 的实际资源组织保持独立，避免页面重建导致 model identity 无意义地重置。

多个 ComboBox 如需共享 model，可以直接指向同一个 source；若展示独立 selection，则每个 ComboBox 使用自己的内部 ListView state。

## 七、TitleBar Theme ComboBox 迁移

当前 TitleBar theme selector 也是旧 fixed-options ComboBox 消费者，必须同步迁移。

可以让 source 直接保存业务值，例如：

```text
WidgetryListModel<ThemeMode>
```

或保存一个包含 label 与 ThemeMode 的 Gallery 业务 item；具体选择保持 Gallery 自身语义清晰即可。

Theme selection 不再根据公开 index 判断：

```text
0 → Dark
1 → Light
```

而改为：

```text
ValueChange<WidgetryListItemId>
        ↓
对应 WidgetryListModel<...>
        ↓
get_by_id(id)
        ↓
ThemeMode
```

这能真实验证 stable identity 的新公开 contract，而不是在消费者处重新把 id 映射回“当前第几个”。

Startup 根据当前 `ThemeMode` 做静默 programmatic selection 时，也使用 source 中对应 entry 的 stable id。

## 八、typed registration 在 Gallery 的落地

Gallery 安装 `WidgetryComboBoxPlugin` 后，还需要为实际使用的业务 item type 调用：

```rust
register_widgetry_combo_box::<T>()
```

如果 Gallery 的普通 ComboBox 和 Theme ComboBox 使用不同 item type，则分别注册。

调用方不额外为这些 ComboBox item types 调 `register_widgetry_list_view::<T>()`；这一点由 `WidgetryComboBoxAppExt` 封装，验证 ComboBox 对其内部 ListView dependency 的装配责任。

Gallery 自己如果还有独立 `WidgetryListView<T>` 页面，则对应业务 type 仍按 ListView 页面自身需要注册，两套注册通过 ListView 的 typed plugin identity 保持幂等。

## 九、Gallery 保持消费者身份

Gallery 只使用公开 API 和公开 hierarchy contract，不读取 ComboBox private cache、private popup runtime 或 ListView private runtime。

不得为了 Gallery 操作方便扩大 ComboBox 私有实现的 visibility。

Gallery 展示效果与交互行为最终通过项目既有 GUI debugging 流程验证；Gallery 自身不要求增加 unit/integration test，但必须保证编译通过。
