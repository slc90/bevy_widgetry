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

## Couplings

- Popup × Focus、Disabled × 输入、Model Revision × Field Materialization。
- 动态 Text / Icon Asset 与实际 Layout 的同帧准备。

## Coverage Map

- `data_contract.rs`：泛型 Source、Model ID、共享 Source 与构造诊断。
- `selection_field.rs`：Selection Authority 到 Field 的投影、通知顺序及 Model Revision。
- `popup_composition.rs`：真实 Pointer / Keyboard、Focus、弹层关闭、动态内容与 Layout。
- `bsn_combo_box.rs`：Field Shell、颜色、Arrow 与 Disabled 镜像。

