# Stage 2 目标 3：StyledButton 测试总结

## 目标

Stage 2 的目标 3 是为 `StyledButton` 建立一套可持续使用的测试体系，重点分为两部分：

1. 样式逻辑测试：验证控件状态到最终样式组件之间的映射是否正确。
2. Visual Regression：验证 `StyledButton` 在真实 Bevy UI 渲染链路中的最终像素结果是否发生意外变化。

当前两部分都已经跑通。

---

## 一、样式逻辑测试

### 1. Resolver 单元测试

首先对状态解析逻辑做纯函数级测试。

核心状态优先级为：

```text
Disabled > Pressed > Hovered > Default
```

这类测试直接验证输入状态与输出颜色之间的关系，不依赖 ECS 调度。

作用是快速确认最核心的样式决策逻辑没有回归。

### 2. ECS 更新集成测试

随后验证状态组件变化后，Bevy system 是否会真正更新 `StyledButton` 的样式组件。

重点覆盖：

- `Hovered` 的 `Changed<T>` 路径
- `Pressed` 的 `Added<T>` / `RemovedComponents<T>` 路径
- `InteractionDisabled` 的 `Added<T>` / `RemovedComponents<T>` 路径
- 多状态同时存在时仍遵守既定优先级

这里也确认了一个重要点：

```text
Added<T> 在概念上属于 Changed<T> 的子集
```

而 `Pressed`、`InteractionDisabled` 这类“存在即代表状态”的组件，更适合使用 `Added` / `RemovedComponents` 来监听状态进入与退出。

### 3. ForegroundColor 传播测试

`StyledButton` 的前景色不是直接写到所有子节点，而是通过 `Propagate<ForegroundColor>` 沿层级传播，再由适配 system 把它应用到 `TextColor`。

测试验证了：

```text
StyledButton
→ Propagate<ForegroundColor>
→ hierarchy propagation
→ child ForegroundColor
→ TextColor
```

因此可以确认前景色传播链路在 ECS 层是可工作的。

---

## 二、Visual Regression 的目标

逻辑测试只能验证：

```text
状态 → ECS 样式组件
```

但不能验证真实渲染结果，例如：

- 尺寸是否变化
- 位置是否偏移
- 边框是否变化
- 渲染结果是否与预期一致
- 多个状态在真实 UI pipeline 中是否正确显示

因此增加了真正走 Bevy Renderer 的 offscreen visual regression。

最终链路为：

```text
StyledButton scene
→ Bevy UI layout
→ GPU render
→ RenderTarget::Image
→ GPU readback
→ CPU pixels
→ 与 baseline PNG 比较
```

---

## 三、Headless / Offscreen 渲染方案

测试不创建真实窗口，也不依赖显示服务器。

核心配置为：

```rust
DefaultPlugins
    .set(WindowPlugin {
        primary_window: None,
        exit_condition: ExitCondition::DontExit,
        ..default()
    })
    .disable::<WinitPlugin>()
    .disable::<PipelinedRenderingPlugin>()
```

这里的“headless”并不是没有 GPU renderer，而是：

```text
无 Window
无 Winit event loop
仍然使用真实 Bevy GPU renderer
```

之所以禁用 `PipelinedRenderingPlugin`，是因为它会在 `cleanup()` 阶段把 `RenderApp` 从主 `App` 中取出并移交给独立渲染线程。

Visual Test 需要直接访问 `RenderApp` 的 `PipelineCache`，因此测试里保留非 pipelined rendering 更简单、确定。

---

## 四、Plugin 生命周期

直接创建 `App` 后调用 `app.update()` 曾出现：

```text
DeviceErrorHandler does not exist in the World
```

原因是 renderer 的部分资源并不是在 `Plugin::build()` 阶段就全部可用，而是在 plugin finish 生命周期中完成初始化。

测试中需要手动完成：

```rust
while app.plugins_state() == PluginsState::Adding {
    bevy::tasks::tick_global_task_pools_on_main_thread();
}

app.finish();
app.cleanup();
```

之后才能安全进入渲染更新流程。

这也说明：

```text
add_plugins() != plugin 已完整初始化
```

