# 06｜将 TextField 页接入按需生命周期

## 目标

按需创建 TextField 演示并确保每次重新访问时输入内容和焦点相关页面态重置。

## 范围

`gallery/src/pages/text_field.rs` 返回的 `impl SceneList` 与现有输入示例，不修改 TextField Widget 的 API。

## 预期产出

TextField 页面只在激活期间存在，输入 UI 子实体与本地值在退出时释放。

## 与前后方案的关系

承接统一封装；下一份为 Tooltip。

## 方案正文

当前 `pages::text_field()` 返回 `impl SceneList`，与多数页面返回 `impl Scene` 的签名不同；公共 `page(target, content: impl SceneList)` 不能假定每个页面都有同样具体 Scene 类型，也不能为了按需生命周期把 TextField 内容重复包裹成新的组件体系。保留原输入控件组合、on/change 行为和主题效果，将这组 SceneList 挂在当前 TextField 页面容器内即可。

`OnEnter(TextField)` 创建新输入实体和 ColorShowcase 区；`OnExit` 同步销毁容器，不保留旧文本值、游标和选择态。若切页时输入框持有全局 `InputFocus`，须在回归测试中确认焦点没有指向已删除实体，避免影响下一页的键盘导航；这属于 Gallery 切页行为的必要验证，而不是更改 TextField Widget 公共接口。无需引入空页面 Resource。

**源代码：** [text_field.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/text_field.rs)。
