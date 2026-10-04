# FileDialog 拆分方案总览

下面是一条按顺序执行的交付链；每个小方案包含自己的目标、范围、输出、完整设计正文、测试与承接关系。

| 顺序 | 文件                                                                                    | 主目标                                                                                                |
| ---- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 01   | [Headless：文件选择业务与公开 contract](01-headless-contract.md)                        | 建立可独立测试的文件选择业务 state，完整定义文件/文件夹/保存、导航、过滤、selection 与存储语义。      |
| 02   | [Headless：后台 filesystem 与 storage runtime](02-headless-async-filesystem-storage.md) | 让所有可能阻塞的 filesystem/存储工作在有界服务中执行，提供 streaming、取消、reactive 唤醒与可靠回收。 |
| 03   | [Style：BSN 内容与多选虚拟化列表](03-bsn-style-virtualized-view.md)                     | 把 headless state 投影为完整可操作的 FileDialog 内容，保持大目录与频繁更新下的 UI 工作量有界。        |
| 04   | [Style：独立窗口、modal 与 lifecycle](04-independent-window-modality-lifecycle.md)      | 使同一 FileDialog BSN 在独立 Window 中正确运行，并提供真正区分 modal/nonmodal 的输入与关闭语义。      |
| 05   | [Gallery：接入新 crate 并移除 rfd](05-gallery-migration-rfd-removal.md)                 | 把新库变成真实 Gallery 的唯一文件选择路径，完成 facade、依赖和旧 native 实现的迁移。                  |
| 06   | [GUI 前置：BRP 独立窗口 keyboard 支持](06-brp-secondary-window-keyboard.md)             | 解决当前 BRP v0.2.1 只能把 keyboard 送往 PrimaryWindow 的工具缺口。                                   |
| 07   | [GUI 测试：BRP 用户场景与 temporal 验收](07-brp-gui-acceptance.md)                      | 通过真实输入、截图和业务状态验证新 FileDialog，而不是只证明代码能编译或 ECS 最终值正确。              |
| 08   | [GUI 性能：150 ms 开窗与持续交互验收](08-latency-performance-acceptance.md)             | 证明实际输入到有效弹窗呈现的 latency 满足目标，并保护大目录、慢 I/O、频繁开关的资源与 frame 成本。    |

01–02 属于 headless；03–04 属于 style 与窗口组合；05 完成 Gallery 迁移；06 是完整独立窗口 keyboard GUI 测试的工具前置；07–08 完成 GUI 和性能验收。

06 涉及另一个仓库，需要单独实施或授权。每阶段的具体限制与验收都保存在对应文件中；本页只负责导航，不替代任何阶段正文。
