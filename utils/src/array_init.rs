use core::{mem::ManuallyDrop as MDrom, ops::Deref};

pub struct ArrayInitializer<F = fn(), D = ()> {
    _fn: F,
    data: D,
}
impl ArrayInitializer {
    pub const fn init<const S: usize, T, F: ~const Fn(usize) -> T>(_fn: F) -> [T; S] {
        ArrayInitializer::<_, &F>::with_data(const |i, _fn| _fn(i), MDrom::new(_fn).deref())
    }
}
impl<D: Copy, T, F: ~const Fn(usize, D) -> T> ArrayInitializer<F, D> {
    pub const fn with_data<'a, const S: usize>(_fn: F, data: D) -> [T; S] {
        let data = ArrayInitializer { _fn, data };
        let _fn = const |i, this: ArrayInitializer<F, D>| ((this._fn)(i, this.data), this);
        let ret = array_init_with_data(_fn, data);
        ret
    }
}
impl<D, T, F: ~const Fn(usize, D) -> (T, D)> ArrayInitializer<F, D> {
    pub const fn fold<'f, const S: usize>(foo: F, data: D) -> [T; S] {
        array_init_with_data(foo, data)
    }
}

const fn array_init_with_data<T, const S: usize, D, F>(_fn: F, data: D) -> [T; S]
where
    F: ~const Fn(usize, D) -> (T, D),
{
    use ::core::mem::{ManuallyDrop as MDrop, MaybeUninit as UnInit};

    union FnReturn<T, D> {
        tuple: MDrop<(T, D)>,
        de_tuple: (MDrop<T>, MDrop<D>),
    }

    let mut i = 0;
    let mut data = MDrop::new(data);
    let _fn = MDrom::new(_fn);
    let mut ret = UnInit::<[T; S]>::uninit().transpose();
    let ret_data = loop {
        if i >= S {
            break ret;
        }
        let fn_return = FnReturn {
            tuple: MDrop::new(_fn(i, MDrop::into_inner(data))),
        };

        let (_ret, _data) = unsafe { fn_return.de_tuple };
        ret[i].write(MDrop::into_inner(_ret));
        data = _data;
        i += 1;
    };
    unsafe { ret_data.transpose().assume_init() }
}
