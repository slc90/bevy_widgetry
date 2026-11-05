use crate::{TrayError, TrayFailure};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
use std::collections::HashSet;
use tray_icon::menu::{IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetryTrayMenu {
    entries: Vec<TrayMenuEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayMenuEntry {
    Item { id: String, text: String },
    Separator,
    Submenu { text: String, entries: Vec<Self> },
}

impl TrayMenuEntry {
    pub fn item(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Item {
            id: id.into(),
            text: text.into(),
        }
    }

    pub fn submenu(text: impl Into<String>, entries: Vec<Self>) -> Self {
        Self::Submenu {
            text: text.into(),
            entries,
        }
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
                    TrayMenuEntry::Item { id, .. } if !ids.insert(id.clone()) => {
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
}

pub(crate) struct NativeMenu {
    pub(crate) root: Menu,
    pub(crate) routes: Vec<(MenuId, String)>,
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
    routes: &mut Vec<(MenuId, String)>,
) -> Result<Box<dyn IsMenuItem>, TrayFailure> {
    Ok(match entry {
        TrayMenuEntry::Item { id, text } => {
            let item = MenuItem::new(text, true, None);
            routes.push((item.id().clone(), id.clone()));
            Box::new(item)
        }
        TrayMenuEntry::Separator => Box::new(PredefinedMenuItem::separator()),
        TrayMenuEntry::Submenu { text, entries } => {
            let submenu = Submenu::new(text, true);
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
