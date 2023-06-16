use crate::{event::traits as __evt, pubsub as __pub, service::traits as __svc};

pub use __pub::traits as pubsub;

pub trait Notifier: Sized + 'static {
    const COUNT: usize;
    const COUNT_SERVICES: usize;
    fn get() -> &'static Self;
}

pub type NotifierServiceGetRet<'s, N, S> = &'s super::GetService<N, S>;
pub trait NotifierService<S: __svc::Service<Self>>: Notifier {
    const ID: usize;
    const NAME: &'static str;
    fn __get(&self) -> NotifierServiceGetRet<Self, S>
    where
        [(); S::COUNT]:;
}
pub trait NotifierServiceEvent<E>: NotifierService<E::Service>
where
    E: __evt::Event<Self, Service: __svc::Service<Self>>,
{
    const ID_COUNT: usize = Self::COUNT_CALC.0;
    const CHANNEL_COUNT: usize = Self::COUNT_CALC.1;

    const COUNT_CALC: crate::calc::CountID;
}

pub trait NotifierPublisher<E>: NotifierServiceEvent<E>
where
    E: __evt::Event<Self, Service: __svc::Service<Self, Impl: pubsub::Publisher<E>>>,
    [(); Self::ID_COUNT]:,
    [(); Self::COUNT_SERVICES]:,
{
    const IDS: &'static [usize] = &Self::ID_CALC.arr;
    const ID_CALC: crate::calc::CalcID<Self, E>;
}
