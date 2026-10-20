### Window：标题栏双击最大化与还原

**功能行为**

- 双击标题栏空白拖动区域，在普通窗口与最大化窗口之间切换。
- 仅当 `Window.enabled_buttons.maximize` 为 `true` 时生效。
- 不影响标题栏内其他交互控件。
- 作为默认行为，不新增公开配置项。

**实现方案**

- 扩展现有 `on_title_bar_press()`，保留第一次鼠标按下时的原生窗口拖动行为。
- 使用 Windows `GetDoubleClickTime()` 和 `GetSystemMetrics(SM_CXDOUBLECLK / SM_CYDOUBLECLK)` 获取系统双击判定参数。
- 根据同一窗口、同一拖动区域内连续两次左键按下的时间及屏幕物理坐标判断双击，不依赖 Bevy 的 `PointerPress.count`。
- 识别到有效双击后，不再启动拖动，转而执行最大化或还原。
- 与现有最大化按钮复用同一套最大化切换逻辑，继续由 `sync_maximize_state()` 同步窗口外观。

**验证要求**

- 单击拖动功能保持正常。
- 双击可以最大化，再次双击可以还原。
- 超出 Windows 双击时间或位置范围时不触发。
- 禁止最大化的窗口不会响应双击最大化。
- 标题栏自定义交互控件不受影响。
- Windows 实机验证原生拖动后的连续鼠标事件处理。
