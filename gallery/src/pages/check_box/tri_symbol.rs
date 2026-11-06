use crate::i18n::{CheckBoxKey, Language, TextKey, localized, tr_args};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, ValueChange},
};
use bevy_widgetry::{
    button::WidgetryButton,
    check_box::{WidgetryCheckState, WidgetryTriStateCheckboxHeadless},
    text::WidgetryText,
};

#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component)]
struct GalleryTriSymbol {
    changes: usize,
}
#[derive(Component)]
struct Symbol;
#[derive(Component)]
struct Status;
#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Set(WidgetryCheckState),
    Cycle,
    Focus,
    Disabled,
}

impl GalleryTriSymbol {
    fn scene() -> impl Scene {
        bsn! {
            @WidgetryTriStateCheckboxHeadless
            Name("GalleryTriSymbol")
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { width: px(96), height: px(64), border_radius: BorderRadius::all(px(12)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            BackgroundColor
            on(changed)
            Children [
                template(|_| Ok(Symbol)) Pickable::IGNORE
                Node { width: px(28), height: px(28), border_radius: BorderRadius::MAX, border: UiRect::all(px(3)) }
                BackgroundColor
                template(|_| Ok(BorderColor::all(Color::WHITE)))
            ]
        }
    }
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("GalleryTriSymbolDemo")
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::CheckBox(CheckBoxKey::TriSymbolDescription)) WidgetryText--
            @GalleryTriSymbol--
            Name("GalleryTriSymbolStatus") template(|_| Ok(Status)) Text("") WidgetryText--
            Node { column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap }
            Children [
                @control(Action::Set(WidgetryCheckState::Unchecked), CheckBoxKey::Unchecked, "GalleryTriSetUnchecked")--
                @control(Action::Set(WidgetryCheckState::Checked), CheckBoxKey::Checked, "GalleryTriSetChecked")--
                @control(Action::Set(WidgetryCheckState::Indeterminate), CheckBoxKey::Indeterminate, "GalleryTriSetIndeterminate")--
                @control(Action::Cycle, CheckBoxKey::TriSymbolCycle, "GalleryTriCycle")--
                @control(Action::Focus, CheckBoxKey::TriSymbolFocus, "GalleryTriFocus")--
                @control(Action::Disabled, CheckBoxKey::TriSymbolDisabled, "GalleryTriDisabled")
            ]
        ]
    }
}

fn control(action: Action, key: CheckBoxKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::CheckBox(key)) WidgetryText }) as Box<dyn SceneList>)} }
        Name(name) template(move |_| Ok(action)) on(operate)
    }
}

pub(super) fn install(app: &mut App) {
    app.register_type::<GalleryTriSymbol>().add_systems(
        PostUpdate,
        update_visuals
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::CheckBox)),
    );
}

fn changed(
    event: On<ValueChange<WidgetryCheckState>>,
    mut symbols: Query<(&WidgetryCheckState, &mut GalleryTriSymbol)>,
) {
    if let Ok((state, mut symbol)) = symbols.get_mut(event.source) {
        symbol.changes += 1;
        info!(entity=?event.source, committed=?state, changes=symbol.changes, "三态符号状态已提交");
    }
}

fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    symbols: Query<Entity, With<GalleryTriSymbol>>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    for root in &symbols {
        match action {
            Action::Set(state) => {
                WidgetryTriStateCheckboxHeadless::set_state(&mut commands, root, state)
            }
            Action::Cycle => WidgetryTriStateCheckboxHeadless::cycle_state(&mut commands, root),
            Action::Focus | Action::Disabled => commands.queue(move |world: &mut World| {
                if matches!(action, Action::Focus) {
                    world
                        .resource_mut::<InputFocus>()
                        .set(root, FocusCause::Navigated);
                } else if world.get::<InteractionDisabled>(root).is_some() {
                    world.entity_mut(root).remove::<InteractionDisabled>();
                } else {
                    world.entity_mut(root).insert(InteractionDisabled);
                }
            }),
        }
        info!(?root, ?action, "请求操作三态符号演示");
    }
}

type LastStatus = Option<(Entity, Language, WidgetryCheckState, usize)>;
fn update_visuals(
    language: Res<Language>,
    mut symbols: Query<(
        Entity,
        &GalleryTriSymbol,
        &WidgetryCheckState,
        Has<InteractionDisabled>,
        &mut BackgroundColor,
    )>,
    mut glyphs: Query<
        (&ChildOf, &mut Node, &mut BackgroundColor),
        (With<Symbol>, Without<GalleryTriSymbol>),
    >,
    mut labels: Query<&mut Text, With<Status>>,
    mut last: Local<LastStatus>,
) {
    for (root, symbol, state, disabled, mut background) in &mut symbols {
        let color = match state {
            WidgetryCheckState::Unchecked => Color::srgb_u8(65, 80, 100),
            WidgetryCheckState::Checked => Color::srgb_u8(20, 140, 110),
            WidgetryCheckState::Indeterminate => Color::srgb_u8(175, 105, 25),
        };
        background.set_if_neq(BackgroundColor(if disabled {
            Color::srgb_u8(105, 110, 120)
        } else {
            color
        }));
        for (parent, mut node, mut fill) in &mut glyphs {
            if parent.parent() != root {
                continue;
            }
            let (width, height, border) = match state {
                WidgetryCheckState::Unchecked => (28, 28, 3),
                WidgetryCheckState::Checked => (28, 28, 0),
                WidgetryCheckState::Indeterminate => (40, 6, 0),
            };
            if node.width != px(width) {
                node.width = px(width);
            }
            if node.height != px(height) {
                node.height = px(height);
            }
            if node.border != UiRect::all(px(border)) {
                node.border = UiRect::all(px(border));
            }
            fill.set_if_neq(BackgroundColor(
                if *state == WidgetryCheckState::Unchecked {
                    Color::NONE
                } else {
                    Color::WHITE
                },
            ));
        }
        let current = Some((root, *language, *state, symbol.changes));
        if *last == current {
            continue;
        }
        *last = current;
        let value = tr_args(
            *language,
            TextKey::CheckBox(CheckBoxKey::TriSymbolStatus),
            &[
                ("state", format!("{state:?}")),
                ("count", symbol.changes.to_string()),
            ],
        );
        for mut text in &mut labels {
            text.set_if_neq(Text(value.clone()));
        }
    }
}
