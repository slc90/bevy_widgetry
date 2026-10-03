# 实现 Cover 中心裁剪与运行时同步

## 目标

在已经存在的 Image Window 背景模型上实现 `WidgetryWindowImageMode::Cover`：保持原图宽高比、始终铺满整个 WindowRoot、超出部分从中心裁掉，并在窗口 resize 后自动根据最终 UI layout 更新采样区域。

## 范围

本方案负责：

- `Cover` 的精确裁剪公式；
- 使用 `ImageNode.rect` 表达源图裁剪区域；
- 图片异步加载后的首次尺寸获取；
- `ComputedNode` 变化后的裁剪更新；
- system 的 layout 后执行约束；
- 零尺寸保护；
- 运行时状态最小化；
- Cover 与 opacity 的组合语义；
- resize 性能和大图策略。

本方案不改变公开背景类型，也不处理 Gallery 或 MessageBox 调用方。

## 预期产出

`Cover` Window 在任意有效窗口尺寸下都能用同一张图片覆盖完整 WindowRoot，保持原图比例并中心裁剪；图片尚未加载时透明，加载后自动进入正常 Cover；resize 只做轻量裁剪计算，不引入额外 resize 事件链或纹理重建。

## 与前后方案的关系

本方案依赖前一方案已经定义 Image Window、`ImageNode`、`WidgetryWindowImageMode::Cover` 和构造期 opacity。完成后，后续调用方迁移和 Gallery 可以直接消费一个完整可用的 Cover 能力。

---

## Cover 的基本表达

Cover 仍然使用：

```rust
NodeImageMode::Stretch
```

但不让它采样整张原图，而是通过：

```rust
ImageNode.rect = Some(...)
```

选择原图中央的一块区域，使这块区域的宽高比与当前 `WindowRoot` 一致。

随后 `Stretch` 只把这块已经按比例裁好的区域映射到整个 WindowRoot。

最终语义是：

```text
保持原图宽高比例
+ 始终铺满窗口
+ 超出的原图部分裁掉
+ 固定从中心裁剪
```

第一版不提供：

- top / bottom / left / right alignment；
- focal point；
- 自定义 crop anchor。

## 裁剪算法

已知：

```text
image_width
image_height
node_width
node_height
```

计算：

```rust
let image_aspect = image_width / image_height;
let node_aspect = node_width / node_height;
```

### 原图比窗口更宽

当：

```text
image_aspect > node_aspect
```

保留完整高度，水平居中裁剪：

```rust
crop_height = image_height;
crop_width = image_height * node_aspect;

x = (image_width - crop_width) / 2.0;
y = 0.0;
```

得到：

```rust
Rect {
    min: Vec2::new(x, 0.0),
    max: Vec2::new(x + crop_width, image_height),
}
```

### 原图比窗口更窄或更高

当：

```text
image_aspect <= node_aspect
```

保留完整宽度，垂直居中裁剪：

```rust
crop_width = image_width;
crop_height = image_width / node_aspect;

x = 0.0;
y = (image_height - crop_height) / 2.0;
```

得到对应 `Rect`。

### 示例

原图：

```text
1000 × 800
```

窗口：

```text
1920 × 1080
```

窗口比例是 `16:9`，Cover 选择原图中央：

```text
1000 × 562.5
```

上下各裁：

```text
118.75 px
```

最后：

```text
1000 × 562.5
→ 1920 × 1080
```

窗口内部始终被图片完整覆盖，不会出现因为保持比例而留下的空白区域。

## Cover 内部状态

公开配置只负责构造，运行时不继续保留一个可任意修改的公开背景 config component。

Cover 只需要一个私有状态组件，例如：

```rust
#[derive(Component)]
struct CoverWindowBackground {
    image_size: Option<UVec2>,
}
```

不需要另外缓存 image handle，因为 `ImageNode` 本身已经持有它。

也不需要保存：

```rust
last_node_size
```

窗口最终布局尺寸直接从 `ComputedNode` 获取，尺寸是否发生变化使用 ECS change detection 判断。

## 图片延迟加载

图片 asset 是异步的。窗口构造时已经可以得到 `Handle<Image>`，但 `Assets<Image>` 不保证立刻存在对应资源。

