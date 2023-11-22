use core::mem::MaybeUninit as UnInit;
use std::marker::Destruct;

pub struct ArrayInitializer {}

#[allow(unused)]
impl ArrayInitializer {
    pub const fn init<const S: usize, T, F: ~const Fn(usize) -> T>(init_func: &F) -> [T; S] {
        let mut i = 0;
        let mut ret = UnInit::<[T; S]>::uninit().transpose();
        let ret_data = loop {
            if i >= S {
                break ret;
            }
            ret[i].write(init_func(i));
            i += 1;
        };
        unsafe { ret_data.transpose().assume_init() }
    }

    const fn with_data<T, D, const S: usize, F>(init_func: &F, data: D) -> [T; S]
    where
        T: ~const Destruct,
        D: ~const Destruct,
        F: ~const Fn(usize, D) -> (T, D),
    {
        let mut i = 0;
        let mut data = data;

        let mut ret = UnInit::<[T; S]>::uninit().transpose();
        let ret_data = loop {
            if i >= S {
                break ret;
            }

            let (new_value, new_data) = init_func(i, data);

            ret[i].write(new_value);
            data = new_data;
            i += 1;
        };
        unsafe { ret_data.transpose().assume_init() }
    }
}
