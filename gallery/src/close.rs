use crate::i18n::{CloseKey, Language, LocalizedText, LocalizedWindowTitle, TextKey, localized};
use bevy::ecs::message::MessageCursor;
use bevy::ecs::system::NonSendMarker;
use bevy::input::keyboard::KeyboardInput;
use bevy::input_focus::{FocusedInput, tab_navigation::TabIndex};
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::Activate;
use bevy::window::{PrimaryWindow, RequestRedraw, WindowCloseRequested};
use bevy::winit::WINIT_WINDOWS;
use bevy_widgetry::{
    button::WidgetryButton,
    radio_group::{WidgetryRadioGroup, WidgetryRadioOption},
    scene::WidgetrySceneCommandsExt,
    text::WidgetryText,
    tray::{WidgetryTrayClick, WidgetryTrayPlugin},
    window::{
        WidgetryModalWindow, WidgetryWindowBackground, WidgetryWindowControlsConfig,
        WidgetryWindowInitialFocus, WidgetryWindowTitle, owned_widgetry_window,
        widgetry_window_target,
    },
};

mod shutdown;

pub(crate) fn request_exit(world: &mut World) -> Result {
    shutdown::begin(world, AppExit::Success)
}

#[derive(Component)]
pub(crate) struct GalleryIndependentWindow;

#[derive(Component)]
struct CloseDialog;

#[derive(Component)]
struct CloseChoices;

#[derive(Component, Clone, Copy)]
enum Choice {
    Exit,
    Hide,
}

#[derive(Resource, Default)]
struct GalleryTrayState {
    dialog: Option<Entity>,
    hidden_windows: Vec<Entity>,
    focus_after_restore: Option<Entity>,
    exiting: bool,
}

pub(crate) fn install(app: &mut App) -> Result {
    app.add_plugins(WidgetryTrayPlugin::new(
        crate::assets::TRAY_ICON,
        "Widget Gallery",
    )?)
    .init_resource::<GalleryTrayState>()
    .add_systems(First, focus_restored)
    .add_systems(Update, (restore_from_tray, close_requests).chain());
    shutdown::install(app);
    Ok(())
}

fn close_requests(
    world: &mut World,
    mut cursor: Local<MessageCursor<WindowCloseRequested>>,
) -> Result {
    let requests: Vec<_> = cursor
        .read(world.resource::<Messages<WindowCloseRequested>>())
        .map(|request| request.window)
        .collect();
    for native in requests {
        if world.get::<PrimaryWindow>(native).is_some() {
            if !world.resource::<GalleryTrayState>().exiting {
                open_confirmation(world, native)?;
            }
            continue;
        }
        if let Some(dialog) = world.resource::<GalleryTrayState>().dialog
            && widgetry_window_target(world, dialog) == Some(native)
        {
            finish(world, dialog, None)?;
            continue;
        }
        let root = world
            .query_filtered::<Entity, With<GalleryIndependentWindow>>()
            .iter(world)
            .find(|root| widgetry_window_target(world, *root) == Some(native));
        if let Some(root) = root {
            world.entity_mut(root).despawn();
            world.flush();
        }
    }
    Ok(())
}

