use super::{Inner as Executor, Task as Instance};
use core::cell::SyncUnsafeCell;
use core::future::Future;
use core::sync::atomic::Ordering::*;
use core::sync::atomic::{AtomicPtr, AtomicUsize};
use core::task::{Context, Poll};
use core::{fmt, mem, pin, ptr};
use varuemb_lockfree::luqueue::Item;

mod stat;
mod state;
pub(super) mod waker;

type FmtFn = fn(&'static Task, &mut fmt::Formatter<'_>, bool) -> fmt::Result;
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
    vtable: VTable,
}
impl Data {
    const fn new() -> Self {
        Self { executor: null_ptr(), vtable: VTable { fmt_fn: null_ptr(), poll_fn: null_ptr() } }
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
    index: AtomicUsize,
}
impl Task {
    const fn new() -> Self {
        Self { data: Data::new(), state: state::State::new(), stat: stat::Statistic::new(), index: AtomicUsize::new(0) }
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
            let executor = executor.as_ref();
            if let Err(index) = self.index.compare_exchange(0, executor.next_index(), AcqRel, Relaxed) {
                panic!("Not ready but index not zero: {index}")
            }
            executor.notify();
        }
    }

    #[inline]
    pub(super) fn index(&self) -> usize {
        self.index.load(Acquire)
    }

    pub(super) fn enqueued(&self) {
        self.index.store(0, Release)
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
            (fmt_fn)(core::mem::transmute(self), f, true)
        }
    }
}
impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let fmt_fn: FmtFn = mem::transmute(self.data.vtable.fmt_fn.load(Acquire).cast_const());
            (fmt_fn)(core::mem::transmute(self), f, false)
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

    unsafe fn init(&'static self, task: T, executor: &'static Executor) -> Result<(), &'static str> {
        ptr::write((*self.future.as_ptr()).get(), task.process());

        self.task.data.fmt_fn(Self::fmt)?.poll_fn(Self::poll)?.executor(executor)?;
        executor.start_task(Ref(&self.task));

        Ok(())
    }

    unsafe fn deinit(&'static self) {
        let is_ready = self.task.state.finish();

        let executor = &*self.task.data.executor.swap(ptr::null_mut(), SeqCst);
        executor.stop_task(Ref(&self.task));
        if is_ready {
            self.task.index.store(0, Relaxed);
            self.task.state.despawn();
        }

        self.task.stat.clear();
        self.task.data.vtable.poll_fn.store(ptr::null_mut(), SeqCst);
        ptr::drop_in_place((*self.future.as_ptr()).get());
    }

    unsafe fn poll(task: &'static Task) {
        let this = task.as_storage::<T>();

        let waker = waker::make_waker_isr(task);
        let mut cx = Context::from_waker(&waker);

        let future = pin::Pin::new_unchecked(&mut *(*this.future.as_ptr()).get());
        task.stat.runned();
        match future.poll(&mut cx) {
            Poll::Ready(result) => {
                T::finish(result);
                this.deinit()
            }
            Poll::Pending => { /*nothing*/ }
        }

        mem::forget(waker);
    }

    fn fmt(task: &'static Task, f: &mut fmt::Formatter<'_>, is_debug: bool) -> fmt::Result {
        let pool = T::pool();
        let this = unsafe { task.as_storage::<T>() };

        let index = pool.0.iter().position(|s| ptr::from_ref(s) == ptr::from_ref(this)).unwrap();
        if pool.0.len() >= 2 {
            write!(f, "Task {}[{}]: ", T::NAME, index)?;
        } else {
            write!(f, "Task {}: ", T::NAME)?;
        }

        if is_debug {
            write!(f, "{:?}, {:?}", this.task.state, this.task.stat)
        } else {
            write!(f, "{}, {}", this.task.state, this.task.stat)
        }
    }
}

pub struct PoolRef<T: Instance>(&'static [Storage<T>]);
impl<T: Instance> PoolRef<T> {
    pub(super) fn spawn(self, task: T, executor: &'static Executor) -> Result<(), T> {
        let Some(storage) = self.0.iter().find(|storage| storage.claim()) else {
            return Err(task);
        };
        if let Err(place) = unsafe { storage.init(task, executor) } {
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
    pub fn as_ref(&'static self) -> PoolRef<T> {
        PoolRef(&self.0)
    }
}
unsafe impl<T: Instance, const SIZE: usize> Sync for Pool<T, SIZE> {}
