use crate::{ProgressBarFailure, WidgetryProgressBar, WidgetryProgressState};
use bevy::prelude::*;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryProgressBarHeadlessProps)]
#[require(Node)]
pub struct WidgetryProgressBarHeadless;

#[derive(Clone, Default)]
pub struct WidgetryProgressBarHeadlessProps {
    pub state: WidgetryProgressState,
}

pub struct WidgetryProgressBarHeadlessPlugin;

impl Plugin for WidgetryProgressBarHeadlessPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        app.register_type::<crate::progress::ProgressState>()
            .register_type::<WidgetryProgressState>();
        bevy_widgetry_internal_log::widgetry_debug!("WidgetryProgressBarHeadlessPlugin 注册完成");
    }
}

impl WidgetryProgressBarHeadless {
    fn scene(props: WidgetryProgressBarHeadlessProps) -> impl Scene {
        bsn! {template(move |context| {
            props.state.validate().map_err(|error| error.with_context(bevy_widgetry_diagnostics::ErrorFrame::new("progress_bar.initialize").field("root",context.entity.id())).into_bevy_error())?;
            Ok(crate::progress::ProgressState(props.state))
        })}
    }

    pub fn state(world: &World, root: Entity) -> Result<WidgetryProgressState, ProgressBarFailure> {
        WidgetryProgressBar::state(world, root)
    }
    pub fn set_state_in_world(
        world: &mut World,
        root: Entity,
        state: WidgetryProgressState,
    ) -> Result<bool, ProgressBarFailure> {
        WidgetryProgressBar::set_state_in_world(world, root, state)
    }
    pub fn set_state(commands: &mut Commands, root: Entity, state: WidgetryProgressState) {
        WidgetryProgressBar::set_state(commands, root, state);
    }
}
