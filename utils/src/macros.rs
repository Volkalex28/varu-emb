#[macro_export]
macro_rules! const_wrapper {
    {$($tt:tt)+} => { const _:() = { $($tt)+ }; };
}

#[macro_export]
macro_rules! count {
    ( $( $e:stmt ,)* ) => {
        ${ count(e) }
    };
}


#[macro_export]
macro_rules! macro_if {
    {{ $($cond:tt)+ } then { $($then:tt)* } $(else { $($else:tt)* })?} => { $($then)* };
    {{              } then { $($then:tt)* } $(else { $($else:tt)* })?} => { $($($else)*)? };
}


#[macro_export]
macro_rules! macro_if_not {
    {{ $($cond:tt)* } then { $($then:tt)* } $(else { $($else:tt)* })?} => {
        $crate::macro_if! { { $($cond)* } then { $($($else)*)? } else { $($then)* } }
    };
}


#[macro_export]
macro_rules! macro_def_or {
    ({ $($def:tt)+ } $($val:tt)+) => { $($val)+ };
    ({ $($def:tt)* })             => { $($def)* };
}
