mod en_us;
mod key;
mod language;
mod zh_cn;

use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::UiSystems;
pub(crate) use key::TextKey;
pub(crate) use language::Language;

#[derive(Component, Clone)]
pub(crate) struct LocalizedText {
    key: TextKey,
    args: Vec<(&'static str, String)>,
    key_args: Vec<(&'static str, TextKey)>,
}

impl LocalizedText {
    pub(crate) fn new(key: TextKey) -> Self {
        Self {
            key,
            args: Vec::new(),
            key_args: Vec::new(),
        }
    }

    pub(crate) fn with_args(key: TextKey, args: Vec<(&'static str, String)>) -> Self {
        Self {
            key,
            args,
            key_args: Vec::new(),
        }
    }

    pub(crate) fn with_key_arg(mut self, name: &'static str, key: TextKey) -> Self {
        self.key_args.push((name, key));
        self
    }

    pub(crate) fn render(&self, language: Language) -> String {
        let mut args = self.args.clone();
        args.extend(
            self.key_args
                .iter()
                .map(|(name, key)| (*name, tr(language, *key).to_owned())),
        );
        tr_args(language, self.key, &args)
    }
}

pub(crate) fn tr(language: Language, key: TextKey) -> &'static str {
    match language {
        Language::English => en_us::tr(key),
        Language::SimplifiedChinese => zh_cn::tr(key),
    }
}

pub(crate) fn tr_args(language: Language, key: TextKey, args: &[(&str, String)]) -> String {
    let template = tr(language, key);
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}').map(|end| start + end) else {
            output.push_str(&rest[start..]);
            return output;
        };
        let name = &rest[start + 1..end];
        if let Some((_, value)) = args.iter().find(|(arg, _)| *arg == name) {
            output.push_str(value);
        } else {
            output.push_str(&rest[start..=end]);
            error!(?language, ?key, parameter = name, "翻译缺少命名参数");
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}

pub(crate) fn localized(key: TextKey) -> impl Scene {
    localized_args(key, Vec::new())
}

pub(crate) fn localized_args(key: TextKey, args: Vec<(&'static str, String)>) -> impl Scene {
    localized_binding(LocalizedText::with_args(key, args))
}

pub(crate) fn localized_binding(binding: LocalizedText) -> impl Scene {
    let initial = binding.clone();
    bsn! {
        template(move |context| {
            Ok(Text::new(initial.render(*context.resource::<Language>())))
        })
        template(move |_| Ok(binding.clone()))
    }
}

#[derive(Component, Clone)]
pub(crate) struct LocalizedWindowTitle(pub(crate) LocalizedText);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Language>()
        .register_type::<Language>()
        .add_systems(
            PostUpdate,
            (refresh_window_titles, refresh_text)
                .chain()
                .before(UiSystems::Prepare),
        );
}

fn refresh_text(
    language: Res<Language>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
    mut texts: ParamSet<(
        Query<(&LocalizedText, &mut Text), Without<EditableText>>,
        Query<(&LocalizedText, &mut Text), (Changed<LocalizedText>, Without<EditableText>)>,
    )>,
) {
    if language.is_changed() {
        redraw.write(bevy::window::RequestRedraw);
        for (binding, mut text) in &mut texts.p0() {
            text.set_if_neq(Text::new(binding.render(*language)));
        }
    } else {
        for (binding, mut text) in &mut texts.p1() {
            text.set_if_neq(Text::new(binding.render(*language)));
        }
    }
}

fn refresh_window_titles(world: &mut World) -> Result {
    let language = *world.resource::<Language>();
    let changed = world.is_resource_changed::<Language>();
    let titles: Vec<_> = world
        .query::<(Entity, Ref<LocalizedWindowTitle>)>()
        .iter(world)
        .filter(|(_, title)| changed || title.is_changed())
        .map(|(entity, title)| (entity, title.0.clone()))
        .collect();
    for (root, binding) in titles {
        let Some(native) = bevy_widgetry::window::widgetry_window_target(world, root) else {
            error!(?root, "本地化窗口缺少 native Window 绑定");
            return Err(BevyError::error("本地化窗口缺少 native Window 绑定"));
        };
        let mut window = world.get_mut::<Window>(native).ok_or_else(|| {
            error!(?root, ?native, "本地化窗口缺少 native Window");
            BevyError::error("本地化窗口缺少 native Window")
        })?;
        let previous = window.title.clone();
        window.title = binding.render(language);
        // 库构造的标题未携带 Gallery 标记，匹配必须限制在窗口首个 child 的 TitleBar 内。
        // 正文或 FileDialog entry 可能恰好与标题同名，遍历整个窗口会误翻译原始数据。
        let mut pending = world
            .get::<Children>(root)
            .and_then(|children| children.first().copied())
            .into_iter()
            .collect::<Vec<_>>();
        while let Some(entity) = pending.pop() {
            if let Some(children) = world.get::<Children>(entity) {
                pending.extend(children.iter());
            }
            if world
                .get::<Text>(entity)
                .is_some_and(|text| text.0 == previous)
                && world.get::<EditableText>(entity).is_none()
                && world.get::<LocalizedText>(entity).is_none()
                && world
                    .get::<Pickable>(entity)
                    .is_some_and(|pickable| *pickable == Pickable::IGNORE)
            {
                world.entity_mut(entity).insert(binding.clone());
            }
        }
    }
    Ok(())
}

// 测试断言用于验证翻译 contract，项目禁止主动 panic 的规则仅约束生产代码。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn named_parameters_preserve_raw_values_and_report_missing() {
        let subscriber = bevy::log::tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(bevy::log::Level::ERROR)
            .finish();
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            assert_eq!(Language::default(), Language::English);
            let args = vec![("index", "{edits}".into()), ("edits", "2".into())];
            assert_eq!(
                tr_args(Language::English, TextKey::ListItem, &args),
                "Item {edits}  (edits: 2)"
            );
            assert_eq!(
                tr_args(Language::SimplifiedChinese, TextKey::ListItem, &args),
                "项目 {edits}  （编辑：2）"
            );
            assert_eq!(
                tr_args(
                    Language::English,
                    TextKey::ListItem,
                    &[("index", "7".into())]
                ),
                "Item 7  (edits: {edits})"
            );
            let binding =
                LocalizedText::new(TextKey::TreeFolder).with_key_arg("label", TextKey::Projects);
            assert_eq!(binding.render(Language::English), "[Folder] Projects");
            assert_eq!(binding.render(Language::SimplifiedChinese), "[文件夹] 项目");
        });
    }
}
