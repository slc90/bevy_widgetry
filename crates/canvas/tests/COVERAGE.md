# Canvas Integration Test Coverage

## Model

`model.rs` 覆盖没有 UI/渲染插件的纯 World 模型：多个矩形共享 Document、负坐标、原生 Transform 更新、无填充与透明填充语义、更新与移除、Document 的 ChildOf 级联清理。

非法尺寸、Paint、Transform、无效 Document、已删除图形和跨 Document 请求应返回 Error Severity 与必要诊断，不创建额外 Entity，不修改旧几何与 Paint。纯几何测试覆盖居中描边、旋转及非均匀整体 Scale 的 Document AABB 和四角；局部尺寸下限为 1 单位。