普通 ECS 测试此前不需要这一步，是因为它们依赖的系统和资源都在 `build()` 阶段已经完成注册。

---

## 五、Render Target

Visual Test 使用一个 GPU `Image` 作为相机输出目标。

当前尺寸统一为：

```rust
const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
```

并通过：

```rust
RenderTarget::Image(image_handle.clone().into())
```

让 `Camera2d` 把 UI 渲染到纹理，而不是窗口。

纹理需要包含以下 usage：

```rust
TextureUsages::TEXTURE_BINDING
    | TextureUsages::COPY_DST
    | TextureUsages::COPY_SRC
    | TextureUsages::RENDER_ATTACHMENT
```

其中 `COPY_SRC` 是之后 GPU readback 所必需的。

---

## 六、UI Pipeline Warm-up

不能在 scene 建好后立即假定第一帧一定能稳定渲染 UI。

因此在正式 readback 前先让 renderer 发现并编译相关 pipeline。

流程为：

```text
spawn scene
→ app.update()
→ 检查 RenderApp::PipelineCache
→ waiting_pipelines() 为空
→ 再开始 readback
```

这里 `waiting_pipelines() == 0` 代表当前没有仍在等待创建的 render pipeline。

它不是“像素一定正确”的数学证明，但比固定写死“先跑 N 帧”更可靠。

---

## 七、GPU Readback

Bevy 0.19.1 使用：

```rust
Readback::texture(image_handle.clone())
```

并监听：

```rust
ReadbackComplete
```

把 GPU texture 内容取回 CPU。

共享结果使用：

```rust
Arc<Mutex<Option<Vec<u8>>>>
```

含义分别为：

- `Arc`：observer 与测试主流程共享同一份状态
- `Mutex`：保证可安全修改
- `Option`：区分“结果尚未回来”和“结果已经回来”

读取完成后通过 `Option::take()` 把 `Vec<u8>` 所有权取出。

---

## 八、One-shot Readback

`Readback` component 如果一直保留，会尝试每帧发起新的 readback。

最初因此出现：

```text
Failed to send readback result: sending into a closed channel
```

原因是第一份结果回来之前，已经提交了多份 GPU readback 请求。

最终方案是：

```text
spawn Readback
→ app.update() 提交唯一一次 readback
→ remove::<Readback>()
→ 继续 app.update() 等 ReadbackComplete
```

这样只会存在一个 in-flight readback，请求完成后测试结束也不会残留后续异步任务。

---

## 九、Readback 数据格式

当前 render target 使用：

```rust
TextureFormat::Bgra8UnormSrgb
```

因此 GPU readback 得到的是一维：

```rust
Vec<u8>
```

每个像素为：

```text
[B, G, R, A]
```

`Vec<u8>` 本身没有 width / height / shape 信息，图像形状由外部的：

```text
WIDTH × HEIGHT × 4 bytes
```

共同解释。

当前：

```text
256 × 256 × 4
```

因此会先检查 readback 总长度是否符合预期。

---

## 十、Sanity Check

在正式 baseline 比较前，曾用两个像素验证整个链路：

- `(0, 0)`：背景
- `(WIDTH / 2, HEIGHT / 2)`：按钮内部

当时得到不同颜色，证明真实 UI 内容已经经过 GPU render 并成功 readback。

这只是开发阶段的临时 sanity check。

正式 visual regression 建立以后不再需要每次保留这类局部像素判断，只保留 readback 长度检查即可。

---

## 十一、Baseline（金标准）

Visual Regression 使用一张 PNG 作为金标准：

```text
tests/baselines/styled_button.png
```

baseline 不是由普通测试自动更新，而是通过单独的 ignored test 生成。

建议结构：

```text
tests/
├── visual_test.rs
├── generate_baseline.rs
└── baselines/
    └── styled_button.png
```

`generate_baseline.rs` 中：

```rust
#[test]
#[ignore]
fn generate_styled_button_baseline() {
    ...
}
```

平时执行：

```bash
cargo test
```

不会覆盖金标准。

只有明确需要建立或更新 baseline 时才运行：

```bash
cargo test --test generate_baseline -- --ignored
```

