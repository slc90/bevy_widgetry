# bevy_widgetry 鼠标光标统一协调方案

## 1. 目标与边界

让 Widgetry 各交互区域显示符合桌面 GUI 语义的系统鼠标光标，解决重叠控件、禁用状态、拖拽和独立原生窗口之间的光标控制权冲突。

优先复用 Bevy 0.20 的 `EntityCursor`、`CursorIcon`、Picking 和调度接口，不重做 Picking、ZIndex、禁用传播或鼠标事件系统。

只管理实际 OS 鼠标 `PointerId::Mouse` 的光标；`Touch`、`Custom` 不产生系统鼠标光标。

窗口原生 Resize 的异常结束路径允许在 Gallery 验证时迭代；不提前引入 Windows 消息钩子。保留原有窗口尺寸调整功能，早期 `resize.rs` 中的光标代码不作为设计依据。

## 2. Crate 与依赖

新增 `crates/cursor`（包名 `bevy_widgetry_cursor`），暴露 `WidgetryCursorPlugin` 及必要的拖拽光标会话接口。

`bevy_widgetry_cursor` 只依赖 Bevy 官方类型，不依赖 `bevy_widgetry_core` 或任一具体控件 crate。

`bevy_widgetry_core` 依赖 `bevy_widgetry_cursor`，由现有 `WidgetryUiPlugin` 自动安装 `WidgetryCursorPlugin`，保证控件独立安装时仍获得功能。

`bevy_widgetry` 聚合 crate 添加依赖/导出公共入口，各控件仅需使用 Bevy 官方 `EntityCursor`，或在需要持续交互时依赖光标 crate 的会话接口。保持依赖图无环。

不可与 Bevy 官方 `CursorIconPlugin` 同时负责窗口光标：前者当前实现会把一个计算结果写到所有 Window，且与本插件存在双重写入风险。

Widgetry 的窗口 `CursorIcon` 只有新的协调器负责写入；迁移/移除 `crates/window/src/title_bar/resize.rs` 对该组件的直接写入。

## 3. 控件光标语义

| 区域 / 状态                                                                                                                       | SystemCursorIcon                                                                                |
| --------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Button、CheckBox、TriStateCheckBox、Radio、ComboBox 可操作区域、Tree 展开按钮、窗口标题栏按钮、MessageBox/FileDialog 中复用的按钮 | `Pointer`                                                                                       |
| 普通 TextField、允许选择/复制文字的 ReadOnly TextField                                                                            | `Text`                                                                                          |
| Table 的列宽调整命中区域与调整会话                                                                                                | `ColResize`                                                                                     |
| ScrollArea 滑块，悬停 / 正在拖拽                                                                                                  | `Grab` / `Grabbing`                                                                             |
| Window 八方向边缘/角落（未开始系统原生 Resize）                                                                                   | 对应 `NResize`、`NeResize`、`EResize`、`SeResize`、`SResize`、`SwResize`、`WResize`、`NwResize` |
| ListView 普通可选中行、Tree 普通节点、Table 普通单元格、FileDialog 文件/文件夹行、空白区域、普通标题栏                            | `Default`                                                                                       |
| 任何实际禁用的交互控件（包括被祖先禁用）                                                                                          | `NotAllowed`                                                                                    |

禁用后优先 `NotAllowed`，与官方 Feathers 一致；ReadOnly 本身不算 Disabled。

对明确的交互控件检查有效 `InteractionDisabled`，该标记由 core 已有的禁用状态投影机制维护。

对于仅是容器/遮挡层而没有声明交互光标的实体，不应仅因祖先禁用就使整个空白区域变成 `NotAllowed`。

控件把正常光标声明到命中实体或可继承的控件祖先上（`EntityCursor::System(...)`）；同一个控件中的子按钮可以覆盖行的 `Default`，不改写外部控件逻辑。

## 4. 统一决策与重叠命中

每个原生 `Window` 单独计算；只根据当前 `PointerLocation.target` 对应的窗口及其 Picking 结果操作。

不要把鼠标所在窗口的光标广播给所有窗口，也不要模拟多只 OS 鼠标。

对窗口实体仅在最终计算出的 `CursorIcon` 与当前值不同时写入，避免无意义 ECS 更新。

**光标控制权优先级：**

1. **Native**：正在执行操作系统原生 Window Move/Resize；Widgetry 暂停写光标，让出控制权。
2. **Dragging**：有效的当前窗口、鼠标与 owner 对应的控件拖拽会话；使用会话光标，与 Hover 无关。
3. **Hover**：根据 Bevy Picking 的最终有效命中目标（含 `Pickable` 阻挡与穿透），从最前方实体向其祖先寻找最近的 `EntityCursor`；对实际禁用的交互控件显示 `NotAllowed`。
4. **Default**：无有效候选时恢复普通箭头。

层级和遮挡顺序由 Bevy 的 `ZIndex` / `GlobalZIndex` → `UiStack` → UI Picking 负责，**cursor 不自行解释 ZIndex**。

注意 `HoverMap` 的命中实体集合是无序映射，不可直接取第一个 key；遇到穿透导致多个 Hover 目标时，要保留官方 `PointerHits` 的 camera/layer order 与 depth 信息来决定最前方，再筛选真实 Hover 候选。

