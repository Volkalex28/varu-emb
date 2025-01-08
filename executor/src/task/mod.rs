use super::{Inner as Executor, Task as Instance};
use core::cell::SyncUnsafeCell;
use core::future::Future;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::*;
use core::task::{Context, Poll};
use core::{fmt, mem, pin, ptr};
use varuemb_lockfree::luqueue::Item;

mod stat;
mod state;
pub(super) mod waker;

type FmtFn = fn(*const Task, &'static Task, &mut fmt::Formatter<'_>, bool) -> fmt::Result;
type PollFn = unsafe fn(&'static Task);

const fn null_ptr<T>() -> AtomicPtr<T> {
    AtomicPtr::new(ptr::null_mut())
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub(super) struct Ref(pub(super) &'static Item<Item<Task>>);
impl Ref {
    pub(super) fn from_task(task: &'static Task) -> Self {
        Self(unsafe { &*(task as *const Task as *const Item<Item<Task>>) })
    }
}

struct VTable {
    fmt_fn: AtomicPtr<()>,
    poll_fn: AtomicPtr<()>,
}

pub(super) struct Data {
    pub(super) executor: AtomicPtr<Executor>,
    pool: AtomicPtr<Task>,
    vtable: VTable,
}
impl Data {
    const fn new() -> Self {
        Self { executor: null_ptr(), pool: null_ptr(), vtable: VTable { fmt_fn: null_ptr(), poll_fn: null_ptr() } }
    }

    fn executor(&self, executor: &'static Executor) -> Result<&Self, &'static str> {
        let executor = (executor as *const Executor).cast_mut();
        if self.executor.compare_exchange(ptr::null_mut(), executor, SeqCst, SeqCst).is_err() {
            return Err("Executor");
        }
        Ok(self)
    }

    fn fmt_fn(&self, fmt_fn: FmtFn) -> Result<&Self, &'static str> {
        self.vtable.fmt_fn.store((fmt_fn as *const ()).cast_mut(), SeqCst);
        Ok(self)
    }

    fn pool(&self, pool: &'static Task) -> Result<&Self, &'static str> {
        self.pool.store(pool.as_ptr().cast_mut(), SeqCst);
        Ok(self)
    }

    fn poll_fn(&self, poll_fn: PollFn) -> Result<&Self, &'static str> {
        if !self.vtable.poll_fn.swap((poll_fn as *const ()).cast_mut(), SeqCst).is_null() {
            return Err("VTable");
        }
        Ok(self)
    }
}

pub(super) struct Task {
    pub(super) data: Data,
    state: state::State,
    stat: stat::Statistic,
}
impl Task {
    const fn new() -> Self {
        Self { data: Data::new(), state: state::State::new(), stat: stat::Statistic::new() }
    }

    pub(super) unsafe fn poll(&'static self) {
        if self.state.begin() {
            let poll_fn: PollFn = core::mem::transmute(self.data.vtable.poll_fn.load(SeqCst).cast_const());
            (poll_fn)(self);
            self.state.end();
        } else {
            self.state.despawn()
        }
    }

    pub(super) unsafe fn wake(&'static self) {
        let Some(executor) = ptr::NonNull::new(self.data.executor.load(Acquire)) else {
            return;
        };
        if self.state.ready() {
            executor.as_ref().enqueue(Ref::from_task(self));
        }
    }

    #[inline]
    unsafe fn as_storage<T: Instance>(&'static self) -> &'static Storage<T> {
        &*self.as_ptr().cast()
    }

    #[inline(always)]
    fn as_ptr(&'static self) -> *const Self {
        self as *const Self
    }

    #[inline]
    pub(super) unsafe fn from_ptr(ptr: *const Self) -> &'static Self {
        ptr.as_ref().unwrap_unchecked()
    }
}
impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let fmt_fn: FmtFn = mem::transmute(self.data.vtable.fmt_fn.load(Acquire).cast_const());
            (fmt_fn)(self.data.pool.load(Relaxed), core::mem::transmute(self), f, true)
        }
    }
}
impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let fmt_fn: FmtFn = mem::transmute(self.data.vtable.fmt_fn.load(Acquire).cast_const());
            (fmt_fn)(self.data.pool.load(Relaxed), core::mem::transmute(self), f, false)
        }
    }
}
impl PartialEq for Task {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        ptr::from_ref(other) == ptr::from_ref(self)
    }
}

