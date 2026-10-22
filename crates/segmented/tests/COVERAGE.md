# Segmented Integration Test Coverage

## States

- 合法非空 Option 集合、初始 Checked 声明、当前唯一 Checked、结构错误与恢复。
- Group Focus、Group / Option 有效 Disabled、用户通知与程序静默。

## Stimuli / Guards

- BSN 构造及任意内容组合、后代 PointerClick / PointerPress、实际键盘派发、程序 Commands、增删层级、标记失效。
- 初始多个 Checked 拒绝且不覆盖声明；空集合、非法 Root / Child / Index 以 Severity::Error 传播。
- Focus 内对应方向轴有效；跳过 Disabled、首尾停止，其他按键和无焦点忽略。Tab / Shift+Tab 跳过 Group。

## Transitions / Invariants

- 首次无 Checked 选择首项；一个 Checked 采用指定项；冲突诊断去重，移除冲突标记后恢复。
- 添加及未选中项删除保持身份，删除选中项选相邻项，全部替换恢复首项。
- 用户变更先提交唯一 Checked，再通知 Root 的索引；重复点击 / 程序设置 / 自动修复不重复发用户事件。
- 禁用仅限制用户操作，允许程序选择与结构补选；多个组及嵌套原生 RadioGroup 独立。
- 排队命令在执行时检查当前 Root / Child / Index；失败不部分改选，不阻止后续合法组命令，同值保留 Checked change tick。

## Coverage Map

- `headless.rs`：初始化、结构、输入、Disabled、事件、队列错误、场景组合与嵌套组隔离；两方向输入轴、首尾 / Disabled、Tab 跳过、结构回退语义一致。
- `group.rs` Unit Test：稳定结构不重复校验、受影响组局部维护、后装 Plugin 和无效初始化请求。
- 共享选择历史的连续 / 批量删除、临时 Checked 项与复用原 Option 清空恢复，同时由 RadioGroup 的实际集成回归保护。

- `style.rs`：同帧高亮转移、Focus / Hover / Disabled 叠加与恢复、四套主题、组与局部字段覆盖、无效颜色和托管拒绝、真实 UI 布局及外部内容独立颜色。
- Style Unit Test：局部工作量与稳定帧零工作、托管输出缺失诊断及重插恢复。

- `layout.rs`：自然宽度 / 水平等宽的真实 ComputedNode 尺寸、非文本内容、固定与 intrinsic 容器、动态追加、Theme / Checked / Disabled 不改变模式、Node 覆盖优先；Vertical 等宽不转等高、四套主题 / 焦点 / Disabled 保持。

## Boundary

当前不含独立 Gallery 页面；不支持运行时任意重排。
