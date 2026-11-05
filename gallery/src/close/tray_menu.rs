use super::{GalleryTrayState, close_requests, request_exit, restore_from_tray, restore_hidden};
use crate::i18n::{self, Language, TextKey, TrayKey};
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use bevy_widgetry::diagnostics::ErrorFrame;
use bevy_widgetry::tray::{
    TrayFailure, TrayMenuEntry, WidgetryTrayMenu, WidgetryTrayMenuClick, WidgetryTraySystems,
};

const RESTORE: &str = "gallery.restore";
const EXIT: &str = "gallery.exit";
const CHECK: &str = "demo.check";
const DISABLED: &str = "demo.disabled";
const TOGGLE_ENABLED: &str = "demo.toggle-enabled";
const RENAME: &str = "demo.rename";
const ADD: &str = "demo.add";
const REMOVE: &str = "demo.remove";
const DYNAMIC: &str = "demo.dynamic";

#[derive(Resource, Default)]
struct Demo {
    renamed: bool,
}

pub(super) fn install(app: &mut App) -> Result {
    let language = *app.world().resource::<Language>();
    app.insert_resource(initial_menu(language).map_err(menu_failure)?)
        .init_resource::<Demo>()
        .add_systems(
            Update,
            dispatch.after(restore_from_tray).before(close_requests),
        )
        .add_systems(Last, refresh_language.before(WidgetryTraySystems::Sync));
    Ok(())
}

fn label(language: Language, key: TrayKey) -> &'static str {
    i18n::tr(language, TextKey::GalleryTray(key))
}

fn initial_menu(language: Language) -> std::result::Result<WidgetryTrayMenu, TrayFailure> {
    WidgetryTrayMenu::new(vec![
        TrayMenuEntry::item(RESTORE, label(language, TrayKey::Restore)),
        TrayMenuEntry::Separator,
        TrayMenuEntry::submenu(
            label(language, TrayKey::Demo),
            vec![
                TrayMenuEntry::check(CHECK, label(language, TrayKey::Check), true),
                TrayMenuEntry::item(DISABLED, label(language, TrayKey::Disabled))
                    .with_enabled(false),
                TrayMenuEntry::item(TOGGLE_ENABLED, label(language, TrayKey::ToggleEnabled)),
                TrayMenuEntry::item(RENAME, label(language, TrayKey::Rename)),
                TrayMenuEntry::item(ADD, label(language, TrayKey::Add)),
                TrayMenuEntry::item(REMOVE, label(language, TrayKey::Remove)),
            ],
        ),
        TrayMenuEntry::Separator,
        TrayMenuEntry::item(EXIT, label(language, TrayKey::Exit)),
    ])
}

