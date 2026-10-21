# Slider 测试覆盖模型

`value_api.rs` 从 crate 外验证 BSN 构造、只读查询和 World / Commands 更新。

- 状态：连续 / 步进、默认 / 显式初始值、方向、Disabled。
- 刺激：BSN 创建、同步写入、延迟写入、命令执行前目标失效。
- 守卫：目标身份和完整状态、有限范围及跨度、正步长、有限值、范围、双端点可达性。
- 转移：真实变化、合法同值、无效写入、创建失败、命令执行时校验。
- 不变量：失败原子性；仅真实变化发事件；Observer 可读取已经提交的值；初始化不通知；失败的 Scene 不残留预约实体。
- 耦合：Disabled 不阻止程序化更新；Commands 可引用前序 Scene 命令尚未创建的 Slider；失败命令交宿主 Error Handler。
- 延迟 Scene 失败：spawn / apply 每次失败只记录一次诊断，保留 Error Severity；spawn 清理失败 Root，apply 保留既有 Root。

`input.rs` 通过真实 Bevy Picking Pipeline 和 KeyboardInput 派发验证输入组合：

- 水平 / 垂直、Mouse / Custom Pointer、连续 / 双端点步进。
- Thumb 抓取偏移不跳值、轨道立即更新并继续拖动、拖出轨道 Clamp、Release / Cancel 无重复事件。
- 相同正反向行程的稀疏 / 密集事件产生相同数值序列。
- Disabled、窗口失焦、Capture 丢失、Pointer Despawn、Hidden 清理手势。
- 焦点守卫、键盘重复、1% 连续步长、端点无事件、无 Home / End、大偏移下浮点步进仍通过 API 校验。
- Observer 在每次通知时读取到匹配的已提交值。
- 独立拖动生命周期：无移动 Start/Completed；Start 先于轨道值变更且其 Observer Commands 可先禁用；多次 Move 只一对边界；Release/DragEnd 任一先到只结算一次。
- Cancel、Disabled/祖先 Disabled、Display::None、Hidden、窗口失焦及 Pointer/Capture 丢失均报告 Cancelled，保留值；结束通知前 Pressed/捕获已清理。
- 非 Primary、非有限位置、已有捕获及 Disabled 拒绝 Press，无伪生命周期；键盘/API 只产生原有真实数值通知。
- 非零 Camera viewport 原点的 Thumb 抓取及轨道投影；未处理方向及 Disabled 按键继续冒泡给父节点。
- Visible 在 Slider 本体或中间容器上覆盖祖先 Hidden 后仍可指针 / 键盘交互；不覆盖祖先 Disabled / Display::None。

局部数值边界及浮点误差由 `value.rs` Unit Tests 补充，方向反转由 `gesture.rs` Unit Test 保护。

`style.rs` 从外部验证真实 BSN / UI Layout：

- 默认及自定义 Track / Thumb 尺寸；水平左右、垂直下上填充；端点及中间值投影。
- 运行期根主轴长度变化后 Thumb 大小不变、行程更新。
- DPI 1 / 1.5、UiScale 1 / 1.5 下，自定义 Thumb 尺寸、抓取偏移与输入行程一致。
- Scale 1.25 的非对称 padding 像素取整后，真实 Track 中点点击仍精确映射数值中点。
- 跨越极不同数量级的连续值域（如 -1e20..1），点击 Track 端点精确到达 max，避免浮点相消丢失端点。
- 四主题、局部颜色覆盖 / 清除、Hovered / Pressed / Focused / Disabled，以及状态切换几何保持。
- 非法尺寸构造失败和预约实体清理。
- 颜色 Commands 执行时校验及错误传播；失败不覆盖原有局部配色，清除恢复默认覆盖配置。
