use crate::text_field::{Parts, TextFieldEditor};
use bevy::{
    prelude::*,
    text::{EditableText, FontCx, LayoutCx},
};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoSize {
    pub min_rows: usize,
    pub max_rows: usize,
}

impl Default for AutoSize {
    fn default() -> Self {
        Self {
            min_rows: 2,
            max_rows: 6,
        }
    }
}

#[derive(Component)]
pub(crate) struct Configuration {
    rows: Option<AutoSize>,
    height_failure: FailureState,
}

impl Configuration {
    pub(crate) fn new(multiline: bool, config: Option<AutoSize>) -> Result<Self, BevyError> {
        if let Some(config) = config
            && (!multiline || config.min_rows == 0 || config.min_rows > config.max_rows)
        {
            widgetry_error!(
                multiline,
                min_rows = config.min_rows,
                max_rows = config.max_rows,
                "TextField AutoSize 配置无效"
            );
            return Err(BevyError::error(
                "TextField AutoSize 要求 multiline 且 1 <= min_rows <= max_rows",
            ));
        }
        Ok(Self {
            rows: config,
            height_failure: FailureState::default(),
        })
    }
}

pub(crate) fn resize_editors(
    mut roots: Query<(Entity, &mut Configuration, &Parts, &Node)>,
    mut editors: Query<(&mut EditableText, &TextFieldEditor)>,
    mut fonts: ResMut<FontCx>,
    mut layouts: ResMut<LayoutCx>,
) -> Result {
    let mut failure = None;
    for (root, mut state, parts, node) in &mut roots {
        let Some(config) = state.rows else { continue };
        let result = if node.height == Val::Auto {
            Ok(())
        } else {
            Err(BevyError::error(
                "TextField AutoSize 要求根 Node.height 为 Auto",
            ))
        };
        let mut new_failure = false;
        if let Err(error) = state.height_failure.observe(
            result,
            |error| {
                new_failure = true;
                widgetry_error!(?root, %error, "TextField AutoSize 高度冲突");
            },
            || widgetry_info!(?root, "TextField AutoSize 高度恢复正常"),
        ) {
            if new_failure {
                failure = Some(error);
            }
            continue;
        }
        let (mut edit, owner) = editors.get_mut(parts.editor).map_err(|_| {
            widgetry_error!(?root, "TextField AutoSize 编辑器缺失");
            BevyError::error("TextField AutoSize 编辑器缺失")
        })?;
        if owner.root != root {
            widgetry_error!(?root, "TextField AutoSize 编辑器身份不匹配");
            return Err(BevyError::error("TextField AutoSize 编辑器身份不匹配"));
        }
        let lines = edit
            .editor_mut()
            .layout(&mut fonts, &mut layouts.0)
            .lines()
            .count();
        let rows = lines.clamp(config.min_rows, config.max_rows) as f32;
        if edit.visible_lines != Some(rows) {
            edit.visible_lines = Some(rows);
        }
    }
    failure.map_or(Ok(()), Err)
}