这可以避免 visual regression 失败时顺手把错误结果也覆盖成新的金标准。

---

## 十二、Baseline 生成方式

生成 baseline 时不需要手写 `Readback + Arc<Mutex<_>>`。

直接使用 Bevy 自带：

```rust
Screenshot::image(image_handle)
    + save_to_disk(...)
```

因此 baseline 流程为：

```text
GPU render
→ Bevy Screenshot
→ PNG
```

GPU → CPU 的物理过程仍然存在，只是由 Bevy screenshot 机制负责管理。

---

## 十三、四状态统一 Baseline

没有为每个状态维护单独 PNG，而是在同一个 offscreen scene 中一次渲染四个按钮。

从上到下分别为：

```text
Default
Hovered
Pressed
Disabled
```

布局使用垂直 Flex：

```rust
flex_direction: FlexDirection::Column,
justify_content: JustifyContent::Center,
align_items: AlignItems::Center,
row_gap: px(8),
```

状态直接通过 ECS component 构造，而不是模拟鼠标输入：

```rust
StyledButton                         // Default
StyledButton + Hovered(true)        // Hover
StyledButton + Hovered(true) + Pressed
StyledButton + InteractionDisabled  // Disabled
```

因为交互行为本身已经由其他测试覆盖，Visual Regression 只负责验证已知状态对应的最终渲染结果。

这样只需要维护一张：

```text
styled_button.png
```

即可覆盖四个视觉状态。

---

## 十四、Baseline 与实际结果统一为 RGBA

GPU readback 原始格式是 BGRA，而 PNG 解码后通常使用 RGBA。

因此比较前先把两边统一成 RGBA。

当前实际结果：

```text
BGRA Vec<u8>
→ Image
→ try_into_dynamic()
→ RGBA
```

baseline：

```text
PNG bytes
→ Image::from_buffer(...)
→ try_into_dynamic()
→ RGBA
```

重点不是“必须使用 RGBA”，而是：

```text
比较前双方必须采用相同的像素格式和通道顺序
```

选择 RGBA 只是因为 PNG / `DynamicImage` 这一侧更自然。

---

## 十五、当前比较策略

目前采用最严格的全图逐字节比较：

```rust
assert_eq!(actual.dimensions(), baseline.dimensions());
assert_eq!(actual.as_raw(), baseline.as_raw());
```

也就是说：

```text
任何一个 RGBA byte 不一致
→ Visual Regression 失败
```

对于当前主要由纯色矩形组成的 `StyledButton`，这种严格比较最简单，也最容易发现变化。

以后如果加入文字、抗锯齿或更复杂的 GPU 渲染效果，再根据实际跨平台稳定性决定是否需要像素容差。

---

## 十六、`Pressed` 同名 API

开发时发现 rust-analyzer 会给出很多名为 `Pressed` 的候选，例如：

```text
ButtonState::Pressed
FocusCause::Pressed
PickingInteraction::Pressed
PressDirection::Pressed
Interaction::Pressed
bevy::ui::Pressed
```

它们只是名字相同，并不一定是同一个类型。

当前 StyledButton 状态所需的是：

```rust
bevy::ui::Pressed
```

它是一个 ECS Component，可以直接挂到 entity 上。

而像：

```rust
Interaction::Pressed
```

则是 enum variant，不是当前要使用的状态 component。

实际开发中不需要背所有路径，只需要先判断当前语义需要的是 Component、enum variant，还是其他类型。

---

## 十七、当前完成状态

Stage 2 目标 3 当前已经完成：

```text
StyledButton 测试
├── Resolver 单元测试            ✅
├── ECS 状态更新集成测试          ✅
├── ForegroundColor 传播测试      ✅
└── Visual Regression            ✅
    ├── headless / offscreen      ✅
    ├── GPU renderer              ✅
    ├── pipeline warm-up          ✅
    ├── one-shot GPU readback     ✅
    ├── 独立 baseline 生成        ✅
    ├── 四状态统一 baseline       ✅
    └── 全图逐字节比较            ✅
```

至此，Stage 2 目标 3 的测试体系已经形成完整闭环。
