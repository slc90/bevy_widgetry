# combo_box Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Source 有效 / 失效；Selection None / 有效 ID。
- Popup Open / Closed、Focus、Disabled、Model Revision 与 Field 内容。

## Stimuli

- Typed Scene 构造、Model CRUD、程序化 Selection、真实 Pointer / Keyboard、Theme / Asset、Popup 生命周期。

## Guards

- Source / Renderer / 尺寸有效。
- Closed 或 Disabled 不能接受隐藏列表的输入；失效 ID / Root 必须拒绝。

## Transitions

- 构造空选择与 Placeholder、显式程序选择、Popup 开关、选项重选及 Escape / Outside Click 关闭。
- Model Move / Remove / Edit 后 Selection 和 Field 投影。
- 程序化 Selection 在执行时提交，Field 更新不生成重复通知。

## Invariants

- Stable ID 在移动后不漂移；共享 Model 的多个 View Selection 独立。
- Authority 先于 `ValueChange`，合法同值不通知。
- Field Shell / List Identity 可保留，旧 Renderer 内容及时清理；隐藏 Popup 不吞掉新的 Keyboard 输入。
- 唯一 Selection Authority 位于内部 ListView；Source-local ID 随 Model Move 保持，Disabled 不修改 Model Metadata。
- Field Open 配色优先于 Hover / Pressed；Popup 可见性驱动 Arrow，SVG 替换保留 Icon Entity 和尺寸，托管部件拒绝公开颜色写入。
- 关闭及时阻断同批剩余 Keyboard；解除禁用保留 Metadata 并可重新打开，Field 投影不生成业务变化通知。

## Couplings

- Popup × Focus、Disabled × 输入、Model Revision × Field Materialization。
- 动态 Text / Icon Asset 与实际 Layout 的同帧准备。

## Coverage Map

- `data_contract.rs`：泛型 Source、Model ID、共享 Source 与构造诊断。
- `selection_field.rs`：Selection Authority 到 Field / Placeholder 的投影、公开本体查询、执行时原始索引、隐藏选择保留、通知顺序及 Model Revision。
- `popup_composition.rs`：真实 Pointer / Keyboard、Focus、弹层关闭、动态内容与 Layout；组合 Override 在局部 Row 输入、滚动复用、Model Edit / Move 及 Field 内容替换后的当帧 Text Foreground，保留 Popup / Focus / Stable Selection 和通知。
- `bsn_combo_box.rs`：Field Shell、颜色、Arrow 与 Disabled 镜像。
- `search.rs`：可选搜索的每实例 Projection 隔离、真实文本键入与结果导航的分流、光标编辑、IME 确认隔离、携带换行正文的 Enter、关闭后同批确认阻断、无结果展示及 Model 自动刷新、隐藏选择与真实删除、重新打开重置查询、有效 Disabled、外部编辑器焦点配色、同周期变化合并和稳定帧不重复匹配，以及 10,000 项结果的有界行数及 Reveal。
- `search.rs` 同时保护同帧文本 / 确认的相对顺序、确认后丢弃剩余正文，以及原生编辑器重新开启冒泡后 Enter / Esc 仍不能触发外层快捷键。

搜索新增 Query 空 / 有匹配 / 无匹配、输入框 Focus 与 List Active 的耦合。过滤不改 Selection / Model，导航不提交选择，只有有效确认才关闭并通知。`benches/search.rs` 在真实 Headless UI / Layout 中固定 1,000 / 10,000 条数据，交替测单结果 / 全部 / 无结果 / 多结果的查询更新（含两次 App Update，使变化后的 Popup 尺寸被虚拟化消费），以及 10,000 项结果窗口 0 / 640px 往返滚动，核对匹配数量、渲染行数和 Text 就绪；不代表原生 Present 或完整 GUI 60 FPS。


私有单元回归保护稳定帧零归属发现、单实例 Owner 修复、独立构造后延后挂接、Popup 换父级、ListView 重建、Root / 部件标记移除、多 T 隔离和销毁清理；计数不进入公共 API。样式单元回归同时保护稳定帧零整套颜色解析 / 应用、单 Field 变化只更新其所属实例、Hovered / Pressed 移除、局部与根 Disabled 添加 / 移除、跨 T 的 Override 隔离、真正 Owner palette 的保留与解除托管、箭头替换及必需输出失败传播与修复后重试。`bsn_combo_box.rs` 继续保护首次颜色消费前的归属与托管部件拒绝公开覆盖写入。

`benches/owners.rs` 使用真实 Headless UI / Layout，固定 10 / 100 / 1000 个 ComboBox、每 Model 100 条数据，测 idle、仅一个实例丢失 Owner 后的修复、单 Field Hover 交替及单 Popup 的 0 / 64px 往返滚动；滚动时固定 8 个可见行，其余实例保持关闭。每次验证所有 Field / List 的归属与 Field 颜色，滚动时核对复用行 Index 和实际 Background / Text Foreground，以及实体数量稳定。该结果不代表原生 Present 或 60 FPS 验收。
