# core Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- UI 构造期 / Materialize / 内容消费阶段。
- Font 默认 / 显式、Icon 未就绪 / 就绪与当前请求。
- Hierarchy 的本地 / 祖先 Disabled、Pointer 直接 / 后代 Hover、Scene 成功 / 失败。

## Stimuli

- BSN Scene Spawn / Apply、Theme、Foreground Reparent、Pointer Input / Hover Map。
- Insert / Remove / Flush / Commands、Asset 就绪与更新。

## Guards

- Icon Scene 必须提供非空初始路径；缺省和显式空路径均以 Error Severity 拒绝，并记录错误。
- 当前 Scene 操作只回收其新建预约空 Entity。
- Disabled OR 继承不能把祖先结果误记为本地请求。
- 以理由类型区分的内部禁用按 OR 组合，同值或撤销单一理由不会覆盖其他理由。
- Hover 只包括当前有效 Pointer / Window 命中。
- 原生 Mouse 失焦按有序输入取消，不制造普通松手 Button 的 Activate；重新聚焦后的真实点击仍可激活，三个鼠标键都遵循原生接受语义。

## Transitions

- Font Fallback 应用于适用的新 TextFont；显式配置保留。
- Scene 失败报告并清理本次预约，既有业务实体不回滚。
- Pointer 生命周期变化刷新 Hover；Disabled 变更在用户输入前生效。

## Invariants

- 原生 Text 不被 Widgetry 标记颜色修改。
- Foreground Source Identity / Theme / Disabled 与最终内容颜色一致。
- 图标异步请求只消费当前 SVG；新 Image 子树稳定、不拦截 Picking。
- 空闲 Frame 不无意义地重写已稳定的状态。
- UI 阶段顺序保证动态内容当帧完成必要准备。
- 字体 Fallback 仅处理新增且等于 Bevy 默认 Sentinel 的 TextFont；注册顺序保留显式配置，运行期改配置不重写既有文本。
- Icon 等待新 SVG 时保留旧图，只有当前请求就绪且尺寸非零才替换；共享像素不共享颜色与 Entity 生命周期。
- Nested / Forward Reference Scene 失败保留外层预约和业务副作用，连续失败不累计预约空实体，也不重复记录已记录错误。

## Couplings

- Disabled × 输入派发；Foreground × UI Hierarchy / Icon / Text。
- Pointer ID × Hover 命中归属；Scene 错误 × Host Handler / Entity Lifecycle。

## Coverage Map

- `color_schedule.rs`、`ui_schedule.rs`：System 顺序与动态内容准备。
- `default_font.rs`：默认字体策略。
- `content_colors.rs`：Foreground / Text / Theme / Disabled。
- `state_diagnostics.rs`：无效颜色原值与嵌套 Template 的同源身份；Text/Disabled 合法同值和重复无日志；失效 Entity/缺失 DisabledRuntime 的同步及真实 queued 失败、原生访问 source；选择集合/初始多选分类，以及失效 Root/选项/非法目标在拒绝前不更改有效 Checked。
- `effective_disabled.rs`、`disabled_input.rs`：有效 Disabled 及官方输入阻止；无 ComputedNode 时按 Node 身份继承与撤销内部理由，仅有 ComputedNode 的原生 UI 仍参与投影，两类 UI 身份移除/重建后的有效状态生命周期。
- `pointer.rs`：有效多 Pointer Hover 投影。
- `icon_diagnostics.rs`：同步独立请求的领域分类/目标/路径/源头身份，以及嵌套 Template 与真实 queued 边界的类型保真。
- `icon.rs`：初始路径校验、非空路径未加载时的构造、公开 Icon API、真实 Loader、倍率下的物理分辨率与逻辑布局，以及颜色 / 生命周期。Icon 源码 Unit Test 保护 1.0/1.25/1.5/2.0 倍率往返、无 max_size 的固有尺寸、多相机隔离、UiScale 只合成一次与同规格缓存复用。
- `scene.rs`：成功 / 失败的同步与 Deferred Scene 组合及资源清理。
- `scene_diagnostics.rs`：Cached/Related/Template 原包装与 source/origin 保真、三项集合不丢成员、同步不调用 Handler 及显式经真实 System 传播、原生 Severity/panic payload，以及显式启用 Bevy backtrace 时普通格式化不输出边界栈。
- `persistent_failures.rs`：显式业务键、惰性源头工厂、同键共享诊断、不同原因/请求与并存 Owner 的独立身份；真实 System 的异常→重复→等待→恢复→再异常、持续失败仍返回 Err、同 origin 只终结报告一次；真实处理 DEBUG 与恢复 INFO 独立、恢复 context 惰性及最终拥有者释放诊断。
- `persistent_failures.rs` 的独立子进程显式设置 `RUST_BACKTRACE=full`，用 native 对照确认 Bevy backtrace 确实启用；受管转换不含新边界栈，仍保留捕获状态为 Captured 的原始 origin。

Text 内部诊断回归在 `text.rs` 验证已确认 UI 文本的自有 ColorState/TextColor 缺失、多个 Owner 的 batch、重复身份与实际恢复一次 INFO；正常缺省和非 UI 文本由原颜色测试保护。

Icon 内部诊断回归在 `icon/diagnostic_tests.rs` 验证栅格失败的多 Owner batch、尺寸/scale、同键复用、等待/恢复/再失败及缺失 ImageNode 的真实 System 传播。真实 AssetLoader 的读取/解析和 native 缺失资产在独立子进程安装全局 Subscriber，覆盖后台源头一次 ERROR、共享原 origin/source、实际保留旧图像或空态后的 handled DEBUG，以及成功加载后的恢复 INFO。

## 观察边界

`ui_schedule.rs` 在字体已就绪后验证 Build / Materialize 两个合法阶段的新建与替换内容；只推进一次 Update，检查 Fallback、Visibility、Stack 和真实 Measurement，不用额外等待掩盖当帧依赖。Disabled 投影在 Pointer / Keyboard Dispatch 前完成，Foreground 属于官方 Propagate，内容配色先于文字检测与 Content。

`pointer.rs` 的手工 HoverMap 只验证投影；真实 Backend 输入属于 test_utils Pipeline 与控件组合测试。Disabled 继承期间本地请求、Late Plugin、Detach / Reparent 和 Flush 都属于实际覆盖的生命周期输入。



`offscreen_diagnostics.rs` 覆盖 Owner 分配缺资源的惰性同源诊断、显式释放目标身份、持续更新失败/原因变化/恢复、其他 Owner 与嵌套租用 Scene/外部 Camera 的保护、真实 Discard Command Error Severity 和重复创建销毁资源有界。层号耗尽/回收及主失败与多个清理失败的独立 origin 在 offscreen 内部单测保护。
