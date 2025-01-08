use super::task::waker::get_task;
use super::{Inner as Executor, PoolProvider, Task};
use core::fmt;
use core::future::poll_fn;
use core::marker::PhantomData;
use core::sync::atomic::Ordering::Acquire;
use core::task::Poll;

#[derive(thiserror_no_std::Error)]
pub enum SpawnError<T: Task> {
    #[error("Pool for tasks {} is full", T::NAME)]
    PoolFull(T),
}
impl<T: Task> fmt::Debug for SpawnError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(thiserror_no_std::Error)]
#[error("Is not varuemb executor")]
pub struct ForCurrentExecutorError;
impl fmt::Debug for ForCurrentExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub struct Spawner<P: 'static = ()> {
    executor: &'static Executor,
    provider: &'static P,
    block_send: PhantomData<*const ()>,
}
impl Spawner<()> {
    pub(super) fn new(executor: &'static Executor) -> Self {
        Self { executor, provider: &(), block_send: PhantomData }
    }

    pub async fn for_current_executor() -> Result<Self, ForCurrentExecutorError> {
        poll_fn(|cx| {
            let executor = get_task(cx.waker()).map(|task| unsafe { &*task.data.executor.load(Acquire) });
            Poll::Ready(executor.map(Self::new))
        })
        .await
        .ok_or(ForCurrentExecutorError)
    }
}
impl<P: 'static> Spawner<P> {
    #[inline]
    pub fn spawn<T: Task>(&self, task: T) -> Result<(), SpawnError<T>>
    where
        P: PoolProvider<T>,
    {
        self.provider.pool().spawn(task, self.executor).map_err(SpawnError::PoolFull)
    }

    pub fn map<T: 'static>(self, provider: &'static T) -> Spawner<T> {
        Spawner { executor: self.executor, provider, block_send: self.block_send }
    }

    #[inline]
    pub fn detach(&self) -> Detached<P> {
        Detached { executor: self.executor, provider: self.provider }
    }
}

pub struct Detached<P: 'static> {
    executor: &'static Executor,
    provider: &'static P,
}
impl<P: 'static> Detached<P> {
    #[inline]
    pub fn spawn<T: Task>(&self, task: T) -> Result<(), SpawnError<T>>
    where
        P: PoolProvider<T>,
        <T as Task>::Fut: Send,
    {
        self.provider.pool().spawn(task, self.executor).map_err(SpawnError::PoolFull)
    }
}
