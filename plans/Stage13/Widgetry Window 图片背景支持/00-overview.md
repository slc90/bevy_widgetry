# Widgetry Window 图片背景支持：拆分方案总览

## 整体目标

为 `crates/window` 增加正式的窗口背景配置能力，使所有 Widgetry Window 在构造时可以选择 Theme 纯色背景或图片背景；图片背景支持 `Stretch`、`Cover` 和构造期整体透明度 `opacity`。能力下沉到 Window crate，Gallery 只负责展示与运行时验证，不把实现留在 Demo 层。

全部小方案按唯一顺序执行：

1. [01-窗口背景API与渲染语义.md](01-窗口背景API与渲染语义.md)  
   先建立公开背景模型、构造函数签名、Theme/Image 互斥语义、图片透明度和 WindowRoot 绘制规则。后续所有工作都以这份契约为基础。
2. [02-Cover裁剪与运行时同步.md](02-Cover裁剪与运行时同步.md)  
   在图片背景基本能力成立后，实现 `Cover` 的中心裁剪、图片延迟加载、layout 后尺寸同步、零尺寸保护以及大图策略。
3. [03-调用方迁移与公共契约同步.md](03-调用方迁移与公共契约同步.md)  
   将新的强制 `background` 参数传播到现有调用方，保持 MessageBox 的 Theme 背景语义，并同步 Window crate 文档和 facade 边界。
4. [04-Gallery展示与资源接入.md](04-Gallery展示与资源接入.md)  
   接入 Gallery 专用测试图片，提供真实独立窗口对照入口，展示 Theme、Stretch、Cover 和 opacity 的视觉行为。
5. [05-测试验证与验收边界.md](05-测试验证与验收边界.md)  
   最后补齐 Theme、Stretch、Cover、opacity、延迟加载、resize、borrowed/owned、MessageBox 和 Gallery 手工验收，并锁定本轮非目标。

这五份方案形成单一路径：先定义能力和语义，再完成 Cover 的运行机制，再迁移现有消费者，然后接入 Gallery，最后统一验证。没有并行分支，也不把单个函数修改继续拆成任务级 TODO。
