use crate::{event::traits as __evt, pubsub::traits as __pub, service::traits as __svc, traits};

pub(crate) struct CheckerID<N, E>
where
    N: traits::NotifierServiceEvent<E>,
    E: __evt::Event<N, Service: __svc::Service<N>>,
{
    pub(crate) checker: fn(usize) -> crate::SubscriberCheckerRet<N, E>,
    pub(crate) meta: &'static [crate::Metadata],
}

impl<N, E> CheckerID<N, E>
where
    N: traits::NotifierServiceEvent<E>,
    E: __evt::Event<N, Service: __svc::Service<N>>,
{
    const fn new() -> Self {
        fn dummy<R>(_: usize) -> R {
            unreachable!()
        }
        Self {
            checker: dummy,
            meta: &[],
        }
    }
}

pub struct CalcID<N, E>
where
    N: traits::NotifierServiceEvent<E>,
    E: __evt::Event<N, Service: __svc::Service<N>>,
    [(); N::ID_COUNT]:,
    [(); N::COUNT_SERVICES]:,
{
    pub(crate) index: usize,
    pub(crate) checkers_index: usize,
    pub(crate) arr: [usize; N::ID_COUNT],
    pub(crate) checkers: [CheckerID<N, E>; N::COUNT_SERVICES],
}

impl<N, E> CalcID<N, E>
where
    N: traits::NotifierServiceEvent<E>,
    E: __evt::Event<N, Service: __svc::Service<N, Impl: __pub::Publisher<E>>>,
    [(); N::ID_COUNT]:,
    [(); N::COUNT_SERVICES]:,
{
    pub const fn default() -> Self {
        Self {
            index: 0,
            checkers_index: 0,
            arr: [0; N::ID_COUNT],
            checkers: unsafe {
                let mut init = varuemb_utils::ArrayInitializer::new();
                while let Some((_, item)) = init.next() {
                    item.init(CheckerID::new())
                }
                init.finish()
            },
        }
    }

    pub const fn add<S>(mut self) -> CalcID<N, E>
    where
        [(); S::COUNT]:,
        N: traits::NotifierService<S> + traits::NotifierPublisher<E>,
        S: ~const __svc::ServiceMetadata<N> + 'static,
        crate::assert::Assert<{ crate::metadata::check::<N, S>() }>: crate::assert::True,
    {
        if crate::is_pubsub_impl::<S, N, E>() {
            self.arr[self.index] = crate::id::<_, S>();
            self.index += 1;
        }
        self.checkers[self.checkers_index] = CheckerID {
            checker: crate::subscriber_checker::<S, N, E> as _,
            meta: S::META,
        };
        self.checkers_index += 1;
        self
    }

    pub const fn verify(self) -> Self {
        if self.index != self.arr.len() || self.checkers_index != self.checkers.len() {
            panic!("Expected number of services is greater than added")
        }
        self
    }
}

#[derive(Clone, Copy)]
pub struct CountID(pub(crate) usize, pub(crate) usize);
impl CountID {
    pub const fn calc<S, N, E>(self) -> Self
    where
        N: traits::NotifierService<S>,
        S: __pub::IsSubscribed<N, E> + __svc::Service<N>,
    {
        let (a, b) = crate::is_pub_impl_and_count::<S, N, E>((self.0, self.1));
        Self(a, b)
    }

    pub const fn default() -> Self {
        Self(0, 0)
    }
}
