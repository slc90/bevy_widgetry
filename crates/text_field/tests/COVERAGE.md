# text_field Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

`diagnostics.rs` 保护普通 / 只读编辑器丢失时的领域分类、Root / editor / frame 与操作上下文、共享 origin 与源头一次 ERROR；计数输出失败发生在正文写入之后时，诊断报告真实阶段，Command 保留诊断进入宿主。缺少必需资源在写入前拒绝，正常截断与同值无错误。content Unit Test 区分 Editor 身份与其 LengthLimit 缺失，原有 AutoSize / IME / Count / Style 测试继续覆盖正常契约。

## States

- 普通 / ReadOnly、Enabled / Disabled、Hover / Focus。
- Text、Caret、Selection、Pending Edit / Paste 和 Theme。

## Stimuli

- BSN Scene、Widgetry 根实体内容 API、Keyboard / Pointer、Selection Drag。
- EditCommand / Paste、Theme、Focus 转移和 Disabled 恢复。

## Guards

- ReadOnly 阻止 Mutation，但允许 Navigation / Selection / Copy。
- Disabled 优先过滤全部用户编辑与 Paste，不阻止程序化内容修改。

## Transitions

- Normal / Hover / Focus / Disabled 外观转换。
- 用户选择文本、按字选取或拖动。
- Enabled → Disabled → Enabled 时不重放旧 Edit Queue。

## Invariants

- Text / Selection 是官方 EditableText State，样式不会重置内容。
- ReadOnly 的 Mutation 不到达真正编辑阶段，合法 Selection 仍改变。
- 禁用期间旧 Edit / Paste 丢弃；恢复后只接受新请求。
- 预编辑隐藏 Caret 时的程序化赋值先结束旧组合态，恢复合法光标和可见几何。
- Queue 只证明特定过滤阶段，另检查 Bevy 编辑消费后的 Text / Selection；Mouse / Custom 的定位、连续点击、Drag / Cancel 共享语义，Disabled 不消费 Selection。

## Couplings

- Pointer Focus × Keyboard；ReadOnly × Selection。
- Disabled × Pending Edits / Paste；Theme × Caret / Selection Colors。

## Coverage Map

- 编辑队列过滤在 `EditableTextSystems` 内、官方 `apply_text_edits` 消费前断言，保留命令及 Paste 的精确检查；AutoSize 的缺失资源单元回归使用不安装 TextPlugin 的隔离 App，防止 Bevy 官方 System 的额外错误干扰领域诊断。

- `widgetry_text_field.rs`：Scene、Layout、Font 策略与完整 Style / Theme。
- `disabled.rs`：编辑阻止、Queue 清理与恢复。
- `read_only.rs`：ReadOnly Mutation 边界、光标移动 / Selection / Focus；不安装 TabNavigationPlugin 时 Pointer / AcquireFocus 仍解析真实编辑器。
- `pointer.rs`：Mouse / Custom Picking 的 Caret、选词与 Drag Selection。
- `composition.rs`：复合根/Frame/Editor 身份、缺省及单/双槽、多个 Children、乱序与非法槽、独立子控件焦点、根聚焦映射和真实键盘 Tab / Shift+Tab 保持编辑器焦点；Filter / SelectAllOnFocus 同帧移除后重加保留当前配置，后续稳定帧仍生效，单独移除则撤销投影。内部 Frame 查询覆盖普通/只读映射，非法目标与缺失 Frame 明确返回 Error 并记录日志。
- `support/mod.rs`：跨用例共享 Fixture，不单独承担业务覆盖。
- `length.rs`：Unicode 字素簇上限、初值/根实体 Setter、队列顺序、边界合并、IME 正式/预编辑、剪贴板、官方通知与提交前长度保证。
- `count.rs`：独立可选计数、Unicode字素簇与上限格式、同步/排队Setter、键盘/选区替换/粘贴/IME正式提交同帧更新、预编辑排除、ReadOnly/Disabled/Theme/Focus保持，以及Frame外右对齐布局与编辑区宽度隔离。
- `auto_size.rs`：单行/固定多行/自动多行、默认及自定义行数范围、显式换行和真实软换行、增长/封顶/官方viewport滚动/删除收缩、宽度/槽宽/字体/行高多帧收敛，Count与高槽不改变编辑区高度策略，以及配置和固定高度冲突的错误传播/恢复。
- `hint.rs`：缺省/空字符串/常显提示、Frame→Count→Hint顺序、自然换行与宽度变化（含无空格长URL/标识符不溢出）、Focus/IME/ReadOnly/Disabled保留、三套内置主题警告色及StyleOwner归属，以及五项能力共存时提示不进入文本/计数/选区/AutoSize测量。

## 覆盖边界

`auto_size.rs` 同时覆盖默认与可选独立 AutoSize：无默认 Frame 的任意层级 Editor、固定自定义根几何/较高装饰、行数夹取和清空回缩、宽度/字体/行高变化与 IME 预编辑排版、主题变化保留正文和选区、未启用的 Headless 保持固定行数，以及失效祖先链/错误 Owner/单行冲突的多根聚合、持续去重与恢复。AutoSize Unit Test 保护 FontCx/LayoutCx 缺失时的领域类型、原始诊断和宿主传播，恢复资源不替换 Editor。默认固定根高度冲突、恢复和再次冲突继续由原有 Integration Test 保护。

Native Clipboard、IME 与真实 OS Focus 仍存在 Headless 验证边界。



`headless.rs` 使用独立HeadlessPlugin和无默认Frame的三层装饰树，验证Root/Editor双向绑定、完整字素初值/Setter、零上限、选择替换与IME预编辑/正式提交、ReadOnly选择及程序赋值、Disabled丢弃Edit/Paste、Root AcquireFocus/深层子控件自身Focus、根字体投影及Editor自有字体、真实Keyboard路由、默认Plugin共存不写自有颜色，以及跨Root共享/错误Owner/脱离实体树/失效Editor拒绝与queued执行时校验隔离。独立嵌套Editor的原生Windows Clipboard粘贴和只读复制复用真实Bevy Clipboard，完成后恢复原可读文本；OS输入法窗口与候选确认仍不由合成IME请求覆盖。

`headless.rs` 的提前编辑入口回归保护独立Editor的字素上限、ReadOnly和Disabled队列阻断、不透明原生过滤器的明确错误及失败前正文保持。ComboBox组合回归负责提前编辑与过滤/确认输入顺序。

插件装配保持宿主字体策略与官方 TextInput 所有权，不自动安装 TabNavigationPlugin；Facade 跨 Widget 用例覆盖 Button / TextField 两种装配顺序的 Pointer 焦点与 FocusLost。
