# Table 严格冻结当前 View 前 N 个显示列

Table 当前的 Body、Column Header、Row Header 与 Corner 组成四区布局，水平滚动同步列头，垂直滚动同步行号。本增量在此基础上让 Table View 能独立配置冻结前 N 个**显示位置**的列，而不按固定 `ColumnId` 名单冻结，也不修改 Model 的列定义。

## 冻结的稳定规则

每个 Table View 单独配置冻结列数 `N`，默认 `N = 0`。冻结区域始终是**该 View 当前显示顺序的最前 N 列**；如果未来用户拖拽改变列顺序，最前面的显示位置随之更新，冻结个数不变，不与原来某一组 `ColumnId` 永久绑定。冻结列保持固定在左侧，未冻结列沿 X 轴正常滚动；纵向滚动时两边行位置仍须同步。

**严格执行配置的 N**：窗口变窄、列宽加大、整个冻结区域超过 Table 可用宽度，都不自动减少有效冻结数量，不替调用方悄悄改变列宽。布局空间不足及相应裁剪展示由调用方重新设计视口尺寸、列宽和冻结配置。冻结数量只因真实列数不足而自然受限：有效数量 `min(N, 当前显示列数)`，保留原始配置 N；以后新增列，若总列数仍未达到 N，则新列继续归入冻结区，无须重新设置。

例如 View 显示 A/B/C/D，`N=2`，冻结 A、B；若其显示次序在列拖拽增量中变为 A/D/B/C，则冻结 A、D。假设 View 宽度 500px 而 A、D 共宽 600px，也仍然冻结 A、D；**不允许**因为可见宽度不足自动退化为冻结 A 或 0 列。此前讨论过的列隐藏本次不做，也不为了冻结列额外引入隐藏状态。

## 布局、滚动与虚拟化集成

现有 `crates/table/src/view.rs` 中 Body/ColumnHeaders 使用同一水平 ScrollPosition，同步算法在 `crates/table/src/projection.rs`。冻结后的几何计算需要让固定列与水平滚动列采用可区分的 X 坐标和剪裁语义，同时保证 Header 与 Body 在边界处对齐。`layout.rs` 的每列宽度、Flexible/Fixed 和最小宽度仍可用，冻结不改变列宽求解的业务配置。

二维可见性计算应单独保留冻结区域里落入可见 Y 范围的单元格，并计算非冻结区域的滚动 X 可见列；不能因为一个统一连续 X 切片排除了冻结列，或者因冻结区域常驻就物化它们的全部 Model 行。Row Header、Corner 的现有垂直/水平固定含义保留。列宽 Resize 与 Hover 仍应命中正确列，跨冻结边界的 Clip、Pointer Hit、UI z-order 需避免非冻结列绘制到冻结列前面。

## Gallery、文档与完成条件

`gallery/src/pages/table.rs` 增加可以观察冻结 `N=0`、`N=1`、`N>1` 和 `N>列数` 的展示/程序配置，并在宽度不足时展示严格冻结而非自动解除；通过横向滚动验证列头与 Cell 对齐，通过纵向滚动验证行列组合。共享 Model 两个 Table View 可以有不同的 N 和列宽；原有 `2000 × 40` 虚拟化示例可用于冻结列的离屏行场景。同步 `TableKey` 中英文说明。

`crates/table/tests/virtualization.rs`、`view.rs`、`interaction.rs`、`resize.rs` 的相应测试覆盖冻结侧常驻且离屏 Y 项不生成、左右区的 Hit 与 Clip、水平/垂直滚动同步、N 边界、宽度不足时配置不变、列增删、列宽调整与共享 Model View 隔离；回归原有 `0` 列/`0` 行、DPI/UiScale、Canvas 几何和稳定帧 Shell 复用。同步 `docs/crates/table.md`、`tests/COVERAGE.md`。