fn open_confirmation(world: &mut World, parent: Entity) -> Result {
    if world
        .resource::<GalleryTrayState>()
        .dialog
        .is_some_and(|dialog| world.entities().contains(dialog))
    {
        return Ok(());
    }
    let binding = LocalizedText::new(TextKey::GalleryClose(CloseKey::Title));
    let title = binding.render(*world.resource::<Language>());
    let root = world.commands().spawn_scene_with_error_handler(bsn! {
        @owned_widgetry_window(
            Window { title, resolution: (480, 260).into(), resizable: false, ..default() },
            WidgetryWindowControlsConfig { minimize_visible: false, maximize_visible: false, resizable: false, ..default() },
            WidgetryWindowBackground::Theme, default(),
            bsn_list! {
                Node { padding: UiRect::left(px(12)), align_items: AlignItems::Center }
                template(|_| Ok(Pickable::IGNORE))
                Children [WidgetryWindowTitle WidgetryText template(|_| Ok(Pickable::IGNORE))]
            },
            bsn_list! {
                Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, padding: UiRect::all(px(24)), row_gap: px(16) }
                Children [
                    @localized(TextKey::GalleryClose(CloseKey::Prompt)) WidgetryText--
                    @WidgetryRadioGroup template(|_| Ok(CloseChoices)) Name("GalleryCloseChoices") Children [
                        @WidgetryRadioOption template(|_| Ok(Choice::Exit)) Name("GalleryCloseExit")
                            Children [@localized(TextKey::GalleryClose(CloseKey::Exit)) WidgetryText]--
                        @WidgetryRadioOption template(|_| Ok(Choice::Hide)) Name("GalleryCloseHide")
                            WidgetryWindowInitialFocus(0)
                            Children [@localized(TextKey::GalleryClose(CloseKey::Hide)) WidgetryText]
                    ]--
                    Node { justify_content: JustifyContent::End, column_gap: px(12) }
                    Children [@dialog_button(true, CloseKey::Confirm)-- @dialog_button(false, CloseKey::Cancel)]
                ]
            })
        Name("GalleryCloseDialog")
        template(|_| Ok(CloseDialog))
        template(move |_| Ok(WidgetryModalWindow { parent }))
        template(move |_| Ok(LocalizedWindowTitle(binding.clone())))
        on(escape)
    }).id();
    world.resource_mut::<GalleryTrayState>().dialog = Some(root);
    world.flush();
    // RadioGroup initializes as children expand; apply the default to the complete scene before its first render.
    let choices = world
        .query_filtered::<Entity, With<CloseChoices>>()
        .single(world)
        .map_err(|error| {
            error!(%error, "Gallery 关闭确认缺少选项组");
            BevyError::error(error)
        })?;
    WidgetryRadioGroup::set_selected(&mut world.commands(), choices, 1);
    world.flush();
    Ok(())
}

fn dialog_button(confirm: bool, label: CloseKey) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::GalleryClose(label)) WidgetryText}) as Box<dyn SceneList>)}, }
        Name({if confirm { "GalleryCloseConfirm" } else { "GalleryCloseCancel" }})
        TabIndex::default()
        Node { min_width: px(96), height: px(36), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(move |_: On<Activate>, mut commands: Commands| {
            commands.queue(move |world: &mut World| -> Result {
                let Some(root) = world.resource::<GalleryTrayState>().dialog else { return Ok(()); };
                let choice = if confirm {
                    Some(world.query_filtered::<&Choice, With<Checked>>().single(world).copied().map_err(|error| {
                        error!(%error, "Gallery 关闭确认缺少唯一选项");
                        BevyError::error(error)
                    })?)
                } else { None };
                finish(world, root, choice)
            });
        })
    }
}

fn escape(
    mut event: On<FocusedInput<KeyboardInput>>,
    roots: Query<(), With<CloseDialog>>,
    mut commands: Commands,
) {
    if roots.contains(event.event_target())
        && event.input.key_code == KeyCode::Escape
        && event.input.state.is_pressed()
    {
        event.propagate(false);
        let root = event.event_target();
        commands.queue(move |world: &mut World| finish(world, root, None));
    }
}

fn finish(world: &mut World, root: Entity, choice: Option<Choice>) -> Result {
    if world.resource::<GalleryTrayState>().dialog != Some(root)
        || world.get::<CloseDialog>(root).is_none()
    {
        return Ok(());
    }
    world.resource_mut::<GalleryTrayState>().dialog = None;
    world.entity_mut(root).despawn();
    // Owned-window cleanup is deferred; finish it before collecting the hide session.
    world.flush();
    match choice {
        None => {}
        Some(Choice::Exit) => {
            request_exit(world)?;
        }
        Some(Choice::Hide) => {
            let mut hidden = Vec::new();
            for (entity, mut window) in world.query::<(Entity, &mut Window)>().iter_mut(world) {
                if window.visible {
                    // Native system commands can leave winit's minimized flag stale.
                    // Synchronize the existing minimized state before its visibility update rewrites window styles.
                    if WINIT_WINDOWS.with_borrow(|native| {
                        native
                            .get_window(entity)
                            .is_some_and(|window| window.is_minimized() == Some(true))
                    }) {
                        window.set_minimized(true);
                    }
                    window.visible = false;
                    hidden.push(entity);
                    info!(title = %window.title, ?entity, "已提交 Gallery 窗口隐藏变更");
                }
            }
            info!(window_count = hidden.len(), "Gallery 已进入托盘隐藏会话");
            let mut state = world.resource_mut::<GalleryTrayState>();
            state.hidden_windows = hidden;
            state.focus_after_restore = None;
        }
    }
    Ok(())
}

