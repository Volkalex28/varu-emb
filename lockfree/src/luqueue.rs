use self::State::*;
use core::ops::Deref;
use core::sync::atomic::Ordering::*;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize};
use core::{fmt, ptr};

enum State<N: 'static> {
    Empty,
    Last,
    Next(&'static Item<N>),
}
impl<N: 'static> State<N> {
    const fn new() -> Self {
        Self::Empty
    }

    #[inline]
    const fn to_ptr(&self) -> *mut Item<N> {
        match self {
            State::Empty => ptr::null_mut(),
            State::Last => ptr::without_provenance_mut(1),
            State::Next(item) => item.to_ptr(),
        }
    }

    #[inline]
    fn from_ptr(ptr: *mut Item<N>) -> Self {
        if ptr == Self::Empty.to_ptr() {
            Self::Empty
        } else if ptr == Self::Last.to_ptr() {
            Self::Last
        } else {
            Self::Next(unsafe { &*ptr })
        }
    }

    #[inline]
    fn as_item(&self) -> Option<&'static Item<N>> {
        match self {
            Next(item) => Some(item),
            _ => None,
        }
    }
}

#[repr(C)]
pub struct Item<N> {
    value: N,
    state: AtomicPtr<Item<N>>,
}
impl<N: 'static> Item<N> {
    pub const fn new(value: N) -> Self {
        Self { value, state: AtomicPtr::new(State::new().to_ptr()) }
    }

    #[inline]
    fn from_ptr(ptr: *mut Self) -> Option<&'static Self> {
        let ptr = ptr::NonNull::new(ptr)?;
        Some(unsafe { ptr.as_ref() })
    }

    #[inline]
    const fn to_ptr(&'static self) -> *mut Self {
        (&raw const *self).cast_mut()
    }
}
impl<N> Deref for Item<N> {
    type Target = N;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<N: PartialEq<T> + 'static, T> PartialEq<T> for Item<N> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.value.eq(other)
    }
}
impl<N: fmt::Debug> fmt::Debug for Item<N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}
impl<N: fmt::Display> fmt::Display for Item<N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

pub struct Taker<N: 'static> {
    count: &'static AtomicUsize,
    current: *mut Item<N>,
}
impl<N: 'static> Taker<N> {
    pub fn next(&mut self) -> Option<&'static Item<N>> {
        let current = State::from_ptr(self.current).as_item()?;

        self.current = current.state.swap(Empty.to_ptr(), SeqCst);

        self.count.fetch_sub(1, SeqCst);

        Some(current)
    }
}
impl<N: 'static> Drop for Taker<N> {
    fn drop(&mut self) {
        while self.next().is_some() {}
    }
}

enum Push<N: 'static> {
    Start,
    TryInsert,
    Registrate(&'static Item<N>),
}

pub struct LUQueue<N: 'static> {
    count: AtomicUsize,
    head: AtomicPtr<Item<N>>,
    pop_lock: AtomicBool,
}
impl<N: 'static> LUQueue<N> {
    pub const fn new() -> Self {
        Self { count: AtomicUsize::new(0), head: AtomicPtr::new(ptr::null_mut()), pop_lock: AtomicBool::new(false) }
    }

    pub fn count(&self) -> usize {
        self.count.load(Relaxed)
    }

    pub fn take(&'static self) -> Taker<N> {
        Taker { count: &self.count, current: self.head.swap(ptr::null_mut(), SeqCst) }
    }

    pub fn push_back(&self, node: &'static Item<N>) -> Option<bool> {
        let mut push = Push::Start;

        let result = loop {
            push = match push {
                Push::Start => match node.state.compare_exchange(Empty.to_ptr(), Last.to_ptr(), SeqCst, SeqCst) {
                    Err(_) => break None,
                    Ok(_) => Push::TryInsert,
                },
                Push::TryInsert => match self.head.compare_exchange(ptr::null_mut(), node.to_ptr(), SeqCst, SeqCst) {
                    Ok(_) => break Some(true),
                    Err(ptr) => Push::Registrate(Item::from_ptr(ptr).unwrap()),
                },
                Push::Registrate(item) => {
                    match item.state.compare_exchange_weak(Last.to_ptr(), node.to_ptr(), SeqCst, SeqCst) {
                        Ok(_) => break Some(false),
                        Err(ptr) => match State::from_ptr(ptr) {
                            Empty => Push::TryInsert,
                            Next(item) => Push::Registrate(item),
                            Last => Push::Registrate(item),
                        },
                    }
                }
            }
        };

        if result.is_some() {
            self.count.fetch_add(1, SeqCst);
        }

        result
    }

    #[inline]
    pub fn pop_front(&'static self) -> Option<&'static Item<N>> {
        self.pop_impl(|_| true)
    }

    #[inline]
    pub fn pop<T>(&'static self, value: &T) -> Option<&'static Item<N>>
    where
        N: PartialEq<T>,
    {
        self.pop_impl(|item| item.eq(value))
    }

    fn pop_impl(&'static self, cmp: impl Fn(&'static N) -> bool) -> Option<&'static Item<N>> {
        self.lock_pop();

        let mut ret = None;
        let mut current = &self.head;
        while let Next(item) = State::from_ptr(current.load(SeqCst)) {
            if !cmp(&item.value) {
                current = &item.state;
                continue;
            }

            let next = {
                let next = State::from_ptr(item.state.swap(Last.to_ptr(), SeqCst)).as_item().map(|n| n.to_ptr());
                next.unwrap_or_else(|| self.get_last(current))
            };
            current.store(next, SeqCst);

            if let Some(missed) = State::from_ptr(item.state.swap(Empty.to_ptr(), SeqCst)).as_item().map(|i| i.to_ptr()) {
                loop {
                    let last = self.get_last(current);
                    current = match current.compare_exchange(last, missed, SeqCst, SeqCst) {
                        Ok(_) => break,
                        Err(ptr) => unsafe { &(*ptr).state },
                    }
                }
            }

            ret = Some(item);
            break;
        }

        if ret.is_some() {
            self.count.fetch_sub(1, SeqCst);
        }
        self.unlock_pop();

        ret
    }

    #[inline(always)]
    fn get_last(&self, ptr: &'static AtomicPtr<Item<N>>) -> *mut Item<N> {
        if ptr::from_ref(ptr) == ptr::from_ref(&self.head) {
            ptr::null_mut()
        } else {
            Last.to_ptr()
        }
    }

    #[inline]
    fn lock_pop(&self) {
        while self.pop_lock.swap(true, Acquire) {}
    }

    #[inline]
    fn unlock_pop(&self) {
        self.pop_lock.store(false, Release)
    }
}
impl<N: 'static> Drop for LUQueue<N> {
    fn drop(&mut self) {
        for item in &*self {
            item.state.store(State::Empty.to_ptr(), Release);
        }
    }
}

impl<'a, N: 'static> IntoIterator for &'a LUQueue<N> {
    type Item = &'static Item<N>;
    type IntoIter = Iter<'a, N>;

    fn into_iter(self) -> Self::IntoIter {
        Iter { ptr: &self.head }
    }
}

pub struct Iter<'a, N> {
    ptr: &'a AtomicPtr<Item<N>>,
}

impl<N: 'static> Iterator for Iter<'_, N> {
    type Item = &'static Item<N>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = State::from_ptr(self.ptr.load(Acquire)).as_item()?;
        self.ptr = &item.state;
        Some(item)
    }
}
