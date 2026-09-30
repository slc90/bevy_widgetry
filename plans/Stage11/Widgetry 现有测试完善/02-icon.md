# 02 Icon：保护异步换图、颜色与首帧显示

依据：slc90/bevy_widgetry，提交 `5d57413cafa65f5ccc47be397e20e917dc2bd00d`。整理日期：2026-09-30。本次仅静态分析与方案编写，未修改仓库、未执行测试。

[总览](00-overview.md) | [前一份](01-shared-test-foundation.md) | [后一份](03-button.md)

## 本份方案

### 目标、范围与预期产出

本方案负责 core 中公开 WidgetryIcon 的完整测试，不新建 icon crate。范围为 crates/core/src/icon.rs、icon/svg.rs 的局部测试与 crates/core/tests/icon.rs；只在复用已有基础设施确有必要时修改 test_utils。产出是一份 Icon Coverage Map，以及有控制的等待/就绪/失败/替换测试。各控件以后只验证自己怎样驱动 Icon，不再各自重测 SVG loader。

### 已有覆盖

runtime_mutations_survive_scene_initialization 已走真实 AssetServer 和 UI 插件，检查图像初次出现时的 visibility/stack、运行期颜色、clear_color 的白色回退和 SVG 最终替换。asynchronous_icons_request_redraw_until_ready 已检查等待及新图提交需要刷新、稳定后停止请求。icon.rs 的局部 raster_failure_logs_state_edges 已验证首次失败、重复失败静默、恢复及再次失败，保留这些诊断断言。

当前换图测试主要等到 handle 改变后再检查；它没有逐步证明“新 SVG 未就绪期间旧 image 一直保留”。现有 clear_color 例子也没有继承的 ForegroundColor。svg.rs 中 Widgetry 自己的缩放、尺寸和 Image 转换没有对应局部测试。这里才是本次增量，不是再写一遍颜色红转白。

### 状态模型与边界

可观察维度为尚未有图/已有图、当前显示资源/请求中的资源、显式色/继承色/白色回退，以及可显示尺寸/零尺寸输入。刺激为首次构造、asset 就绪或失败、set_svg、set_color、clear_color、前景色传播和 entity 销毁。关键不变量是最多一个由 Icon 管理的图像 child、换图等待不清空旧图、显式色优先、图像 child 不拦截 picking。cache 和 pending marker 不加入公开业务状态机；只在局部测试里作为控制资源就绪的必要实现接缝。

max_size 是当前构造配置，没有公开运行期 setter。本方案不要求新增运行时尺寸 API、热重载协议或 cache 回收策略。共享 cache 持有 strong handle 是现有设计；销毁一个 Icon 后不能把“Assets<Image> 必须清空”当成正确性要求。

### 单元测试的补充

在 svg.rs 同模块用小型、尺寸确定的 SVG 验证原始尺寸、宽边受限、长边受限、非正方形和取整后的 Image 尺寸、像素缓冲长度及格式。这些测试保护 Widgetry 对 resvg 的参数选择和输出转换，不测试第三方全部 SVG 标准。尺寸存在 ceil，不断言数学长宽比在整数像素上完全相等；使用能说明取整语义的输入。零尺寸不生成图像的行为在 Icon 层验证，不能要求底层 Pixmap 接受零尺寸。

沿用现有保留 handle/注入 SvgAsset 的局部接缝，确定性地控制 A 已显示、B 尚未就绪、C 后请求并先就绪。每步运行真实 Icon systems 后断言：等待时仍是 A，最终采用当前请求 C；B 后到不会把图像退回旧请求；在等待期间改颜色仍影响当前显示的图像。另覆盖 A→B pending→A 的取消替换，以及尚未 materialize/等待替换时 despawn，不产生孤儿 child 或后续异常。这里不需要把私有 SvgAsset 变成 pub。

### 集成测试需要加强的证明

公开 API 的真实加载测试保留，同时把首次出现图像的那一帧作为观察点，补有效 Image asset、对应尺寸、真实 camera 下的 ComputedNode，以及 Pickable::IGNORE。用预加载资源的替换场景单独验证同帧传播；冷加载允许实际等待，不能宣称首次网络/磁盘请求当帧就有图。

增加父节点传播前景色→已有 Icon 更新、设置显式色后父色变化不覆盖、clear_color 后恢复当前继承色的连续场景。状态变化后检查实际 ImageNode.color，而不只检查 Icon component。两个同 SVG/尺寸的 Icon 可共享 image，但颜色独立；删除一个不影响另一个。该场景保护共享像素和独立外观的组合语义，不测 cache 容器的具体条目布局。

