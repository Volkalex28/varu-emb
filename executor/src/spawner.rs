use super::{task::waker::get_task, Inner as Executor, Task};
use core::fmt;
use core::future::poll_fn;
use core::marker::PhantomData;
use core::sync::atomic::Ordering::Acquire;
use core::task::Poll;

#[derive(thiserror_no_std::Error)]
pub enum SpawnError<T: Task> {
    #[error("Pool for task {} is full", T::NAME)]
    PoolFull(T),
}
impl<T: Task> fmt::Debug for SpawnError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub struct Spawner {
    executor: &'static Executor,
    block_send: PhantomData<*const ()>,
}
impl Spawner {
    pub(super) fn new(executor: &'static Executor) -> Self {
        Self {
            executor,
            block_send: PhantomData,
        }
    }

    #[inline]
    pub fn spawn<T: Task>(&self, task: T) -> Result<(), SpawnError<T>> {
        T::pool()
            .spawn(task, self.executor)
            .map_err(SpawnError::PoolFull)
    }

    pub async fn for_current_executor() -> Self {
        poll_fn(|cx| {
            let task = get_task(cx.waker());
            let executor = unsafe { &*task.data.executor.load(Acquire) };
            Poll::Ready(Self::new(executor))
        })
        .await
    }

    #[inline]
    #[allow(unused)]
    pub fn detach(&self) -> Detached {
        Detached {
            executor: self.executor,
        }
    }
}

#[allow(unused)]
pub struct Detached {
    executor: &'static Executor,
}
impl Detached {
    #[inline]
    #[allow(unused)]
    pub fn spawn<T: Task>(&self, task: T) -> Result<(), SpawnError<T>>
    where
        <T as Task>::Fut: Send,
    {
        T::pool()
            .spawn(task, self.executor)
            .map_err(SpawnError::PoolFull)
    }
}
