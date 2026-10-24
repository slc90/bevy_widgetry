mod en_us;
mod key;
mod language;
mod zh_cn;

use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::UiSystems;
pub(crate) use key::{
    ButtonKey, CheckBoxKey, ColorShowcaseKey, ComboBoxKey, CommonKey, CursorKey, HeaderKey,
    ListViewKey, NavigationKey, NumberInputKey, ProgressBarKey, ScrollAreaKey, SegmentedKey,
    SliderKey, SplitterKey, TableKey, TabsKey, TextFieldKey, TextKey, TooltipKey, TreeKey,
    WaveformKey, WindowKey,
};
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

fn refresh_window_titles(
    world: &mut World,
    mut closing: Local<bevy::ecs::message::MessageCursor<bevy::window::WindowClosing>>,
) -> Result {
    let closing = world
        .get_resource::<Messages<bevy::window::WindowClosing>>()
        .map(|messages| {
            closing
                .read(messages)
                .map(|event| event.window)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !closing.is_empty() {
        let ended = world
            .query_filtered::<Entity, With<LocalizedWindowTitle>>()
            .iter(world)
            .filter(|root| {
                bevy_widgetry::window::widgetry_window_target(world, *root)
                    .is_some_and(|native| closing.contains(&native))
            })
            .collect::<Vec<_>>();
        for root in ended {
            world.entity_mut(root).remove::<LocalizedWindowTitle>();
        }
    }
    let language = *world.resource::<Language>();
    let changed = world.is_resource_changed::<Language>();
    let titles: Vec<_> = world
        .query::<(Entity, Ref<LocalizedWindowTitle>)>()
        .iter(world)
        .filter(|(_, title)| changed || title.is_changed())
        .map(|(entity, title)| (entity, title.0.clone()))
        .collect();
    for (root, binding) in titles {
        bevy_widgetry::window::WidgetryWindowTitle::set_in_world(
            world,
            root,
            binding.render(language),
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn title_parameters_and_language_update_bound_text_without_translating_raw_body() {
        use bevy_widgetry::window::{
            WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
            WidgetryWindowTitle, owned_widgetry_window, widgetry_window_target,
        };
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        install(&mut app);
        let binding = LocalizedText::with_args(
            TextKey::Window(WindowKey::WindowImageTitle),
            vec![("index", "1".into())],
        )
        .with_key_arg("mode", TextKey::Window(WindowKey::Cover50));
        let raw = binding.render(Language::English);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @owned_widgetry_window(Window {title: "Initial".into(), ..default()},
                    WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme,
                    Default::default(), bsn_list! {WidgetryWindowTitle},
                    bsn_list! {Text({raw.clone()}) Name("RawBody")})
                template(move |_| Ok(LocalizedWindowTitle(binding.clone())))
            })
            .unwrap()
            .id();
        app.update();
        let native = widgetry_window_target(app.world(), root).unwrap();
        let title = app
            .world_mut()
            .query_filtered::<Entity, With<WidgetryWindowTitle>>()
            .single(app.world())
            .unwrap();
        let body = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "RawBody")
            .unwrap()
            .0;
        assert_eq!(app.world().get::<Window>(native).unwrap().title, raw);
        assert_eq!(app.world().get::<Text>(title).unwrap().0, raw);
        assert!(app.world().get::<LocalizedText>(title).is_none());
        app.world_mut()
            .get_mut::<LocalizedWindowTitle>(root)
            .unwrap()
            .0
            .args = vec![("index", "2".into())];
        app.update();
        let expected = app
            .world()
            .get::<LocalizedWindowTitle>(root)
            .unwrap()
            .0
            .render(Language::English);
        assert_ne!(expected, raw);
        assert_eq!(app.world().get::<Window>(native).unwrap().title, expected);
        assert_eq!(app.world().get::<Text>(title).unwrap().0, expected);
        *app.world_mut().resource_mut::<Language>() = Language::SimplifiedChinese;
        app.update();
        let expected = app
            .world()
            .get::<LocalizedWindowTitle>(root)
            .unwrap()
            .0
            .render(Language::SimplifiedChinese);
        assert_eq!(app.world().get::<Window>(native).unwrap().title, expected);
        assert_eq!(app.world().get::<Text>(title).unwrap().0, expected);
        assert_eq!(app.world().get::<Text>(body).unwrap().0, raw);
        app.world_mut()
            .write_message(bevy::window::WindowClosing { window: native });
        app.world_mut().despawn(native);
        *app.world_mut().resource_mut::<Language>() = Language::English;
        app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        assert!(app.world().get::<LocalizedWindowTitle>(root).is_none());
        app.world_mut()
            .write_message(bevy::window::WindowClosed { window: native });
        app.update();
        assert!(!app.world().entities().contains(root));
    }

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
                tr_args(
                    Language::English,
                    TextKey::ListView(ListViewKey::ListItem),
                    &args
                ),
                "Item {edits}  (edits: 2)"
            );
            assert_eq!(
                tr_args(
                    Language::SimplifiedChinese,
                    TextKey::ListView(ListViewKey::ListItem),
                    &args
                ),
                "项目 {edits}  （编辑：2）"
            );
            assert_eq!(
                tr_args(
                    Language::English,
                    TextKey::ListView(ListViewKey::ListItem),
                    &[("index", "7".into())]
                ),
                "Item 7  (edits: {edits})"
            );
            let binding = LocalizedText::new(TextKey::Tree(TreeKey::TreeFolder))
                .with_key_arg("label", TextKey::Tree(TreeKey::Projects));
            assert_eq!(binding.render(Language::English), "[Folder] Projects");
            assert_eq!(binding.render(Language::SimplifiedChinese), "[文件夹] 项目");
        });
    }
}
