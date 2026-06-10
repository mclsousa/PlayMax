use std::sync::atomic::{AtomicBool, Ordering};

pub struct SyncGuard {
    in_progress: AtomicBool,
}

impl SyncGuard {
    pub fn new() -> Self {
        Self {
            in_progress: AtomicBool::new(false),
        }
    }

    pub fn is_in_progress(&self) -> bool {
        self.in_progress.load(Ordering::SeqCst)
    }

    /// Returns true if the guard was acquired.
    pub fn try_acquire(&self) -> bool {
        self.in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn release(&self) {
        self.in_progress.store(false, Ordering::SeqCst);
    }
}

impl Default for SyncGuard {
    fn default() -> Self {
        Self::new()
    }
}
