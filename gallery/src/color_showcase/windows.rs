use super::overrides::{PURPLE, TEAL};
use crate::assets::GalleryImage;
use crate::i18n::ColorShowcaseKey;
use crate::i18n::WindowKey;
use crate::i18n::{
    Language, LocalizedText, LocalizedWindowTitle, TextKey, localized, localized_binding,
};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::PrimaryWindow;
use bevy_widgetry::{
    button::WidgetryButton,
    file_dialog::{WidgetryFileDialog, WidgetryFileDialogColorOverrides, WidgetryFileDialogWindow},
    message_box::{
        WidgetryMessageBoxButtons, WidgetryMessageBoxColorOverrides, widgetry_message_box,
    },
    scene::WidgetrySceneCommandsExt,
    text::WidgetryText,
    theme::WidgetryThemeMode,
    window::{
        WidgetryWindowBackground, WidgetryWindowImageBackground, WidgetryWindowImageMode,
        owned_widgetry_window,
    },
};

#[derive(Clone, Copy, Debug)]
enum Kind {
    Window,
    Image,
    MessageBox,
    FileDialog,
}

#[derive(Component)]
struct Examples {
    kind: Kind,
    pure: Option<Entity>,
    sample: Option<Entity>,
    applied: bool,
}

#[derive(Component, Clone, Copy)]
struct Action {
    owner: Option<Entity>,
    operation: Operation,
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    OpenPure,
    OpenSample,
    Apply,
    Clear,
    Theme,
}

pub(super) fn scene() -> impl Scene {
    bsn! { Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(16) } Children [
        @example(Kind::Window,TextKey::Window(WindowKey::WindowOnlyFrameNormalBorder7C3AedClear))--
        @example(Kind::Image,TextKey::Window(WindowKey::WindowImageSameBorderOverrideCover50))--
        @example(Kind::MessageBox,TextKey::Window(WindowKey::MessageboxOnlyBodyNormalBackground0F766EApply))--
        @example(Kind::FileDialog,TextKey::Window(WindowKey::FiledialogOnlyEntrySelectedBackground0F766ENonmodal))
    ] }
}

fn example(kind: Kind, label: TextKey) -> impl Scene {
    bsn! {
        template(move |_| Ok(Examples { kind, pure: None, sample: None, applied: false }))
        Name({format!("ColorWindow:{kind:?}")})
        on(cleanup)
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            @localized(label) WidgetryText--
            Node { column_gap: px(8), flex_wrap: FlexWrap::Wrap, row_gap: px(6) } Children [
                @action(None, Operation::OpenPure,TextKey::Window(WindowKey::OpenPureTheme))--
                @action(None, Operation::OpenSample,TextKey::Window(WindowKey::OpenOverrideInstance))--
                @action(None, Operation::Apply,TextKey::ColorShowcase(ColorShowcaseKey::ApplySampleOverrides))--
                @action(None, Operation::Clear,TextKey::ColorShowcase(ColorShowcaseKey::ClearOverrides))
            ]
        ]
    }
}

fn action(owner: Option<Entity>, operation: Operation, label: TextKey) -> impl Scene {
    bsn! {
        @WidgetryButton template(move |_| Ok(Action { owner, operation }))
        Name({format!("ColorWindowAction:{operation:?}")})
        on(operate) Children [@localized(label) WidgetryText]
    }
}

fn operate(event: On<Activate>, actions: Query<&Action>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity).copied() else {
        return;
    };
    let button = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        if matches!(action.operation, Operation::Theme) {
            let mode = match *world.resource::<WidgetryThemeMode>() {
                WidgetryThemeMode::Dark => WidgetryThemeMode::Light,
                WidgetryThemeMode::Light => WidgetryThemeMode::PinkDream,
                WidgetryThemeMode::PinkDream => WidgetryThemeMode::KamuriViolet,
                WidgetryThemeMode::KamuriViolet => WidgetryThemeMode::Dark,
            };
            WidgetryThemeMode::set_in_world(world, mode)?;
            info!(?mode, "从颜色示例窗口切换 Theme");
            return Ok(());
        }
        let mut owner = action.owner.unwrap_or(button);
        while world.get::<Examples>(owner).is_none() {
            let Some(parent) = world.get::<ChildOf>(owner) else {
                return Ok(());
            };
            owner = parent.parent();
        }
        let Some(examples) = world.get::<Examples>(owner) else {
            return Ok(());
        };
        let kind = examples.kind;
        let sample = examples.sample;
        let pure = examples.pure;
        let applied = examples.applied;
        match action.operation {
            Operation::Apply | Operation::Clear => {
                let apply = matches!(action.operation, Operation::Apply);
                if let Some(root) = sample
                    && world.get_entity(root).is_ok()
                {
                    colors(world, root, kind, apply)?;
                }
                if let Some(mut examples) = world.get_mut::<Examples>(owner) {
                    examples.applied = apply;
                }
                info!(?kind, apply, root = ?sample, "设置窗口示例颜色覆盖");
            }
            Operation::OpenPure | Operation::OpenSample => {
                let is_sample = matches!(action.operation, Operation::OpenSample);
                let existing = if is_sample { sample } else { pure };
                if existing.is_some_and(|root| world.get_entity(root).is_ok()) {
                    return Ok(());
                }
                let root = open(world, owner, kind, is_sample)?;
                if let Some(mut examples) = world.get_mut::<Examples>(owner) {
                    if is_sample {
                        examples.sample = Some(root);
                    } else {
                        examples.pure = Some(root);
                    }
                }
                world.commands().queue(move |world: &mut World| -> Result {
                    if world.get::<Examples>(owner).is_none() {
                        if let Ok(root) = world.get_entity_mut(root) {
                            root.despawn();
                        }
                        return Ok(());
                    }
                    if is_sample && applied {
                        colors(world, root, kind, true)?;
                    }
                    info!(?kind, ?root, is_sample, "打开颜色展示窗口");
                    Ok(())
                });
            }
            Operation::Theme => {}
        }
        Ok(())
    });
}

