use super::Task;
use core::task::{RawWaker, RawWakerVTable, Waker};

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop);

unsafe fn clone(task: *const ()) -> RawWaker {
    RawWaker::new(task, &VTABLE)
}
unsafe fn wake(task: *const ()) {
    Task::from_ptr(task.cast()).wake()
}
unsafe fn drop(_: *const ()) {
    /*nothing*/
}

#[allow(unused)]
pub unsafe fn make_waker(task: &'static Task) -> Waker {
    Waker::from_raw(RawWaker::new(task.as_ptr().cast(), &VTABLE))
}

#[allow(unused)]
#[cfg(version("1.84"))]
pub fn get_task(waker: &Waker) -> Option<&'static Task> {
    let vtable = waker.vtable();

    if vtable != &VTABLE {
        return None;
    }

    Some(unsafe { Task::from_ptr(waker.data().cast()) })
}

#[allow(unused)]
#[cfg(not(version("1.84")))]
pub fn get_task(waker: &Waker) -> Option<&'static Task> {
    let raw = waker.as_raw();

    if raw.vtable() != &VTABLE {
        return None;
    }

    Some(unsafe { Task::from_ptr(raw.data().cast()) })
}
