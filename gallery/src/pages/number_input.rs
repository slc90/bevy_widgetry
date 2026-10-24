use crate::{
    gallery::{GalleryPage, mount_page, unmount_page},
    i18n::{Language, LocalizedText, NumberInputKey, TextKey, localized},
};
use bevy::{
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, ValueChange},
};
use bevy_widgetry::{
    button::WidgetryButton,
    number_input::{WidgetryNumber, WidgetryNumberInput, WidgetryNumberInputProps},
    text::WidgetryText,
};

pub(crate) struct NumberInputDemoPlugin;

impl Plugin for NumberInputDemoPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<NumberDemoState<i32>>()
            .register_type::<NumberDemoState<u8>>()
            .register_type::<NumberDemoState<f64>>()
            .register_type::<NumberDemoState<i64>>()
            .add_systems(OnEnter(GalleryPage::NumberInput), enter)
            .add_systems(OnExit(GalleryPage::NumberInput), exit)
            .add_systems(
                PostUpdate,
                (
                    refresh::<i32>,
                    refresh::<u8>,
                    refresh::<f64>,
                    refresh::<i64>,
                )
                    .after(bevy::input_focus::InputFocusSystems::FocusChangeEvents)
                    .run_if(in_state(GalleryPage::NumberInput)),
            );
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct NumberDemoState<T: WidgetryNumber> {
    events: u64,
    last: Option<T>,
}

#[derive(Component, FromTemplate)]
struct ExampleParts {
    input: Entity,
    status: Entity,
}

#[derive(Component)]
struct Preset<T: WidgetryNumber>(T);

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::NumberInput, bsn_list! {@scene()})?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::NumberInput)
}

fn label(key: NumberInputKey) -> impl Scene {
    bsn! {@localized(TextKey::NumberInput(key)) WidgetryText}
}

fn scene() -> impl Scene {
    let examples: Vec<Box<dyn Scene>> = vec![
        Box::new(example(
            "NumberSigned",
            NumberInputKey::Signed,
            WidgetryNumberInputProps {
                value: Some(0_i32),
                min: Some(0),
                max: Some(10),
                step: 3,
                ..default()
            },
            vec![0, 9, 10],
            false,
            false,
            false,
        )),
        Box::new(example(
            "NumberUnsigned",
            NumberInputKey::Unsigned,
            WidgetryNumberInputProps {
                value: Some(254_u8),
                ..default()
            },
            vec![0, 255],
            false,
            false,
            false,
        )),
        Box::new(example(
            "NumberFloat",
            NumberInputKey::Float,
            WidgetryNumberInputProps {
                value: Some(1.234567_f64),
                step: 0.1,
                decimals: Some(2),
                ..default()
            },
            vec![0.0, 50.0, 100.0],
            false,
            false,
            false,
        )),
        Box::new(example(
            "NumberReadOnly",
            NumberInputKey::ReadOnly,
            WidgetryNumberInputProps {
                value: Some(50.0_f64),
                read_only: true,
                decimals: Some(2),
                ..default()
            },
            vec![0.0, 50.0, 100.0],
            false,
            false,
            false,
        )),
        Box::new(example(
            "NumberDisabled",
            NumberInputKey::Disabled,
            WidgetryNumberInputProps {
                value: Some(50_i32),
                min: Some(0),
                max: Some(100),
                step: 10,
                ..default()
            },
            vec![0, 50, 100],
            true,
            false,
            false,
        )),
        Box::new(example(
            "NumberInherited",
            NumberInputKey::InheritedDisabled,
            WidgetryNumberInputProps {
                value: Some(50_i32),
                step: 5,
                ..default()
            },
            vec![0, 50, 100],
            false,
            true,
            false,
        )),
        Box::new(example(
            "NumberFixed",
            NumberInputKey::Fixed,
            WidgetryNumberInputProps {
                value: Some(7_i32),
                min: Some(7),
                max: Some(7),
                step: 3,
                ..default()
            },
            vec![7],
            false,
            false,
            false,
        )),
        Box::new(example(
            "NumberLimits",
            NumberInputKey::Limits,
            WidgetryNumberInputProps {
                value: Some(i64::MIN),
                step: i64::MAX,
                ..default()
            },
            vec![i64::MIN, 0, i64::MAX],
            false,
            false,
            false,
        )),
        Box::new(example(
            "NumberNarrow",
            NumberInputKey::Narrow,
            WidgetryNumberInputProps {
                value: Some(10_i32),
                step: 3,
                ..default()
            },
            vec![0, 50, 100],
            false,
            false,
            true,
        )),
    ];
    bsn! {
        Name::new("NumberInputDemo")
        Node {width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(16),flex_shrink:0.0}
        Children [
            @label(NumberInputKey::Description) Node {max_width:percent(100)}--
            Node {width:percent(100),flex_wrap:FlexWrap::Wrap,column_gap:px(24),row_gap:px(24),align_items:AlignItems::Start}
            Children [{examples}]
        ]
    }
}

