# combo_box 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Source 有效／失效，Selection None／有效 ID，Popup Open／Closed，Focus、Disabled、Field 文本和 Model Revision。

## Stimuli
- Typed BSN 构造、Model CRUD、公开选择 API、真实 Pointer／Keyboard、搜索输入、Theme 与 Popup 生命周期。

## Guards
- Source、Renderer、尺寸和目标身份有效；关闭／禁用 Popup 不接收隐藏列表输入。
- 无效 ID、错误 Field 绑定或过期命令不得部分提交 Selection／Popup 状态。

## Transitions
- 初始化空选择／占位符 → 打开 Popup → 选择／重新选择 → 关闭；Escape／外部点击可取消。
- Model 移动／删除／编辑后，Field 与 Selection 依据当前稳定 ID 更新；程序命令以执行时状态校验。

## Invariants
- Model 移动不改变 Stable ID；共享 Model 的多个 View 分别维护 Selection。
- Selection Authority 位于内部 ListView，先提交再通知；合法同值、仅 Field 投影和自动修复不重复发业务通知。
- Popup 的显示、Focus、键盘和搜索输入不越界；隐藏 Popup 不吞掉后续输入。
- Field Shell／List 身份可保留，Renderer 替换清理旧内容；Theme／Override 和 Owner 作用域在动态部件上正确。
- Search／编辑器错误保留下层原因链与 ComboBox 操作上下文，不伪造成功查询。

## Couplings
- Model Revision × Selection／Field；Popup × Focus／Keyboard；Search × Field／隐藏选择；Theme × Renderer／Owner。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `bsn_combo_box.rs` | Typed Scene、部件归属和默认组合结构。 |
| `data_contract.rs` | Source／ID／Selection 的公开数据语义与失效处理。 |
| `diagnostics.rs` | 多个业务／配色失败的诊断传播、去重及恢复。 |
| `headless.rs` | 独立无默认 Style 的 Selection、Popup 与输入行为。 |
| `popup_composition.rs` | Popup 内容重建、键盘／焦点和组合控件边界。 |
| `search.rs` | 搜索绑定、TextField 编辑、结果过滤和错误传播。 |
| `selection_field.rs` | Selection 与 Field 显示投影、Theme 和内容身份。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/field.rs` | Field 内部结构、部件故障和诊断生命周期。 |
| `src/style.rs` | 归属、调色板更新及稳定帧的局部工作量。 |