将所有当前 BuiltinIcon 经真实 loader/raster 路径各验证一次，确认 asset 测试里“非空 bytes”的资源能真正显示。循环只围绕现有枚举，不引入新的 asset 格式。错误路径与零尺寸按已有“不生成新 image”的行为检查，不制造占用巨大内存的随机测试。确定性测试已足够，本阶段不添加 proptest。

### 组织与验收

core/tests/icon.rs 开头保存业务模型和主要 owner；需要拆文件时，只把异步生命周期、颜色/显示阶段分开，保留总览。局部受控资源测试仍在 icon.rs，纯像素尺寸测试在 svg.rs。运行 bevy_widgetry_core，及直接依赖内建图标的 CheckBox、Window、ComboBox 对应测试。

条件 BRP 场景只选本阶段相关的图标换源/主题变化：观察旧图是否短暂消失、颜色是否落后于背景。截图不能证明所有帧无闪烁；异步中间状态的长期证明由上述受控测试承担。

### 定向验证入口

以下为本份涉及的 package 测试入口，不是全部验收条件。必要的格式、lint、共享 helper 消费者验证和各节声明的时序/环境边界仍须满足；最终全仓验证由 13 负责。

```text
cargo test -p bevy_widgetry_core
cargo test -p bevy_widgetry_check_box
cargo test -p bevy_widgetry_window
cargo test -p bevy_widgetry_combo_box
```

### 在执行链中的位置

接在共享基础之后，复用真实 UI 与明确推进边界。下一份 Button 只需验证自己的前景色与内容组合，不复制本份的加载、raster 和换图全套测试。

### 基线证据与实施入口

以下链接固定到本次盘点提交。已有覆盖来自这些测试中的实际断言；新增、增强和组织调整是本方案提出的实施内容。未来分支已变化时，先核对差异，再决定是否仍需补充同一场景。

- [crates/core/src/icon.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/core/src/icon.rs)
- [crates/core/src/icon/svg.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/core/src/icon/svg.rs)
- [crates/core/tests/icon.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/core/tests/icon.rs)
- [crates/asset/tests/embedded.rs](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/crates/asset/tests/embedded.rs)

## 本方案适用的共同约束

### 实施范围与证据

执行的是测试完善，不是新增业务能力。默认修改测试代码、测试模块注释和确有当前消费者的测试辅助设施；需要实际使用 proptest 的阶段才允许添加对应 dev-dependency 和锁文件变化。不修改规则，不顺手升级 Bevy、rstest、BRP，不增加生产 API、公开私有 marker 或新控件。若新增测试暴露失败，先判断是测试环境不完整、期望不符合现有 contract，还是生产回归；保留复现证据，未经当前任务授权不顺手扩大为生产修复。不能为了得到绿色结果删掉原有断言、增加无依据的等待或改写业务语义。

### 按现行规则执行，不依赖历史讨论

Codex 开始实际修改前读取 AGENTS.md、docs/architecture.md、rules/task-scope.md、rules/development.md、rules/code.md、rules/testing.md、rules/documentation.md；涉及 GUI、诊断日志、依赖或提交时继续读取相应规则。本方案作为用户明确提供的任务输入，不要求主动读取仓库 plans/ 中的历史设计。按规则创建并持续更新根目录临时进度记录。测试代码和工程配置变更也需要独立 code-review：全新 reviewer subagent 只审查，原实施者处理 findings，之后由另一个全新 reviewer 复查。Windows 下使用 pwsh。

### 覆盖模型落在测试旁边

复杂测试模块在开头写中文 //! 注释，列出可观察状态维度、stimuli、guards、invariants 和明确的业务耦合；简单测试只写有内容的范围说明，不填空模板。多文件测试由一个现有主测试模块保存 Coverage Map，说明各文件主要负责的 contract；具体转换仍由 Rust 测试表达，不另建完整 Markdown transition table 或强制可执行 Rust 状态机。不做状态笛卡尔积；正常、边界、重复/no-op、拒绝、失效引用仅按实际语义覆盖。每个测试函数前保留或补充中文的场景与目的说明，遵守项目不新增 doctest 的要求。

### 断言证明结果，不重复实现

测试准备阶段可以设置前置状态，但不能直接写入被测结果后宣称输入链路通过。每个转换检查明确的期望结果、相关事件的来源/值/次数，以及本次可能破坏的不变量。assert_*_invariants 辅助函数不能代替场景专属期望：例如 move 完全没执行也可能仍满足“id 唯一”。观察 ECS 状态时使用真实系统执行到合同规定的阶段；model 修改后的待修复瞬间不等于修复后的稳定 invariant。公开组合行为通过公共入口验证；只有依赖私有状态的局部算法、诊断或明确调度约束才留在同模块单元测试。不能仅因为测试使用 App 就机械搬迁。

### 共享设施与测试隔离

