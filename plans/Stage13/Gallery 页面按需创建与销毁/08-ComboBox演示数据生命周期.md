# 08｜将 ComboBox 演示 Model 改为页面私有数据

## 目标

让 ComboBox 页访问时才创建四个示例 ListModel，并在离开时回收其资源，同时保持 Theme Header ComboBox 独立。

## 范围

`gallery/src/pages/combo_box.rs` 与 ComboBox ColorShowcase 调用；不改 `crates/combo_box`。

## 预期产出

页面重进时静态/动态图示例与选择态归零；不再由 Plugin::build 提前创建四个 Model。

## 与前后方案的关系

依赖第 01/02 份公共生命周期协议；与相邻的 ListView 属于独立页面，没有数据源复用关系，仅在顺序上先迁入。

## 方案正文

### 注册与私有状态拆开

当前 `ComboBoxDemoPlugin::build()` 既负责 `register_widgetry_combo_box::<ComboBoxDemoItem>()`，又创建四个 `WidgetryListModel<ComboBoxDemoItem>`，然后 `insert_resource(ComboBoxDemoSources([Entity; 4]))`。其中三个静态 Model 分别包含文字、文字+图标、纯图标，第 4 个动态 Model 由 `populate_dynamic` 生成 Apple/Banana/Orange/Grape/Pear 等条目。页面内可 Insert/Remove/Move/Edit/ToggleDisabled/Select/Clear/Reset，这些行为都应在再次进入页面时从初态开始。

类型注册及 `PostUpdate` `update_status.after(UiSystems::Layout)` 的 System 注册继续在 `Plugin::build()` 一次完成；`update_status` 增加 `run_if(in_state(GalleryPage::ComboBox))`。四个模型构建、`ComboBoxDemoSources` 插入移至唯一独占 `OnEnter(ComboBox)`，先完成四个 Model 实体，再用 `pages::combo_box(sources)` 构建页面根（包括动态 Button 和颜色示例的入口）。`ComboBoxDemoSources` 记录拥有的四个 Entity，不包含常驻 Header 主题 Model。

退出时由同一个独占 `OnExit(ComboBox)` 先销毁 ComboBox 页 UI（含内部 Popup、虚拟列表与颜色示例已展开的 content），再 `despawn` 四个私有 Model，最后 `remove_resource::<ComboBoxDemoSources>()`。根据 `crates/combo_box/src/popup.rs`，Popup 是 ComboBox 控件实体的 ChildOf 子树，按 UI 根递归销毁即可，不需要单独销毁其他窗口或修改 Widget crate。Observer `on_selection_changed` 等继续依赖其事件源和 UI 实体存在性判断。

`color_examples(world)` 会读取 `ComboBoxDemoSources[0]`；只有 ComboBox 当前页且 Resource 有效、用户展开 ColorShowcase 时才能调用。主题栏 `WidgetryComboBox<WidgetryThemeMode>` 仍使用 `main.rs::setup()` 中独立创建的常驻源，不能被清理。

验证：首次进入四种展示正确；切换离开时 Model Entity 全部消失；重进后动态图编辑状态复位；主题选择仍有效；展开 Popup 后切页不会遗留浮层。

**源代码：** [combo_box.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/combo_box.rs)、[crates/combo_box/popup.rs](https://github.com/slc90/bevy_widgetry/blob/main/crates/combo_box/src/popup.rs)。
