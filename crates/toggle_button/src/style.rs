use crate::{WidgetryToggleButton, WidgetryToggleButtonColorOverrides};
use bevy::a11y::AccessibilityNode;
use bevy::ecs::entity::EntityHashMap;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled, Pressed};
use bevy::ui_widgets::{Checkbox, CheckboxPlugin, ValueChange};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState},
};
use bevy_widgetry_theme::{WidgetryButtonStateColors, WidgetryTheme};
use std::sync::Arc;

#[derive(Default)]
pub struct WidgetryToggleButtonProps {
    pub selected: bool,
    pub colors: WidgetryToggleButtonColorOverrides,
    pub content: Option<Box<dyn Scene>>,
    pub children: Option<Box<dyn SceneList>>,
}

impl WidgetryToggleButton {
    pub(crate) fn scene(props: WidgetryToggleButtonProps) -> impl Scene {
        let WidgetryToggleButtonProps {
            selected,
            colors,
            content,
            children,
        } = props;
        bsn! {
            template(move |context| colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            Checkbox
            template(move |_| {
                let mut node = accesskit::Node::new(accesskit::Role::Button);
                node.set_toggled(if selected { accesskit::Toggled::True } else { accesskit::Toggled::False });
                Ok(AccessibilityNode(node))
            })
            @{selected.then_some(bsn! { Checked })}
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            Hovered(false)
            TabIndex(-1)
            Node {
                min_height: px(32), padding: UiRect::axes(px(12), px(6)),
                border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(4)),
                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            }
            BackgroundColor BorderColor
            template(|_| Ok(ResolvedForeground::default()))
            @{content}
            Children [{children}]
        }
    }
}

fn commit_input(
    event: On<ValueChange<bool>>,
    query: Query<(), With<WidgetryToggleButton>>,
    mut commands: Commands,
) {
    if query.contains(event.source) {
        WidgetryToggleButton::set_selected(&mut commands, event.source, event.value);
    }
}

// Checked is the sole authority, including when callers use native ECS mutations.
type StyleData = (
    &'static crate::colors::ColorState,
    &'static Hovered,
    Has<Checked>,
    Has<Pressed>,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut ResolvedForeground,
);
type StyleFilter = (
    With<WidgetryToggleButton>,
    Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryToggleButton>>,
);

fn apply(
    mode: &WidgetryTheme,
    (overrides, hovered, selected, pressed, disabled, mut background, mut border, mut foreground): <StyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let theme = mode;
    let (base, overrides) = if selected {
        let states = &theme.check_box.checked;
        let (base, overrides) = if disabled {
            (states.disabled, &overrides.0.selected_disabled)
        } else if pressed {
            (states.pressed, &overrides.0.selected_pressed)
        } else if hovered.0 {
            (states.hovered, &overrides.0.selected_hovered)
        } else {
            (states.normal, &overrides.0.selected)
        };
        (
            WidgetryButtonStateColors {
                background: base.background,
                border: base.border,
                foreground: base.mark,
            },
            overrides,
        )
    } else {
        let states = &theme.button;
        if disabled {
            (states.disabled, &overrides.0.disabled)
        } else if pressed {
            (states.pressed, &overrides.0.pressed)
        } else if hovered.0 {
            (states.hovered, &overrides.0.hovered)
        } else {
            (states.normal, &overrides.0.normal)
        }
    };
    let colors = overrides.resolve(&base);
    background.set_if_neq(BackgroundColor(colors.background));
    border.set_if_neq(BorderColor::all(colors.border));
    foreground.set_if_neq(ResolvedForeground(colors.foreground));
}

fn update_colors(
    mode: Res<WidgetryTheme>,
    mut query: Query<(Entity, StyleData), StyleFilter>,
    roots: Query<
        (
            Entity,
            Has<crate::colors::ColorState>,
            Has<Hovered>,
            Has<BackgroundColor>,
            Has<BorderColor>,
            Has<ResolvedForeground>,
        ),
        StyleFilter,
    >,
    mut failures: Local<EntityHashMap<FailureState<Vec<&'static str>>>>,
    changed: Query<
        (),
        Or<(
            Added<WidgetryToggleButton>,
            Changed<crate::colors::ColorState>,
            Changed<Hovered>,
            Added<Checked>,
            Added<Pressed>,
            Added<InteractionDisabled>,
        )>,
    >,
    mut removed_checked: RemovedComponents<Checked>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
) -> Result {
    let mut batch = DiagnosticBatch::default();
    failures.retain(|root, _| roots.contains(*root));
    for (root, overrides, hovered, background, border, foreground) in &roots {
        match query.get_mut(root) {
            Ok((_, item)) => {
                let recovered = failures.remove(&root).is_some_and(|mut state| {
                    state.recover(|| ErrorFrame::new("toggle_button.style").field("root", root))
                });
                if mode.is_changed() || changed.contains(root) || recovered {
                    apply(&mode, item);
                }
            }
            Err(source) => {
                let missing: Vec<_> = [
                    ("ColorState", overrides),
                    ("Hovered", hovered),
                    ("BackgroundColor", background),
                    ("BorderColor", border),
                    ("ResolvedForeground", foreground),
                ]
                .into_iter()
                .filter_map(|(part, present)| (!present).then_some(part))
                .collect();
                let error = failures
                    .entry(root)
                    .or_default()
                    .failure(missing.clone(), || {
                        Arc::new(crate::error::failure(
                            crate::ToggleButtonError::MissingStyle {
                                root,
                                missing,
                                source,
                            },
                            "toggle_button.style",
                        ))
                    });
                for diagnostic in error.shared_diagnostics() {
                    batch.push_shared(diagnostic);
                }
            }
        }
    }
    if mode.is_changed() {
        removed_checked.clear();
        removed_pressed.clear();
        removed_disabled.clear();
    } else {
        for entity in removed_checked
            .read()
            .chain(removed_pressed.read())
            .chain(removed_disabled.read())
        {
            if let Ok((_, item)) = query.get_mut(entity) {
                apply(&mode, item);
            }
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

pub struct WidgetryToggleButtonPlugin;
impl Plugin for WidgetryToggleButtonPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<CheckboxPlugin>() {
            app.add_plugins(CheckboxPlugin);
        }
        app.add_observer(commit_input);
        app.add_systems(PostUpdate, update_colors.in_set(WidgetryUiSystems::Colors));
        bevy_widgetry_internal_log::widgetry_debug!("WidgetryToggleButtonPlugin 注册完成");
    }
}
