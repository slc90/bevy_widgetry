use crate::assets::GalleryIcon;
use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::ButtonKey;
use crate::i18n::{CommonKey, NavigationKey};
use crate::i18n::{TextKey, localized};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryLinkButton},
    icon::WidgetryIcon,
    radio_group::{WidgetryRadioGroup, WidgetryRadioOption},
};

pub(crate) struct ButtonDemoPlugin;

impl Plugin for ButtonDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GalleryPage::Button), enter)
            .add_systems(OnExit(GalleryPage::Button), exit)
            .add_systems(
                Update,
                finish_loading_shortcut.run_if(in_state(GalleryPage::Button)),
            );
    }
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Button, bsn_list! { @scene() })?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Button)
}

#[derive(Component)]
struct ButtonDemo;

#[derive(Component)]
struct ButtonDemoAction(&'static str);

#[derive(Component)]
struct SwitchableLink;

pub(crate) fn scene() -> impl Scene {
    bsn! {
        #ButtonDemo
        template(|_| Ok(ButtonDemo))
        Node { flex_direction: FlexDirection::Column, row_gap: px(16), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Navigation(NavigationKey::Button)) bevy_widgetry::text::WidgetryText--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [@localized(TextKey::Common(CommonKey::Normal)) bevy_widgetry::text::WidgetryText-- @button_row(false)]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [@localized(TextKey::Common(CommonKey::Disabled)) bevy_widgetry::text::WidgetryText-- @button_row(true)]--
            @link_examples()--
            @loading_example()--
            @localized(TextKey::Button(ButtonKey::RadioButton)) bevy_widgetry::text::WidgetryText--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [@localized(TextKey::Button(ButtonKey::Horizontal)) bevy_widgetry::text::WidgetryText--
                    @WidgetryRadioGroup
                    Node { flex_direction: FlexDirection::Row, column_gap: px(16) }
                    on(on_radio_changed)
                    Children [@radio_option(TextKey::Common(CommonKey::Apple))-- @radio_option(TextKey::Common(CommonKey::Banana))-- @radio_option(TextKey::Common(CommonKey::Orange))]
                ]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [@localized(TextKey::Button(ButtonKey::Grid)) bevy_widgetry::text::WidgetryText--
                    @WidgetryRadioGroup
                    Node {
                        display: Display::Grid,
                        grid_template_columns: {vec![RepeatedGridTrack::auto(2)]},
                        column_gap: px(16),
                    }
                    on(on_radio_changed)
                    Children [@radio_option(TextKey::Common(CommonKey::Apple))-- @radio_option(TextKey::Common(CommonKey::Banana))-- @radio_option(TextKey::Common(CommonKey::Orange))-- @radio_option(TextKey::Common(CommonKey::Grape))]
                ]--
            Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [@localized(TextKey::Common(CommonKey::Disabled)) bevy_widgetry::text::WidgetryText--
                    @WidgetryRadioGroup InteractionDisabled
                    Children [@radio_option(TextKey::Common(CommonKey::Apple))-- @radio_option(TextKey::Common(CommonKey::Banana))-- @radio_option(TextKey::Common(CommonKey::Orange))]
                ]
        ]
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct LoadingDemo {
    activations: usize,
}

fn loading_example() -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Button(ButtonKey::LoadingDescription)) bevy_widgetry::text::WidgetryText--
            Node { column_gap: px(12), align_items: AlignItems::Center }
            Children [
                @WidgetryButton {
                    @children: {Some(Box::new(bsn_list! { @localized(TextKey::Button(ButtonKey::StartLoading)) bevy_widgetry::text::WidgetryText }) as Box<dyn SceneList>)},
                }
                Name("ButtonLoadingDemo") template(|_| Ok(LoadingDemo::default())) on(start_loading)
                Node { min_width: px(160), height: px(40) }--
                @WidgetryButton {
                    @children: {Some(Box::new(bsn_list! { @localized(TextKey::Button(ButtonKey::FinishLoading)) bevy_widgetry::text::WidgetryText }) as Box<dyn SceneList>)},
                }
                Name("ButtonFinishLoading") on(finish_loading)
            ]
        ]
    }
}

fn start_loading(event: On<Activate>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        WidgetryButton::set_loading_in_world(world, entity, true)?;
        let mut demo = world.get_mut::<LoadingDemo>(entity).ok_or_else(|| {
            error!(?entity, "Loading 演示状态缺失");
            BevyError::error("Loading 演示状态缺失")
        })?;
        demo.activations += 1;
        info!(
            ?entity,
            activations = demo.activations,
            "开始 Button Loading 演示"
        );
        Ok(())
    });
}

fn finish_loading(
    _event: On<Activate>,
    buttons: Query<Entity, With<LoadingDemo>>,
    mut commands: Commands,
) {
    for entity in &buttons {
        queue_finish_loading(&mut commands, entity);
    }
}

