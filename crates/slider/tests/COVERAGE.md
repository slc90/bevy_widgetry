# Slider 测试覆盖模型

`value_api.rs` 从 crate 外验证 BSN 构造、只读查询和 World / Commands 更新。

- 状态：连续 / 步进、默认 / 显式初始值、方向、Disabled。
- 刺激：BSN 创建、同步写入、延迟写入、命令执行前目标失效。
- 守卫：目标身份和完整状态、有限范围及跨度、正步长、有限值、范围、双端点可达性。
- 转移：真实变化、合法同值、无效写入、创建失败、命令执行时校验。
- 不变量：失败原子性；仅真实变化发事件；Observer 可读取已经提交的值；初始化不通知；失败的 Scene 不残留预约实体。
- 耦合：Disabled 不阻止程序化更新；Commands 可引用前序 Scene 命令尚未创建的 Slider；失败命令交宿主 Error Handler。
- 延迟 Scene 失败：spawn / apply 每次失败只记录一次诊断，保留 Error Severity；spawn 清理失败 Root，apply 保留既有 Root。

局部数值边界及浮点误差由 `value.rs` Unit Tests 补充。输入与视觉尚未接入。
