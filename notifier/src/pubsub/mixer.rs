use crate::{
    event,
    pubsub::{__evt, traits::*},
    traits::*,
};
use core::{future::pending, marker::PhantomData};
use std::mem::ManuallyDrop;

pub trait Mixer<N: Notifier>: core::fmt::Debug {}

pub struct MixCount<P, M>(usize, PhantomData<*const (P, M)>);
impl<P: PubSub, M: Mixer<P::Notifier>> const Default for MixCount<P, M> {
    fn default() -> Self {
        Self(0, Default::default())
    }
}
impl<P: PubSub, M> MixCount<P, M> {
    pub const fn calc<E>(mut self) -> Self
    where
        P: Subscriber<E>,
        E: IsPublisher<P>,
    {
        trait Helper<P, E, M> {
            const ADD: usize;
        }
        impl<P, E, M> Helper<P, E, M> for MixCount<P, M>
        where
            P: Subscriber<E>,
            E: IsPublisher<P>,
        {
            default const ADD: usize = 0;
        }
        impl<P, E, M> Helper<P, E, M> for MixCount<P, M>
        where
            E: IsPublisher<P>,
            P: MixMapper<M, E, Data = MixData<P, M, E>>,
            M: Mixer<P::Notifier>,
        {
            const ADD: usize = 1;
        }

        self.0 += <Self as Helper<P, E, M>>::ADD;
        self
    }
}

pub struct MixData<P, M, E>
where
    P: PubSub,
    E: __evt::Event<P::Notifier>,
{
    subscriber: super::Subscriber<P::Notifier, E>,
    mapper: fn(E) -> M,
}

pub trait FromPublisher<P: PubSub, M, E: __evt::Event<P::Notifier>> {
    fn from_publisher(p: &'static P) -> Self;
    fn wrap_data(data: &mut Self) -> Option<&mut MixData<P, M, E>>;
}
impl<P: PubSub, M, E: __evt::Event<P::Notifier>> FromPublisher<P, M, E> for () {
    fn from_publisher(_: &'static P) -> Self {}
    fn wrap_data(_: &mut Self) -> Option<&mut MixData<P, M, E>> {
        None
    }
}
impl<N: Notifier, P, M, E> FromPublisher<P, M, E> for MixData<P, M, E>
where
    E: __evt::Event<N> + IsPublisher<P>,
    M: Mixer<N>,
    P: Subscribed<E, Notifier = N> + MixMapper<M, E, Notifier = N>,
{
    fn from_publisher(publisher: &'static P) -> Self {
        Self {
            subscriber: Subscribed::<E>::subscriber(publisher),
            mapper: <P as MixMapper<M, E>>::MAPPER,
        }
    }
    fn wrap_data(data: &mut Self) -> Option<&mut MixData<P, M, E>> {
        Some(data)
    }
}

pub trait MixMapper<M: Mixer<Self::Notifier>, E: IsPublisher<Self>>: Subscriber<E> {
    type Data: FromPublisher<Self, M, E>;
    const MAPPER: fn(E) -> M;
}

impl<M, E, P> MixMapper<M, E> for P
where
    P: Subscriber<E>,
    E: IsPublisher<Self>,
    M: Mixer<Self::Notifier>,
{
    default type Data = ();
    default const MAPPER: fn(E) -> M = |_| unreachable!();
}

pub struct MixerMapper<P, M, E>
where
    E: IsPublisher<P>,
    M: Mixer<P::Notifier>,
    P: MixMapper<M, E>,
{
    data: P::Data,
    _mixer: PhantomData<*const M>,
}
const _: () = {
    union Mix<M, SM> {
        mix: ManuallyDrop<M>,
        self_mix: ManuallyDrop<SM>,
    }
    impl<N: Notifier, P, M, E> MixerMapper<P, M, E>
    where
        M: Mixer<N>,
        E: IsPublisher<P>,
        P: Subscribed<E, Notifier = N> + MixMapper<M, E, Notifier = N>,
    {
        pub fn new(publisher: &'static P) -> Self {
            Self {
                data: FromPublisher::from_publisher(publisher),
                _mixer: Default::default(),
            }
        }

        pub async fn map(&mut self) -> event::Event<N, M> {
            match P::Data::wrap_data(&mut self.data) {
                Some(data) => data.subscriber.next().await.map(data.mapper),
                None => pending().await,
            }
        }

        pub fn try_map(&mut self) -> Option<event::Event<N, M>> {
            let data = P::Data::wrap_data(&mut self.data)?;
            let event = data.subscriber.try_next()?;
            Some(event.map(data.mapper))
        }
    }
};

pub trait SubscriberMixer<M>: PubSub
where
    M: Mixer<Self::Notifier>,
{
    const COUNT: usize = Self::COUNT_CALC.0;
    const COUNT_CALC: MixCount<Self, M>;

    type Mixed;

    fn __new_mixed(&'static self) -> Self::Mixed;
    async fn __mixed(mixed: &mut Self::Mixed) -> event::Event<Self::Notifier, M>;
    fn __try_mixed(mixed: &mut Self::Mixed) -> Option<event::Event<Self::Notifier, M>>;
}