fn example<T: WidgetryNumber>(
    name: &'static str,
    key: NumberInputKey,
    props: WidgetryNumberInputProps<T>,
    presets: Vec<T>,
    disabled: bool,
    inherited: bool,
    narrow: bool,
) -> impl Scene {
    let buttons: Vec<_> = presets.into_iter().map(|value| {
        let text = LocalizedText::with_args(TextKey::NumberInput(NumberInputKey::Set), vec![("value",value.to_string())]);
        bsn! {
            Name({format!("{name}Set{value}")})
            @WidgetryButton {@children:bsn_list! { Text::new("") WidgetryText template(move |_|Ok(text.clone())) }}
            bevy::ui_widgets::ActivateOnPress
            template(move |_|Ok(Preset(value))) on(set_preset::<T>)
        }
    }).collect();
    let local = disabled.then_some(bsn! {InteractionDisabled});
    let parent = inherited.then_some(bsn! {InteractionDisabled});
    bsn! {
        ExampleParts {input:#NumberExampleInput,status:#NumberExampleStatus}
        Node {width:px(380),max_width:percent(100),min_width:px(180),flex_direction:FlexDirection::Column,row_gap:px(8),flex_shrink:0.0}
        Children [
            @label(key) Node {max_width:percent(100)}--
            Node {width:px(if narrow {180.0} else {300.0}),overflow:Overflow::clip()}
            Children [
                Name({format!("{name}Parent")}) @{parent}
                Node {width:px(if narrow {50.0} else {300.0})}
                Children [
                    #NumberExampleInput Name::new(name)
                    @WidgetryNumberInput<T> {@value:{props.value},@min:{props.min},@max:{props.max},@step:{props.step},@decimals:{props.decimals},@read_only:{props.read_only}}
                    @{local} Node {width:px(if narrow {1.0} else {300.0})}
                    template(|_|Ok(NumberDemoState::<T>::default())) on(changed::<T>)
                ]
            ]--
            #NumberExampleStatus Name({format!("{name}Status")}) Text::new("") WidgetryText Node {max_width:percent(100)}--
            Node {max_width:percent(100),column_gap:px(6),row_gap:px(6),flex_wrap:FlexWrap::Wrap} Children [{buttons}]
        ]
    }
}

fn changed<T: WidgetryNumber>(
    event: On<ValueChange<T>>,
    mut states: Query<&mut NumberDemoState<T>>,
) {
    if let Ok(mut state) = states.get_mut(event.source) {
        state.events += 1;
        state.last = Some(event.value);
        info!(source=?event.source,value=%event.value,"NumberInput 正式值已变化");
    }
}

fn set_preset<T: WidgetryNumber>(
    event: On<Activate>,
    presets: Query<&Preset<T>>,
    parents: Query<&ChildOf>,
    examples: Query<&ExampleParts>,
    mut commands: Commands,
) -> Result {
    let Ok(preset) = presets.get(event.entity) else {
        return Ok(());
    };
    let mut cursor = event.entity;
    while let Ok(parent) = parents.get(cursor) {
        cursor = parent.parent();
        if let Ok(parts) = examples.get(cursor) {
            let (root, value) = (parts.input, preset.0);
            commands.queue(move |world: &mut World| -> Result {
                let changed = WidgetryNumberInput::<T>::set_value_in_world(world, root, value)?;
                info!(?root,%value,changed,"通过公开 API 设置 NumberInput");
                Ok(())
            });
            return Ok(());
        }
    }
    error!(entity=?event.entity,"NumberInput 预设按钮缺失示例归属");
    Err(BevyError::error("NumberInput preset example is missing"))
}

fn refresh<T: WidgetryNumber>(world: &mut World) -> Result {
    let mut changed = false;
    let examples = world
        .query::<&ExampleParts>()
        .iter(world)
        .map(|parts| (parts.input, parts.status))
        .collect::<Vec<_>>();
    for (input, status) in examples {
        let Some(state) = world.get::<NumberDemoState<T>>(input) else {
            continue;
        };
        let value = WidgetryNumberInput::<T>::value(world, input)?;
        let text = LocalizedText::with_args(
            TextKey::NumberInput(NumberInputKey::Status),
            vec![
                ("value", value.to_string()),
                ("events", state.events.to_string()),
                (
                    "last",
                    state.last.map_or_else(|| "—".into(), |v| v.to_string()),
                ),
            ],
        );
        let rendered = text.render(*world.resource::<Language>());
        let current = world.get::<Text>(status).ok_or_else(|| {
            error!(?status, "NumberInput 状态文字缺失");
            BevyError::error("NumberInput status Text is missing")
        })?;
        if current.0 != rendered {
            world.entity_mut(status).insert((text, Text(rendered)));
            changed = true;
        }
    }
    if changed {
        // Blur commits follow UI text layout, so Reactive windows need one more frame to render the new status.
        world.write_message(bevy::window::RequestRedraw);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::input_focus::InputFocus;
    use bevy_widgetry::{scene::spawn_scene, text_field::WidgetryTextField};

    #[test]
    fn blur_commits_and_updates_the_status_in_the_same_frame() {
        let mut app = bevy_widgetry_test_utils::text_edit_app();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<GalleryPage>()
            .init_resource::<Language>()
            .add_plugins((
                bevy_widgetry::number_input::WidgetryNumberInputPlugin,
                bevy_widgetry::button::WidgetryButtonPlugin,
                bevy_widgetry::scroll_area::WidgetryScrollAreaPlugin,
                NumberInputDemoPlugin,
            ));
        spawn_scene(
            app.world_mut(),
            bsn! {Node template(|_|Ok(crate::gallery::GalleryPageHost))},
        )
        .unwrap();
        app.world_mut()
            .resource_mut::<NextState<GalleryPage>>()
            .set(GalleryPage::NumberInput);
        app.update();
        app.update();
        let root = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "NumberSigned")
            .unwrap()
            .0;
        let status = app
            .world_mut()
            .query::<&ExampleParts>()
            .iter(app.world())
            .find(|parts| parts.input == root)
            .unwrap()
            .status;
        let field = app
            .world()
            .get::<Children>(root)
            .unwrap()
            .iter()
            .find(|child| app.world().get::<WidgetryTextField>(*child).is_some())
            .unwrap();
        WidgetryTextField::focus_in_world(app.world_mut(), field).unwrap();
        app.update();
        WidgetryTextField::set_text_in_world(app.world_mut(), field, "5").unwrap();
        app.world_mut()
            .resource_mut::<Messages<bevy::window::RequestRedraw>>()
            .clear();
        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();
        assert_eq!(
            WidgetryNumberInput::<i32>::value(app.world(), root).unwrap(),
            5
        );
        assert_eq!(
            app.world()
                .get::<NumberDemoState<i32>>(root)
                .unwrap()
                .events,
            1
        );
        assert_eq!(
            app.world().get::<Text>(status).unwrap().0,
            "Official: 5 · changes: 1 · last event: 5"
        );
        assert!(
            !app.world()
                .resource::<Messages<bevy::window::RequestRedraw>>()
                .is_empty()
        );
        app.world_mut()
            .resource_mut::<Messages<bevy::window::RequestRedraw>>()
            .clear();
        app.update();
        assert!(
            app.world()
                .resource::<Messages<bevy::window::RequestRedraw>>()
                .is_empty()
        );
    }
}
