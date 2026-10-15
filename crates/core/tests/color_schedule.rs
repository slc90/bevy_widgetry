#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::ecs::schedule::{IntoSystemSet, NodeId, ScheduleGraph, SystemSet};
use bevy::input_focus::InputFocusSystems;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy_widgetry_core::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use std::collections::HashSet;

fn node(graph: &ScheduleGraph, set: impl SystemSet) -> NodeId {
    NodeId::Set(graph.system_sets.get_key(set.intern()).unwrap())
}

fn precedes(graph: &ScheduleGraph, before: impl SystemSet, after: impl SystemSet) {
    let target = node(graph, after);
    let mut pending = vec![node(graph, before)];
    let mut visited = HashSet::new();
    while let Some(current) = pending.pop() {
        if current == target {
            return;
        }
        if visited.insert(current) {
            pending.extend(graph.dependency().graph().neighbors(current));
        }
    }
    assert!(
        visited.contains(&target),
        "schedule dependency does not reach {target:?}"
    );
}

#[test]
fn disabled_projection_precedes_both_input_dispatch_paths() {
    let mut app = App::new();
    app.add_plugins(WidgetryUiPlugin);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(PreUpdate).unwrap().graph();
    precedes(
        graph,
        WidgetryUiSystems::Disabled,
        PickingSystems::ProcessInput,
    );
    precedes(
        graph,
        WidgetryUiSystems::Disabled,
        InputFocusSystems::Dispatch,
    );
}

#[test]
fn materialization_and_color_outputs_precede_text_consumption() {
    let mut app = App::new();
    app.add_plugins(WidgetryUiPlugin);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(PostUpdate).unwrap().graph();
    for (before, after) in [
        (WidgetryUiSystems::Build, WidgetryUiSystems::Materialize),
        (WidgetryUiSystems::Materialize, WidgetryUiSystems::Disabled),
        (WidgetryUiSystems::Disabled, WidgetryUiSystems::StyleOwners),
        (WidgetryUiSystems::StyleOwners, WidgetryUiSystems::Colors),
        (WidgetryUiSystems::Colors, WidgetryUiSystems::ContentColors),
    ] {
        precedes(graph, before, after);
    }
    precedes(graph, WidgetryUiSystems::Colors, UiSystems::Propagate);
    precedes(
        graph,
        UiSystems::Propagate,
        WidgetryUiSystems::ContentColors,
    );
    precedes(
        graph,
        WidgetryUiSystems::Disabled,
        bevy::text::EditableTextSystems,
    );
    precedes(graph, WidgetryUiSystems::ContentColors, UiSystems::Content);
    precedes(
        graph,
        WidgetryUiSystems::ContentColors,
        bevy::text::detect_text_needs_rerender.into_system_set(),
    );
    assert!(graph.hierarchy().graph().contains_edge(
        node(graph, UiSystems::Propagate),
        node(graph, WidgetryUiSystems::Foreground),
    ));
}
