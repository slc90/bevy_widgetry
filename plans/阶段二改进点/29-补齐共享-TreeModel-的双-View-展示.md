# 补齐共享 TreeModel 的双 View 展示

## 目标与范围

扩展 `crates/tree/tests/view.rs` 等相关回归，并在 Gallery Tree 页面增加并排 Shared TreeModel 示例。复用现有基础共享 Source 测试，不从零重复建设。

## 修改设计

两个 View 引用同一个 TreeModel。展开／折叠和 Selection 必须同步，因为它们属于 Model；各 View 的滚动和 Disabled 保持独立。共享的层级修改、节点插入／删除和业务 Component 更新，在两侧反映同一业务数据。

禁用一侧后，它不消费用户操作，但仍显示另一侧或程序 API 提交的共享变化。滚动操作不被当作共享 Model 状态复制；实际键盘 Focus 仍按输入系统正常转移，不承诺两个 View 同时获焦。

本项不要求两个 View 使用不同 Renderer。RendererRegistry 当前按 Component 类型在 App 级注册，不能为了展示差异新增逐 View Registry。可以使用现有布局、尺寸和颜色配置帮助区分两侧，不修改 TreeModel／TreeView 的状态归属。

Gallery 通过真实展开、选择和共享数据编辑展示行为，使用稳定 Name 与强类型中英文文案。懒加载仍由调用方响应 Unknown／Loading／Loaded 流程，不因为双 View 展示重复发起同一 Model 的业务请求。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

验证两侧展开／选择同步，层级和业务组件变化同步，滚动与禁用隔离；删除选中节点后的 Repair 按现有规则发生，销毁 View 不销毁业务 Node。已有共享基础用例保留，只补缺失耦合。

用 BRP 实际操作两侧、禁用其中一侧、切换语言与主题，离开页面后检查临时 View／Model 清理。更新 Tree Coverage 和相关展示说明；Gallery 不强制新增自身单元测试。

## 预期产出

明确展示“共享业务状态、独立滚动和输入限制”的 Tree 双 View 场景及相应回归。

## 与前后方案的关系

顺序上接 [方案 28：让 Tree 列表和展开按钮配色按变化更新](28-让-Tree-列表和展开按钮配色按变化更新.md)。

完成后进入 [方案 30：让 Table 可见区域样式按变化更新](30-让-Table-可见区域样式按变化更新.md)。

作为 Tree 的独立展示增量，验证既有共享 Model 契约，不要求扩大 Renderer Registry。
