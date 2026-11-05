use crate::{TrayError, TrayFailure};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
use std::collections::HashSet;
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetryTrayMenu {
    entries: Vec<TrayMenuEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayMenuEntry {
    Item {
        id: String,
        text: String,
        enabled: bool,
    },
    Check {
        id: String,
        text: String,
        enabled: bool,
        checked: bool,
    },
    Separator,
    Submenu {
        text: String,
        enabled: bool,
        entries: Vec<Self>,
    },
}

impl TrayMenuEntry {
    pub fn item(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Item {
            id: id.into(),
            text: text.into(),
            enabled: true,
        }
    }

    pub fn submenu(text: impl Into<String>, entries: Vec<Self>) -> Self {
        Self::Submenu {
            text: text.into(),
            enabled: true,
            entries,
        }
    }

    pub fn check(id: impl Into<String>, text: impl Into<String>, checked: bool) -> Self {
        Self::Check {
            id: id.into(),
            text: text.into(),
            enabled: true,
            checked,
        }
    }

    pub fn with_enabled(mut self, value: bool) -> Self {
        match &mut self {
            Self::Item { enabled, .. }
            | Self::Check { enabled, .. }
            | Self::Submenu { enabled, .. } => *enabled = value,
            Self::Separator => {}
        }
        self
    }
}

impl WidgetryTrayMenu {
    pub fn new(entries: Vec<TrayMenuEntry>) -> Result<Self, TrayFailure> {
        fn validate(
            entries: &[TrayMenuEntry],
            ids: &mut HashSet<String>,
        ) -> Result<(), TrayFailure> {
            for entry in entries {
                match entry {
                    TrayMenuEntry::Item { id, .. } | TrayMenuEntry::Check { id, .. }
                        if !ids.insert(id.clone()) =>
                    {
                        return Err(TrayFailure::new(
                            TrayError::DuplicateMenuId { id: id.clone() },
                            ErrorFrame::new("validate tray menu").field("item", id),
                        ));
                    }
                    TrayMenuEntry::Submenu { entries, .. } => validate(entries, ids)?,
                    _ => {}
                }
            }
            Ok(())
        }
        validate(&entries, &mut HashSet::new())?;
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[TrayMenuEntry] {
        &self.entries
    }

    pub fn entry(&self, id: &str) -> Option<&TrayMenuEntry> {
        fn find<'a>(entries: &'a [TrayMenuEntry], id: &str) -> Option<&'a TrayMenuEntry> {
            entries.iter().find_map(|entry| match entry {
                TrayMenuEntry::Item { id: key, .. } | TrayMenuEntry::Check { id: key, .. }
                    if key == id =>
                {
                    Some(entry)
                }
                TrayMenuEntry::Submenu { entries, .. } => find(entries, id),
                _ => None,
            })
        }
        find(&self.entries, id)
    }

    pub(crate) fn accepts_activation(&self, id: &str) -> bool {
        fn accepts(entries: &[TrayMenuEntry], id: &str) -> bool {
            entries.iter().any(|entry| match entry {
                TrayMenuEntry::Item {
                    id: key, enabled, ..
                }
                | TrayMenuEntry::Check {
                    id: key, enabled, ..
                } => key == id && *enabled,
                TrayMenuEntry::Submenu {
                    enabled, entries, ..
                } => *enabled && accepts(entries, id),
                TrayMenuEntry::Separator => false,
            })
        }
        accepts(&self.entries, id)
    }

    pub fn replace(&mut self, entries: Vec<TrayMenuEntry>) -> Result<bool, TrayFailure> {
        let next = Self::new(entries)?;
        if *self == next {
            return Ok(false);
        }
        *self = next;
        Ok(true)
    }

    pub fn edit(
        &mut self,
        edit: impl FnOnce(&mut Vec<TrayMenuEntry>),
    ) -> Result<bool, TrayFailure> {
        let mut next = self.entries.clone();
        edit(&mut next);
        self.replace(next)
    }

    pub fn set_text(&mut self, id: &str, text: impl Into<String>) -> Result<bool, TrayFailure> {
        let text = text.into();
        self.update_item(id, |entry| {
            match entry {
                TrayMenuEntry::Item { text: value, .. }
                | TrayMenuEntry::Check { text: value, .. } => *value = text,
                _ => {}
            }
            Ok(())
        })
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<bool, TrayFailure> {
        self.update_item(id, |entry| {
            match entry {
                TrayMenuEntry::Item { enabled: value, .. }
                | TrayMenuEntry::Check { enabled: value, .. } => *value = enabled,
                _ => {}
            }
            Ok(())
        })
    }

    pub fn set_checked(&mut self, id: &str, checked: bool) -> Result<bool, TrayFailure> {
        self.update_item(id, |entry| match entry {
            TrayMenuEntry::Check { checked: value, .. } => {
                *value = checked;
                Ok(())
            }
            _ => Err(TrayFailure::new(
                TrayError::NotCheckItem { id: id.into() },
                ErrorFrame::new("set tray menu check state").field("item", id),
            )),
        })
    }

    fn update_item(
        &mut self,
        id: &str,
        update: impl FnOnce(&mut TrayMenuEntry) -> Result<(), TrayFailure>,
    ) -> Result<bool, TrayFailure> {
        fn find<'a>(entries: &'a mut [TrayMenuEntry], id: &str) -> Option<&'a mut TrayMenuEntry> {
            entries.iter_mut().find_map(|entry| match entry {
                TrayMenuEntry::Item { id: key, .. } | TrayMenuEntry::Check { id: key, .. }
                    if key == id =>
                {
                    Some(entry)
                }
                TrayMenuEntry::Submenu { entries, .. } => find(entries, id),
                _ => None,
            })
        }
        let mut next = self.entries.clone();
        let entry = find(&mut next, id).ok_or_else(|| {
            TrayFailure::new(
                TrayError::MissingMenuItem { id: id.into() },
                ErrorFrame::new("update tray menu item").field("item", id),
            )
        })?;
        update(entry)?;
        self.replace(next)
    }
}

