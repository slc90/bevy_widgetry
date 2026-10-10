use super::overrides::{PURPLE, TEAL};
use crate::assets::GalleryImage;
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
        @example(Kind::Window, "Window：仅 frame.normal.border=#7C3AED；已打开实例可直接清除。")--
        @example(Kind::Image, "Window Image：同一 border 覆盖；Cover / 50% opacity 不因切换 Theme 或清除覆盖改变。")--
        @example(Kind::MessageBox, "MessageBox：仅 body.normal.background=#0F766E；modal 内提供应用 / 清除和 Theme 切换。")--
        @example(Kind::FileDialog, "FileDialog：仅 entry.selected.background=#0F766E；NonModal，选中文件后清除，无需重新打开。")
    ] }
}

fn example(kind: Kind, label: &'static str) -> impl Scene {
    bsn! {
        template(move |_| Ok(Examples { kind, pure: None, sample: None, applied: false }))
        Name({format!("ColorWindow:{kind:?}")})
        on(cleanup)
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text(label) WidgetryText--
            Node { column_gap: px(8), flex_wrap: FlexWrap::Wrap, row_gap: px(6) } Children [
                @action(None, Operation::OpenPure, "打开纯 Theme")--
                @action(None, Operation::OpenSample, "打开覆盖实例")--
                @action(None, Operation::Apply, "应用示例覆盖")--
                @action(None, Operation::Clear, "清除覆盖")
            ]
        ]
    }
}

fn action(owner: Option<Entity>, operation: Operation, label: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton template(move |_| Ok(Action { owner, operation }))
        Name({format!("ColorWindowAction:{operation:?}")})
        on(operate) Children [Text(label) WidgetryText]
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
    let title = format!(
        "{kind:?} · {}",
        if sample { "覆盖实例" } else { "纯 Theme" }
    );
    let content: Box<dyn SceneList> = Box::new(bsn_list! {
        Text("其他颜色继续跟随 Theme；清除只操作已打开的覆盖实例。") WidgetryText--
        Node { column_gap: px(8), flex_wrap: FlexWrap::Wrap, row_gap: px(6) } Children [
            @action(Some(owner), Operation::Apply, "应用示例覆盖")--
            @action(Some(owner), Operation::Clear, "清除覆盖")--
            @action(Some(owner), Operation::Theme, "循环切换 Theme")
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
                bsn! { @owned_widgetry_window(Window { title: title.clone(), resolution: (640, 400).into(), ..default() }, default(), background, default(), bsn_list!{Text(title) WidgetryText}, content) },
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
            bsn! { @{scene} Name({format!("ColorWindowInstance:{kind:?}:{sample}")}) },
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
