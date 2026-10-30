# 修复 Page Lifecycle Benchmark 的 Gallery 导航

## 目标

修复 `gallery/benches/page_lifecycle.py` 对旧版页面顺序和固定屏幕坐标的依赖，使它能够适应当前 Gallery 的可滚动侧边栏。

只恢复现有 Benchmark 的可靠性，不扩大测量范围，也不改变 Gallery 页面功能。

## 修改范围

涉及：

- `gallery/benches/page_lifecycle.py`
- `gallery/benches/page_lifecycle.md`

### 导航机制

使用 Gallery 已有的 `ButtonNav`、`ListViewNav`、`TreeNav`、`TableNav`、`WaveformNav` 等稳定 `Name` 定位导航节点。

通过 BRP 查询目标节点的实际几何位置和可视区域。

如果导航项在可视区域之外，先通过 BRP 滚动侧边栏，等待布局更新后重新查询位置，再执行真实鼠标点击。

移除使用旧版 `PAGES.index(page)` 计算固定 Y 坐标的方式。不能直接修改 ECS 页面状态来代替真实 GUI 操作。

点击完成后，通过对应页面的 `*Page` 节点确认实际切换结果。

### 生命周期检查

核对现有脚本中的页面名称、`SOURCES` 资源类型路径、页面 Root 与 Header 的定位是否仍然正确。

保留以下检查：

- 重复点击同一页面时，Page Root 身份稳定。
- 离开页面后，旧 Page Root、对应 Resource 及其持有的 Model/Tree Roots 已释放。
- Gallery Header 身份保持稳定。
- `waveform_sources` 只在 Waveform 页面存在。
- `active_entities`、`main_update_ms` 等现有指标不变。

保留原有测试数据规模，包括 ListView 的 4/10000/20/20、Tree 的 10000 File 和 Table 的 2000×40 虚拟化数据。

### 文档与验收

更新 `page_lifecycle.md` 中的导航方式与执行说明。

保留 Windows 64 位、PowerShell 7 环境，以及默认 `--pages ListView --warmup 3 --cycles 20` 配置。

保留单次同步导航 250ms CPU 预算，并明确该预算不替代持续交互的 16.7ms 帧预算。

测量仍不包含 CSV 写入、独立 Render Thread/GPU、Winit idle 和 BRP 往返耗时，不能解释为输入到实际显示的延迟。

验收时通过 BRP 执行原有测量场景，确认可见及需要滚动的导航按钮均可正确点击，生命周期断言全部通过，CSV 数据有效，原有性能预算未改变。

结束时释放 Custom Pointer，并通过 BRP 正常关闭 Gallery。

## 边界

不修改 Gallery 业务逻辑，不重构 BRP，不增加新的 Benchmark 场景或指标。