pub(crate) struct NativeMenu {
    pub(crate) root: Menu,
    pub(crate) routes: Vec<(MenuId, String, bool)>,
}

impl NativeMenu {
    pub(crate) fn new(model: &WidgetryTrayMenu) -> Result<Self, TrayFailure> {
        let mut menu = Self {
            root: Menu::new(),
            routes: Vec::new(),
        };
        for entry in model.entries() {
            let item = build_entry(entry, &mut menu.routes)?;
            menu.root.append(item.as_ref()).map_err(menu_failure)?;
        }
        crate::native::verify_menu(&menu.root, model.entries())?;
        Ok(menu)
    }
}

fn build_entry(
    entry: &TrayMenuEntry,
    routes: &mut Vec<(MenuId, String, bool)>,
) -> Result<Box<dyn IsMenuItem>, TrayFailure> {
    Ok(match entry {
        TrayMenuEntry::Item { id, text, enabled } => {
            let item = MenuItem::new(text, *enabled, None);
            routes.push((item.id().clone(), id.clone(), false));
            Box::new(item)
        }
        TrayMenuEntry::Check {
            id,
            text,
            enabled,
            checked,
        } => {
            let item = CheckMenuItem::new(text, *enabled, *checked, None);
            routes.push((item.id().clone(), id.clone(), true));
            Box::new(item)
        }
        TrayMenuEntry::Separator => Box::new(PredefinedMenuItem::separator()),
        TrayMenuEntry::Submenu {
            text,
            entries,
            enabled,
        } => {
            let submenu = Submenu::new(text, *enabled);
            for entry in entries {
                let item = build_entry(entry, routes)?;
                submenu.append(item.as_ref()).map_err(menu_failure)?;
            }
            Box::new(submenu)
        }
    })
}

fn menu_failure(source: tray_icon::menu::Error) -> TrayFailure {
    TrayFailure::new(
        TrayError::Menu { source },
        ErrorFrame::new("create tray menu"),
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn model_edits_are_atomic_and_update_nested_items_by_stable_id() {
        let mut menu = WidgetryTrayMenu::new(vec![TrayMenuEntry::submenu(
            "More",
            vec![
                TrayMenuEntry::item("open", "Open"),
                TrayMenuEntry::check("mute", "Mute", false),
            ],
        )])
        .unwrap();
        assert!(menu.set_text("open", "Restore").unwrap());
        assert!(!menu.set_text("open", "Restore").unwrap());
        assert!(menu.set_enabled("open", false).unwrap());
        assert!(menu.set_checked("mute", true).unwrap());
        assert!(matches!(
            menu.entry("mute"),
            Some(TrayMenuEntry::Check { checked: true, .. })
        ));
        let before = menu.clone();
        assert!(
            menu.edit(|entries| entries.push(TrayMenuEntry::item("mute", "Duplicate")))
                .is_err()
        );
        assert_eq!(menu, before);
        assert!(menu.set_checked("open", true).is_err());
        assert!(menu.set_text("absent", "Missing").is_err());
        assert_eq!(menu, before);
        assert!(
            menu.edit(|entries| {
                if let TrayMenuEntry::Submenu { entries, .. } = &mut entries[0] {
                    entries.insert(1, TrayMenuEntry::Separator);
                    entries.push(TrayMenuEntry::item("dynamic", "Dynamic"));
                    entries.swap(0, 2);
                }
            })
            .unwrap()
        );
        assert!(menu.entry("dynamic").is_some());
        assert!(menu.edit(|entries| entries.clear()).unwrap());
        assert!(menu.entries().is_empty());
    }

    #[test]
    fn duplicate_business_ids_are_rejected_across_submenus() {
        let failure = WidgetryTrayMenu::new(vec![
            TrayMenuEntry::item("open", "Open"),
            TrayMenuEntry::submenu("More", vec![TrayMenuEntry::item("open", "Again")]),
        ])
        .unwrap_err();
        assert!(matches!(failure.kind(), TrayError::DuplicateMenuId { id } if id == "open"));
    }

    #[test]
    fn native_tree_preserves_order_and_only_routes_actions() {
        let model = WidgetryTrayMenu::new(vec![
            TrayMenuEntry::item("open", "Open"),
            TrayMenuEntry::Separator,
            TrayMenuEntry::submenu("More", vec![TrayMenuEntry::item("quit", "Quit")]),
        ])
        .unwrap();
        let native = NativeMenu::new(&model).unwrap();
        let items = native.root.items();
        assert_eq!(items[0].as_menuitem().unwrap().text(), "Open");
        assert!(items[1].as_predefined_menuitem().is_some());
        let submenu = items[2].as_submenu().unwrap();
        assert_eq!(submenu.text(), "More");
        assert_eq!(submenu.items()[0].as_menuitem().unwrap().text(), "Quit");
        assert_eq!(native.routes.len(), 2);
        assert_eq!(native.routes[0].1, "open");
        assert_eq!(native.routes[1].1, "quit");
        let other = NativeMenu::new(&model).unwrap();
        assert_ne!(native.routes[0].0, other.routes[0].0);
    }
}
