use crate::retention::{Control, Retained, Retention, cancel_if_requested, retain_result};
use crate::*;
use bevy_widgetry_job::{JobId, JobManager, JobOptions, JobReceiver, SendError, SubmitError};
use std::sync::Arc;

pub(crate) fn spawn(
    manager: &JobManager,
    service: Arc<Retention>,
    control: Control,
    job: WidgetryFileDialogSelectionJob,
) -> Result<(JobId, JobReceiver<WidgetryFileDialogPreparedSelection>), SubmitError> {
    manager.spawn_cpu(
        JobOptions {
            label: Some("FileDialog selection".into()),
            ..Default::default()
        },
        move |context| async move {
            if cancel_if_requested(&context) {
                return Ok(());
            }
            let prepared = job.prepare().map_err(|error| error.to_string())?;
            let retained = vec![
                Retained::Snapshot(prepared.snapshot.clone()),
                Retained::Selection(prepared.selected.clone()),
            ];
            if !retain_result(&context, &service, &control, retained).await? {
                return Ok(());
            }
            match context.send(prepared).await {
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