fn queue_finish_loading(commands: &mut Commands, entity: Entity) {
    commands.queue(move |world: &mut World| -> Result {
        if WidgetryButton::set_loading_in_world(world, entity, false)? {
            info!(?entity, "结束 Button Loading 演示");
        }
        Ok(())
    });
}

fn finish_loading_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Query<Entity, With<LoadingDemo>>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::F1) {
        for entity in &buttons {
            queue_finish_loading(&mut commands, entity);
        }
    }
}

fn link_examples() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Button(ButtonKey::LinkButton)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Button(ButtonKey::LinkDescription)) bevy_widgetry::text::WidgetryText--
            @WidgetryLinkButton { @url: {"https://bevy.org/learn/quick-start/getting-started/apps/".to_owned()} }
                Name("LinkButtonNormal") on(on_link_activated)--
            @WidgetryLinkButton { @url: {"https://bevy.org".to_owned()} }
                InteractionDisabled Name("LinkButtonDisabled")--
            @localized(TextKey::Button(ButtonKey::ProgrammaticUrl)) bevy_widgetry::text::WidgetryText--
            @WidgetryLinkButton { @url: {"https://www.baidu.com".to_owned()} }
                template(|_| Ok(SwitchableLink)) Name("LinkButtonSwitchable") on(on_link_activated)--
            @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Button(ButtonKey::SwitchUrl)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, } Name("LinkButtonSwitchUrl") on(switch_url)

        ]
    }
}

fn switch_url(
    _event: On<Activate>,
    links: Query<Entity, With<SwitchableLink>>,
    mut commands: Commands,
) -> Result {
    let entity = links.single().map_err(|error| {
        error!(%error, "LinkButton 演示缺少唯一切换目标");
        BevyError::error(error.to_string())
    })?;
    commands.queue(move |world: &mut World| -> Result {
        let next = if WidgetryLinkButton::url(world, entity)? == "https://www.baidu.com" {
            "https://www.google.com"
        } else {
            "https://www.baidu.com"
        };
        WidgetryLinkButton::set_url_in_world(world, entity, next)?;
        info!(?entity, url = next, "程序化切换 LinkButton URL");
        Ok(())
    });
    Ok(())
}

fn on_link_activated(event: On<Activate>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        if world.get::<InteractionDisabled>(entity).is_none() {
            let url = WidgetryLinkButton::url(world, entity)?;
            info!(?entity, url, "激活 LinkButton，发起系统浏览器请求");
        }
        Ok(())
    });
}

fn radio_option(label: TextKey) -> impl Scene {
    bsn! { @WidgetryRadioOption Children [@localized(label) bevy_widgetry::text::WidgetryText] }
}

fn on_radio_changed(event: On<ValueChange<usize>>) {
    info!(demo = "radio", entity = ?event.source, index = event.value, "选择 Radio 示例选项");
}

fn button_row(disabled: bool) -> impl Scene {
    let buttons = bsn_list! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Common(CommonKey::Text)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
            template(|_| Ok(ButtonDemoAction("Text")))
            on(on_demo_activated)
            @{disabled.then_some(bsn! { InteractionDisabled })}
            Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            --
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@star()-- @localized(TextKey::Common(CommonKey::IconText)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, @content: {Some(Box::new(bsn! { Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: px(6) } }) as Box<dyn bevy::prelude::Scene>)}, }
            template(|_| Ok(ButtonDemoAction("Icon + Text")))
            on(on_demo_activated)
            @{disabled.then_some(bsn! { InteractionDisabled })}
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: px(6) }
            --
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Button(ButtonKey::TextIcon)) bevy_widgetry::text::WidgetryText-- @star()}) as Box<dyn bevy::prelude::SceneList>)}, @content: {Some(Box::new(bsn! { Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: px(6) } }) as Box<dyn bevy::prelude::Scene>)}, }
            template(|_| Ok(ButtonDemoAction("Text + Icon")))
            on(on_demo_activated)
            @{disabled.then_some(bsn! { InteractionDisabled })}
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: px(6) }
            --
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@star()}) as Box<dyn bevy::prelude::SceneList>)}, }
            template(|_| Ok(ButtonDemoAction("Icon")))
            on(on_demo_activated)
            @{disabled.then_some(bsn! { InteractionDisabled })}
            Node { width: px(32), height: px(32), padding: UiRect::all(px(6)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }

    };
    bsn! {
        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(12) }
        Children [{buttons}]
    }
}

fn on_demo_activated(
    event: On<Activate>,
    buttons: Query<&ButtonDemoAction, Without<InteractionDisabled>>,
) {
    let Ok(action) = buttons.get(event.entity) else {
        return;
    };
    info!(demo = "button", button = action.0, "激活按钮示例");
}

fn star() -> impl Scene {
    bsn! {
        @WidgetryIcon {
            @path: { GalleryIcon::ButtonStar.path() },
            @max_size: { Some(UVec2::new(16, 16)) },
        }
    }
}
