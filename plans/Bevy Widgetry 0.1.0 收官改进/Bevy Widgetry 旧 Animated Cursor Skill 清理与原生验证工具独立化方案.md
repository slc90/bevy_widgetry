# Bevy Widgetry 旧 Animated Cursor Skill 清理与原生验证工具独立化方案

## 一、目标与范围

现有 `animated-cursor-authoring` Skill 已由独立的 **Character Cursor Atelier** 取代，不再需要保留在 Bevy Widgetry 仓库中。

本次工作删除旧 Skill，并解除仓库辅助工具对它的依赖，同时完整保留 Gallery 的 Windows 原生 OS 光标验证能力。

最终要求：

- Widgetry 仓库不再包含旧 Skill，也不依赖外部安装的 Skill。
- Gallery 仍然能够独立执行真实 OS 光标捕获、动画帧匹配及热点验证。
- Kamuri 素材生成流程和现有运行时行为不变。
- 不修改任何 `plans/` 历史方案。

## 二、实施内容

### 1. 删除旧 Skill

删除整个目录：

`skills/animated-cursor-authoring/`

包括其中的：

- `SKILL.md`
- `references/`
- `scripts/`

不在 Widgetry 仓库中复制或安装新的 Character Cursor Atelier。

删除后，如仓库根目录的 `skills/` 已为空，一并移除该目录。

### 2. 独立化 Gallery 原生光标验证工具

目标文件：

`crates/asset/tools/native_cursor_probe.py`

#### 现有问题

当前脚本只是一个转发入口，通过 `runpy.run_path()` 执行：

`skills/animated-cursor-authoring/scripts/native_windows.py`

旧 Skill 删除后，该路径失效，原生验证工具无法继续工作。

#### 修改方案

将原 `native_windows.py` 中与**现有应用 OS 光标捕获和验证**有关的实现迁移到 `native_cursor_probe.py`，使其成为无需依赖旧 Skill 的独立 Python 工具。

保留以下功能：

1. 使用 Windows API 获取当前真实 OS Cursor。
2. 读取光标 RGBA 位图、尺寸、Hotspot 和可见状态。
3. 根据窗口标题定位已有 Gallery 原生窗口。
4. 按目标配置中的物理 Client 坐标定位真实鼠标。
5. 捕获指定语义的连续动画帧和实际采样时间。
6. 将实际捕获位图与预期 PNG Atlas 帧进行匹配。
7. 验证尺寸、Hotspot、可见状态及动画帧变化。
8. 保存原生捕获结果、APNG 动画预览和 Motion Strip。
9. 支持将目标窗口临时置于前台，并在操作结束或异常时清理临时状态。

保留现有应用验证所需的命令行参数：

- `--title`：目标窗口标题。
- `--targets`：待验证光标及对应坐标的 JSON 文件。
- `--foreground`：是否临时置前目标窗口。
- `--output`：验证结果输出目录。

继续使用现有 Target JSON 结构及相对路径解析方式，避免原有验证输入需要重新制作。

保留现有输出文件及主要字段约定，包括：

- `native.json`
- `native.apng`
- `native-contact.png`
- 源帧匹配结果、尺寸、Hotspot 和采样信息。

#### 删除不需要的功能

不再保留与 Gallery 验证无关的通用能力：

- 临时原生窗口样本播放器。
- `--samples` 模式。
- 不指定 Target 的通用光标观察模式及其专属参数。
- 仅服务于上述模式的代码和命令行分支。

原生光标验证工具只负责检查**已有应用中的真实 OS Cursor**，不承担动画素材制作、图集生成或独立样本播放。

#### 实现要求

- 尽量复用已经验证过的捕获、位图转换、哈希匹配和结果输出逻辑。
- 不为了迁移而重新设计验证算法。
- 不增加不必要的模块抽象或新的第三方依赖。
- 保留必要的 Windows 资源释放、异常处理和临时窗口状态恢复。
- 对无法定位窗口、目标位置错误、位图不匹配、Hotspot 不一致或动画未推进等情况给出明确失败信息。
- 不修改 Gallery 自身的 Rust 代码，也不在 Gallery 内嵌测试逻辑。

### 3. 更新素材工具说明

目标文件：

`crates/asset/tools/README.md`

移除：

- 旧 `animated-cursor-authoring` Skill 的相对链接。
- 通过旧 Skill 路径执行 `cursor_tool.py` 的命令。
- 关于旧 Skill 内置原生验证工具的说明。
- 其他删除后失效的工具路径和引用。

保留并整理：

