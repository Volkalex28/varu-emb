use super::task::Ref;
use super::Inner as Executor;
use core::fmt;
use varuemb_lockfree::luqueue::{Item, LUQueue};

pub struct Task(Ref);
impl fmt::Debug for Task {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for Task {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0 .0, f)
    }
}
pub struct Thread {
    pub(super) name: &'static str,
    pub(super) executor: &'static Executor,
}
impl fmt::Display for Thread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Thread {}: ", self.name)?;
        f.debug_list().entries(self.list()).finish()
    }
}
impl fmt::Debug for Thread {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl PartialEq for Thread {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && (&raw const *self.executor == &raw const *other.executor)
    }
}
impl Thread {
    #[inline]
    #[allow(unused)]
    pub fn name(&self) -> &'static str {
        self.name
    }

    #[inline]
    pub fn list(&self) -> impl Iterator<Item = Task> {
        self.executor.list.into_iter().map(|task| Task(Ref(task)))
    }
}

pub struct Statistic {
    threads: LUQueue<Thread>,
}
impl Statistic {
    #[inline]
    pub const fn new() -> Self {
        Self { threads: LUQueue::new() }
    }

    #[inline]
    pub(super) fn new_thread(&'static self, thread: &'static Item<Thread>) -> Option<&'static Item<Thread>> {
        self.threads.push_back(thread).map(|_| thread)
    }

    #[inline]
    pub(super) fn delete_thread(&'static self, thread: &'static Item<Thread>) -> Option<&'static Item<Thread>> {
        self.threads.pop(thread)
    }

    #[inline]
    pub fn list(&'static self) -> impl Iterator<Item = &'static Thread> {
        self.threads.into_iter().map(|task| &**task)
    }

    #[inline]
    #[allow(unused)]
    pub fn get(&'static self, name: &'static str) -> Option<&'static Thread> {
        self.list().find(|thread| thread.name == name)
    }
}
