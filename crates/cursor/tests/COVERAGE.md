# cursor Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Default / Hover / Dragging / Native 光标控制权。
- System / Kamuri、动画当前语义和图集就绪状态。
- 多 Native Window 与 Mouse / Custom / Touch。

## Stimuli

- 真实 PointerInput、Hover Hits、Disabled、Owner / Window 生命周期。
- Cursor Theme 切换、Asset 就绪 / 失败和受控动画时间前进。

## Guards

- 只有 Mouse 可以改变 OS Cursor / 获取 Lease。
- 最近有效声明及 Owner 优先；原生 Move / Resize 期间让权。

## Transitions

- Hover 继承与遮挡切换。
- Drag 取得 / 释放 Lease，取消 / 失焦 / Despawn 清理。
- Pointer / Wait 进入、保持、相互切换、退出，静态 / System Fallback。

## Invariants

- 不同 Window 隔离，失效 Lease 不残留。
- 静态不重复写入 Cursor 或请求 Redraw。
- Pointer / Wait Frame 原点和速率正确；丢失 Asset 时保留业务语义回退。
- 与 Bevy Cursor Writer 冲突时报告，不同时写入。
- Capture 优先于可见 Hit，Lease 校验所属按键和 Owner；Cursor 在 Last 阶段同帧消费，前方遮挡不泄漏后方声明。
- 34 种系统语义与 11 种 Hint 的静态图集项独立；动画与 Native、Blur、Disabled、Capture、Custom Pointer、Window 生命周期交错时停止或恢复正确语义。

## Couplings

- Loading 内部标记在后代命中与祖先 Disabled 下优先 Wait，撤销后恢复 NotAllowed / 最近声明。
- Disabled × 光标继承、Capture × Hover、Native × Lease。
- Kamuri Theme × Atlas Readiness / Animation / Fallback。

## Coverage Map

- `coordination.rs`：Hover、窗口隔离、Lease、Native 让权、动画、素材回退、安装冲突及静态性能不变量。
- ReadOnly 在 System/Kamuri 的独立索引、Cell 回退及素材恢复，最近声明/同实体官方声明优先、祖先 Disabled、Capture、Native Lease 和 Custom Pointer 隔离；稳定帧不重写 Cursor 或请求重绘。真实系统光标外观由 Gallery 的 Windows Mouse 验收补充。

## 覆盖边界

Headless 测试保护 ECS 请求与渲染配置，不证明系统级 Cursor 的真实视觉表现。

