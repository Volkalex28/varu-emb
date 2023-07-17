use core::mem::MaybeUninit as UnInit;

pub struct ArrayInitializer {}
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
}