复用 test_utils 现有 scene_app、add_ui_plugins、text_input_app、press_key、pointer 与日志辅助设施。重复出现的通用输入、时间、camera、asset 等环境装配收敛到 test_utils；控件特有的 fixture 和 contract 断言可以放在该控件 tests/support 中，不让 test_utils 反向依赖所有控件。所有新辅助设施都必须有本轮实际测试消费者，不预建通用测试框架。fixture 独立持有 App、事件记录和资源；查询在多实例测试中按 root/source/hierarchy 约束，不能全世界找到第一个或假定 single。不使用真实桌面、系统剪贴板或全局时间作为普通 headless 回归的隐含前提。

### 时序、异步资源和测试完成条件

把“等待资源首次就绪”和“资源已就绪后某次操作应当在哪个阶段生效”分开。资源准备可以使用带期限、失败诊断的条件等待；被测操作后的同帧保证应在明确的一次 update、flush 或消费阶段断言，不能用 advance_until 最终成功来证明。需要多轮 layout 的合法收敛按真实合同检查，不强行改成一帧。颜色、文字、icon、visibility、stack、layout 等检查应说明各自观察点。验证刷新请求时按新消息区间读取或清空，避免历史 RequestRedraw 造成假阳性；稳定后也检查不再无意义刷新。私有 cache 不作为业务状态维度，只有其承载明确增量更新/调度合同的部分才做局部回归。

### BRP 只负责本阶段相关的真实场景

各分方案的 BRP 段是条件验收场景：仅补测试和注释、没有修改 GUI 可观察行为时，不为了形式强制启动 Gallery。若本阶段获授权修复了相关 GUI 行为，或用户明确要求运行时验收，再按规则用 BRP 执行列出的少量真实用户路径。不能直接改 selection、focus 或滚动结果冒充用户操作，不重新跑全部历史功能。动态文字、icon、popup 和 subtree 关注操作后立即可见的过程与最终状态，而不是先等稳定再截图。连续 MCP 截图存在采样间隔，不能声称已经排除了所有单帧闪烁或精确测出了输入到显示的延迟；未能观察的时序明确记录，并由可表达的 headless 阶段断言补永久回归。保留 desktop_app 的事件驱动设置，不引入 Win32 自动化或持续高频刷新来掩盖问题。原生窗口状态等 BRP 无法忠实覆盖的路径按规则记录人工验证边界。

### 本轮完成标准

每份方案完成时，Coverage Model/Map 与实际测试对应；已有有价值的测试没有因整理丢失；新增场景具有明确触发输入和独立期望，不仅断言“不崩溃”。局部合同的测试优先用普通 #[test] 或现有 rstest，proptest 不代替已知边界和确定性回归。补已有正确行为的测试可以初次即通过，不人为制造失败；真实 bug 回归按 Red/Green 记录。执行该分方案指定 package 与直接消费者的测试、必要格式和 lint 检查，并记录未运行项与环境限制。最后的 facade 方案完成 workspace 整体验证。只有依赖结构等 architecture 事实确实变化时才同步 docs/architecture.md。

## 来源与整体边界

本方案依据 GitHub main 的固定快照 5d57413cafa65f5ccc47be397e20e917dc2bd00d，以及该快照已经更新的测试、GUI 验证和开发规则编写。目标是整理现有测试：保留已经有价值的回归保护，增强证明不足的断言，补上能够从当前 contract 和代码路径确认的覆盖缺口。不是重新设计控件，也不是再修改一次测试规则。

本次进行了源码和测试的静态盘点，没有执行 cargo test、覆盖率工具、原生窗口或 BRP。因此，文中的“已有覆盖”表示存在对应测试及断言，不表示本次运行通过；“补充”表示本模块缺少足够直接的验证，不声称其他模块完全没有间接覆盖。方案中的测试文件新名称均是拟新增位置，不冒充现有文件。实施时应先核对当前分支与基线的差异。

范围覆盖 Button、CheckBox、RadioGroup、TextField、Tooltip、ScrollArea、ListView、ComboBox、Window、MessageBox 十个控件；二态/三态 CheckBox、普通/只读 TextField 分别留在所属控件方案中。公开的 WidgetryIcon 虽位于 core，也单独形成一份方案。共享测试基础与 facade 各有一份配套方案，共十三份，按编号顺序实施。Gallery 不新增永久自动化测试，asset/log 不因这轮盘点而扩出另一套无关重构。

- [rules/testing.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/testing.md)
- [rules/gui-debugging.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/gui-debugging.md)
- [rules/development.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/development.md)
- [rules/documentation.md](https://github.com/slc90/bevy_widgetry/blob/5d57413cafa65f5ccc47be397e20e917dc2bd00d/rules/documentation.md)
