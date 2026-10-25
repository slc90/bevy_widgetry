use crate::worker::{Control, CpuSendError, CpuWork, Service};
use crate::*;
use bevy_widgetry_job::{
    JobContext, JobId, JobManager, JobOptions, JobReceiver, SendError, SubmitError,
};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct ProjectionResult {
    pub snapshot: Arc<WidgetryFileDialogSnapshot>,
    pub state: WidgetryFileDialogDirectoryState,
    pub selection: WidgetryFileDialogPreparedSelection,
}

pub(crate) fn spawn(
    manager: &JobManager,
    service: Arc<Service>,
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
            if canceled(&context) {
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
            if canceled(&context) {
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
            if !retain(&context, &service, &control, &result).await? {
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

fn canceled(context: &JobContext<ProjectionResult>) -> bool {
    if context.is_cancelled() {
        context.acknowledge_cancel();
        true
    } else {
        false
    }
}

async fn retain(
    context: &JobContext<ProjectionResult>,
    service: &Service,
    control: &Control,
    result: &ProjectionResult,
) -> Result<bool, String> {
    loop {
        if canceled(context) {
            return Ok(false);
        }
        if control.stopped(service) {
            return Err("FileDialog projection service stopped".into());
        }
        let capacity = service.capacity.listen();
        let (reply, ready) = futures_channel::oneshot::channel();
        let mut work = CpuWork::RetainProjection(control.clone(), result.clone(), reply);
        loop {
            if canceled(context) {
                return Ok(false);
            }
            if control.stopped(service) {
                return Err("FileDialog projection service stopped".into());
            }
            match service.try_send_cpu(work) {
                Ok(()) => break,
                Err(CpuSendError::Busy(returned)) => {
                    work = *returned;
                    futures_timer::Delay::new(Duration::from_millis(1)).await;
                }
                Err(CpuSendError::Closed) => {
                    return Err("FileDialog projection retention channel closed".into());
                }
                Err(CpuSendError::Fault(error)) => return Err(error),
            }
        }
        // Publish only after the background owner holds the large output through UI and App teardown.
        match ready.await {
            Ok(true) => return Ok(!canceled(context)),
            Ok(false) => {
                bevy::tasks::futures_lite::future::race(context.cancelled(), capacity).await;
            }
            Err(error) => {
                if canceled(context) {
                    return Ok(false);
                }
                return Err(error.to_string());
            }
        }
    }
}
