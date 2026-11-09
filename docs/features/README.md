# 功能目录

按需要的能力查找。每篇的顶层章节对应功能，子章节展开创建、修改、查询、通知和具体配置。

## 入口与内容基础

| 文档 | 能力 |
| --- | --- |
| [统一入口](bevy_widgetry.md) | 依赖、Feature 与插件装配 |
| [Core](core.md) | 文本、字体、Icon、Disabled 与 Scene 适配 |
| [内建资源](asset.md) | 字体、图标与 Shader 语义资源 |
| [颜色主题](theme.md) | 预设、运行时切换与自定义颜色 |
| [光标](cursor.md) | 系统、静态与动画光标主题 |

## 按钮与选择

| 文档 | 能力 |
| --- | --- |
| [Button](button.md) | 激活、Loading、链接与自定义外观 |
| [HoldButton](hold_button.md) | 长按确认与充能反馈 |
| [ToggleButton](toggle_button.md) | 二态切换和已提交变化通知 |
| [CheckBox](check_box.md) | 二态和三态选择 |
| [RadioGroup](radio_group.md) | 互斥选项与动态成员 |
| [Segmented](segmented.md) | 分段单选、方向输入与布局 |
| [DatePicker](date_picker.md) | 日期选择、月份浏览与导航 |

## 输入与布局

| 文档 | 能力 |
| --- | --- |
| [TextField](text_field.md) | 文本编辑、只读与内容槽位 |
| [NumberInput](number_input.md) | 原类型数值、草稿提交与步进 |
| [Slider](slider.md) | 数值调整、拖拽与交互策略 |
| [ProgressBar](progress_bar.md) | 确定和不确定进度展示 |
| [Splitter](splitter.md) | 双区域布局、比例和拖动 |
| [Tabs](tabs.md) | 标签页选择、内容和滚动 |
| [Collapsible](collapsible.md) | 展开、折叠和标题内容 |
| [ScrollArea](scroll_area.md) | 横向、纵向和双轴滚动 |
| [Tooltip](tooltip.md) | 悬停提示与自定义内容 |

## 数据视图

| 文档 | 能力 |
| --- | --- |
| [ListView](list_view.md) | 泛型列表、选择、投影和排序 |
| [ComboBox](combo_box.md) | 选择、弹层、搜索与自定义部件 |
| [Tree](tree.md) | 层级展示、懒加载与节点移动 |
| [Table](table.md) | 二维表格、冻结列、编辑和复制 |
| [Canvas](canvas.md) | 图形、视图变换、编辑与导出 |
| [Waveform](waveform.md) | 实时与回放波形、降采样和控制 |
| [磁盘波形源](waveform_file.md) | 索引记录构建与后台局部读取 |

## 窗口与桌面能力

| 文档 | 能力 |
| --- | --- |
| [Window](window.md) | 自绘窗口、Modal 与标题 |
| [MessageBox](message_box.md) | 非阻塞确认与结果事件 |
| [FileDialog](file_dialog.md) | 文件选择、导航、过滤和保存路径 |
| [窗口缩放](ui_scale.md) | 统一应用缩放与排除窗口 |
| [Splash](splash.md) | 启动呈现与主窗交接 |
| [Tray](tray.md) | 托盘图标、菜单与消息 |
| [电源抑制](power_inhibit.md) | 空闲息屏与休眠请求开关 |

## 任务、诊断与工程辅助

| 文档 | 能力 |
| --- | --- |
| [后台任务](job.md) | 提交、结果、取消与全局关闭 |
| [诊断](diagnostics.md) | 领域错误、上下文和 ECS 适配 |
| [内部日志](internal_log.md) | 统一 Target 与结构化记录 |
| [应用日志](app_logging.md) | 终端、文件输出与显式刷新 |
| [EXE 图标](app_icon_build.md) | 构建期嵌入应用 ICO |
| [测试辅助](test_utils.md) | 受控 App、输入与错误捕获 |

[整体设计](../design/architecture.md) · [模块设计](../design/modules/README.md) · [文档入口](../README.md)