当：

```rust
image_size == None
```

Cover 同步系统尝试：

```rust
images.get(&image_node.image)
```

如果图片尚未加载：

```text
不做任何处理
→ Image WindowRoot 没有 BackgroundColor
→ 背景保持透明
```

不报错，不切回 Theme，也不创建临时 fallback。

图片第一次可用后：

```text
读取原图尺寸
→ 缓存 image_size
→ 根据当前 ComputedNode.size 立即计算 Cover rect
```

原图固有尺寸只需要取得一次。正常窗口 resize 不重复依赖 asset 尺寸查询。

本轮不为 asset hot reload 或运行时替换图片定义尺寸变化契约，因为运行时替换图片本身不在本次支持范围内。

## resize 后同步

Cover 不监听 native `WindowResized`，也不把 native window 尺寸当作裁剪 authority。

依赖链保持为：

```text
native window resize
→ WindowRoot layout 改变
→ ComputedNode.size 改变
→ Cover 更新 ImageNode.rect
```

真正决定背景采样区域的是最终 UI layout 尺寸，而不是一份额外维护的 window-size cache。

系统处理逻辑为：

```text
for each Cover WindowRoot:

    if image_size 未取得:
        尝试从 Assets<Image> 获取
        失败 → continue
        成功 → 缓存尺寸，并要求本帧更新 rect

    if image 首次就绪
       或 ComputedNode 本帧发生变化:
        根据 image_size + ComputedNode.size
        重算居中 Cover rect
        写入 ImageNode.rect
```

窗口尺寸没有变化时，不重复计算 crop rect。

## system ordering

Cover 同步必须读取已经完成本帧 layout 的 `ComputedNode.size`，因此 system 放在 `PostUpdate`，并明确运行在 UI layout 之后。

可以放入：

```rust
UiSystems::PostLayout
```

或者使用等价的显式 ordering，只要能保证读取的是本帧最终 layout 结果。

不要在 layout 之前计算，然后再自己猜 WindowRoot 的最终像素尺寸。

## 零尺寸与无效尺寸

布局初始化、窗口最小化或中间状态可能产生零尺寸。

如果：

```text
node_width <= 0
或
node_height <= 0
或
image_width == 0
或
image_height == 0
```

本轮不生成新的 crop rect。

节点恢复到有效尺寸后，由后续 layout 变化重新计算。

helper 自身必须避免产生 NaN / Infinity，也不因为这些正常中间状态 panic。

## Cover 与 opacity

`opacity` 只通过 `ImageNode.color` 作用于最终图片 alpha，不参与裁剪公式。

同一张 Cover 图片可以是：

```text
opacity = 1.0
→ 完全不透明地中心裁剪并铺满

opacity = 0.5
→ 使用完全相同的 crop rect，但整张绘制结果半透明
```

由于 Image 模式没有 `BackgroundColor`，降低 opacity 后透出的是 native window 后面的桌面或其他窗口，而不是 Theme 色。

## 性能边界

Cover resize 时每次真正做的只是少量浮点乘除与一次 `Rect` 更新。

即使用户持续拖动窗口，窗口本身也已经在进行 UI layout、UI 顶点更新和重绘，Cover 增加的计算量很小。

不为此引入：

- 独立 resize event 转发；
- `AssetEvent<Image>` 流程；
- background child 尺寸同步；
- 每窗口 observer；
- 自己维护的 `last_node_size`；
- 复杂缓存系统。

## 大图策略

Widgetry Window 不限制调用方传入图片的分辨率，也不自动 downscale。

库只消费：

```rust
Handle<Image>
```

图片尺寸、格式、压缩和 GPU 内存成本属于应用资产策略。

窗口 resize 不会生成一张新的、按窗口尺寸缩放后的纹理。无论 `Stretch` 还是 `Cover`，GPU 都继续采样原有 texture；resize 只改变绘制几何和采样区域。

因此大图的主要风险是原始 texture 的 CPU/GPU 内存占用与设备纹理尺寸上限，而不是 Cover 的四则运算。

本轮不增加：

- 图片最大尺寸限制；
- 自动降采样；
- 自动生成窗口分辨率副本；
- mipmap 策略；
- 纹理缓存策略。
