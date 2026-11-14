# PathDisplay 测试覆盖模型

## 行为契约

- 输入为借用的 Windows 路径，只简化 VerbatimDisk 与 VerbatimUNC 的开头；其他命名空间和正文中的相似文字保持原样。
- 显示转换不规范化正文，不截断长路径，也不修改原始路径；保留大小写、Unicode、空格、分隔符、尾部分隔符和点组件。
- 非 Unicode UTF-16 路径使用替代字符进行有损显示，原始路径仍保留完整编码。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
| --- | --- |
| `display_path.rs` | 从公共 API 验证前缀选择、路径正文保留、无截断及非 Unicode 有损显示契约。 |

### Unit Test

无独立 Unit Test；转换契约通过外部公共 API 验证。
