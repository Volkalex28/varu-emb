#[macro_export]
macro_rules! select {
    {
        $ident0:ident = $fut0:expr => $handler0:block
        $ident1:ident = $fut1:expr => $handler1:block
    } => {
        match $crate::__private::embassy_futures::select::select($fut0, $fut1).await {
            $crate::__private::embassy_futures::select::Either::First(mut $ident0) => { $handler0 }
            $crate::__private::embassy_futures::select::Either::Second(mut $ident1) => { $handler1 }
        }
    };
    {
        $ident0:ident = $fut0:expr => $handler0:block
        $ident1:ident = $fut1:expr => $handler1:block
        $ident2:ident = $fut2:expr => $handler2:block
    } => {
        match $crate::__private::embassy_futures::select::select3($fut0, $fut1, $fut2).await {
            $crate::__private::embassy_futures::select::Either3::First(mut $ident0) => { $handler0 }
            $crate::__private::embassy_futures::select::Either3::Second(mut $ident1) => { $handler1 }
            $crate::__private::embassy_futures::select::Either3::Third(mut $ident2) => { $handler2 }
        }
    };
    {
        $ident0:ident = $fut0:expr => $handler0:block
        $ident1:ident = $fut1:expr => $handler1:block
        $ident2:ident = $fut2:expr => $handler2:block
        $ident3:ident = $fut3:expr => $handler3:block
    } => {
        match $crate::__private::embassy_futures::select::select4($fut0, $fut1, $fut2, $fut3).await {
            $crate::__private::embassy_futures::select::Either4::First(mut $ident0) => { $handler0 }
            $crate::__private::embassy_futures::select::Either4::Second(mut $ident1) => { $handler1 }
            $crate::__private::embassy_futures::select::Either4::Third(mut $ident2) => { $handler2 }
            $crate::__private::embassy_futures::select::Either4::Fourth(mut $ident3) => { $handler3 }
        }
    };
    {$($ident:ident = $fut:expr => $handler:block)+} => {
        $crate::select!($(F as $ident = $fut => $handler)+; private)
    };
    {$($name:ident as $ident:ident = $fut:expr => $handler:block)+; private} => {{
        mod __select { $crate::__private::paste::paste! {
            #[derive(Debug, Clone)]
            pub enum Either< $([< $name:camel:upper ${ index() } >] , )+ > {
                $([< $name:camel ${ index() } >] ([< $name:camel:upper ${ index() } >]) , )+
            }

            #[derive(Debug)]
            pub struct Select< $([< $name:camel:upper ${ index() } >] , )+ > {
                $([< $name:lower:snake _ ${ index() } >] : [< $name:camel:upper ${ index() } >] , )+
            }

            impl< $([< $name:camel:upper ${ index() } >] , )+ > Select< $([< $name:camel:upper ${ index() } >] ,)+ > {
                #[inline]
                #[allow(clippy::too_many_arguments)]
                pub fn new($([< $name:lower:snake _ ${ index() } >] : [< $name:camel:upper ${ index() } >] , )+) -> Self {
                    Self { $([< $name:lower:snake _ ${ index() } >], )+ }
                }
            }

            use ::core::marker::Unpin;
            impl< $([< $name:upper:camel ${ index() } >] : Unpin , )+ > Unpin for Select< $([< $name:upper:camel ${ index() } >] , )+ > {}

            use ::core::future::Future;
            use ::core::pin::Pin;
            use ::core::task::{Context, Poll};
            impl< $([< $name:camel:upper ${ index() } >] , )+ > Future for Select< $([< $name:camel:upper ${ index() } >] , )+ >
            where
                $([< $name:camel:upper ${ index() } >] : Future , )+
            {
                type Output = Either< $([< $name:camel:upper ${ index() } >] ::Output, )+ >;

                fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                    let this = unsafe { self.get_unchecked_mut() };
                    $(
                        let [< $name:snake:lower _ ${ index() } >] = unsafe { Pin::new_unchecked(&mut this. [< $name:snake:lower _ ${ index() } >] ) };
                        if let Poll::Ready(x) = [< $name:snake:lower _ ${ index() } >] .poll(cx) {
                            return Poll::Ready(Either:: [< $name:camel ${ index() } >] (x));
                        }
                    )+
                    Poll::Pending
                }
            }
        }}

        $crate::__private::paste::paste! {
            match __select::Select::new($($fut,)+).await {
                $(__select::Either:: [< $name:camel ${ index() } >] (mut $ident) => { $handler } )+
            }
        }
    }};
}

