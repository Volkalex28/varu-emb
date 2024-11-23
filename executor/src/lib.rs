#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(version("1.83")), feature(waker_getters))]
#![feature(sync_unsafe_cell)]
#![feature(debug_closure_helpers)]
#![feature(cfg_version)]

use self::application::Application as App;
use core::future::Future;
use core::marker::PhantomData;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::*;
use varuemb_lockfree::luqueue::{Item, LUQueue};

pub use proc::*;

pub mod application;
pub mod spawner;
pub mod task;

pub trait TaskName: Sized + 'static {
    const NAME: &'static str;

    fn name() -> &'static str;
}

pub trait Task: TaskName {
    type Fut: Future + 'static;

    fn process(self) -> Self::Fut;
    fn finish(result: <Self::Fut as Future>::Output) {
        drop(result)
    }

    fn pool() -> task::PoolRef<Self>;
}

#[derive(thiserror_no_std::Error, Debug)]
pub enum Error {
    #[error("Pool for task {0} is full")]
    PoolFull(&'static str),
}
impl<T: Task> From<spawner::SpawnError<T>> for Error {
    fn from(value: spawner::SpawnError<T>) -> Self {
        match value {
            spawner::SpawnError::PoolFull(_) => Self::PoolFull(T::NAME),
        }
    }
}

pub trait Pender {
    /// - `bool`: A boolean value indicating whether the executor is still active.
    ///   - `true`: The executor is still active and should continue running.
    ///   - `false`: The executor is no longer active and should stop running.
    fn wait(&mut self) -> bool;
    fn notify(&self);
}

struct Inner {
    notify: fn(&'static Self),
    list: LUQueue<Item<task::Task>>,
    queue: LUQueue<task::Task>,
    index: AtomicUsize,
}

impl Inner {
    const fn new(notify: fn(&'static Self)) -> Self {
        Self { notify, list: LUQueue::new(), queue: LUQueue::new(), index: AtomicUsize::new(0) }
    }

    #[inline]
    fn spawner(&'static self) -> spawner::Spawner {
        spawner::Spawner::new(self)
    }

    fn poll(&'static self) {
        loop {
            let list = self.list.into_iter();
            let to_enqueue = list.filter_map(|task| {
                let index = core::num::NonZeroUsize::new(task.index())?;
                Some((index.get(), task))
            });
            if let Some((_, task)) = to_enqueue.min_by_key(|(index, _)| *index) {
                self.enqueue(task::Ref::from_task(task));
                continue;
            }
            break;
        }

        let mut taker = self.queue.take();
        while let Some(task) = taker.next() {
            unsafe { task.poll() };
        }
    }

    #[inline]
    fn notify(&'static self) {
        (self.notify)(self)
    }

    #[inline]
    unsafe fn start_task(&'static self, task: task::Ref) {
        if self.list.push_back(task.0).is_some() {
            task.0.wake()
        }
    }

    #[inline]
    fn stop_task(&'static self, task: task::Ref) {
        self.list.pop(task.0);
    }

    #[inline]
    fn enqueue(&'static self, task: task::Ref) -> bool {
        let res = self.queue.push_back(task.0);
        task.0.enqueued();
        res.is_some_and(|is_first| is_first)
    }

    fn next_index(&self) -> usize {
        let mut index = 0;
        while index == 0 {
            index = self.index.fetch_add(1, SeqCst)
        }
        index
    }
}

#[repr(C)]
pub struct Executor<P: Pender> {
    inner: Inner,
    pender: P,
    name: &'static str,
    block_send: PhantomData<*const ()>,
}

impl<P: Pender> Executor<P> {
    #[inline]
    pub fn new(name: &'static str, pender: P) -> Self {
        Self { inner: Inner::new(Self::notify), pender, name, block_send: PhantomData }
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn run(mut self, app: Option<&'static App>, spawn: impl FnOnce(spawner::Spawner) -> Result<(), Error>) {
        let inner: &'static Inner = unsafe { core::mem::transmute(&self.inner) };

        if let Err(err) = spawn(inner.spawner()) {
            panic!("{err}")
        }

        use application::Thread;
        let thread = Item::new(Thread { name: self.name, executor: inner });
        if let Some(app) = app {
            let thread: &'static Item<Thread> = unsafe { core::mem::transmute(&thread) };
            app.new_thread(thread)
        }

        while self.pender.wait() {
            inner.poll();
        }
    }

    fn notify(this: &'static Inner) {
        let this = unsafe { &*(this as *const Inner as *const Self) };
        this.pender.notify();
    }
}