fn dispatch(world: &mut World, mut cursor: Local<MessageCursor<WidgetryTrayMenuClick>>) -> Result {
    let ids: Vec<_> = cursor
        .read(world.resource::<Messages<WidgetryTrayMenuClick>>())
        .map(|event| event.id.clone())
        .collect();
    for id in ids {
        match id.as_str() {
            RESTORE => restore_hidden(world, "tray_menu"),
            EXIT => {
                info!(
                    already_exiting = world.resource::<GalleryTrayState>().exiting,
                    "用户通过托盘菜单请求正常退出 Gallery"
                );
                request_exit(world)?;
            }
            CHECK | TOGGLE_ENABLED | RENAME | ADD | REMOVE | DISABLED | DYNAMIC => {
                if world.resource::<GalleryTrayState>().exiting {
                    info!(item = %id, "Gallery 正在退出，忽略托盘演示操作");
                } else {
                    demo_action(world, &id)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn demo_action(world: &mut World, id: &str) -> Result {
    let language = *world.resource::<Language>();
    match id {
        CHECK => {
            let checked = match world.resource::<WidgetryTrayMenu>().entry(CHECK) {
                Some(TrayMenuEntry::Check { checked, .. }) => !checked,
                _ => return Err(invalid_demo(CHECK)),
            };
            world
                .resource_mut::<WidgetryTrayMenu>()
                .set_checked(CHECK, checked)
                .map_err(menu_failure)?;
            info!(checked, "托盘复选演示已提交宿主勾选状态");
        }
        TOGGLE_ENABLED => {
            let enabled = match world.resource::<WidgetryTrayMenu>().entry(DISABLED) {
                Some(TrayMenuEntry::Item { enabled, .. }) => !enabled,
                _ => return Err(invalid_demo(DISABLED)),
            };
            world
                .resource_mut::<WidgetryTrayMenu>()
                .set_enabled(DISABLED, enabled)
                .map_err(menu_failure)?;
            info!(enabled, "托盘演示项已切换启用状态");
        }
        RENAME => {
            let renamed = !world.resource::<Demo>().renamed;
            let key = if renamed {
                TrayKey::Renamed
            } else {
                TrayKey::Rename
            };
            world
                .resource_mut::<WidgetryTrayMenu>()
                .set_text(RENAME, label(language, key))
                .map_err(menu_failure)?;
            world.resource_mut::<Demo>().renamed = renamed;
            info!(renamed, "托盘演示项已切换文字");
        }
        ADD => {
            let mut menu = world.resource_mut::<WidgetryTrayMenu>();
            let changed = if menu.entry(DYNAMIC).is_some() {
                false
            } else {
                edit_demo(&mut menu, |entries| {
                    entries.push(TrayMenuEntry::item(
                        DYNAMIC,
                        label(language, TrayKey::Dynamic),
                    ))
                })?
            };
            info!(changed, "托盘演示已处理添加动态项操作");
        }
        REMOVE => {
            let changed = edit_demo(&mut world.resource_mut::<WidgetryTrayMenu>(), |entries| {
                entries.retain(
                    |entry| !matches!(entry, TrayMenuEntry::Item { id, .. } if id == DYNAMIC),
                )
            })?;
            info!(changed, "托盘演示已处理删除动态项操作");
        }
        DISABLED | DYNAMIC => info!(item = id, "用户激活托盘演示菜单项"),
        _ => {}
    }
    Ok(())
}

fn edit_demo(
    menu: &mut WidgetryTrayMenu,
    edit: impl FnOnce(&mut Vec<TrayMenuEntry>),
) -> std::result::Result<bool, BevyError> {
    let mut root = menu.entries().to_vec();
    let Some(TrayMenuEntry::Submenu { entries, .. }) = root.get_mut(2) else {
        return Err(invalid_demo("demo submenu"));
    };
    edit(entries);
    menu.replace(root).map_err(menu_failure)
}

fn refresh_language(
    language: Res<Language>,
    demo: Res<Demo>,
    mut menu: ResMut<WidgetryTrayMenu>,
) -> Result {
    if !language.is_changed() {
        return Ok(());
    }
    menu.edit(|entries| translate(entries, *language, demo.renamed))
        .map_err(menu_failure)?;
    Ok(())
}

fn translate(entries: &mut [TrayMenuEntry], language: Language, renamed: bool) {
    for entry in entries {
        match entry {
            TrayMenuEntry::Item { id, text, .. } | TrayMenuEntry::Check { id, text, .. } => {
                let key = match id.as_str() {
                    RESTORE => TrayKey::Restore,
                    EXIT => TrayKey::Exit,
                    CHECK => TrayKey::Check,
                    DISABLED => TrayKey::Disabled,
                    TOGGLE_ENABLED => TrayKey::ToggleEnabled,
                    RENAME => {
                        if renamed {
                            TrayKey::Renamed
                        } else {
                            TrayKey::Rename
                        }
                    }
                    ADD => TrayKey::Add,
                    REMOVE => TrayKey::Remove,
                    DYNAMIC => TrayKey::Dynamic,
                    _ => continue,
                };
                *text = label(language, key).into();
            }
            TrayMenuEntry::Submenu { text, entries, .. } => {
                *text = label(language, TrayKey::Demo).into();
                translate(entries, language, renamed);
            }
            TrayMenuEntry::Separator => {}
        }
    }
}

fn menu_failure(failure: TrayFailure) -> BevyError {
    failure
        .with_context(ErrorFrame::new("gallery.tray.menu"))
        .into_bevy_error()
}

fn invalid_demo(id: &str) -> BevyError {
    crate::diagnostics::at(
        "Gallery tray demo menu structure is invalid",
        ErrorFrame::new("gallery.tray.menu").field("item", id),
    )
}

#[cfg(test)]
mod tests;
