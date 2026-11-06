use crate::i18n::{ButtonKey, Language, TextKey, localized, tr, tr_args};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_widgetry::button::{WidgetryButton, WidgetryOpenUrl, WidgetryOpenUrlState};

const LEARN: &str = "https://bevy.org/learn/";
const QUICK_START: &str = "https://bevy.org/learn/quick-start/getting-started/apps/";

#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component)]
struct GalleryLinkCard {
    activations: usize,
}

#[derive(Component)]
struct Status;

impl GalleryLinkCard {
    fn scene() -> impl Scene {
        bsn! {
            @WidgetryOpenUrl { @url: {LEARN.to_owned()} }
            Name("GalleryLinkCard")
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { width: px(300), height: px(76), padding: UiRect::all(px(16)), column_gap: px(14), align_items: AlignItems::Center, border_radius: BorderRadius::all(px(14)) }
            BackgroundColor({Color::srgb_u8(27, 49, 78)})
            on(activated)
            Children [
                Text("↗") Pickable::IGNORE TextFont { font_size: FontSize::Px(36.0) } TextColor({Color::srgb_u8(103, 232, 249)})--
                @localized(TextKey::Button(ButtonKey::LinkCardAction)) Pickable::IGNORE TextColor({Color::WHITE})
            ]
        }
    }
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("GalleryLinkCardDemo")
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Button(ButtonKey::LinkCardDescription)) bevy_widgetry::text::WidgetryText--
            @GalleryLinkCard--
            Name("GalleryLinkCardStatus") template(|_| Ok(Status)) Text("") bevy_widgetry::text::WidgetryText--
            @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Button(ButtonKey::LinkCardSwitch)) bevy_widgetry::text::WidgetryText }) as Box<dyn SceneList>)} }
            Name("GalleryLinkCardSwitchUrl") on(switch_url)
        ]
    }
}

pub(super) fn install(app: &mut App) {
    app.register_type::<GalleryLinkCard>().add_systems(
        PostUpdate,
        update_status
            .before(bevy::ui::UiSystems::Prepare)
            .run_if(in_state(crate::gallery::GalleryPage::Button)),
    );
}

fn activated(
    event: On<Activate>,
    mut cards: Query<
        (&mut GalleryLinkCard, &WidgetryOpenUrlState),
        Without<bevy::ui::InteractionDisabled>,
    >,
) {
    if let Ok((mut card, target)) = cards.get_mut(event.entity) {
        card.activations += 1;
        info!(entity=?event.entity, url=target.url(), activations=card.activations, "激活外部链接卡片，发起系统浏览器请求");
    }
}

fn switch_url(
    _: On<Activate>,
    cards: Query<Entity, With<GalleryLinkCard>>,
    mut commands: Commands,
) -> Result {
    let entity = cards.single().map_err(|error| {
        crate::diagnostics::at(
            error,
            bevy_widgetry::diagnostics::ErrorFrame::new("链接卡片演示缺少唯一目标"),
        )
    })?;
    commands.queue(move |world: &mut World| -> Result {
        let next = if WidgetryOpenUrl::url(world, entity)
            .map_err(|error| error.into_bevy_error())?
            == LEARN
        {
            QUICK_START
        } else {
            LEARN
        };
        WidgetryOpenUrl::set_url_in_world(world, entity, next)
            .map_err(|error| error.into_bevy_error())?;
        info!(?entity, url = next, "程序化切换链接卡片目标");
        Ok(())
    });
    Ok(())
}

fn update_status(
    language: Res<Language>,
    cards: Query<(Ref<GalleryLinkCard>, Ref<WidgetryOpenUrlState>)>,
    mut labels: Query<&mut Text, With<Status>>,
) {
    for (card, target) in &cards {
        if !language.is_changed() && !card.is_changed() && !target.is_changed() {
            continue;
        }
        let destination = tr(
            *language,
            TextKey::Button(if target.url() == LEARN {
                ButtonKey::LinkCardLearn
            } else {
                ButtonKey::LinkCardQuickStart
            }),
        );
        let value = tr_args(
            *language,
            TextKey::Button(ButtonKey::LinkCardStatus),
            &[
                ("destination", destination.to_owned()),
                ("count", card.activations.to_string()),
            ],
        );
        for mut label in &mut labels {
            label.set_if_neq(Text(value.clone()));
        }
    }
}