fn restore_from_tray(world: &mut World, mut cursor: Local<MessageCursor<WidgetryTrayClick>>) {
    let clicked = cursor
        .read(world.resource::<Messages<WidgetryTrayClick>>())
        .count()
        > 0;
    if !clicked
        || world.resource::<GalleryTrayState>().exiting
        || world
            .resource::<GalleryTrayState>()
            .hidden_windows
            .is_empty()
    {
        return;
    }
    let hidden = std::mem::take(&mut world.resource_mut::<GalleryTrayState>().hidden_windows);
    let mut count = 0;
    for entity in hidden {
        if let Some(mut window) = world.get_mut::<Window>(entity)
            && !window.visible
        {
            window.visible = true;
            count += 1;
            info!(title = %window.title, ?entity, "已提交 Gallery 窗口恢复变更");
        }
    }
    info!(window_count = count, "Gallery 已结束托盘隐藏会话");
    let primary = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .ok();
    world.resource_mut::<GalleryTrayState>().focus_after_restore = primary;
    world.write_message(RequestRedraw);
}

fn focus_restored(
    _main_thread: NonSendMarker,
    mut state: ResMut<GalleryTrayState>,
    windows: Query<&Window>,
    modals: Query<&WidgetryModalWindow>,
) {
    let Some(primary) = state.focus_after_restore.take() else {
        return;
    };
    // Visibility reaches winit in Last; focus only on the following update.
    // Leave modal focus to Widgetry instead of activating its blocked parent.
    if modals.iter().any(|modal| modal.parent == primary) {
        return;
    }
    if windows.get(primary).is_ok_and(|window| window.visible) {
        WINIT_WINDOWS.with_borrow(|windows| {
            if let Some(window) = windows.get_window(primary) {
                window.focus_window();
                info!(entity = ?primary, "已请求 Gallery 主窗口聚焦");
            }
        });
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn close_dialog_defaults_to_tray_before_first_update() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins((
            bevy_widgetry::window::WidgetryWindowPlugin,
            bevy_widgetry::button::WidgetryButtonPlugin,
            bevy_widgetry::radio_group::WidgetryRadioGroupPlugin,
        ))
        .init_resource::<GalleryTrayState>()
        .init_resource::<Language>();
        let parent = app.world_mut().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), default(), WidgetryWindowBackground::Theme,
                default(), bsn_list!{}, bsn_list!{})
        }).unwrap().id();
        let native = widgetry_window_target(app.world(), parent).unwrap();
        open_confirmation(app.world_mut(), native).unwrap();
        for _ in 0..2 {
            let choices: Vec<_> = app
                .world_mut()
                .query::<(&Choice, Has<Checked>)>()
                .iter(app.world())
                .map(|(choice, checked)| (matches!(choice, Choice::Hide), checked))
                .collect();
            assert_eq!(choices.len(), 2);
            assert!(choices.iter().all(|(hide, checked)| hide == checked));
            app.update();
        }
    }

    #[test]
    fn restore_only_uses_this_session_and_preserves_minimize_requests() {
        let mut app = App::new();
        app.init_resource::<GalleryTrayState>()
            .add_message::<WidgetryTrayClick>()
            .add_message::<RequestRedraw>()
            .add_systems(Update, restore_from_tray);
        let hidden = Window {
            visible: false,
            ..default()
        };
        let primary = app.world_mut().spawn((hidden.clone(), PrimaryWindow)).id();
        let prehidden = app.world_mut().spawn(hidden.clone()).id();
        let mut minimized = hidden;
        minimized.set_minimized(true);
        let minimized = app.world_mut().spawn(minimized).id();
        let stale = app.world_mut().spawn(Window::default()).id();
        app.world_mut().despawn(stale);
        app.world_mut()
            .resource_mut::<GalleryTrayState>()
            .hidden_windows = vec![primary, minimized, stale];
        app.world_mut().write_message(WidgetryTrayClick);
        app.update();
        assert!(app.world().get::<Window>(primary).unwrap().visible);
        assert!(!app.world().get::<Window>(prehidden).unwrap().visible);
        assert!(app.world().get::<Window>(minimized).unwrap().visible);
        assert_eq!(
            app.world_mut()
                .get_mut::<Window>(minimized)
                .unwrap()
                .internal
                .take_minimize_request(),
            Some(true)
        );
        assert!(
            app.world()
                .resource::<GalleryTrayState>()
                .hidden_windows
                .is_empty()
        );
        app.world_mut().get_mut::<Window>(primary).unwrap().visible = false;
        app.world_mut().write_message(WidgetryTrayClick);
        app.update();
        assert!(!app.world().get::<Window>(primary).unwrap().visible);
        assert!(!app.world().entities().contains(stale));
    }
}