#[macro_export]
macro_rules! join {
    (
        $ident0:tt $(=> $fut0:expr)?,
        $ident1:tt $(=> $fut1:expr)?
        $(, $ident2:tt $(=> $fut2:expr)?
            $(, $ident3:tt $(=> $fut3:expr)?
                $(, $ident4:tt $(=> $fut4:expr)?)?
            )?
        )? $(,)?
    ) => {
        $crate::join! (@parse 0
            $ident0 $(=> $fut0)?,
            $ident1 $(=> $fut1)?,
            $( $ident2 $(=> $fut2)?,
                $( $ident3 $(=> $fut3)?,
                    $( $ident4 $(=> $fut4)?,)?
                )?
            )?
        )
    };

    ($ident0:tt $(=> $fut0:expr)? $(, $ident:tt $(=> $fut:expr)?)+ $(,)?) => {
        $crate::join! (@parse 1 $ident0 $(=> $fut0)?, $( $ident $(=> $fut)?, )+ )
    };

    (@parse $n:tt { $($head:tt)* } |$index:expr => $name:ident ; ;  $($tail:tt)* ) => {
        $crate::join! { @parse $n { $($head)* { $name => $name } } $($tail)*}
    };
    (@parse $n:tt { $($head:tt)* } |$index:expr => $fut:expr ; ;  $($tail:tt)* ) => { $crate::__private::paste::paste! {
        $crate::join! { @parse $n { $($head)* { [< Fut $index >] => $fut } } $($tail)* }
    } };
    (@parse $n:tt { $($head:tt)* } |$index:expr => $name:ident ; $fut:expr ;  $($tail:tt)* ) => {
        $crate::join! { @parse $n { $($head)* { $name => $fut } } $($tail)* }
    };
    (@parse $n:tt { $($head:tt)+ } ) => { $crate::join! { @impl $n => $($head)+ } };
    (@parse $n:tt $($ident:tt $(=> $fut:expr)? ,)+) => { $crate::join! ( @parse $n {} $(| ${ index() } => $ident ; $($fut)? ; )+ ) };

    {@impl 0 => $({ $name:ident => $fut:expr })+} => { $crate::join! (@impl 0 => $($name,)+ ) ($($fut,)+) };
    {@impl 0 => $name0:ident, $name1:ident,} => { $crate::__private::embassy_futures::join::join };
    {@impl 0 => $name0:ident, $name1:ident, $name2:ident,} => { $crate::__private::embassy_futures::join::join3 };
    {@impl 0 => $name0:ident, $name1:ident, $name2:ident, $name3:ident,} => { $crate::__private::embassy_futures::join::join4 };
    {@impl 0 => $name0:ident, $name1:ident, $name2:ident, $name3:ident, $name4:ident,} => { $crate::__private::embassy_futures::join::join5 };

    {@impl 1 => $({ $name:ident => $fut:expr })+} => {{
        mod __join { $crate::__private::paste::paste! {
            pub struct Output< $([< $name:camel:upper ${ index() } >] , )+ > {$(
                pub [< $name:lower:snake >] : [< $name:camel:upper ${ index() } >] ,
            )+ }

            pub struct Join< $([< $name:camel:upper ${ index() } >] : Future , )+ >
            {
                $([< $name:lower:snake _ ${ index() } >] : $crate::execution::MaybeDone< [< $name:camel:upper ${ index() } >] >, )+
            }

            impl< $([< $name:camel:upper ${ index() } >] , )+ > ::core::fmt::Debug for Join< $([< $name:camel:upper ${ index() } >] ,)+ >
            where $(
                [< $name:camel:upper ${ index() } >] : Future + ::core::fmt::Debug,
                [< $name:camel:upper ${ index() } >] ::Output: ::core::fmt::Debug,
            )+
            {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    const COUNT: usize = $crate::macros::count!( $($name,)+ );
                    f.debug_struct($crate::__private::const_format::concatcp!("Join", COUNT))
                        $( .field(stringify!([< Fut ${ index() } >]), &self. [< $name:lower:snake _ ${ index() } >]))+
                        .finish()
                }
            }

            impl< $([< $name:camel:upper ${ index() } >] : Future , )+ > Join< $([< $name:camel:upper ${ index() } >] ,)+ > {
                #[inline]
                #[allow(clippy::too_many_arguments)]
                pub fn new($([< $name:lower:snake _ ${ index() } >] : [< $name:camel:upper ${ index() } >] , )+) -> Self {
                    Self { $([< $name:lower:snake _ ${ index() } >] : $crate::execution::MaybeDone::Future([< $name:lower:snake _ ${ index() } >]), )+ }
                }
            }

            use ::core::future::Future;
            use ::core::pin::Pin;
            use ::core::task::{Context, Poll};
            impl< $([< $name:camel:upper ${ index() } >] , )+ > Future for Join< $([< $name:camel:upper ${ index() } >] , )+ >
            where
                $([< $name:camel:upper ${ index() } >] : Future , )+
            {
                type Output = Output< $([< $name:camel:upper ${ index() } >] ::Output, )+ >;

                fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                    let this = unsafe { self.get_unchecked_mut() };
                    let mut all_done = true;
                    $(
                        all_done &= unsafe { Pin::new_unchecked(&mut this. [< $name:snake:lower _ ${ index() } >] ) }.poll(cx);
                    )+
                    if all_done {
                        Poll::Ready(Output { $(
                            [< $name:lower:snake >] : this.[< $name:lower:snake _ ${ index() } >].take_output() ,
                        )+ })
                    } else {
                        Poll::Pending
                    }
                }
            }
        }}

        $crate::__private::paste::paste! { __join::Join::new($($fut,)+) }
    }};
}

#[derive(Debug)]
pub enum MaybeDone<Fut: core::future::Future> {
    Future(Fut),
    Done(Fut::Output),
    Gone,
}

impl<Fut: core::future::Future> MaybeDone<Fut> {
    pub fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> bool {
        let this = unsafe { self.get_unchecked_mut() };
        match this {
            Self::Future(fut) => match unsafe { core::pin::Pin::new_unchecked(fut) }.poll(cx) {
                core::task::Poll::Ready(res) => {
                    *this = Self::Done(res);
                    true
                }
                core::task::Poll::Pending => false,
            },
            _ => true,
        }
    }

    pub fn take_output(&mut self) -> Fut::Output {
        match &*self {
            Self::Done(_) => {}
            Self::Future(_) | Self::Gone => panic!("take_output when MaybeDone is not done."),
        }
        match core::mem::replace(self, Self::Gone) {
            MaybeDone::Done(output) => output,
            _ => unreachable!(),
        }
    }
}

impl<Fut: core::future::Future + Unpin> Unpin for MaybeDone<Fut> {}
