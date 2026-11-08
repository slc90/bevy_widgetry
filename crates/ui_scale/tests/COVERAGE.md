# Ui Scale 覆盖模型

- 状态：应用缩放值、Window 的逻辑尺寸基准、普通/排除窗口、插件安装前已有/后建窗口。
- 刺激：初始资源、Scale 更新、相同值、用户物理 Resize、排除标记加入/移除、Window 移除后重建。
- 不变量：普通窗口采用当前统一 Scale，缩放保留逻辑设计范围，用户 Resize 成为下一次基准；排除窗口包括同批 Bundle 创建时不被改写，同值不标记 Window 变化。
- `scaling.rs` 验证真实 App/World 组合；后续 Windows DPI/工作区增强另增加相应覆盖。
- 运行期修改原生 Resize Constraints 立即约束尺寸，后续 Scale 使用新约束；放宽约束恢复未丢失的设计尺寸基准。
- `camera.rs` 使用 Bevy 真实 Camera System 与显式 Viewport，验证缩小比例时窗口与相机在同帧收敛，避免新输出纹理配旧 scissor。
- `validation.rs` 验证非有限/非正 Scale 的领域分类、请求值、状态保留及源头一次 ERROR；传播保留 origin，不虚报 handled，真实 Startup System 的错误以 Error Severity 到达宿主 Handler。有限正数不受 Gallery 四档限制。
