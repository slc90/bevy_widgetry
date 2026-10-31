use std::{
    backtrace::Backtrace,
    fmt,
    panic::Location,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

static NEXT_ID: Mutex<Vec<u64>> = Mutex::new(Vec::new());

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ErrorId(Box<[u64]>);

impl ErrorId {
    fn allocate() -> Self {
        // No user code runs under this lock. Retaining a poisoned counter still preserves uniqueness.
        let mut next = NEXT_ID
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        increment(&mut next);
        Self(next.clone().into_boxed_slice())
    }
}

fn increment(value: &mut Vec<u64>) {
    for limb in value.iter_mut() {
        let (next, carry) = limb.overflowing_add(1);
        *limb = next;
        if !carry {
            return;
        }
    }
    value.push(1);
}

impl fmt::Display for ErrorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut limbs = self.0.iter().rev();
        if let Some(first) = limbs.next() {
            write!(f, "{first:x}")?;
        }
        for limb in limbs {
            write!(f, "{limb:016x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ErrorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub struct ErrorOrigin {
    id: ErrorId,
    location: &'static Location<'static>,
    backtrace: Backtrace,
    handled: AtomicBool,
}

impl ErrorOrigin {
    #[track_caller]
    pub(crate) fn capture() -> Self {
        Self {
            id: ErrorId::allocate(),
            location: Location::caller(),
            backtrace: Backtrace::force_capture(),
            handled: AtomicBool::new(false),
        }
    }

    pub fn id(&self) -> &ErrorId {
        &self.id
    }
    pub fn location(&self) -> &'static Location<'static> {
        self.location
    }
    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    pub(crate) fn mark_handled(&self) -> bool {
        !self.handled.swap(true, Ordering::Relaxed)
    }
}

impl fmt::Debug for ErrorOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrorOrigin")
            .field("id", &self.id)
            .field("location", &self.location)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_macros)]
    use super::*;

    #[test]
    fn ids_grow_instead_of_reusing_after_carry() {
        let mut counter = vec![u64::MAX, u64::MAX];
        let before = ErrorId(counter.clone().into_boxed_slice());
        increment(&mut counter);
        let after = ErrorId(counter.clone().into_boxed_slice());
        assert_eq!(counter, vec![0, 0, 1]);
        assert_ne!(before, after);
        assert_eq!(after.to_string(), "100000000000000000000000000000000");
        increment(&mut counter);
        assert_eq!(counter, vec![1, 0, 1]);
    }
}
