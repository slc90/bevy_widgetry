use crate::*;

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
        true
    }

    pub fn end(&mut self) {
        self.executing = false;
    }

    pub fn cancel(&mut self) -> CancelResult {
        match self.status.state {
            JobState::Pending | JobState::Running => {
                self.cancel_requested = true;
                self.status.state = JobState::Stopping;
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
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

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
            JobOutcome::Failed(JobError::Business("failed".into())),
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
            life.finish(JobOutcome::Failed(JobError::Panic("late".into())));
            assert_eq!(life.status, status);
        }
    }
}