若 Pointer Capture 生效，应遵从捕获结果，而非底层可见实体的命中。遮挡浮层不应泄漏下方控件光标。

## 5. 拖拽与原生窗口操作

提供仅表示**光标控制权**的临时会话（可命名 `CursorLease`），至少关联 `Window`、`PointerId::Mouse`、owner 实体、按键与光标类型。

不会自行捕获鼠标或重派发 Pointer 事件；控件已有 `PointerCaptureMap` 时可以复用其 Hover 结果，但没有捕获也可以显式保持拖拽光标。

**内部拖拽：**

- Table 列宽调整开始：获取 `ColResize` 会话；ScrollArea 滑块拖动开始：获取 `Grabbing` 会话。
- 结束：控件正常 Release/DragEnd、Cancel、失焦、目标被销毁/禁用、窗口失效时释放。
- 将未收到完整高级事件的情况交给原始 `PointerInput` 和当前实体有效性兜底清理；结束时按当前 Hover 重新决策。

**原生窗口操作：**

- Window native Resize/Move 开始时切入 `Native`，让 Winit/OS 接管。
- 不把 `PointerOut`、`WindowResized` 当作原生操作结束信号。
- 优先用有效释放、取消、失焦与恢复后的指针活动完成退出。
- Bevy 0.20 缺乏可靠的原生操作完成事件，剩余异常路径列为 Gallery 实测项；必要时依据实测结果做针对性调整。

会话只影响所属窗口，且必须校验 owner、窗口、鼠标及当前有效性，防止失效会话长期占用光标。

## 6. 调度与同帧更新

单一协调系统安排在 `Last` 中，并显式 `.before(CursorSystems::Update)`；不要把最终 `CursorIcon` 写入散落在 `PointerOver` / `PointerOut` 观察者中。

**实际依赖顺序：**

1. `PreUpdate`：core 禁用同步 → `PickingSystems::ProcessInput` → `Backend` → `Hover` → `PostHover` → `Last`，以及相关控件安排在 Picking 后的会话清理。
2. `PostUpdate`：Widgetry 构造/物化动态 UI、禁用投影与控件状态更新。
3. `Last`：Widgetry 统一解析有效鼠标光标，提交**每窗口**的 `CursorIcon` 修改。
4. `Last`：Bevy / Winit `CursorSystems::Update` 处理修改。

对有先后依赖的 deferred `Commands` 应明确调度/flush 边界，确保后端读取到新值；能直接修改已存在的窗口 `CursorIcon` 时优先避免不必要的 deferred 写入。

要求普通 Hover、Disabled 和拖拽会话变化不因自定义调度额外滞后一帧。

**真实边界：**当帧才创建或重新布局的 UI 可能尚未参与本帧 `PreUpdate` Picking，因此这类命中变化需等待下一轮 Picking；不能声称所有动态 UI 在创建当帧必然拥有正确的 Hover 目标。

## 7. 直接影响的实现位置

- `Cargo.toml` workspace、`crates/cursor/`、`crates/core/Cargo.toml`、`crates/core/src/ui.rs`：新 crate、注册及调度。
- `crates/bevy_widgetry` 公共聚合入口：导出使用方需要的 `WidgetryCursorPlugin` / 会话接口。
- 各基础控件的 Scene 构造：Button、Checkbox、Radio、ComboBox、TextField、ScrollArea、ListView/Tree、Table、Window；共享组合控件通过基础控件继承生效，避免 FileDialog/MessageBox 重复实现。
- `crates/table/src/resize.rs` 与 `crates/scroll_area/src/pointer.rs`：连接既有开始/结束/取消事件与光标会话。
- `crates/window/src/title_bar/resize.rs`：移除旧 `CursorIcon` 写入，设置方向 Hover 声明，接入 native 操作让权。
- 不改动 core 已有的 Hover/Disabled 状态计算职责与 widget 业务逻辑。

## 8. 验收与测试

**单元/集成测试：**

父子实体光标继承、`Default` 显式覆盖、带穿透的重叠命中不依赖哈希迭代顺序、Disabled/ReadOnly 切换、未拥有光标声明的遮挡层、多窗口隔离、拖拽离开目标、释放/取消/失焦/销毁/禁用、窗口关闭、无变化时避免重复写入、旧光标写入者已清除。

**Gallery 实测：**

Button/Checkbox/Radio/ComboBox/TextField；Tree 展开箭头与普通节点；ListView/Table/FileDialog 可选中行；Table 列宽拖动出边界；ScrollArea 悬停 Grab / 拖动 Grabbing；独立双窗口切换；Modal/ComboBox 浮层遮挡；祖先 Disabled；八方向原生 Window Resize，特别检查 PointerOut、Windows 原生拖拽释放信号缺失及焦点恢复。

测试中发现的原生 Resize 结束问题必须补全并重新验证，但不以对旧 `resize.rs` 光标实现的兼容为目标。

## 9. 官方参考

- [Bevy 0.20 `EntityCursor` 与 `CursorIconPlugin`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_picking/src/cursor.rs)
- [Bevy 0.20 Picking 与调度](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_picking/src/lib.rs)
- [Bevy 0.20 UI Picking](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/picking_backend.rs)
- [Bevy 0.20 `CursorSystems::Update`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_window/src/cursor/mod.rs)
- [Bevy 0.20 Windows/Winit 光标后端](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_winit/src/cursor/mod.rs)
