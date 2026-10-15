# 将数据视图文案归入各页面的 Key 类型

## 目标与范围

迁移 ListView、Tree、Table 的页面专属 Key，包括新增 Shared Model 示例中的说明与状态文本。修改 Key、两种语言匹配和使用点，不改变 Model 或 Renderer API。

## 修改设计

把各页面的数据规模、虚拟化说明、懒加载状态、表格区域、列布局和共享数据示例归入对应页面类型。通用的插入、删除或选择操作只有在语义确实相同时才共用 Key；不能把 Tree 共享 Selection 与 ListView 独立 Selection 的说明混成一个模糊文案。

同步迁移 Renderer 构造内容中的 Key、事件更新后的 LocalizedText 和嵌套 Key 参数。行复用或业务组件更新后，新内容仍按当前语言创建；不要为了刷新翻译而修改 Stable ID、Model Revision 或重新建立整个数据模型。

沿用公共分类，不复制 Common、Header、Navigation 或 ColorShowcase 的定义。中英文仍分别穷尽匹配；页面 Key 的强类型归属不能通过字符串拼接或全局查找表绕开。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

检查三个页面及其 Shared Model 示例在切换语言、虚拟化滚动、插入、删除、移动、编辑和懒加载之后的文本。原始业务数据不被当成翻译 Key，命名参数及原有翻译内容不变。

语言切换不改变 ListView 各 View 独立状态、Tree 共享展开与 Selection、Table 各 View 的 Layout 和列宽。通过 BRP 检查可见内容和 ECS 身份；保留已有测试对参数替换和状态隔离的保护，不为翻译迁移重做全部控件测试。

## 预期产出

数据视图的文案按页面职责组织，并在共享 Model、动态 Renderer 和虚拟化场景下保持原有国际化行为。

## 与前后方案的关系

顺序上接 [方案 42：将基础控件文案归入各页面的 Key 类型](42-将基础控件文案归入各页面的-Key-类型.md)。

完成后进入 [方案 44：完成窗口与显示页面的 Key 分层](44-完成窗口与显示页面的-Key-分层.md)。

依赖公共 Key 结构，并覆盖前面新增的 Shared Model 示例；不改变三种数据视图不同的状态归属。
