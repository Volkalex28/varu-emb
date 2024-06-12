use core::mem::MaybeUninit as UnInit;

pub struct Item<'a, T> {
    i: &'a mut usize,
    inner: &'a mut UnInit<T>,
}

impl<'a, T> Item<'a, T> {
    pub const fn init(self, value: T) {
        self.inner.write(value);
        *self.i += 1;
    }
}

pub struct ArrayInitializer<T, const S: usize> {
    i: usize,
    data: [UnInit<T>; S],
}

#[allow(unused)]
impl<T, const S: usize> ArrayInitializer<T, S> {
    pub const fn new() -> Self {
        Self {
            i: 0,
            data: UnInit::uninit_array(),
        }
    }

    pub const fn next(&mut self) -> Option<(usize, Item<T>)> {
        if self.i == S {
            return None;
        }
        let index = self.i;
        let item = Item {
            i: &mut self.i,
            inner: &mut self.data[index],
        };
        Some((index, item))
    }

    pub const unsafe fn finish(self) -> [T; S] {
        if self.i != S {
            panic!()
        }
        UnInit::array_assume_init(self.data)
    }
}
