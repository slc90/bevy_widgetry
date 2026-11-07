# Cursor Integration Test Coverage

## States / Stimuli / Guards

- Default、Hover、Dragging、Native 控制权，System、外部静态与图集动画，Asset 等待/失败/恢复及每窗口真实时间相位。
- PointerInput、Hover Hits、Capture、Loading、祖先 Disabled、Theme 切换、Owner/Window 生命周期。
- 仅 Mouse 取得 OS Cursor/Lease；最近有效声明优先，同实体 EntityCursor 优于 Hint；Native Move/Resize 让权。

## Transitions / Invariants / Couplings

- Hover 继承与遮挡，Lease 取得/取消/Release/Blur/Despawn；无 Hover 时有效 Lease/Capture 保持。
- 窗口隔离，Owner 和 Window 失效不残留 Lease；Custom/Touch 不改变原生 Cursor。
- Loading 优先 Wait，Disabled 继承优先 NotAllowed，撤销后恢复最近声明。ReadOnly 默认 Cell，不附带编辑资格。
- 同帧不重复写 Cursor；System、显式 Custom、无 Mouse 位置、失焦、Native 操作和失败图集停止动画唤醒。
- 同值主题不重启，新语义/动画重新进入首帧；真实时间不受 Virtual 暂停或单帧上限影响。
- 共享图片像素故障复用资产诊断；动画声明尺寸故障独立，避免静态后备误报恢复。源头/实际后备/恢复按身份关联且只记录一次。

## Coverage Map

- `coordination.rs`：继承、遮挡、所有 Hint 系统后备、Loading/Disabled、窗口隔离、Lease/Capture/Native、Custom Pointer 隔离及两种安装顺序的 writer 冲突。冲突检查 Severity::Error、源头诊断及栈；System-only 不安装无用 Atlas。
- `static_theme.rs`：部分映射、整图/区域/热点、显式 Custom、无 Asset、queued 提交、多窗口、非法路径/定义与真实来源错误、像素及热点故障、恢复及停止唤醒。同图异热点具有不同原生缓存身份，切回复用原身份。外部路径冷加载在静止 Mouse 下持续至成功或失败，未使用语义不加载。
- `animation.rs`：小型合成 RGBA 图集的任意区域顺序、独立 Duration、帧边界/循环/极大时间、同值保持/换源重启、窗口进入相位、无 Hover 有效 Lease、Blur/Native/Cancel/System/Despawn 停止唤醒；非法定义、共享尺寸诊断、CPU/格式失败、静态/System 后备与恢复、真实路径惰性冷加载。
- `presentation/diagnostics` 单测：RGBA 格式/CPU 长度、独立资产与原因键、等待不恢复、真实 Loader 受管诊断集合保留原 source/origin。

全部库回归使用通用语义及合成素材，不依赖任何角色。真实45项角色素材、完整映射及对应生命周期覆盖属于 Gallery 的 `assets/cursors/tests.rs` 与 `cursor_theme/tests.rs`。

Headless 测试证明 ECS 请求与配置，不证明原生位图/桌面 Focus。OS 验收使用真实 Windows Mouse；同 Handle 位图热更新及缓存回收限制见功能文档。
