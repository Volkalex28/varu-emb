#[macro_export]
macro_rules! select {
    {impl unwrap_ident { $($prev:tt)* }; #[cfg($cond:meta)] $ident:pat = $fut:expr => $handler:block $($rest:tt)* } => {
        $crate::select!{ impl unwrap_ident { $($prev)* #[cfg($cond)] F as $ident = $fut => $handler }; $($rest)* }
    };
    {impl unwrap_ident { $($prev:tt)* }; } => {
        $crate::select!{ impl final; $($prev)* }
    };

    {impl unwrap_cfg { $($prev:tt)* }; #[cfg($cond:meta)] $ident:pat = $fut:expr => $handler:block $($rest:tt)*} => {
        $crate::select!{ impl unwrap_cfg { $($prev)* #[cfg($cond)] $ident = $fut => $handler }; $($rest)* }
    };
    {impl unwrap_cfg { $($prev:tt)* }; $ident:pat = $fut:expr => $handler:block $($rest:tt)* } => {
        $crate::select!{ impl unwrap_cfg { $($prev)* #[cfg(not(any()))] $ident = $fut => $handler }; $($rest)* }
    };
    {impl unwrap_cfg { $($prev:tt)* }; } => {
        $crate::select!{ impl unwrap_ident {}; $($prev)* }
    };

    {impl final; $(#[cfg($cond:meta)] $name:ident as $ident:pat = $fut:expr => $handler:block)+ } => {{
        mod __select { $crate::__private::paste::paste! {
            #[derive(Debug, Clone)]
            pub enum Either< $(#[cfg($cond)] [< $name:camel:upper ${ index() } >] , )+ > {
                $(#[cfg($cond)] [< $name:camel ${ index() } >] ([< $name:camel:upper ${ index() } >]) , )+
            }

            #[derive(Debug)]
            pub struct Select< $(#[cfg($cond)] [< $name:camel:upper ${ index() } >] , )+ > {
                $(#[cfg($cond)] [< $name:lower:snake _ ${ index() } >] : [< $name:camel:upper ${ index() } >] , )+
            }


            #[$crate :: cfg_impl_block]
            $(#[cfg_attr($cond , bound([< $name:camel:upper ${ index() } >]))])+
            impl Select {
                #[inline]
                #[allow(clippy::too_many_arguments)]
                pub fn new($(#[cfg($cond)] [< $name:lower:snake _ ${ index() } >] : [< $name:camel:upper ${ index() } >] , )+) -> Self {
                    Self { $(#[cfg($cond)] [< $name:lower:snake _ ${ index() } >], )+ }
                }
            }

            use ::core::marker::Unpin;
            #[$crate :: cfg_impl_block]
            $(#[cfg_attr($cond , bound([< $name:camel:upper ${ index() } >] : Unpin))])+
            impl Unpin for Select {}

            use ::core::future::Future;
            use ::core::pin::Pin;
            use ::core::task::{Context, Poll};

            #[$crate :: cfg_impl_block]
            $(#[cfg_attr($cond , bound([< $name:camel:upper ${ index() } >] : Future))])+
            impl Future for Select {

                #[$crate :: cfg_type_alias]
                $(#[cfg_attr($cond , bound(= <[< $name:camel:upper ${ index() } >] as Future> :: Output))])+
                type Output = Either;

                fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                    let this = unsafe { self.get_unchecked_mut() };
                    $(
                        #[cfg($cond)]
                        let [< $name:snake:lower _ ${ index() } >] = unsafe { Pin::new_unchecked(&mut this. [< $name:snake:lower _ ${ index() } >] ) };
                        #[cfg($cond)]
                        if let Poll::Ready(x) = [< $name:snake:lower _ ${ index() } >] .poll(cx) {
                            return Poll::Ready(Either:: [< $name:camel ${ index() } >] (x));
                        }
                    )+
                    Poll::Pending
                }
            }
        }}

        $crate::__private::paste::paste! {
            match __select::Select::new($(#[cfg($cond)] $fut,)+).await {
                $(
                    #[cfg($cond)]
                    #[allow(unused_mut)]
                    __select::Either:: [< $name:camel ${ index() } >] ($ident) => { $handler }
                )+
            }
        }
    }};

    {$($(#[cfg($cond:meta)])? $ident:pat = $fut:expr => $handler:block)+} => {
        $crate::select!{ impl unwrap_cfg {}; $( $(#[cfg($cond)])? $ident = $fut => $handler )+ }
    };
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
