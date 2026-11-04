use crate::*;
use bevy_widgetry_internal_log::widgetry_debug;

pub(crate) struct Lifecycle {
    pub status: JobStatus,
    pub cancel_requested: bool,
    started: bool,
    executing: bool,
    acknowledged: bool,
}

impl Lifecycle {
    pub fn new(status: JobStatus) -> Self {
        Self {
            status,
            cancel_requested: false,
            started: false,
            executing: false,
            acknowledged: false,
        }
    }

    pub fn start(&mut self) -> bool {
        if self.status.state != JobState::Pending {
            return false;
        }
        self.started = true;
        self.executing = true;
        self.status.state = JobState::Running;
        self.record("running", "Job 执行体开始运行");
        true
    }

    pub fn end(&mut self) {
        if self.executing {
            self.record("body_exited", "Job 执行体已结束");
        }
        self.executing = false;
    }

    pub fn cancel(&mut self) -> CancelResult {
        match self.status.state {
            JobState::Pending | JobState::Running => {
                self.cancel_requested = true;
                self.status.state = JobState::Stopping;
                self.record("cancel_requested", "Job 已请求取消");
                CancelResult::CancelRequested
            }
            JobState::Stopping => CancelResult::AlreadyStopping,
            JobState::Completed | JobState::Failed | JobState::Canceled => {
                CancelResult::AlreadyFinished
            }
        }
    }

    pub fn acknowledge(&mut self) -> bool {
        if !self.executing || !self.cancel_requested || self.status.state != JobState::Stopping {
            return false;
        }
        self.acknowledged = true;
        true
    }

    pub fn finish(&mut self, outcome: JobOutcome) {
        if self.status.outcome.is_some() {
            return;
        }
        let outcome = match outcome {
            JobOutcome::Completed
                if self.cancel_requested && (!self.started || self.acknowledged) =>
            {
                JobOutcome::Canceled
            }
            outcome => outcome,
        };
        self.status.state = match &outcome {
            JobOutcome::Completed => JobState::Completed,
            JobOutcome::Failed(_) => JobState::Failed,
            JobOutcome::Canceled => JobState::Canceled,
        };
        self.status.outcome = Some(outcome);
        self.record("finished", "Job 已收割并确认终态");
    }

    pub(crate) fn record(&self, stage: &str, message: &str) {
        widgetry_debug!(
            job_id = self.status.id.get(),
            kind = ?self.status.kind,
            label = ?self.status.label,
            state = ?self.status.state,
            stage,
            "{message}"
        );
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn logs_follow_actual_transitions_and_never_repeat_terminal_results() {
        use bevy_widgetry_test_utils::LogCapture;
        for (started, acknowledge, expected) in [
            (false, false, JobState::Canceled),
            (true, false, JobState::Completed),
            (true, true, JobState::Canceled),
        ] {
            let logs = LogCapture::default();
            logs.run(|| {
                let mut life = pending();
                if started {
                    assert!(life.start());
                    assert!(!life.start());
                }
                assert_eq!(life.cancel(), CancelResult::CancelRequested);
                assert_eq!(life.cancel(), CancelResult::AlreadyStopping);
                if acknowledge {
                    assert!(life.acknowledge());
                } else if !started {
                    assert!(!life.acknowledge());
                }
                life.end();
                life.finish(JobOutcome::Completed);
                life.finish(JobOutcome::Completed);
                assert_eq!(life.cancel(), CancelResult::AlreadyFinished);
                assert_eq!(life.status.state, expected);
            });
            let records = logs.records();
            let stages = if started {
                vec!["running", "cancel_requested", "body_exited", "finished"]
            } else {
                vec!["cancel_requested", "finished"]
            };
            assert_eq!(records.len(), stages.len());
            for (record, stage) in records.iter().zip(stages) {
                assert!(record.matches_field("stage", stage));
            }
            for record in &records {
                assert_eq!(record.level, bevy::log::Level::DEBUG);
                assert_eq!(record.fields["job_id"], "1");
                assert_eq!(record.fields["kind"], "Io");
            }
            assert!(
                records
                    .last()
                    .unwrap()
                    .matches_field("state", &format!("{expected:?}"))
            );
        }
    }

    fn pending() -> Lifecycle {
        Lifecycle::new(JobStatus {
            id: JobId(1),
            kind: JobKind::Io,
            state: JobState::Pending,
            outcome: None,
            label: None,
        })
    }

    #[test]
    fn acknowledgement_is_valid_only_inside_execution_after_request() {
        let mut life = pending();
        assert!(!life.acknowledge());
        assert!(life.start());
        assert!(!life.start());
        assert!(!life.acknowledge());
        life.end();
        assert_eq!(life.cancel(), CancelResult::CancelRequested);
        assert!(!life.acknowledge());
        life.finish(JobOutcome::Completed);
        assert_eq!(life.status.state, JobState::Completed);
        assert!(!life.acknowledge());
    }

    #[test]
    fn every_terminal_state_is_irreversible_after_cancel_or_late_completion() {
        for outcome in [
            JobOutcome::Completed,
            JobOutcome::Canceled,
            JobOutcome::Failed(std::io::Error::other("failed").into()),
        ] {
            let mut life = pending();
            assert!(life.start());
            if outcome == JobOutcome::Canceled {
                life.cancel();
                assert!(life.acknowledge());
            }
            life.end();
            life.finish(outcome.clone());
            let status = life.status.clone();
            assert_eq!(life.cancel(), CancelResult::AlreadyFinished);
            assert!(!life.start());
            assert!(!life.acknowledge());
            life.finish(JobOutcome::Failed(
                std::io::Error::other("late failure").into(),
            ));
            assert_eq!(life.status, status);
        }
    }
}