fn open(world: &mut World, owner: Entity, kind: Kind, sample: bool) -> Result<Entity, BevyError> {
    let binding = LocalizedText::with_args(
        TextKey::Window(WindowKey::OverrideWindowTitle),
        vec![("kind", format!("{kind:?}"))],
    )
    .with_key_arg(
        "sample",
        if sample {
            TextKey::Window(WindowKey::OverrideInstance)
        } else {
            TextKey::ColorShowcase(ColorShowcaseKey::PureTheme)
        },
    );
    let title = binding.render(*world.resource::<Language>());
    let content: Box<dyn SceneList> = Box::new(bsn_list! {
        @localized(TextKey::ColorShowcase(ColorShowcaseKey::OtherColorsFollowThemeClearOnlyAffects)) WidgetryText--
        Node { column_gap: px(8), flex_wrap: FlexWrap::Wrap, row_gap: px(6) } Children [
            @action(Some(owner), Operation::Apply,TextKey::ColorShowcase(ColorShowcaseKey::ApplySampleOverrides))--
            @action(Some(owner), Operation::Clear,TextKey::ColorShowcase(ColorShowcaseKey::ClearOverrides))--
            @action(Some(owner), Operation::Theme,TextKey::Window(WindowKey::CycleTheme))
        ]
    });
    let scene: Box<dyn Scene> = match kind {
        Kind::Window | Kind::Image => {
            let background = if matches!(kind, Kind::Image) {
                WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
                    image: world
                        .resource::<AssetServer>()
                        .load(GalleryImage::WindowBackground1.path()),
                    mode: WidgetryWindowImageMode::Cover,
                    opacity: 0.5,
                })
            } else {
                WidgetryWindowBackground::Theme
            };
            Box::new(
                bsn! { @owned_widgetry_window(Window { title: title.clone(), resolution: (640, 400).into(), ..default() }, default(), background, default(), bsn_list!{@localized_binding(binding.clone()) WidgetryText}, content) },
            )
        }
        Kind::MessageBox => {
            let parent = world
                .query_filtered::<Entity, With<PrimaryWindow>>()
                .single(world)
                .map_err(|error| {
                    error!(%error, "颜色展示缺少主窗口");
                    BevyError::error(format!("颜色展示缺少主窗口：{error}"))
                })?;
            Box::new(
                bsn! { @widgetry_message_box(parent, title, WidgetryMessageBoxButtons::Ok, default(), content) },
            )
        }
        Kind::FileDialog => Box::new(bsn! {
            @WidgetryFileDialog {
                @initial_directory: {Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")))},
                @window: {Some(WidgetryFileDialogWindow { native: Window { title, ..WidgetryFileDialogWindow::default().native }, ..default() })},
            }
        }),
    };
    Ok(world
        .commands()
        .spawn_scene_with_error_handler(
            bsn! { @{scene} Name({format!("ColorWindowInstance:{kind:?}:{sample}")}) template(move |_| Ok(LocalizedWindowTitle(binding.clone()))) },
        )
        .id())
}

fn colors(world: &mut World, root: Entity, kind: Kind, apply: bool) -> Result<bool, BevyError> {
    match kind {
        Kind::Window | Kind::Image => {
            if !apply {
                return bevy_widgetry::window::WidgetryWindowColorOverrides::clear_in_world(
                    world, root,
                );
            }
            let mut colors = bevy_widgetry::window::WidgetryWindowColorOverrides::default();
            colors.frame.normal.border = Some(PURPLE);
            bevy_widgetry::window::WidgetryWindowColorOverrides::set_in_world(world, root, colors)
        }
        Kind::MessageBox => {
            if !apply {
                return WidgetryMessageBoxColorOverrides::clear_in_world(world, root);
            }
            let mut colors = WidgetryMessageBoxColorOverrides::default();
            colors.body.normal.background = Some(TEAL);
            WidgetryMessageBoxColorOverrides::set_in_world(world, root, colors)
        }
        Kind::FileDialog => {
            if !apply {
                return WidgetryFileDialogColorOverrides::clear_in_world(world, root);
            }
            let mut colors = WidgetryFileDialogColorOverrides::default();
            colors.entry.selected.background = Some(TEAL);
            WidgetryFileDialogColorOverrides::set_in_world(world, root, colors)
        }
    }
}

fn cleanup(
    event: On<bevy::ecs::lifecycle::DespawnEvent>,
    examples: Query<&Examples>,
    mut commands: Commands,
) {
    let Ok(examples) = examples.get(event.entity) else {
        return;
    };
    for root in [examples.pure, examples.sample].into_iter().flatten() {
        commands.queue(move |world: &mut World| {
            if let Ok(root) = world.get_entity_mut(root) {
                root.despawn();
            }
        });
    }
}
