use crate::{
    FileDialogFailure, FileDialogOperationFailure, WidgetryFileDialogAction,
    WidgetryFileDialogFilter, WidgetryFileDialogHeadless, WidgetryFileDialogHeadlessPlugin,
    WidgetryFileDialogHeadlessProps, WidgetryFileDialogMode, WidgetryFileDialogReply,
    WidgetryFileDialogState,
};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_debug;
use std::path::PathBuf;

#[derive(SceneComponent, FromTemplate, Default)]
#[scene(WidgetryFileDialogProps)]
#[require(crate::colors::ColorState, WidgetryFileDialogHeadless)]
pub struct WidgetryFileDialog;

pub struct WidgetryFileDialogPlugin;

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogProps {
    pub colors: crate::WidgetryFileDialogColorOverrides,
    pub mode: WidgetryFileDialogMode,
    pub initial_directory: Option<PathBuf>,
    pub fallback_directory: Option<PathBuf>,
    pub storage_scope: Option<String>,
    pub filters: Vec<WidgetryFileDialogFilter>,
    pub allow_different_extension: bool,
    pub window: Option<crate::WidgetryFileDialogWindow>,
}

impl Default for WidgetryFileDialogProps {
    fn default() -> Self {
        let WidgetryFileDialogHeadlessProps {
            mode,
            initial_directory,
            fallback_directory,
            storage_scope,
            filters,
            allow_different_extension,
        } = WidgetryFileDialogHeadlessProps::default();
        Self {
            colors: default(),
            mode,
            initial_directory,
            fallback_directory,
            storage_scope,
            filters,
            allow_different_extension,
            window: None,
        }
    }
}

impl From<WidgetryFileDialogProps> for WidgetryFileDialogHeadlessProps {
    fn from(props: WidgetryFileDialogProps) -> Self {
        Self {
            mode: props.mode,
            initial_directory: props.initial_directory,
            fallback_directory: props.fallback_directory,
            storage_scope: props.storage_scope,
            filters: props.filters,
            allow_different_extension: props.allow_different_extension,
        }
    }
}

impl WidgetryFileDialogState {
    pub fn new(props: WidgetryFileDialogProps) -> Result<Self, FileDialogFailure> {
        Self::from_headless_props(props.into())
    }
}

impl Plugin for WidgetryFileDialogPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryFileDialogHeadlessPlugin>() {
            app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        }
        crate::style::install(app);
        crate::window::install(app);
        widgetry_debug!("WidgetryFileDialogPlugin 注册完成");
    }
}

impl WidgetryFileDialog {
    fn scene(props: WidgetryFileDialogProps) -> impl Scene {
        let colors = props.colors.clone();
        let window = props.window.clone();
        let behavior = WidgetryFileDialogHeadlessProps::from(props);
        (
            bsn! {
                template(move |_| colors.clone().initial())
                WidgetryFileDialog
                @WidgetryFileDialogHeadless {
                    @mode: {behavior.mode},
                    @initial_directory: {behavior.initial_directory},
                    @fallback_directory: {behavior.fallback_directory},
                    @storage_scope: {behavior.storage_scope},
                    @filters: {behavior.filters},
                    @allow_different_extension: {behavior.allow_different_extension},
                }
            },
            crate::window::scene(window),
        )
    }

    pub fn apply(
        world: &mut World,
        root: Entity,
        action: WidgetryFileDialogAction,
    ) -> Result<bool, FileDialogOperationFailure> {
        WidgetryFileDialogHeadless::apply(world, root, action)
    }

    pub fn queue(commands: &mut Commands, root: Entity, action: WidgetryFileDialogAction) {
        WidgetryFileDialogHeadless::queue(commands, root, action);
    }

    pub fn flush_preferences(world: &mut World) -> Result<(), FileDialogOperationFailure> {
        WidgetryFileDialogHeadless::flush_preferences(world)
    }

    pub fn deliver(
        world: &mut World,
        root: Entity,
        reply: WidgetryFileDialogReply,
    ) -> Result<bool, FileDialogOperationFailure> {
        WidgetryFileDialogHeadless::deliver(world, root, reply)
    }
}