#[repr(C)]
struct Storage<T: Instance> {
    task: Item<Item<Task>>,
    future: mem::MaybeUninit<SyncUnsafeCell<T::Fut>>,
}
impl<T: Instance> Storage<T> {
    const INIT: Storage<T> = Storage::new();

    const fn new() -> Self {
        Self { task: Item::new(Item::new(Task::new())), future: mem::MaybeUninit::uninit() }
    }

    #[inline]
    fn claim(&'static self) -> bool {
        self.task.state.spawn()
    }

    unsafe fn init(
        &'static self,
        pool_ref: PoolRef<'static, T>,
        task: T,
        executor: &'static Executor,
    ) -> Result<(), &'static str> {
        ptr::write((*self.future.as_ptr()).get(), task.__process());

        self.task
            .data
            .poll_fn(Self::poll)?
            .fmt_fn(pool_ref.1)?
            .pool(&pool_ref.0[0].task)?
            .executor(executor)?;
        executor.start_task(Ref(&self.task));

        Ok(())
    }

    unsafe fn deinit(&'static self) {
        let is_ready = self.task.state.finish();

        let executor = &*self.task.data.executor.swap(ptr::null_mut(), SeqCst);
        executor.stop_task(Ref(&self.task));
        if is_ready {
            self.task.state.despawn();
        }

        self.task.stat.clear();
        self.task.data.vtable.poll_fn.store(ptr::null_mut(), SeqCst);
        ptr::drop_in_place((*self.future.as_ptr()).get());
    }

    unsafe fn poll(task: &'static Task) {
        let this = task.as_storage::<T>();

        let waker = waker::make_waker(task);
        let mut cx = Context::from_waker(&waker);

        let future = pin::Pin::new_unchecked(&mut *(*this.future.as_ptr()).get());
        task.stat.runned();
        match future.poll(&mut cx) {
            Poll::Ready(result) => {
                T::__finish(result);
                this.deinit()
            }
            Poll::Pending => { /*nothing*/ }
        }

        mem::forget(waker);
    }
}

pub struct PoolRef<'a, T: Instance>(&'a [Storage<T>], FmtFn);
impl<T: Instance> PoolRef<'static, T> {
    pub(super) fn spawn(self, task: T, executor: &'static Executor) -> Result<(), T> {
        let Some(storage) = self.0.iter().find(|storage| storage.claim()) else {
            return Err(task);
        };
        if let Err(place) = unsafe { storage.init(self, task, executor) } {
            panic!("[{}] {place} is already initialized", T::NAME)
        }
        Ok(())
    }
}

pub struct Pool<T: Instance, const SIZE: usize>([Storage<T>; SIZE]);
impl<T: Instance, const SIZE: usize> Pool<T, SIZE> {
    pub const fn new() -> Self {
        Self([Storage::INIT; SIZE])
    }

    #[inline(always)]
    pub fn as_ref(&self) -> PoolRef<T> {
        PoolRef(&self.0, Self::fmt)
    }

    fn fmt(this: *const Task, task: &'static Task, f: &mut fmt::Formatter<'_>, is_debug: bool) -> fmt::Result {
        let pool = unsafe { &*this.cast::<Self>() };

        let index = pool.0.iter().position(|s| s.task == *task).unwrap();
        if SIZE > 1 {
            write!(f, "Task {}[{}]: ", T::NAME, index)?;
        } else {
            write!(f, "Task {}: ", T::NAME)?;
        }

        if is_debug {
            write!(f, "{:?}, {:?}", task.state, task.stat)
        } else {
            write!(f, "{}, {}", task.state, task.stat)
        }
    }
}
unsafe impl<T: Instance, const SIZE: usize> Sync for Pool<T, SIZE> {}
