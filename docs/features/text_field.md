# TextField

TextField 支持单行、多行、Unicode 字素长度限制、只读、前后槽位、计数、提示和自动高度。默认控件提供完整外观，Headless 允许将实际编辑器放进自己的布局。

```rust
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::text_field::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryTextFieldPlugin);
```

## 文本编辑

### 创建单行输入框

```rust
bsn! {
    @WidgetryTextField {
        @value: {"初始名称".to_owned()},
        @max_length: {Some(20)}
    }
    Node { width: px(280) }
}
```

value 只初始化正文，max_length 在构造后固定。长度按 Unicode 扩展字素簇计数，不按字节或 Unicode 标量值；超长正文保留能够容纳的完整字素前缀。

### 查询和替换正文

```rust
fn rename(world: &mut World, root: Entity) -> Result {
    WidgetryTextFieldHeadless::set_text_in_world(
        world, root, "新的名称".to_owned(),
    ).map_err(|error| error.into_bevy_error())?;

    let text = WidgetryTextFieldHeadless::text(world, root)
        .map_err(|error| error.into_bevy_error())?;
    info!(%text, "正文已替换");
    Ok(())
}
```

排队入口为 `set_text(&mut commands, root, text)`。默认根与自定义根使用相同 API；不要在根上添加第二个 EditableText，也不要绕过内容 API 直接改写内部编辑器正文。

## 只读与禁用

### 展示可选择、可复制的正文

```rust
bsn! {
    @WidgetryReadOnlyTextField {
        @value: {"这段文字可以选择和复制".to_owned()}
    }
    Node { width: px(360) }
}
```

只读过滤修改文本的输入，但保留导航、选择和复制。Disabled 的限制更强，会阻止用户编辑相关输入，并清理尚未处理的 Edit/Paste 请求。

```rust
commands.entity(root).insert(InteractionDisabled);
```

两种状态都允许通过公开内容 API 进行程序化赋值。主题、Hover 和焦点变化不应重置正文、Caret 或 Selection。

## 多行与自动高度

### 创建会随内容增长的编辑区

```rust
bsn! {
    @WidgetryTextField {
        @multiline: true,
        @auto_size: {Some(AutoSize { min_rows: 2, max_rows: 6 })},
        @value: {"第一行\n第二行".to_owned()}
    }
    Node { width: px(420), height: Val::Auto }
}
```

AutoSize 按真实排版行数计算，软换行也计入，超过最大行数后使用编辑器视口滚动。它要求 multiline，并满足 `1 <= min_rows <= max_rows`。默认完整 Style 的根高度保持 Auto，不再同时固定高度。

不使用 AutoSize 时，可通过 multiline、visible_lines 和 visible_width 选择固定编辑视口。允许换行与是否自动改变高度是两项独立配置。

## 前后槽位

### 在正文两侧放置辅助内容

```rust
bsn! {
    @WidgetryTextField {
        @children: {Some(Box::new(bsn_list! {
            WidgetryTextFieldPrefix
            Children [Text("https://") WidgetryText]--
            WidgetryTextFieldSuffix
            Children [Text(".example") WidgetryText]
        }) as Box<dyn SceneList>)}
    }
    Node { width: px(400) }
}
```

Prefix 和 Suffix 各至多一个，内部可以包含多个装饰或按钮，声明顺序不改变最终前缀—编辑器—后缀排列。槽位文字不属于正文，不参与长度限制、复制选择或计数。

槽位保留自身宽度，编辑器使用剩余空间；过宽内容不会被自动缩放或隐藏。

## 计数与提示

### 显示字数和固定提示

```rust
bsn! {
    @WidgetryTextField {
        @max_length: {Some(100)},
        @show_count: true,
        @hint_text: {Some("请勿输入敏感信息".to_owned())}
    }
    Node { width: px(360) }
}
```

计数在边框外下方显示正式文本字素数，不包含 IME 预编辑和槽位。提示是独立警告文本，不是 Placeholder，也不会因 Focus、只读或禁用而隐藏。提示和计数的高度计入 Root，但不计入 AutoSize 的编辑行数。

## 自定义编辑器布局

### 将 Editor 放在装饰容器中

```rust
app.add_plugins(WidgetryTextFieldHeadlessPlugin);

let custom = bsn! {
    #Root @WidgetryTextFieldHeadless { editor: #Editor }
    Children [
        Node { padding: UiRect::all(px(12)) }
        Children [
            #Editor @WidgetryTextFieldEditor {
                root: #Root,
                @value: {"搜索".to_owned()},
                @max_length: {Some(32)}
            }
        ]
    ]
};
```

Root 是公开 API 身份，Editor 保存原生 EditableText、Caret 和 Selection。双向绑定创建后固定，Editor 必须留在根的后代中，不跨根共享或搬迁。

独立 AutoSize 可通过 `WidgetryTextFieldAutoSize` 和对应 Plugin 组合。它只调整 Editor 的 visible_lines，自定义根和装饰的几何仍由应用管理。