- `build_kamuri_animations.py` 的素材重建命令。
- 当前 Kamuri 素材输入与生成流程。
- 生成的 Atlas、Manifest、预览及其他输出内容。
- `--install` 的现有用途。
- Gallery Cursor Benchmark 的使用说明。
- 原生鼠标与 BRP 模拟指针验证的区别。

更新 Windows 原生验证章节，明确说明：

- 使用仓库内的 `native_cursor_probe.py`。
- 如何准备已有 Gallery 窗口及 Target JSON。
- 如何执行真实 OS Cursor 捕获。
- 如何查看和判断验证结果。
- 工具仅用于开发和验证，不参与最终程序运行。

可将 Character Cursor Atelier 作为独立素材制作工具简要提及，但不使用固定的本地安装路径，也不将其作为 Widgetry 的必需依赖。

### 4. 检查其他引用

对仓库进行旧 Skill 路径与相关工具名称的检查。

重点搜索：

- `animated-cursor-authoring`
- `skills/animated-cursor-authoring`
- `native_windows.py`
- `cursor_tool.py`
- `native_cursor_probe.py`

检查是否有其他脚本、配置或当前文档依赖已删除的内容。

仅修复本次删除直接造成的失效引用，不顺便重构无关代码或文档。

`plans/` 中即使存在历史路径或旧工具名称，也全部保留，不进行修改。

## 三、明确不修改的内容

以下内容不属于本次任务：

- `plans/` 下的全部历史方案。
- `crates/asset/tools/build_kamuri_animations.py` 的动画生成逻辑。
- 现有 Kamuri 角色素材、动画 Atlas 和静态后备资源。
- `crates/asset/src/cursor_animation.rs` 及相关资源目录。
- Cursor、Asset、Gallery 的 Rust 运行时实现。
- Kamuri 动画的视觉设计、语义、帧数、时序和 Hotspot。
- Cargo Workspace、Feature、依赖关系和公共 API。
- 新 Skill 的功能开发或维护。

本次只处理旧 Skill 移除及其直接依赖的清理，不扩大到其他工程改进。

## 四、验证与验收

### 1. 静态检查

确认：

- 旧 Skill 目录已删除。
- `native_cursor_probe.py` 不再引用旧 Skill。
- Python 脚本能够正常解析和启动。
- 参数解析与无效输入处理正常。
- 当前文档和辅助脚本不存在失效路径。
- `plans/` 没有任何修改。

### 2. Gallery 原生验证

在 Windows 环境下启动 Gallery，进入能够展示 Kamuri Cursor 的页面。

使用真实鼠标和新独立化的 `native_cursor_probe.py` 执行验证。

至少选择具有不同语义和动画特征的光标，覆盖普通指针、文本光标及等待等代表性场景，并确认：

- 能够正确定位 Gallery 原生窗口。
- 能够捕获真实 OS 光标，而不是 BRP 模拟指针。
- 实际尺寸与预期一致。
- 实际 Hotspot 与配置一致。
- 动画能够持续推进，不是单张静态图。
- 捕获位图能够匹配预期 Atlas 中的动画帧。
- 输出文件能够正常读取和查看。
- 验证结束后不会残留临时窗口置前状态或未释放的原生资源。

如果现有 45 种语义的 Target 配置和 Gallery 测试条件仍然可用，应继续执行原有的全语义验证，避免因工具迁移损失既有覆盖范围。

### 3. Kamuri 素材生成链

使用现有生成器执行必要的素材重建检查，确认：

- 现有生成器不依赖旧 Skill。
- 能够生成预期数量的图集及相关元数据。
- 现有资源目录和运行时资源契约保持不变。

本次不要求重新设计或重新生成提交仓库的动画素材。

### 4. 最终检查

检查最终变更范围，确保只有旧 Skill 删除、原生验证脚本独立化，以及直接相关的 README 更新和必要验证内容。

验证结果如实记录。未执行的 Windows 原生测试不得标记为通过。

## 五、完成标准

本次任务完成需要同时满足：

1. `animated-cursor-authoring` 已从仓库移除。
2. `native_cursor_probe.py` 可以独立执行 Gallery 的真实 OS 光标验证。
3. 原有关键验证能力没有因迁移而丢失。
4. `crates/asset/tools/README.md` 中的工具路径与使用说明准确。
5. Kamuri 素材生成链、运行时资源和 Gallery 原有功能未受影响。
6. `plans/` 完全保持不变。
7. 不再存在由旧 Skill 删除引起的失效依赖。

最终结果应当是：**角色动画素材创作由独立 Skill 负责，Widgetry 仓库只保留自身需要的素材生成和实际应用验证工具，两者不产生强制依赖。**
