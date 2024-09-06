use core::{
    fmt,
    sync::atomic::{AtomicUsize, Ordering::*},
};

#[derive(Debug)]
pub struct Statistic {
    run_count: AtomicUsize,
}
impl fmt::Display for Statistic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Statistic").field("run_times", &self.run_count).finish()
    }
}
impl Statistic {
    pub const fn new() -> Self {
        Self { run_count: AtomicUsize::new(0) }
    }

    #[inline]
    pub fn clear(&self) {
        self.run_count.store(0, Relaxed);
    }

    #[inline]
    pub fn runned(&self) {
        self.run_count.fetch_add(1, Relaxed);
    }
}
