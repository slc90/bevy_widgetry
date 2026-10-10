# 03｜将 Button 默认页接入按需生命周期

## 目标

让首次进入 Gallery 的 Button 页面由 `OnEnter(Button)` 单独创建，并在离开时完全释放页面 UI。

## 范围

`gallery/src/pages/button.rs` 的 `scene()`、Button 与 RadioGroup 演示；不修改其 Widget 行为。

## 预期产出

初始 Button 页面正常显示，重复点击 Button 不重建，离开再回来示例交互态恢复默认。

## 与前后方案的关系

依赖第 01/02 份提供的默认状态、PageHost 和公共封装。它是后续其他页面迁移的最小 UI-only 参考。

## 方案正文

`button::scene()` 无 `DemoSources` Resource，主内容使用 Button、RadioGroup、Text、Icon 的 BSN 组合以及 `on_radio_changed`、`on_demo_activated` 等事件。`OnEnter(Button)` 将原 `pages::button()` 放入公共 `page(Button, content)` 外壳并同步挂到 PageHost；`OnExit(Button)` 同步销毁对应唯一 `GalleryPageContent(Button)` 根即可。按钮激活后的文字等局部状态继续依赖 Component/事件处理，不制造空 Resource。

首次加载由 Startup 排队 `GalleryPage::Button` 进入后完成；同页导航用 `NextState::set_if_different` 保证不会重新加载。保留原 `#ButtonPage` 等可供 BRP/界面回归检查的 Name，颜色展示仍提供 Button、RadioGroup 等目录，进入时默认折叠、主题切换仍生效。此页不清理 Header 使用的全局 `WidgetryListModel<WidgetryThemeMode>`。

验证：打开 Gallery 即可见 Button；激活按钮或 RadioGroup 后，切至别页再返回时状态重置；主窗口与主题选择器保持原实体和设置。

**源代码：** [button.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/button.rs)。
