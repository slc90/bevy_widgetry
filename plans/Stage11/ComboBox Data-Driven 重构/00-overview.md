# ComboBox Data-Driven 重构：拆分方案总览

## 整体目标

将当前基于固定 `options + factory + ListBox/ListItem` 的 `WidgetryComboBox` 重构为真正的 data-driven 泛型控件，并直接建立在现有 `WidgetryListModel<T>`、`WidgetryListView<T>` 与 `WidgetryButton` 之上。

最终定位：

```text
WidgetryComboBox<T>
=
WidgetryButton
+ WidgetryListView<T>
+ Popup lifecycle / Field projection
```

整个改造遵循 ListView 已经建立的原则：业务数据与 UI projection 分离、stable item identity 作为选择权威、动态 CRUD 不依赖当前 index、renderer 可重复生成 UI 内容、View state 与 Model 生命周期分离。ComboBox 不再重复实现自己的列表数据、row、selection、disabled item 或 keyboard list navigation。

## 唯一执行顺序

1. [01-generic-combobox-data-contract.md](01-generic-combobox-data-contract.md)  
   先确定 `WidgetryComboBox<T>` 如何直接消费 `WidgetryListModel<T>`，并建立新的泛型 props、stable-id selection contract 与 typed registration 入口。这一阶段确定后续所有实现的公共边界。

2. [02-selection-authority-and-field-projection.md](02-selection-authority-and-field-projection.md)  
   将内部 `WidgetryListViewState.selected` 设为唯一 selection authority，定义默认选择、删除后的空选择、revision/index 变化以及程序化选择时 Field 如何稳定投影当前值。

3. [03-listview-popup-composition.md](03-listview-popup-composition.md)  
   让 Popup 只保留 Popover 与显隐等组合职责，列表、rows、滚动、虚拟化、disabled item、pointer 和 keyboard 行为全部交给 `WidgetryListView<T>`，并补齐 ComboBox 特有的 popup lifecycle。

4. [04-crate-integration-and-gallery-migration.md](04-crate-integration-and-gallery-migration.md)  
   在新组合关系稳定后，收敛源码模块和 crate 依赖，迁移 facade 与 Gallery，确保实际消费者完全使用新的 model-driven 泛型 API。

5. [05-behavior-validation-and-architecture-docs.md](05-behavior-validation-and-architecture-docs.md)  
   最后以新的 architecture 为准重写 ComboBox integration tests，验证与 ListView 的真实组合边界，并同步 architecture 文档和明确的非目标，完成重构闭环。

## 前后关系

这五份方案形成单一执行链。前两份先确定数据、API 与 state authority；第三份才能安全删除旧 Popup/ListBox 实现；第四份在新内部结构稳定后迁移 workspace 消费者；第五份再以最终结构进行系统验证和文档收口。后续方案不得重新引入前面已经删除的 ComboBox 专属 collection、selection 或 row abstraction。
