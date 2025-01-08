#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(version("1.83")), feature(waker_getters))]
#![feature(sync_unsafe_cell)]
#![feature(debug_closure_helpers)]
#![feature(cfg_version)]

use self::statistic::Statistic;
use core::future::Future;
use core::marker::PhantomData;
use varuemb_lockfree::luqueue::{Item, LUQueue};

pub use proc::*;

pub mod spawner;
pub mod statistic;
pub mod task;

pub trait TaskName: Sized + 'static {
    const NAME: &'static str;

    fn name() -> &'static str;
}

pub trait Task: TaskName {
    type Fut: Future + 'static;
    // type Pool: PoolProvider<Self>;

    fn __process(self) -> Self::Fut;
    fn __finish(result: <Self::Fut as Future>::Output) {
        drop(result)
    }

    // fn pool() -> task::PoolRef<Self>;
}

#[derive(thiserror_no_std::Error, Debug)]
pub enum Error {
    #[error("Pool for task {0} is full")]
    PoolFull(&'static str),
    #[error("{}", spawner::ForCurrentExecutorError)]
    InvalidExecutor,
}
impl<T: Task> From<spawner::SpawnError<T>> for Error {
    fn from(value: spawner::SpawnError<T>) -> Self {
        match value {
            spawner::SpawnError::PoolFull(_) => Self::PoolFull(T::NAME),
        }
    }
}
impl From<spawner::ForCurrentExecutorError> for Error {
    fn from(_: spawner::ForCurrentExecutorError) -> Self {
        Self::InvalidExecutor
    }
}

pub struct Inner {
    notify: fn(&'static Self),
    list: LUQueue<Item<task::Task>>,
    queue: LUQueue<task::Task>,
}

impl Inner {
    pub const fn new(notify: fn(&'static Self)) -> Self {
        Self { notify, list: LUQueue::new(), queue: LUQueue::new() }
    }

    #[inline]
    pub fn spawner(&'static self) -> spawner::Spawner<()> {
        spawner::Spawner::new(self)
    }

    pub fn poll(&'static self) {
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
    fn enqueue(&'static self, task: task::Ref) {
        if self.queue.push_back(task.0).is_some_and(|is_first| is_first) {
            self.notify();
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

pub trait Execution: AsRef<Statistic> {
    type Pender<'a>: Pender + 'a
    where
        Self: 'a;

    fn make_pender<'a>(&'a self, name: &'static str) -> Self::Pender<'a>;
}

pub trait PoolProvider<T: Task> {
    fn pool(&self) -> task::PoolRef<T>;
}

#[repr(C)]
pub struct Executor<'e, E: Execution> {
    inner: Inner,
    name: &'static str,
    execution: &'e E,
    pender: E::Pender<'e>,
    block_send: PhantomData<*const ()>,
}

impl<'e, E: Execution + 'static> Executor<'e, E> {
    #[inline]
    pub fn new(name: &'static str, execution: &'e E) -> Self {
        Self {
            inner: Inner::new(Executor::<'static, E>::notify),
            execution,
            pender: execution.make_pender(name),
            name,
            block_send: PhantomData,
        }
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl<E: Execution> Executor<'static, E> {
    pub fn run(mut self, spawn: impl FnOnce(spawner::Spawner<E>) -> Result<(), Error>) -> Result<(), Error> {
        use statistic::Thread;

        let inner: &'static Inner = unsafe { core::mem::transmute(&self.inner) };

        spawn(inner.spawner().map(self.execution))?;

        let thread = Item::new(Thread { name: self.name, executor: inner });
        let thread = self.execution.as_ref().new_thread(unsafe { core::mem::transmute(&thread) });

        while self.pender.wait() {
            inner.poll();
        }

        if let Some(thread) = thread {
            self.execution.as_ref().delete_thread(thread);
        }

        Ok(())
    }

    fn notify(this: &'static Inner) {
        let this = unsafe { &*(&raw const *this).cast::<Self>() };
        this.pender.notify();
    }
}
