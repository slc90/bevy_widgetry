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

- 构造默认选择、Popup 开关、选项重选及 Escape / Outside Click 关闭。
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
- `selection_field.rs`：Selection Authority 到 Field 的投影、通知顺序及 Model Revision。
- `popup_composition.rs`：真实 Pointer / Keyboard、Focus、弹层关闭、动态内容与 Layout。
- `bsn_combo_box.rs`：Field Shell、颜色、Arrow 与 Disabled 镜像。


私有单元回归保护稳定帧零归属发现、单实例 Owner 修复、独立构造后延后挂接、Popup 换父级、ListView 重建、Root / 部件标记移除、多 T 隔离和销毁清理；计数不进入公共 API。`bsn_combo_box.rs` 继续保护首次颜色消费前的归属与托管部件拒绝公开覆盖写入。

`benches/owners.rs` 使用真实 Headless UI / Layout，固定 10 / 100 / 1000 个 ComboBox、每 Model 100 条数据，测 idle 与仅一个实例丢失 Owner 后的修复；每次验证所有 Field / List 的归属、颜色及实体数量稳定。该结果不代表原生 Present 或 60 FPS 验收。
