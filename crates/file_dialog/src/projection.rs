use crate::retention::{Control, Retained, Retention, cancel_if_requested, retain_result};
use crate::*;
use bevy_widgetry_job::{JobId, JobManager, JobOptions, JobReceiver, SendError, SubmitError};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ProjectionResult {
    pub snapshot: Arc<WidgetryFileDialogSnapshot>,
    pub state: WidgetryFileDialogDirectoryState,
    pub selection: WidgetryFileDialogPreparedSelection,
}

pub(crate) fn spawn(
    manager: &JobManager,
    service: Arc<Retention>,
    control: Control,
    state: Arc<WidgetryFileDialogState>,
    source: Arc<WidgetryFileDialogSnapshot>,
    status: WidgetryFileDialogDirectoryState,
) -> Result<(JobId, JobReceiver<ProjectionResult>), SubmitError> {
    manager.spawn_cpu(
        JobOptions {
            label: Some("FileDialog projection".into()),
            ..Default::default()
        },
        move |context| async move {
            if cancel_if_requested(&context) {
                return Ok(());
            }
            let snapshot = if source.token().query_revision != state.token().query_revision {
                Arc::new(
                    source
                        .reproject(state.token(), &state.query())
                        .map_err(|error| error.to_string())?,
                )
            } else {
                source
            };
            if cancel_if_requested(&context) {
                return Ok(());
            }
            let selection = state
                .projection_selection_job(snapshot.clone())
                .map_err(|error| error.to_string())?
                .prepare()
                .map_err(|error| error.to_string())?;
            let result = ProjectionResult {
                snapshot,
                state: status,
                selection,
            };
            if !retain_result(
                &context,
                &service,
                &control,
                vec![
                    Retained::Snapshot(result.snapshot.clone()),
                    Retained::Selection(result.selection.selected.clone()),
                ],
            )
            .await?
            {
                return Ok(());
            }
            match context.send(result).await {
                Ok(()) => Ok(()),
                Err(SendError::Canceled) => {
                    context.acknowledge_cancel();
                    Ok(())
                }
                Err(error) => Err(error.to_string()),
            }
        },
    )
}
